use chrono::{Datelike, Timelike};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use crate::engine::types::ExecutionConfig;
use crate::engine::{
    execution::run_setups_with_metrics,
    types::{EntryModel, SetupCandidate, StopModel, TargetModel, TrailingModel},
};
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::fee_config::FeeConfig;
use crate::model::position_direction::PositionDirection;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;
use std::sync::Arc;

/// TTrades Fractal Model - Multi-Timeframe Implementation
/// Higher timeframe for bias/structure, lower timeframe for entry
pub struct TTradesFractalMTF {
    pub htf_data: Arc<Vec<CandleStick>>, // Higher timeframe (1h, 4h)
    pub ltf_data: Arc<Vec<CandleStick>>, // Lower timeframe (5m, 15m)
    pub config: FractalMTFConfig,
}

#[derive(Clone)]
pub struct FractalMTFConfig {
    pub rr_target: Decimal,
    pub fee_config: FeeConfig,
    pub htf_name: &'static str, // For display: "4h", "1h"
    pub ltf_name: &'static str, // For display: "15m", "5m"
    pub slippage_ticks_per_side: i32,
    pub tick_size: Decimal,
    pub log_progress: bool,
    pub entry_variant: EntryVariant,
    pub cisd_variant: CisdVariant,
    pub use_ifvg_filter: bool,
    pub ifvg_lookback: usize,
    pub reversal_confirm_mode: ReversalConfirmMode,
    pub weekday_mask: u8,
    pub killzone_mode: KillzoneMode,
    pub poi_padding_bps: i32,
    pub ob_sweep_tolerance_bps: i32,
    pub failure_swing_lookback_bars: usize,
    pub failure_swing_breakout_close_only: bool,
    pub failure_swing_retest_tolerance_bps: i32,
    pub failure_swing_min_reclaim_ratio_bps: i32,
    pub stop_buffer_bps: i32,
    pub htf_bias_strict: bool,
    pub require_htf_fvg: bool,
    pub require_killzone_level_hit: bool,
    pub killzone_level_hit_lookback_bars: usize,
    pub killzone_reclaim_min_sweep_ticks: i32,
}

#[derive(Clone, Copy, Debug)]
pub enum EntryVariant {
    Close,
    ObLevel,
    ObMidpoint,
}

#[derive(Clone, Copy, Debug)]
pub enum CisdVariant {
    BodyFlip,
    StrictWickBreak,
    LastSeriesCloseBreak,
    FailureSwing,
    KillzoneReclaim,
    ContinuationBreak,
}

#[derive(Clone, Copy)]
pub enum ReversalConfirmMode {
    CisdOnly,
    IfvgOnly,
    CisdAndIfvg,
    CisdOrIfvg,
}

#[derive(Clone, Copy)]
pub enum KillzoneMode {
    Off,
    NyOnly,
    LondonNy,
    NamedSessions,
}

