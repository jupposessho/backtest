use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use crate::engine::execution::run_setups;
use crate::engine::entry_policies::resolve_entry_policy;
use crate::engine::types::{
    EntryModel, EntryPolicy, ExecutionConfig as EngineExecutionConfig, SetupCandidate, StopModel,
    TargetModel, TrailingModel,
};
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;

pub struct Mc {
    pub data: Vec<CandleStick>,
    pub config: McConfig,
}

#[derive(Clone)]
pub struct McConfig {
    pub mode: McMode,
    pub pattern: SignalPattern,
    pub entry_mode: EntryMode,
    pub rr_target: Decimal,
    pub trade_window: Option<TimeWindow>,
    pub prev_open_fill_window_candles: usize,
    pub trailing_stop: TrailingStopConfig,
    pub level_filters: LevelFilters,
    pub trend_filter: TrendFilter,
    pub fvg_filter: FvgConfig,
    pub daily_open_time: NaiveTime,
    pub execution: ExecutionConfig,
}

impl Default for McConfig {
    fn default() -> Self {
        Self {
            mode: McMode::Auto,
            pattern: SignalPattern::Mc,
            entry_mode: EntryMode::Close,
            rr_target: Decimal::from_f32(1.5).unwrap(),
            trade_window: Some(TimeWindow::default()),
            prev_open_fill_window_candles: 3,
            trailing_stop: TrailingStopConfig::default(),
            level_filters: LevelFilters::default(),
            trend_filter: TrendFilter::None,
            fvg_filter: FvgConfig::default(),
            daily_open_time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
            execution: ExecutionConfig::default(),
        }
    }
}

#[derive(Clone)]
pub enum MarketEntryMode {
    SignalClose,
    NextBarOpen,
}

#[derive(Clone)]
pub struct ExecutionConfig {
    pub market_entry: MarketEntryMode,
    pub commission_rate_per_side: Decimal,
    pub fee_rate_per_side: Decimal,
    pub slippage_ticks_per_side: i32,
    pub tick_size: Decimal,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            market_entry: MarketEntryMode::NextBarOpen,
            commission_rate_per_side: Decimal::ZERO,
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 0,
            tick_size: Decimal::from_f32(0.01).unwrap(),
        }
    }
}

#[derive(Clone)]
pub enum McMode {
    Auto,
    ReversalDaily,
    ContinuationEma200,
    ContinuationStructure,
}

#[derive(Clone)]
pub enum SignalPattern {
    Mc,
    Engulfing,
}

#[derive(Clone)]
pub enum EntryMode {
    Close,
    PrevOpen,
    PairMidpoint,
    PairExtreme,
}

#[derive(Clone)]
pub enum TrailingStopMode {
    None,
    StepHalfR,
    /// Set SL to break even at 1R
    BreakEven1R,
    /// Set SL to 0.5R at 1.5R
    Trail05RAt15R,
    /// Set SL to 1R at 2R
    Trail1RAt2R,
    /// Progressive: BE at 1R, 0.5R at 1.5R, 1R at 2R, 1.5R at 2.5R, etc.
    Progressive,
}

#[derive(Clone)]
pub struct TrailingStopConfig {
    pub mode: TrailingStopMode,
}

impl Default for TrailingStopConfig {
    fn default() -> Self {
        Self {
            mode: TrailingStopMode::None,
        }
    }
}

#[derive(Clone)]
pub struct TimeWindow {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl Default for TimeWindow {
    fn default() -> Self {
        Self {
            start: NaiveTime::from_hms_opt(5, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        }
    }
}

#[derive(Clone)]
pub struct LevelFilters {
    pub enabled: bool,
    pub sweep_window_candles: usize,
}

impl Default for LevelFilters {
    fn default() -> Self {
        Self {
            enabled: true,
            sweep_window_candles: 5,
        }
    }
}

#[derive(Clone)]
pub enum TrendFilter {
    None,
    MarketStructure,
    Ema { fast: usize, slow: usize },
}

#[derive(Clone)]
pub struct FvgConfig {
    pub enabled: bool,
    pub timeframes: Vec<FvgTimeframe>,
    pub touch_window_candles: usize,
}

impl Default for FvgConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeframes: vec![FvgTimeframe::H1, FvgTimeframe::H4],
            touch_window_candles: 3,
        }
    }
}

#[derive(Clone, Copy)]
pub enum FvgTimeframe {
    H1,
    H4,
}

#[derive(Clone, Copy)]
struct PriceRange {
    high: DecimalVec,
    low: DecimalVec,
}

impl PriceRange {
    fn from_candle(c: CandleStick) -> Self {
        Self {
            high: c.high,
            low: c.low,
        }
    }

    fn update(&mut self, c: CandleStick) {
        if c.high > self.high {
            self.high = c.high;
        }
        if c.low < self.low {
            self.low = c.low;
        }
    }

    fn contains(&self, price: DecimalVec) -> bool {
        price >= self.low && price <= self.high
    }
}

#[derive(Clone)]
struct PeriodRangeTracker<K: Copy + PartialEq> {
    current_key: Option<K>,
    current_range: Option<PriceRange>,
    prev_range: Option<PriceRange>,
}

impl<K: Copy + PartialEq> PeriodRangeTracker<K> {
    fn new() -> Self {
        Self {
            current_key: None,
            current_range: None,
            prev_range: None,
        }
    }

    fn update(&mut self, key: K, candle: CandleStick) {
        match self.current_key {
            Some(k) if k == key => {
                if let Some(ref mut range) = self.current_range {
                    range.update(candle);
                }
            }
            _ => {
                self.prev_range = self.current_range.take();
                self.current_key = Some(key);
                self.current_range = Some(PriceRange::from_candle(candle));
            }
        }
    }
}

#[derive(Clone)]
struct SweepEvent {
    direction: PositionDirection,
    range: PriceRange,
    expires_at: usize,
}

#[derive(Clone)]
struct FvgZone {
    direction: PositionDirection,
    low: DecimalVec,
    high: DecimalVec,
    start_time: i64,
}

impl FvgZone {
    fn touched_by(&self, candle: CandleStick) -> bool {
        candle.high >= self.low && candle.low <= self.high && candle.open_time >= self.start_time
    }
}

#[derive(Clone, Copy)]
enum TrendState {
    Up,
    Down,
    Neutral,
}

#[derive(Clone, Copy)]
struct PendingLimit {
    direction: PositionDirection,
    entry: DecimalVec,
    sl: DecimalVec,
    expires_at: usize,
}

#[derive(Clone, Copy)]
struct PendingMarket {
    direction: PositionDirection,
    sl: DecimalVec,
    signal_index: usize,
}

#[derive(Clone, Copy)]
struct ActivePosition {
    position: Position,
    current_sl: DecimalVec,
    initial_risk: Decimal,
}

impl Mc {
    fn wick_sizes(candle: CandleStick) -> (DecimalVec, DecimalVec) {
        let body_top = if candle.open > candle.close {
            candle.open
        } else {
            candle.close
        };
        let body_bottom = if candle.open < candle.close {
            candle.open
        } else {
            candle.close
        };

        let top_wick = candle.high - body_top;
        let bottom_wick = body_bottom - candle.low;

        (top_wick, bottom_wick)
    }

    fn body_top(candle: CandleStick) -> DecimalVec {
        if candle.open > candle.close {
            candle.open
        } else {
            candle.close
        }
    }

    fn body_bottom(candle: CandleStick) -> DecimalVec {
        if candle.open < candle.close {
            candle.open
        } else {
            candle.close
        }
    }

    fn is_bullish_mc(actual: CandleStick, previous: CandleStick) -> bool {
        let (top_wick, bottom_wick) = Self::wick_sizes(actual);

        actual.low < previous.low
            && actual.high > previous.high
            && actual.close > previous.high
            && bottom_wick > top_wick
    }

    fn is_bearish_mc(actual: CandleStick, previous: CandleStick) -> bool {
        let (top_wick, bottom_wick) = Self::wick_sizes(actual);

        actual.high > previous.high
            && actual.low < previous.low
            && actual.close < previous.low
            && top_wick > bottom_wick
    }