impl Default for FractalMTFConfig {
    fn default() -> Self {
        Self {
            rr_target: Decimal::from(2),
            fee_config: FeeConfig::default(),
            htf_name: "4h",
            ltf_name: "15m",
            slippage_ticks_per_side: 0,
            tick_size: Decimal::new(1, 2),
            log_progress: false,
            entry_variant: EntryVariant::Close,
            cisd_variant: CisdVariant::BodyFlip,
            use_ifvg_filter: false,
            ifvg_lookback: 64,
            reversal_confirm_mode: ReversalConfirmMode::CisdOnly,
            weekday_mask: 0b0111_1111,
            killzone_mode: KillzoneMode::Off,
            poi_padding_bps: 0,
            ob_sweep_tolerance_bps: 0,
            failure_swing_lookback_bars: 24,
            failure_swing_breakout_close_only: false,
            failure_swing_retest_tolerance_bps: 25,
            failure_swing_min_reclaim_ratio_bps: 5000,
            stop_buffer_bps: 5,
            htf_bias_strict: false,
            require_htf_fvg: false,
            require_killzone_level_hit: false,
            killzone_level_hit_lookback_bars: 12,
            killzone_reclaim_min_sweep_ticks: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KzSession {
    Asia,
    London,
    NyAm,
    NyPm,
}

#[derive(Clone, Copy, Debug)]
struct SessionLevel {
    session: KzSession,
    start_ts: i64,
    end_ts: i64,
    high: DecimalVec,
    low: DecimalVec,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HTFBias {
    Bullish,
    Bearish,
    None,
}

#[derive(Debug, Clone)]
struct HTFSetup {
    bias: HTFBias,
    poi_high: DecimalVec, // Point of interest high
    poi_low: DecimalVec,  // Point of interest low
    swing_high: DecimalVec,
    swing_low: DecimalVec,
    start_time: i64,
    used: bool,
}

#[derive(Debug, Clone)]
struct FVG {
    high: DecimalVec,
    low: DecimalVec,
    direction: PositionDirection,
}

impl TTradesFractalMTF {
    fn session_for_ts(&self, ts: i64) -> Option<KzSession> {
        let ny = to_new_york_time(ts);
        let hm = (ny.hour(), ny.minute());
        if hm >= (20, 0) {
            Some(KzSession::Asia)
        } else if hm >= (2, 0) && hm < (5, 0) {
            Some(KzSession::London)
        } else if hm >= (9, 15) && hm < (12, 0) {
            Some(KzSession::NyAm)
        } else if hm >= (14, 0) && hm < (16, 30) {
            Some(KzSession::NyPm)
        } else {
            None
        }
    }

    fn session_level_allowed_for_entry(entry_session: KzSession, level_session: KzSession) -> bool {
        match entry_session {
            KzSession::Asia => matches!(level_session, KzSession::NyPm),
            KzSession::London => matches!(level_session, KzSession::Asia),
            KzSession::NyAm => matches!(level_session, KzSession::Asia | KzSession::London),
            KzSession::NyPm => matches!(
                level_session,
                KzSession::Asia | KzSession::London | KzSession::NyAm
            ),
        }
    }

    fn build_session_levels(&self) -> Vec<SessionLevel> {
        let mut out = Vec::new();
        let mut active_session: Option<KzSession> = None;
        let mut session_start_ts = 0i64;
        let mut session_high = DecimalVec(Decimal::ZERO);
        let mut session_low = DecimalVec(Decimal::ZERO);
        let mut session_end_ts = 0i64;

        for candle in self.ltf_data.iter().copied() {
            let current_session = self.session_for_ts(candle.open_time);
            match (active_session, current_session) {
                (Some(active), Some(current)) if active == current => {
                    if candle.high > session_high {
                        session_high = candle.high;
                    }
                    if candle.low < session_low {
                        session_low = candle.low;
                    }
                    session_end_ts = candle.close_time;
                }
                (Some(active), Some(current)) => {
                    out.push(SessionLevel {
                        session: active,
                        start_ts: session_start_ts,
                        end_ts: session_end_ts,
                        high: session_high,
                        low: session_low,
                    });
                    active_session = Some(current);
                    session_start_ts = candle.open_time;
                    session_end_ts = candle.close_time;
                    session_high = candle.high;
                    session_low = candle.low;
                }
                (Some(active), None) => {
                    out.push(SessionLevel {
                        session: active,
                        start_ts: session_start_ts,
                        end_ts: session_end_ts,
                        high: session_high,
                        low: session_low,
                    });
                    active_session = None;
                }
                (None, Some(current)) => {
                    active_session = Some(current);
                    session_start_ts = candle.open_time;
                    session_end_ts = candle.close_time;
                    session_high = candle.high;
                    session_low = candle.low;
                }
                (None, None) => {}
            }
        }

        if let Some(active) = active_session {
            out.push(SessionLevel {
                session: active,
                start_ts: session_start_ts,
                end_ts: session_end_ts,
                high: session_high,
                low: session_low,
            });
        }

        out
    }

    fn has_recent_killzone_level_hit(
        &self,
        ltf_index: usize,
        direction: PositionDirection,
        levels: &[SessionLevel],
    ) -> bool {
        let ts = self.ltf_data[ltf_index].open_time;
        let Some(entry_session) = self.session_for_ts(ts) else {
            return false;
        };
        let start = ltf_index.saturating_sub(self.config.killzone_level_hit_lookback_bars);

        for level in levels {
            if level.end_ts >= ts {
                continue;
            }
            if !Self::session_level_allowed_for_entry(entry_session, level.session) {
                continue;
            }
            for i in start..=ltf_index {
                let c = self.ltf_data[i];
                match direction {
                    PositionDirection::Long => {
                        if c.low <= level.low {
                            return true;
                        }
                    }
                    PositionDirection::Short => {
                        if c.high >= level.high {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn detect_killzone_reclaim(
        &self,
        ltf_index: usize,
        direction: PositionDirection,
        levels: &[SessionLevel],
    ) -> bool {
        let current = self.ltf_data[ltf_index];
        let ts = current.open_time;
        let Some(entry_session) = self.session_for_ts(ts) else {
            return false;
        };
        let min_sweep = self.config.tick_size
            * Decimal::from(self.config.killzone_reclaim_min_sweep_ticks.max(0));

        for level in levels {
            if level.end_ts >= ts {
                continue;
            }
            if !Self::session_level_allowed_for_entry(entry_session, level.session) {
                continue;
            }
            match direction {
                PositionDirection::Long => {
                    if current.low <= DecimalVec(level.low.0 - min_sweep) && current.close > level.low
                    {
                        return true;
                    }
                }
                PositionDirection::Short => {
                    if current.high >= DecimalVec(level.high.0 + min_sweep)
                        && current.close < level.high
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn in_killzone(&self, ts: i64) -> bool {
        let ny = to_new_york_time(ts);
        let h = ny.hour() as i32;
        match self.config.killzone_mode {
            KillzoneMode::Off => true,
            KillzoneMode::NyOnly => (8..=11).contains(&h),
            KillzoneMode::LondonNy => (2..=5).contains(&h) || (8..=11).contains(&h),
            KillzoneMode::NamedSessions => self.session_for_ts(ts).is_some(),
        }
    }

    fn in_weekday(&self, ts: i64) -> bool {
        let ny = to_new_york_time(ts);
        let d = ny.weekday().num_days_from_monday() as u8;
        (self.config.weekday_mask & (1u8 << d)) != 0
    }

    fn passes_time_filters(&self, ts: i64) -> bool {
        self.in_weekday(ts) && self.in_killzone(ts)
    }

    fn has_ifvg_confirmation(
        &self,
        ltf_index: usize,
        expected_direction: PositionDirection,
    ) -> bool {
        if ltf_index < 3 {
            return false;
        }
        let start = ltf_index.saturating_sub(self.config.ifvg_lookback.max(3));
        let current = self.ltf_data[ltf_index];
        let previous = self.ltf_data[ltf_index - 1];

        match expected_direction {
            PositionDirection::Long => {
                for i in start..ltf_index.saturating_sub(1) {
                    if i + 2 >= self.ltf_data.len() {
                        break;
                    }
                    let c1 = self.ltf_data[i];
                    let c3 = self.ltf_data[i + 2];
                    if c3.high < c1.low {
                        let gap_high = c1.low;
                        if previous.close <= gap_high && current.close > gap_high {
                            return true;
                        }
                    }
                }
                false
            }
            PositionDirection::Short => {
                for i in start..ltf_index.saturating_sub(1) {
                    if i + 2 >= self.ltf_data.len() {
                        break;
                    }
                    let c1 = self.ltf_data[i];
                    let c3 = self.ltf_data[i + 2];
                    if c3.low > c1.high {
                        let gap_low = c1.high;
                        if previous.close >= gap_low && current.close < gap_low {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    fn reversal_confirmed(&self, cisd_match: bool, ifvg_match: bool) -> bool {
        match self.config.reversal_confirm_mode {
            ReversalConfirmMode::CisdOnly => cisd_match,
            ReversalConfirmMode::IfvgOnly => ifvg_match,
            ReversalConfirmMode::CisdAndIfvg => cisd_match && ifvg_match,
            ReversalConfirmMode::CisdOrIfvg => cisd_match || ifvg_match,
        }
    }

    fn select_entry(&self, ltf_close: DecimalVec, ob_level: DecimalVec) -> DecimalVec {
        match self.config.entry_variant {
            EntryVariant::Close => ltf_close,
            EntryVariant::ObLevel => ob_level,
            EntryVariant::ObMidpoint => DecimalVec((ltf_close.0 + ob_level.0) / Decimal::from(2)),
        }
    }

    fn is_limit_variant(&self) -> bool {
        !matches!(self.config.entry_variant, EntryVariant::Close)
    }

    fn entry_model(&self, entry: DecimalVec) -> EntryModel {
        match self.config.entry_variant {
            EntryVariant::Close => EntryModel::NextBarOpen,
            EntryVariant::ObLevel | EntryVariant::ObMidpoint => EntryModel::LimitTouch {
                price: entry,
                expiry_bars: 24,
            },
        }
    }

    fn execution_config(&self) -> ExecutionConfig {
        let hundred = Decimal::from(100);
        ExecutionConfig {
            commission_rate_per_side: self.config.fee_config.maker_fee_pct / hundred,
            fee_rate_per_side: self.config.fee_config.taker_fee_pct / hundred,
            slippage_ticks_per_side: self.config.slippage_ticks_per_side,
            tick_size: self.config.tick_size,
        }
    }

    /// Determine daily bias from HTF
    /// Simplified: look at recent HTF candles for trend direction
    fn determine_htf_bias(&self, htf_index: usize) -> HTFBias {
        if htf_index < 3 {
            return HTFBias::None;
        }

        let current = self.htf_data[htf_index];
        let prev1 = self.htf_data[htf_index - 1];
        let prev2 = self.htf_data[htf_index - 2];
        let prev3 = self.htf_data[htf_index - 3];

        // Check for higher highs and higher lows (bullish)
        let hh = current.high > prev1.high && prev1.high > prev2.high;
        let hl = current.low > prev1.low && prev1.low > prev2.low;

        // Check for lower highs and lower lows (bearish)
        let lh = current.high < prev1.high && prev1.high < prev2.high;
        let ll = current.low < prev1.low && prev1.low < prev2.low;

        if hh && hl {
            HTFBias::Bullish
        } else if lh && ll {
            HTFBias::Bearish
        } else if self.config.htf_bias_strict {
            HTFBias::None
        } else {
            // Check close position relative to previous candles
            let bullish_closes = (current.close > current.open) as i32
                + (prev1.close > prev1.open) as i32
                + (prev2.close > prev2.open) as i32;

            if bullish_closes >= 2 {
                HTFBias::Bullish
            } else if bullish_closes <= 1 {
                HTFBias::Bearish
            } else {
                HTFBias::None
            }
        }
    }

    /// Find swing high in recent HTF candles (3-bar local reversal: prev < curr > next)
    fn find_htf_swing_high(&self, start: usize, end: usize) -> DecimalVec {
        let end = end.min(self.htf_data.len() - 1);
        // Walk backward to find most recent swing high
        if end >= start + 1 {
            for i in (start + 1..end).rev() {
                let prev = self.htf_data[i - 1];
                let curr = self.htf_data[i];
                let next = self.htf_data[i + 1];
                if curr.high > prev.high && curr.high > next.high {
                    return curr.high;
                }
            }
        }
        // Fallback: range maximum
        let mut highest = self.htf_data[start].high;
        for i in start..=end {
            if self.htf_data[i].high > highest {
                highest = self.htf_data[i].high;
            }
        }
        highest
    }

    /// Find swing low in recent HTF candles (3-bar local reversal: prev > curr < next)
    fn find_htf_swing_low(&self, start: usize, end: usize) -> DecimalVec {
        let end = end.min(self.htf_data.len() - 1);
        // Walk backward to find most recent swing low
        if end >= start + 1 {
            for i in (start + 1..end).rev() {
                let prev = self.htf_data[i - 1];
                let curr = self.htf_data[i];
                let next = self.htf_data[i + 1];
                if curr.low < prev.low && curr.low < next.low {
                    return curr.low;
                }
            }
        }
        // Fallback: range minimum
        let mut lowest = self.htf_data[start].low;
        for i in start..=end {
            if self.htf_data[i].low < lowest {
                lowest = self.htf_data[i].low;
            }
        }
        lowest
    }

    /// Detect FVG on HTF
    fn detect_htf_fvg(&self, htf_index: usize) -> Option<FVG> {
        if htf_index < 2 {
            return None;
        }

        let candle1 = self.htf_data[htf_index - 2];
        let candle3 = self.htf_data[htf_index];

        // Bullish FVG: gap up
        if candle3.low > candle1.high {
            return Some(FVG {
                high: candle3.low,
                low: candle1.high,
                direction: PositionDirection::Long,
            });
        }

        // Bearish FVG: gap down
        if candle3.high < candle1.low {
            return Some(FVG {
                high: candle1.low,
                low: candle3.high,
                direction: PositionDirection::Short,
            });
        }

        None
    }

    /// Create HTF setup when conditions are met
    fn create_htf_setup(&self, htf_index: usize) -> Option<HTFSetup> {
        let bias = self.determine_htf_bias(htf_index);

        if bias == HTFBias::None {
            return None;
        }

        let current = self.htf_data[htf_index];
        let lookback = 10.min(htf_index);
        let start = htf_index.saturating_sub(lookback);

        let swing_high = self.find_htf_swing_high(start, htf_index);
        let swing_low = self.find_htf_swing_low(start, htf_index);

        // Find FVG or use swing points as POI
        let fvg = self.detect_htf_fvg(htf_index);
        let (poi_high, poi_low) = if let Some(fvg) = fvg {
            (fvg.high, fvg.low)
        } else {
            if self.config.require_htf_fvg {
                return None;
            }
            match bias {
                HTFBias::Bullish => (
                    swing_low,
                    swing_low - DecimalVec(swing_low.0 * Decimal::from_f32(0.0025).unwrap()),
                ),
                HTFBias::Bearish => (
                    swing_high + DecimalVec(swing_high.0 * Decimal::from_f32(0.0025).unwrap()),
                    swing_high,
                ),
                HTFBias::None => return None,
            }
        };

        Some(HTFSetup {
            bias,
            poi_high,
            poi_low,
            swing_high,
            swing_low,
            start_time: current.open_time,
            used: false,
        })
    }

    /// Check if LTF price is in HTF POI zone
    fn is_in_poi(&self, low: DecimalVec, high: DecimalVec, setup: &HTFSetup) -> bool {
        let bps = Decimal::from(self.config.poi_padding_bps);
        let pad = bps / Decimal::from(10_000);
        let pad_abs = setup.poi_high.0 * pad;
        let padded_low = DecimalVec(setup.poi_low.0 - pad_abs);
        let padded_high = DecimalVec(setup.poi_high.0 + pad_abs);
        high >= padded_low && low <= padded_high
    }

    /// Detect CISD (Change in State of Delivery) on LTF
    fn detect_ltf_cisd(&self, ltf_index: usize, expected_direction: PositionDirection) -> bool {
        if ltf_index < 3 {
            return false;
        }

        let current = self.ltf_data[ltf_index];
        let prev1 = self.ltf_data[ltf_index - 1];
        let prev2 = self.ltf_data[ltf_index - 2];
        let prev3 = self.ltf_data[ltf_index - 3];

        match self.config.cisd_variant {
            CisdVariant::BodyFlip => match expected_direction {
                PositionDirection::Long => {
                    let bearish_count = (prev3.close < prev3.open) as i32
                        + (prev2.close < prev2.open) as i32
                        + (prev1.close < prev1.open) as i32;
                    bearish_count >= 2 && current.close > current.open
                }
                PositionDirection::Short => {
                    let bullish_count = (prev3.close > prev3.open) as i32
                        + (prev2.close > prev2.open) as i32
                        + (prev1.close > prev1.open) as i32;
                    bullish_count >= 2 && current.close < current.open
                }
            },
            CisdVariant::StrictWickBreak => match expected_direction {
                PositionDirection::Long => {
                    let series = [prev3, prev2, prev1];
                    let max_down_close = series
                        .iter()
                        .filter(|c| c.close < c.open)
                        .map(|c| c.close)
                        .max_by(|a, b| a.0.cmp(&b.0));
                    if let Some(level) = max_down_close {
                        current.high > level
                    } else {
                        false
                    }
                }
                PositionDirection::Short => {
                    let series = [prev3, prev2, prev1];
                    let min_up_close = series
                        .iter()
                        .filter(|c| c.close > c.open)
                        .map(|c| c.close)
                        .min_by(|a, b| a.0.cmp(&b.0));
                    if let Some(level) = min_up_close {
                        current.low < level
                    } else {
                        false
                    }
                }
            },
            CisdVariant::LastSeriesCloseBreak => match expected_direction {
                PositionDirection::Long => {
                    let series = [prev3, prev2, prev1];
                    let last_down_close = series
                        .iter()
                        .rev()
                        .find(|c| c.close < c.open)
                        .map(|c| c.close);
                    if let Some(level) = last_down_close {
                        current.close > level
                    } else {
                        false
                    }
                }
                PositionDirection::Short => {
                    let series = [prev3, prev2, prev1];
                    let last_up_close = series
                        .iter()
                        .rev()
                        .find(|c| c.close > c.open)
                        .map(|c| c.close);
                    if let Some(level) = last_up_close {
                        current.close < level
                    } else {
                        false
                    }
                }
            },
            CisdVariant::FailureSwing => {
                let scan_start = ltf_index.saturating_sub(self.config.failure_swing_lookback_bars);
                if ltf_index.saturating_sub(scan_start) < 7 {
                    return false;
                }

                let retest_tol = Decimal::from(self.config.failure_swing_retest_tolerance_bps)
                    / Decimal::from(10_000);
                let min_reclaim_ratio =
                    Decimal::from(self.config.failure_swing_min_reclaim_ratio_bps)
                        / Decimal::from(10_000);

                let mut swing_high_idx: Option<usize> = None;
                let mut swing_low_idx: Option<usize> = None;
                for i in ((scan_start + 1)..ltf_index.saturating_sub(1)).rev() {
                    let prev = self.ltf_data[i - 1];
                    let curr = self.ltf_data[i];
                    let next = self.ltf_data[i + 1];
                    if swing_high_idx.is_none() && curr.high > prev.high && curr.high > next.high {
                        swing_high_idx = Some(i);
                    }
                    if swing_low_idx.is_none() && curr.low < prev.low && curr.low < next.low {
                        swing_low_idx = Some(i);
                    }
                    if swing_high_idx.is_some() && swing_low_idx.is_some() {
                        break;
                    }
                }

                match expected_direction {
                    PositionDirection::Short => {
                        let Some(h1_idx) = swing_high_idx else {
                            return false;
                        };

                        let mut l1_idx: Option<usize> = None;
                        for i in (h1_idx + 1)..ltf_index.saturating_sub(1) {
                            let prev = self.ltf_data[i - 1];
                            let curr = self.ltf_data[i];
                            let next = self.ltf_data[i + 1];
                            if curr.low < prev.low && curr.low < next.low {
                                l1_idx = Some(i);
                                break;
                            }
                        }
                        let Some(l1_idx) = l1_idx else {
                            return false;
                        };

                        let h1 = self.ltf_data[h1_idx];
                        let l1 = self.ltf_data[l1_idx];
                        let swing_range = h1.high.0 - l1.low.0;
                        if swing_range <= Decimal::ZERO {
                            return false;
                        }

                        let mut h2_idx: Option<usize> = None;
                        let retest_floor = h1.high.0 * (Decimal::ONE - retest_tol);
                        for i in (l1_idx + 1)..ltf_index.saturating_sub(1) {
                            let prev = self.ltf_data[i - 1];
                            let curr = self.ltf_data[i];
                            let next = self.ltf_data[i + 1];
                            let reclaim = (curr.high.0 - l1.low.0) / swing_range;
                            if curr.high > prev.high
                                && curr.high > next.high
                                && curr.high < h1.high
                                && curr.high.0 >= retest_floor
                                && reclaim >= min_reclaim_ratio
                            {
                                h2_idx = Some(i);
                            }
                        }
                        let Some(h2_idx) = h2_idx else {
                            return false;
                        };

                        let clean_break =
                            ((h2_idx + 1)..ltf_index).all(|i| self.ltf_data[i].low >= l1.low);
                        if !clean_break {
                            return false;
                        }

                        let broke = if self.config.failure_swing_breakout_close_only {
                            current.close < l1.low
                        } else {
                            current.low < l1.low || current.close < l1.low
                        };

                        broke
                    }
                    PositionDirection::Long => {
                        let Some(l1_idx) = swing_low_idx else {
                            return false;
                        };

                        let mut h1_idx: Option<usize> = None;
                        for i in (l1_idx + 1)..ltf_index.saturating_sub(1) {
                            let prev = self.ltf_data[i - 1];
                            let curr = self.ltf_data[i];
                            let next = self.ltf_data[i + 1];
                            if curr.high > prev.high && curr.high > next.high {
                                h1_idx = Some(i);
                                break;
                            }
                        }
                        let Some(h1_idx) = h1_idx else {
                            return false;
                        };

                        let l1 = self.ltf_data[l1_idx];
                        let h1 = self.ltf_data[h1_idx];
                        let swing_range = h1.high.0 - l1.low.0;
                        if swing_range <= Decimal::ZERO {
                            return false;
                        }

                        let mut l2_idx: Option<usize> = None;
                        let retest_ceiling = l1.low.0 * (Decimal::ONE + retest_tol);
                        for i in (h1_idx + 1)..ltf_index.saturating_sub(1) {
                            let prev = self.ltf_data[i - 1];
                            let curr = self.ltf_data[i];
                            let next = self.ltf_data[i + 1];
                            let reclaim = (h1.high.0 - curr.low.0) / swing_range;
                            if curr.low < prev.low
                                && curr.low < next.low
                                && curr.low > l1.low
                                && curr.low.0 <= retest_ceiling
                                && reclaim >= min_reclaim_ratio
                            {
                                l2_idx = Some(i);
                            }
                        }
                        let Some(l2_idx) = l2_idx else {
                            return false;
                        };

                        let clean_break =
                            ((l2_idx + 1)..ltf_index).all(|i| self.ltf_data[i].high <= h1.high);
                        if !clean_break {
                            return false;
                        }

                        let broke = if self.config.failure_swing_breakout_close_only {
                            current.close > h1.high
                        } else {
                            current.high > h1.high || current.close > h1.high
                        };

                        broke
                    }
                }
            }
            CisdVariant::KillzoneReclaim => false,
            CisdVariant::ContinuationBreak => match expected_direction {
                PositionDirection::Long => {
                    prev2.close > prev2.open
                        && prev1.close < prev1.open
                        && current.close > current.open
                        && current.close > prev1.high
                }
                PositionDirection::Short => {
                    prev2.close < prev2.open
                        && prev1.close > prev1.open
                        && current.close < current.open
                        && current.close < prev1.low
                }
            },
        }
    }

    /// Detect continuation order block on LTF
    fn detect_ltf_order_block(
        &self,
        ltf_index: usize,
        direction: PositionDirection,
    ) -> Option<DecimalVec> {
        if ltf_index < 5 {
            return None;
        }

        let current = self.ltf_data[ltf_index];

        // Look back for recent swing point
        let lookback = 10.min(ltf_index);
        let tol = Decimal::from(self.config.ob_sweep_tolerance_bps) / Decimal::from(10_000);

        match direction {
            PositionDirection::Long => {
                // Find recent swing low
                let mut swing_low = self.ltf_data[ltf_index - 1].low;
                for i in (ltf_index.saturating_sub(lookback))..ltf_index {
                    if self.ltf_data[i].low < swing_low {
                        swing_low = self.ltf_data[i].low;
                    }
                }

                // Check if current candle swept below and closed bullish
                let max_allowed_low = DecimalVec(swing_low.0 + swing_low.0 * tol);
                if current.low <= max_allowed_low && current.close > current.open {
                    return Some(swing_low);
                }
            }
            PositionDirection::Short => {
                // Find recent swing high
                let mut swing_high = self.ltf_data[ltf_index - 1].high;
                for i in (ltf_index.saturating_sub(lookback))..ltf_index {
                    if self.ltf_data[i].high > swing_high {
                        swing_high = self.ltf_data[i].high;
                    }
                }

                // Check if current candle swept above and closed bearish
                let min_allowed_high = DecimalVec(swing_high.0 - swing_high.0 * tol);
                if current.high >= min_allowed_high && current.close < current.open {
                    return Some(swing_high);
                }
            }
        }

        None
    }

    /// Find LTF index that corresponds to HTF time
    fn find_ltf_index_for_htf(&self, htf_time: i64) -> usize {
        for (i, candle) in self.ltf_data.iter().enumerate() {
            if candle.open_time >= htf_time {
                return i;
            }
        }
        self.ltf_data.len() - 1
    }

    /// Get next HTF candle close time
    fn get_next_htf_time(&self, htf_index: usize) -> i64 {
        if htf_index + 1 < self.htf_data.len() {
            self.htf_data[htf_index + 1].open_time
        } else {
            i64::MAX
        }
    }

    pub fn detect_setups(&self) -> Vec<SetupCandidate> {
        let mut setups = Vec::new();
        let session_levels = if self.config.require_killzone_level_hit {
            self.build_session_levels()
        } else {
            Vec::new()
        };
        let mut current_setup: Option<HTFSetup> = None;
        let mut poi_hits = 0;
        let mut cisd_matches = 0;
        let mut order_blocks = 0;

        let mut htf_index = 0;
        let mut ltf_index = 0;
        if self.config.log_progress {
            println!("Starting TTrades Fractal MTF setup detection:");
            println!("  HTF: {} candles", self.htf_data.len());
            println!("  LTF: {} candles", self.ltf_data.len());
        }

        while htf_index < self.htf_data.len() && ltf_index < self.ltf_data.len() {
            let next_htf_time = self.get_next_htf_time(htf_index);

            // Update HTF setup using the *previous* (completed) HTF candle to avoid look-ahead bias
            if htf_index > 0 {
                if let Some(setup) = self.create_htf_setup(htf_index - 1) {
                    current_setup = Some(setup);
                }
            }

            // Find LTF candles within this HTF candle's time range
            while ltf_index < self.ltf_data.len()
                && self.ltf_data[ltf_index].open_time < next_htf_time
            {
                let ltf_candle = self.ltf_data[ltf_index];

                if let Some(setup) = current_setup.clone() {
                    if !self.passes_time_filters(ltf_candle.open_time) {
                        ltf_index += 1;
                        continue;
                    }
                    if self.is_in_poi(ltf_candle.low, ltf_candle.high, &setup) {
                        poi_hits += 1;

                        match setup.bias {
                            HTFBias::Bullish => {
                                let trigger_ok = match self.config.cisd_variant {
                                    CisdVariant::KillzoneReclaim => self.detect_killzone_reclaim(
                                        ltf_index,
                                        PositionDirection::Long,
                                        &session_levels,
                                    ),
                                    _ => {
                                        if self.config.require_killzone_level_hit
                                            && !self.has_recent_killzone_level_hit(
                                                ltf_index,
                                                PositionDirection::Long,
                                                &session_levels,
                                            )
                                        {
                                            false
                                        } else {
                                            let cisd_match = self.detect_ltf_cisd(
                                                ltf_index,
                                                PositionDirection::Long,
                                            );
                                            let ifvg_match = self.has_ifvg_confirmation(
                                                ltf_index,
                                                PositionDirection::Long,
                                            );
                                            self.reversal_confirmed(cisd_match, ifvg_match)
                                        }
                                    }
                                };
                                if trigger_ok {
                                    cisd_matches += 1;
                                    if let Some(ob_level) = self
                                        .detect_ltf_order_block(ltf_index, PositionDirection::Long)
                                    {
                                        order_blocks += 1;
                                        let entry = self.select_entry(ltf_candle.close, ob_level);
                                        let stop_buffer =
                                            Decimal::from(self.config.stop_buffer_bps)
                                                / Decimal::from(10_000);
                                        let sl = ob_level - DecimalVec(ob_level.0 * stop_buffer);
                                        if entry > sl {
                                            setups.push(SetupCandidate {
                                                direction: PositionDirection::Long,
                                                signal_index: ltf_index,
                                                entry: self.entry_model(entry),
                                                stop: StopModel::FixedPrice(sl),
                                                target: TargetModel::FixedR(self.config.rr_target),
                                                trailing: TrailingModel::None,
                                                max_hold_bars: None,
                                            });
                                        }
                                    }
                                }
                            }
                            HTFBias::Bearish => {
                                let trigger_ok = match self.config.cisd_variant {
                                    CisdVariant::KillzoneReclaim => self.detect_killzone_reclaim(
                                        ltf_index,
                                        PositionDirection::Short,
                                        &session_levels,
                                    ),
                                    _ => {
                                        if self.config.require_killzone_level_hit
                                            && !self.has_recent_killzone_level_hit(
                                                ltf_index,
                                                PositionDirection::Short,
                                                &session_levels,
                                            )
                                        {
                                            false
                                        } else {
                                            let cisd_match = self.detect_ltf_cisd(
                                                ltf_index,
                                                PositionDirection::Short,
                                            );
                                            let ifvg_match = self.has_ifvg_confirmation(
                                                ltf_index,
                                                PositionDirection::Short,
                                            );
                                            self.reversal_confirmed(cisd_match, ifvg_match)
                                        }
                                    }
                                };
                                if trigger_ok {
                                    cisd_matches += 1;
                                    if let Some(ob_level) = self
                                        .detect_ltf_order_block(ltf_index, PositionDirection::Short)
                                    {
                                        order_blocks += 1;
                                        let entry = self.select_entry(ltf_candle.close, ob_level);
                                        let stop_buffer =
                                            Decimal::from(self.config.stop_buffer_bps)
                                                / Decimal::from(10_000);
                                        let sl = ob_level + DecimalVec(ob_level.0 * stop_buffer);
                                        if sl > entry {
                                            setups.push(SetupCandidate {
                                                direction: PositionDirection::Short,
                                                signal_index: ltf_index,
                                                entry: self.entry_model(entry),
                                                stop: StopModel::FixedPrice(sl),
                                                target: TargetModel::FixedR(self.config.rr_target),
                                                trailing: TrailingModel::None,
                                                max_hold_bars: None,
                                            });
                                        }
                                    }
                                }
                            }
                            HTFBias::None => {}
                        }
                    }
                }

                ltf_index += 1;
            }

            htf_index += 1;
        }

        if self.config.log_progress {
            println!("  POI hits: {}", poi_hits);
            println!("  CISD matches: {}", cisd_matches);
            println!("  Order blocks: {}", order_blocks);
        }

        setups
    }
}

impl TradingModel for TTradesFractalMTF {
    fn execute(&self) -> BacktestResult {
        let setups = self.detect_setups();
        let (trades, metrics) =
            run_setups_with_metrics(&self.ltf_data, &setups, &self.execution_config());

        if self.config.log_progress {
            println!("  Setups: {}", metrics.setup_count);
            println!("  Limit orders placed: {}", metrics.limit_orders_placed);
            println!("  Limit orders filled: {}", metrics.limit_orders_filled);
            println!("  Limit orders expired: {}", metrics.limit_orders_expired);
            println!("  Skipped same dir: {}", metrics.skipped_open_same_dir);
            println!(
                "  Skipped opposite dir: {}",
                metrics.skipped_open_opposite_dir
            );
            println!("  Total trades: {}", trades.len());
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fractal_mtf_config_default() {
        let config = FractalMTFConfig::default();
        assert_eq!(config.rr_target, Decimal::from(2));
        assert_eq!(config.htf_name, "4h");
        assert_eq!(config.ltf_name, "15m");
    }
}