    fn is_bullish_engulfing(actual: CandleStick, previous: CandleStick) -> bool {
        if !previous.downclose() || !actual.upclose() {
            return false;
        }
        let prev_top = Self::body_top(previous);
        let prev_bottom = Self::body_bottom(previous);
        let curr_top = Self::body_top(actual);
        let curr_bottom = Self::body_bottom(actual);

        curr_bottom < prev_bottom && curr_top > prev_top
    }

    fn is_bearish_engulfing(actual: CandleStick, previous: CandleStick) -> bool {
        if !previous.upclose() || !actual.downclose() {
            return false;
        }
        let prev_top = Self::body_top(previous);
        let prev_bottom = Self::body_bottom(previous);
        let curr_top = Self::body_top(actual);
        let curr_bottom = Self::body_bottom(actual);

        curr_top > prev_top && curr_bottom < prev_bottom
    }

    fn signal_matches(
        pattern: &SignalPattern,
        actual: CandleStick,
        previous: CandleStick,
    ) -> (bool, bool) {
        match pattern {
            SignalPattern::Mc => (
                Self::is_bullish_mc(actual, previous),
                Self::is_bearish_mc(actual, previous),
            ),
            SignalPattern::Engulfing => (
                Self::is_bullish_engulfing(actual, previous),
                Self::is_bearish_engulfing(actual, previous),
            ),
        }
    }

    fn to_break_even(position: &mut Position) {
        position.move_to_break_even();
    }

    fn risk_amount(position: &Position) -> Decimal {
        match position.direction {
            PositionDirection::Long => position.entry.0 - position.sl.0,
            PositionDirection::Short => position.sl.0 - position.entry.0,
        }
    }

    fn target_price(position: &Position, rr: Decimal) -> Option<DecimalVec> {
        let risk = Self::risk_amount(position);
        if risk <= Decimal::ZERO {
            return None;
        }
        Some(match position.direction {
            PositionDirection::Long => DecimalVec(position.entry.0 + risk * rr),
            PositionDirection::Short => DecimalVec(position.entry.0 - risk * rr),
        })
    }

    fn reached_one_r(position: &Position, candle: CandleStick) -> bool {
        let risk = Self::risk_amount(position);
        match position.direction {
            PositionDirection::Long => candle.high.0 >= position.entry.0 + risk,
            PositionDirection::Short => candle.low.0 <= position.entry.0 - risk,
        }
    }

    fn apply_trailing(
        active: &mut ActivePosition,
        candle: CandleStick,
        trailing: &TrailingStopConfig,
    ) {
        match trailing.mode {
            TrailingStopMode::None => {
                if !active.position.at_break_even && Self::reached_one_r(&active.position, candle) {
                    active.position.at_break_even = true;
                }
            }
            TrailingStopMode::StepHalfR => {
                if active.initial_risk <= Decimal::ZERO {
                    return;
                }
                let one = Decimal::from_f32(1.0).unwrap();
                let step = Decimal::from_f32(0.5).unwrap();
                let attained_r = match active.position.direction {
                    PositionDirection::Long => {
                        (candle.high.0 - active.position.entry.0) / active.initial_risk
                    }
                    PositionDirection::Short => {
                        (active.position.entry.0 - candle.low.0) / active.initial_risk
                    }
                };

                if attained_r >= one {
                    let steps = ((attained_r - one) / step).trunc();
                    let target_r = steps * step;
                    let new_sl = match active.position.direction {
                        PositionDirection::Long => {
                            DecimalVec(active.position.entry.0 + target_r * active.initial_risk)
                        }
                        PositionDirection::Short => {
                            DecimalVec(active.position.entry.0 - target_r * active.initial_risk)
                        }
                    };

                    match active.position.direction {
                        PositionDirection::Long => {
                            if new_sl > active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                        PositionDirection::Short => {
                            if new_sl < active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                    }

                    if active.current_sl == active.position.entry {
                        active.position.at_break_even = true;
                    }
                }
            }
            TrailingStopMode::BreakEven1R => {
                if active.initial_risk <= Decimal::ZERO {
                    return;
                }
                let one = Decimal::from_f32(1.0).unwrap();
                let attained_r = match active.position.direction {
                    PositionDirection::Long => {
                        (candle.high.0 - active.position.entry.0) / active.initial_risk
                    }
                    PositionDirection::Short => {
                        (active.position.entry.0 - candle.low.0) / active.initial_risk
                    }
                };

                // Set SL to break even at 1R
                if attained_r >= one {
                    active.current_sl = active.position.entry;
                    active.position.at_break_even = true;
                }
            }
            TrailingStopMode::Trail05RAt15R => {
                if active.initial_risk <= Decimal::ZERO {
                    return;
                }
                let one = Decimal::from_f32(1.0).unwrap();
                let half = Decimal::from_f32(0.5).unwrap();
                let one_and_half = Decimal::from_f32(1.5).unwrap();
                let attained_r = match active.position.direction {
                    PositionDirection::Long => {
                        (candle.high.0 - active.position.entry.0) / active.initial_risk
                    }
                    PositionDirection::Short => {
                        (active.position.entry.0 - candle.low.0) / active.initial_risk
                    }
                };

                // Set SL to break even at 1R, then to 0.5R at 1.5R
                if attained_r >= one_and_half {
                    let new_sl = match active.position.direction {
                        PositionDirection::Long => {
                            DecimalVec(active.position.entry.0 + half * active.initial_risk)
                        }
                        PositionDirection::Short => {
                            DecimalVec(active.position.entry.0 - half * active.initial_risk)
                        }
                    };

                    match active.position.direction {
                        PositionDirection::Long => {
                            if new_sl > active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                        PositionDirection::Short => {
                            if new_sl < active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                    }
                } else if attained_r >= one {
                    // Set to break even at 1R
                    active.current_sl = active.position.entry;
                    active.position.at_break_even = true;
                }
            }
            TrailingStopMode::Trail1RAt2R => {
                if active.initial_risk <= Decimal::ZERO {
                    return;
                }
                let one = Decimal::from_f32(1.0).unwrap();
                let two = Decimal::from_f32(2.0).unwrap();
                let attained_r = match active.position.direction {
                    PositionDirection::Long => {
                        (candle.high.0 - active.position.entry.0) / active.initial_risk
                    }
                    PositionDirection::Short => {
                        (active.position.entry.0 - candle.low.0) / active.initial_risk
                    }
                };

                // Set SL to break even at 1R, then to 1R at 2R
                if attained_r >= two {
                    let new_sl = match active.position.direction {
                        PositionDirection::Long => {
                            DecimalVec(active.position.entry.0 + one * active.initial_risk)
                        }
                        PositionDirection::Short => {
                            DecimalVec(active.position.entry.0 - one * active.initial_risk)
                        }
                    };

                    match active.position.direction {
                        PositionDirection::Long => {
                            if new_sl > active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                        PositionDirection::Short => {
                            if new_sl < active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                    }
                } else if attained_r >= one {
                    // Set to break even at 1R
                    active.current_sl = active.position.entry;
                    active.position.at_break_even = true;
                }
            }
            TrailingStopMode::Progressive => {
                if active.initial_risk <= Decimal::ZERO {
                    return;
                }
                let one = Decimal::from_f32(1.0).unwrap();
                let half = Decimal::from_f32(0.5).unwrap();
                let attained_r = match active.position.direction {
                    PositionDirection::Long => {
                        (candle.high.0 - active.position.entry.0) / active.initial_risk
                    }
                    PositionDirection::Short => {
                        (active.position.entry.0 - candle.low.0) / active.initial_risk
                    }
                };

                // Progressive: BE at 1R, 0.5R at 1.5R, 1R at 2R, 1.5R at 2.5R, etc.
                if attained_r >= one {
                    // Calculate target SL based on progression
                    // At 1R: 0R (break even)
                    // At 1.5R: 0.5R
                    // At 2R: 1R
                    // At 2.5R: 1.5R
                    // At 3R: 2R
                    // Pattern: SL = max(0, attained_r - 1R)
                    let target_r = if attained_r >= one + half {
                        // Once we're past 1.5R, move SL up in 0.5R increments
                        // Floor to nearest 0.5R boundary
                        let excess = attained_r - one;
                        let steps = (excess / half).floor();
                        steps * half
                    } else {
                        // Between 1R and 1.5R, keep at break even
                        Decimal::ZERO
                    };

                    let new_sl = match active.position.direction {
                        PositionDirection::Long => {
                            DecimalVec(active.position.entry.0 + target_r * active.initial_risk)
                        }
                        PositionDirection::Short => {
                            DecimalVec(active.position.entry.0 - target_r * active.initial_risk)
                        }
                    };

                    match active.position.direction {
                        PositionDirection::Long => {
                            if new_sl > active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                        PositionDirection::Short => {
                            if new_sl < active.current_sl {
                                active.current_sl = new_sl;
                            }
                        }
                    }

                    if active.current_sl == active.position.entry {
                        active.position.at_break_even = true;
                    }
                }
            }
        }
    }

    fn trading_day_key(dt: chrono::DateTime<chrono_tz::Tz>, daily_open: NaiveTime) -> NaiveDate {
        let mut date = dt.date_naive();
        if dt.time() < daily_open {
            date = date.pred_opt().unwrap();
        }
        date
    }

    fn compute_fvg_zones(data: &[CandleStick], tf: FvgTimeframe) -> Vec<FvgZone> {
        let tf_secs = match tf {
            FvgTimeframe::H1 => 60 * 60,
            FvgTimeframe::H4 => 4 * 60 * 60,
        };

        let mut htf: Vec<CandleStick> = vec![];
        let mut current_bucket: Option<i64> = None;

        for c in data.iter().copied() {
            let bucket = c.open_time - (c.open_time % tf_secs);
            match current_bucket {
                Some(b) if b == bucket => {
                    if let Some(last) = htf.last_mut() {
                        if c.high > last.high {
                            last.high = c.high;
                        }
                        if c.low < last.low {
                            last.low = c.low;
                        }
                        last.close = c.close;
                        last.close_time = c.close_time;
                    }
                }
                _ => {
                    current_bucket = Some(bucket);
                    htf.push(CandleStick {
                        open_time: bucket,
                        close_time: c.close_time,
                        open: c.open,
                        high: c.high,
                        low: c.low,
                        close: c.close,
                    });
                }
            }
        }

        let mut zones = vec![];
        if htf.len() < 3 {
            return zones;
        }

        for i in 2..htf.len() {
            let c1 = htf[i - 2];
            let c3 = htf[i];
            if c1.high < c3.low {
                zones.push(FvgZone {
                    direction: PositionDirection::Long,
                    low: c1.high,
                    high: c3.low,
                    start_time: c3.open_time,
                });
            } else if c1.low > c3.high {
                zones.push(FvgZone {
                    direction: PositionDirection::Short,
                    low: c3.high,
                    high: c1.low,
                    start_time: c3.open_time,
                });
            }
        }

        zones
    }

    fn update_swings(
        prev: CandleStick,
        actual: CandleStick,
        next: CandleStick,
        swing_lows: &mut Vec<CandleStick>,
        swing_highs: &mut Vec<CandleStick>,
    ) {
        if actual.high > prev.high && actual.high > next.high {
            swing_highs.retain(|&c| c.high >= actual.high);
            swing_highs.push(actual);
        }
        if actual.low < prev.low && actual.low < next.low {
            swing_lows.retain(|&c| c.low <= actual.low);
            swing_lows.push(actual);
        }
    }

    fn trend_from_swings(swing_lows: &Vec<CandleStick>, swing_highs: &Vec<CandleStick>) -> TrendState {
        if swing_lows.len() < 2 || swing_highs.len() < 2 {
            return TrendState::Neutral;
        }
        let last_low = swing_lows[swing_lows.len() - 1].low;
        let prev_low = swing_lows[swing_lows.len() - 2].low;
        let last_high = swing_highs[swing_highs.len() - 1].high;
        let prev_high = swing_highs[swing_highs.len() - 2].high;

        if last_low > prev_low && last_high > prev_high {
            TrendState::Up
        } else if last_low < prev_low && last_high < prev_high {
            TrendState::Down
        } else {
            TrendState::Neutral
        }
    }

    fn entry_price(entry_mode: &EntryMode, actual: CandleStick, previous: CandleStick) -> DecimalVec {
        match entry_mode {
            EntryMode::Close => actual.close,
            EntryMode::PrevOpen => previous.open,
            EntryMode::PairMidpoint => DecimalVec((actual.high.0 + actual.low.0) / Decimal::from(2)),
            EntryMode::PairExtreme => match actual.close > previous.close {
                true => actual.low,
                false => actual.high,
            },
        }
    }

    fn mode_from_config(config: &McConfig) -> McMode {
        match &config.mode {
            McMode::Auto => match &config.trend_filter {
                TrendFilter::Ema { .. } => McMode::ContinuationEma200,
                TrendFilter::MarketStructure => McMode::ContinuationStructure,
                TrendFilter::None => McMode::ReversalDaily,
            },
            other => (*other).clone(),
        }
    }

    fn slip_value(execution: &ExecutionConfig) -> Decimal {
        Decimal::from_i32(execution.slippage_ticks_per_side).unwrap_or(Decimal::ZERO)
            * execution.tick_size
    }

    fn apply_entry_slippage(
        direction: PositionDirection,
        price: DecimalVec,
        execution: &ExecutionConfig,
    ) -> DecimalVec {
        let slip = Self::slip_value(execution);
        match direction {
            PositionDirection::Long => DecimalVec(price.0 + slip),
            PositionDirection::Short => DecimalVec(price.0 - slip),
        }
    }

    fn apply_exit_slippage(
        direction: PositionDirection,
        price: DecimalVec,
        execution: &ExecutionConfig,
    ) -> DecimalVec {
        let slip = Self::slip_value(execution);
        match direction {
            PositionDirection::Long => DecimalVec(price.0 - slip),
            PositionDirection::Short => DecimalVec(price.0 + slip),
        }
    }

    fn trade_costs(entry: DecimalVec, exit: DecimalVec, execution: &ExecutionConfig) -> (Decimal, Decimal, Decimal) {
        let notional = (entry.0 + exit.0) / Decimal::from(2);
        let commission = notional * execution.commission_rate_per_side * Decimal::from(2);
        let fees = notional * execution.fee_rate_per_side * Decimal::from(2);
        let slippage = Self::slip_value(execution).abs() * Decimal::from(2);
        (commission, slippage, fees)
    }

    fn build_trade(
        position: Position,
        close_time: i64,
        exit_price: DecimalVec,
        result: TradeResult,
        execution: &ExecutionConfig,
    ) -> Trade {
        let (commission, slippage, fees) = Self::trade_costs(position.entry, exit_price, execution);
        Trade {
            direction: position.direction,
            open_time: position.open_time,
            close_time,
            entry: position.entry,
            sl: position.sl,
            tp: exit_price,
            result,
            commission,
            slippage,
            fees,
        }
    }

    fn as_engine_execution(&self) -> EngineExecutionConfig {
        EngineExecutionConfig {
            commission_rate_per_side: self.config.execution.commission_rate_per_side,
            fee_rate_per_side: self.config.execution.fee_rate_per_side,
            slippage_ticks_per_side: self.config.execution.slippage_ticks_per_side,
            tick_size: self.config.execution.tick_size,
        }
    }

    fn trailing_model(&self) -> TrailingModel {
        match self.config.trailing_stop.mode {
            TrailingStopMode::None => TrailingModel::None,
            TrailingStopMode::BreakEven1R => TrailingModel::BreakEvenAtR(Decimal::ONE),
            TrailingStopMode::Trail05RAt15R => TrailingModel::StepAtR {
                trigger_r: Decimal::from_f32(1.5).unwrap(),
                lock_r: Decimal::from_f32(0.5).unwrap(),
            },
            TrailingStopMode::Trail1RAt2R => TrailingModel::StepAtR {
                trigger_r: Decimal::from(2),
                lock_r: Decimal::ONE,
            },
            TrailingStopMode::Progressive => TrailingModel::ProgressiveHalfR {
                start_r: Decimal::ONE,
                step_r: Decimal::from_f32(0.5).unwrap(),
            },
            TrailingStopMode::StepHalfR => TrailingModel::ProgressiveHalfR {
                start_r: Decimal::ONE,
                step_r: Decimal::from_f32(0.5).unwrap(),
            },
        }
    }

    fn detect_setups(&self) -> Vec<SetupCandidate> {
        let rr_target = self.config.rr_target;
        let mut setups: Vec<SetupCandidate> = vec![];

        let mut day_tracker: PeriodRangeTracker<NaiveDate> = PeriodRangeTracker::new();
        let mut sweep_events: Vec<SweepEvent> = vec![];
        let mut swing_lows: Vec<CandleStick> = vec![];
        let mut swing_highs: Vec<CandleStick> = vec![];
        let mut trend_state = TrendState::Neutral;
        let mut ema_slow: Option<Decimal> = None;

        let mut fvg_zones: Vec<FvgZone> = vec![];
        if self.config.fvg_filter.enabled {
            for tf in &self.config.fvg_filter.timeframes {
                fvg_zones.extend(Self::compute_fvg_zones(&self.data, *tf));
            }
        }
        let mut last_fvg_touch_long: Option<usize> = None;
        let mut last_fvg_touch_short: Option<usize> = None;

        let mode = Self::mode_from_config(&self.config);

        let mut ind = 0usize;
        while ind < self.data.len() {
            let actual = self.data[ind];
            let ny_dt = to_new_york_time(actual.open_time);
            let ny_time = ny_dt.time();

            let trading_day = Self::trading_day_key(ny_dt, self.config.daily_open_time);
            day_tracker.update(trading_day, actual);

            if ind >= 2 {
                let prev = self.data[ind - 1];
                let prev_prev = self.data[ind - 2];
                Self::update_swings(prev_prev, prev, actual, &mut swing_lows, &mut swing_highs);
                trend_state = Self::trend_from_swings(&swing_lows, &swing_highs);
            }

            if let TrendFilter::Ema { fast: _, slow } = self.config.trend_filter {
                let close = actual.close.0;
                let slow_alpha =
                    Decimal::from_i32(2).unwrap() / Decimal::from_i32((slow + 1) as i32).unwrap();
                ema_slow = Some(match ema_slow {
                    None => close,
                    Some(prev) => slow_alpha * close + (Decimal::ONE - slow_alpha) * prev,
                });
            }

            if self.config.fvg_filter.enabled {
                if fvg_zones
                    .iter()
                    .any(|z| z.direction == PositionDirection::Long && z.touched_by(actual))
                {
                    last_fvg_touch_long = Some(ind);
                }
                if fvg_zones
                    .iter()
                    .any(|z| z.direction == PositionDirection::Short && z.touched_by(actual))
                {
                    last_fvg_touch_short = Some(ind);
                }
            }

            sweep_events.retain(|e| e.expires_at >= ind);
            if self.config.level_filters.enabled {
                if let Some(r) = day_tracker.prev_range {
                    if actual.high > r.high {
                        sweep_events.push(SweepEvent {
                            direction: PositionDirection::Short,
                            range: r,
                            expires_at: ind + self.config.level_filters.sweep_window_candles,
                        });
                    }
                    if actual.low < r.low {
                        sweep_events.push(SweepEvent {
                            direction: PositionDirection::Long,
                            range: r,
                            expires_at: ind + self.config.level_filters.sweep_window_candles,
                        });
                    }
                }
            }

            if ind > 0 {
                let previous = self.data[ind - 1];
                let in_trade_window = self
                    .config
                    .trade_window
                    .as_ref()
                    .map(|w| is_time_in_session(ny_time, w.start, w.end))
                    .unwrap_or(true);

                if in_trade_window {
                    let (bullish_signal, bearish_signal) =
                        Self::signal_matches(&self.config.pattern, actual, previous);

                    let mut fvg_ok = !self.config.fvg_filter.enabled;
                    if self.config.fvg_filter.enabled {
                        if bullish_signal {
                            fvg_ok = last_fvg_touch_long
                                .map(|i_touch| {
                                    ind >= i_touch
                                        && (ind - i_touch) <= self.config.fvg_filter.touch_window_candles
                                })
                                .unwrap_or(false);
                        } else if bearish_signal {
                            fvg_ok = last_fvg_touch_short
                                .map(|i_touch| {
                                    ind >= i_touch
                                        && (ind - i_touch) <= self.config.fvg_filter.touch_window_candles
                                })
                                .unwrap_or(false);
                        }
                    }

                    let level_ok = match mode {
                        McMode::ReversalDaily => {
                            if bullish_signal {
                                sweep_events.iter().any(|e| {
                                    e.direction == PositionDirection::Long && e.range.contains(actual.close)
                                })
                            } else if bearish_signal {
                                sweep_events.iter().any(|e| {
                                    e.direction == PositionDirection::Short && e.range.contains(actual.close)
                                })
                            } else {
                                false
                            }
                        }
                        _ => true,
                    };

                    let trend_ok = match mode {
                        McMode::ContinuationEma200 => {
                            if let Some(slow) = ema_slow {
                                if bullish_signal {
                                    actual.close.0 > slow
                                } else if bearish_signal {
                                    actual.close.0 < slow
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        McMode::ContinuationStructure => match trend_state {
                            TrendState::Up => bullish_signal,
                            TrendState::Down => bearish_signal,
                            TrendState::Neutral => bullish_signal || bearish_signal,
                        },
                        McMode::ReversalDaily => true,
                        McMode::Auto => true,
                    };

                    if level_ok && fvg_ok && trend_ok {
                        if bullish_signal {
                            if let Some(setup) = self.build_setup(
                                PositionDirection::Long,
                                ind,
                                actual,
                                previous,
                                rr_target,
                            ) {
                                setups.push(setup);
                            }
                        } else if bearish_signal {
                            if let Some(setup) = self.build_setup(
                                PositionDirection::Short,
                                ind,
                                actual,
                                previous,
                                rr_target,
                            ) {
                                setups.push(setup);
                            }
                        }
                    }
                }
            }
            ind += 1;
        }
        setups
    }

    fn build_setup(
        &self,
        direction: PositionDirection,
        ind: usize,
        actual: CandleStick,
        previous: CandleStick,
        rr_target: Decimal,
    ) -> Option<SetupCandidate> {
        let sl = match direction {
            PositionDirection::Long => actual.low,
            PositionDirection::Short => actual.high,
        };

        let entry_model = match self.config.entry_mode {
            EntryMode::Close => match self.config.execution.market_entry {
                MarketEntryMode::SignalClose => EntryModel::SignalClose,
                MarketEntryMode::NextBarOpen => EntryModel::NextBarOpen,
            },
            EntryMode::PrevOpen => EntryModel::LimitByPolicy {
                policy: EntryPolicy::ObPrevOpen,
                expiry_bars: self.config.prev_open_fill_window_candles,
            },
            EntryMode::PairMidpoint => EntryModel::LimitByPolicy {
                policy: EntryPolicy::ObPairMidpoint,
                expiry_bars: self.config.prev_open_fill_window_candles,
            },
            EntryMode::PairExtreme => EntryModel::LimitByPolicy {
                policy: EntryPolicy::ObPairExtreme,
                expiry_bars: self.config.prev_open_fill_window_candles,
            },
        };

        let probe_entry = match entry_model {
            EntryModel::SignalClose => actual.close,
            EntryModel::NextBarOpen => actual.close,
            EntryModel::LimitTouch { price, .. } => price,
            EntryModel::LimitByPolicy { policy, .. } => resolve_entry_policy(policy, direction, actual, previous),
        };
        let risk = match direction {
            PositionDirection::Long => probe_entry.0 - sl.0,
            PositionDirection::Short => sl.0 - probe_entry.0,
        };
        if risk <= Decimal::ZERO {
            return None;
        }

        Some(SetupCandidate {
            direction,
            signal_index: ind,
            entry: entry_model,
            stop: StopModel::FixedPrice(sl),
            target: TargetModel::FixedR(rr_target),
            trailing: self.trailing_model(),
        })
    }
}

impl TradingModel for Mc {
    fn execute(&self) -> BacktestResult {
        let setups = self.detect_setups();
        let exec_cfg = self.as_engine_execution();
        let trades = run_setups(&self.data, &setups, &exec_cfg);
        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}

fn is_time_in_session(t: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start <= end {
        t >= start && t <= end
    } else {
        t >= start || t <= end
    }
}
