use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;

use chrono::{NaiveTime, Timelike};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;

const ANCHOR_MINUTES: i64 = 27;

#[derive(Clone, Copy)]
pub struct EmaPoint {
    pub ts: i64,
    pub value: Decimal,
}

#[derive(Clone, Copy, Debug)]
pub enum SessionFilter {
    All,
    NyOpen,
    NyAm,
}

#[derive(Clone, Copy, Debug)]
pub enum EntryVariant {
    BreakoutOnly,
    RmbRetestOnly,
    BreakoutOrRmbRetest,
}

#[derive(Clone, Copy, Debug)]
pub enum BagMode {
    RealOnly,
    AllowSyntheticFallback,
}

#[derive(Clone)]
pub struct FractalAlignmentConfig {
    pub rr_target: Decimal,
    pub fast_ema_period: usize,
    pub slow_ema_period: usize,
    pub anchor_ema_period: usize,
    pub pivot_strength: usize,
    pub max_setup_bars: usize,
    pub max_trigger_bars: usize,
    pub max_hold_bars: usize,
    pub session: SessionFilter,
    pub session_start: NaiveTime,
    pub session_end: NaiveTime,
    pub commission_per_side_points: Decimal,
    pub slippage_ticks_per_side: i32,
    pub tick_size: Decimal,
    pub require_anchor_bias: bool,
    pub entry_variant: EntryVariant,
    pub bag_mode: BagMode,
    pub min_bag_gap_ticks: i32,
    pub inversion_min_body_ticks: i32,
    pub inversion_close_pct: Decimal,
    pub max_bars_after_bag_confirm: usize,
    pub require_anchor_expansion: bool,
    pub anchor_range_lookback: usize,
    pub anchor_range_min_mult: Decimal,
    pub rmb_edge_tolerance_ticks: i32,
    pub stop_buffer_ticks: i32,
    pub bag_search_extension_bars: usize,
    pub max_anchor_bucket_span: i64,
}

impl Default for FractalAlignmentConfig {
    fn default() -> Self {
        Self {
            rr_target: Decimal::from_f32(2.0).unwrap(),
            fast_ema_period: 9,
            slow_ema_period: 21,
            anchor_ema_period: 9,
            pivot_strength: 1,
            max_setup_bars: 18,
            max_trigger_bars: 12,
            max_hold_bars: 45,
            session: SessionFilter::NyAm,
            session_start: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            session_end: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            commission_per_side_points: Decimal::from_f32(0.31).unwrap(),
            slippage_ticks_per_side: 1,
            tick_size: Decimal::new(25, 2),
            require_anchor_bias: true,
            entry_variant: EntryVariant::BreakoutOrRmbRetest,
            bag_mode: BagMode::AllowSyntheticFallback,
            min_bag_gap_ticks: 1,
            inversion_min_body_ticks: 1,
            inversion_close_pct: Decimal::from_f32(0.7).unwrap(),
            max_bars_after_bag_confirm: 6,
            require_anchor_expansion: true,
            anchor_range_lookback: 20,
            anchor_range_min_mult: Decimal::from_f32(1.1).unwrap(),
            rmb_edge_tolerance_ticks: 1,
            stop_buffer_ticks: 1,
            bag_search_extension_bars: 6,
            max_anchor_bucket_span: 2,
        }
    }
}

pub struct FractalAlignmentPlaybook {
    pub data_1m: Arc<Vec<CandleStick>>,
    pub data_3m: Arc<Vec<CandleStick>>,
    pub data_27m: Arc<Vec<CandleStick>>,
    pub config: FractalAlignmentConfig,
}

#[derive(Clone, Copy, PartialEq)]
enum SwingKind {
    High,
    Low,
}

#[derive(Clone, Copy)]
struct SwingPoint {
    index: usize,
    price: DecimalVec,
    kind: SwingKind,
}

#[derive(Clone, Copy)]
struct PendingSetup {
    direction: PositionDirection,
    explosion_level: DecimalVec,
    stop_level: DecimalVec,
    bag_inversion_level: DecimalVec,
    rmb_low: DecimalVec,
    rmb_high: DecimalVec,
    bag_confirmed: bool,
    bag_confirmed_index: usize,
    trigger_expires_index: usize,
}

#[derive(Clone, Copy)]
struct PendingEntry {
    direction: PositionDirection,
    entry_index: usize,
    stop_level: DecimalVec,
    target_level: DecimalVec,
}

fn rmb_zone_long(
    candles: &[CandleStick],
    csd_index: usize,
    swept_index: usize,
) -> (DecimalVec, DecimalVec) {
    let csd_high = candles[csd_index].high;
    let swept_high = candles[swept_index].high;
    if csd_high <= swept_high {
        (csd_high, swept_high)
    } else {
        (swept_high, csd_high)
    }
}

fn rmb_zone_short(
    candles: &[CandleStick],
    csd_index: usize,
    swept_index: usize,
) -> (DecimalVec, DecimalVec) {
    let csd_low = candles[csd_index].low;
    let swept_low = candles[swept_index].low;
    if csd_low <= swept_low {
        (csd_low, swept_low)
    } else {
        (swept_low, csd_low)
    }
}

pub fn resample_from_1m(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if data.is_empty() {
        return vec![];
    }

    let bucket = minutes * 60;
    let mut out = Vec::new();
    let mut current = data[0];
    let mut current_bucket = current.open_time / bucket;

    for candle in data.iter().copied().skip(1) {
        let bucket_id = candle.open_time / bucket;
        if bucket_id != current_bucket {
            out.push(current);
            current = candle;
            current_bucket = bucket_id;
        } else {
            if candle.high > current.high {
                current.high = candle.high;
            }
            if candle.low < current.low {
                current.low = candle.low;
            }
            current.close = candle.close;
            current.close_time = candle.close_time;
        }
    }

    out.push(current);
    out
}

pub fn ema_series(data: &[CandleStick], period: usize) -> Vec<EmaPoint> {
    let mut out = Vec::new();
    if period == 0 || data.len() < period {
        return out;
    }

    let k = Decimal::from(2) / Decimal::from_usize(period + 1).unwrap();
    let mut seed = Decimal::ZERO;
    for candle in data.iter().take(period) {
        seed += candle.close.0;
    }
    let mut ema = seed / Decimal::from_usize(period).unwrap();
    out.push(EmaPoint {
        ts: data[period - 1].close_time,
        value: ema,
    });

    for candle in data.iter().skip(period) {
        ema = candle.close.0 * k + ema * (Decimal::ONE - k);
        out.push(EmaPoint {
            ts: candle.close_time,
            value: ema,
        });
    }

    out
}

fn latest_ema(series: &[EmaPoint], idx: &mut usize, ts: i64) -> Option<Decimal> {
    while *idx + 1 < series.len() && series[*idx + 1].ts <= ts {
        *idx += 1;
    }
    if series
        .get(*idx)
        .map(|point| point.ts <= ts)
        .unwrap_or(false)
    {
        Some(series[*idx].value)
    } else {
        None
    }
}

fn candle_body_ticks(candle: CandleStick, tick_size: Decimal) -> Decimal {
    if tick_size <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    (candle.close.0 - candle.open.0).abs() / tick_size
}

fn in_session(ts: i64, config: &FractalAlignmentConfig) -> bool {
    let time = to_new_york_time(ts).time();
    if matches!(config.session, SessionFilter::All) {
        return true;
    }

    let current = time.num_seconds_from_midnight();
    let start = config.session_start.num_seconds_from_midnight();
    let end = config.session_end.num_seconds_from_midnight();
    current >= start && current <= end
}

fn anchor_bucket(ts: i64) -> i64 {
    ts / (ANCHOR_MINUTES * 60)
}

fn push_swing(swings: &mut VecDeque<SwingPoint>, swing: SwingPoint) {
    if let Some(last) = swings.back_mut() {
        if last.kind == swing.kind {
            let should_replace = match swing.kind {
                SwingKind::High => swing.price > last.price,
                SwingKind::Low => swing.price < last.price,
            };
            if should_replace {
                *last = swing;
            }
            return;
        }
    }

    swings.push_back(swing);
    while swings.len() > 6 {
        swings.pop_front();
    }
}

fn build_setup(
    swings: &VecDeque<SwingPoint>,
    candles: &[CandleStick],
    current_index: usize,
    config: &FractalAlignmentConfig,
) -> Option<PendingSetup> {
    if swings.len() < 4 {
        return None;
    }

    let last: Vec<SwingPoint> = swings.iter().rev().take(4).copied().collect();
    let a = last[3];
    let b = last[2];
    let c = last[1];
    let d = last[0];

    let anchor_start = anchor_bucket(candles[a.index].open_time);
    let anchor_end = anchor_bucket(candles[d.index].open_time);
    if (anchor_end - anchor_start).abs() > config.max_anchor_bucket_span {
        return None;
    }

    if d.index.saturating_sub(a.index) > config.max_setup_bars {
        return None;
    }

    if current_index <= d.index {
        return None;
    }

    let min_gap = config.tick_size
        * Decimal::from_i32(config.min_bag_gap_ticks.max(1)).unwrap_or(Decimal::ONE);
    let stop_buffer = config.tick_size
        * Decimal::from_i32(config.stop_buffer_ticks.max(1)).unwrap_or(Decimal::ONE);
    let bag_end = (d.index + config.bag_search_extension_bars).min(candles.len().saturating_sub(1));
    let bearish_bag = detect_bearish_bag(candles, c.index.saturating_add(1), bag_end, min_gap);
    let bullish_bag = detect_bullish_bag(candles, c.index.saturating_add(1), bag_end, min_gap);

    if a.kind == SwingKind::High
        && b.kind == SwingKind::Low
        && c.kind == SwingKind::High
        && d.kind == SwingKind::Low
        && c.price > a.price
        && d.price < a.price
    {
        let bag_inversion_level = match bearish_bag {
            Some(value) => value,
            None => match config.bag_mode {
                BagMode::RealOnly => return None,
                BagMode::AllowSyntheticFallback => {
                    DecimalVec((c.price.0 + d.price.0) / Decimal::from(2))
                }
            },
        };
        let (rmb_low, rmb_high) = rmb_zone_long(candles, a.index, c.index);
        return Some(PendingSetup {
            direction: PositionDirection::Long,
            explosion_level: a.price,
            stop_level: DecimalVec(d.price.0 - stop_buffer),
            bag_inversion_level,
            rmb_low,
            rmb_high,
            bag_confirmed: false,
            bag_confirmed_index: 0,
            trigger_expires_index: current_index + config.max_trigger_bars,
        });
    }

    if a.kind == SwingKind::Low
        && b.kind == SwingKind::High
        && c.kind == SwingKind::Low
        && d.kind == SwingKind::High
        && c.price < a.price
        && d.price > a.price
    {
        let bag_inversion_level = match bullish_bag {
            Some(value) => value,
            None => match config.bag_mode {
                BagMode::RealOnly => return None,
                BagMode::AllowSyntheticFallback => {
                    DecimalVec((c.price.0 + d.price.0) / Decimal::from(2))
                }
            },
        };
        let (rmb_low, rmb_high) = rmb_zone_short(candles, a.index, c.index);
        return Some(PendingSetup {
            direction: PositionDirection::Short,
            explosion_level: a.price,
            stop_level: DecimalVec(d.price.0 + stop_buffer),
            bag_inversion_level,
            rmb_low,
            rmb_high,
            bag_confirmed: false,
            bag_confirmed_index: 0,
            trigger_expires_index: current_index + config.max_trigger_bars,
        });
    }

    None
}

fn detect_bearish_bag(
    candles: &[CandleStick],
    start: usize,
    end: usize,
    min_gap: Decimal,
) -> Option<DecimalVec> {
    if start >= candles.len() || end >= candles.len() || start >= end {
        return None;
    }

    let mut best: Option<(Decimal, DecimalVec)> = None;
    for i in start..=end {
        for j in i + 1..=end {
            if candles[j].high < candles[i].low {
                let width = candles[i].low.0 - candles[j].high.0;
                if width < min_gap {
                    continue;
                }
                match best {
                    None => best = Some((width, candles[i].low)),
                    Some((w, _)) if width > w => best = Some((width, candles[i].low)),
                    _ => {}
                }
            }
        }
    }
    best.map(|(_, level)| level)
}

fn detect_bullish_bag(
    candles: &[CandleStick],
    start: usize,
    end: usize,
    min_gap: Decimal,
) -> Option<DecimalVec> {
    if start >= candles.len() || end >= candles.len() || start >= end {
        return None;
    }

    let mut best: Option<(Decimal, DecimalVec)> = None;
    for i in start..=end {
        for j in i + 1..=end {
            if candles[j].low > candles[i].high {
                let width = candles[j].low.0 - candles[i].high.0;
                if width < min_gap {
                    continue;
                }
                match best {
                    None => best = Some((width, candles[i].high)),
                    Some((w, _)) if width > w => best = Some((width, candles[i].high)),
                    _ => {}
                }
            }
        }
    }
    best.map(|(_, level)| level)
}

fn trade_result(direction: PositionDirection, entry: DecimalVec, exit: DecimalVec) -> TradeResult {
    let delta = match direction {
        PositionDirection::Long => exit.0 - entry.0,
        PositionDirection::Short => entry.0 - exit.0,
    };
    if delta > Decimal::ZERO {
        TradeResult::Winner
    } else if delta < Decimal::ZERO {
        TradeResult::Expense
    } else {
        TradeResult::BreakEven
    }
}

impl TradingModel for FractalAlignmentPlaybook {
    fn execute(&self) -> BacktestResult {
        let fast_1m = ema_series(&self.data_1m, self.config.fast_ema_period);
        let slow_1m = ema_series(&self.data_1m, self.config.slow_ema_period);
        let fast_3m = ema_series(&self.data_3m, self.config.fast_ema_period);
        let slow_3m = ema_series(&self.data_3m, self.config.slow_ema_period);
        let anchor_27m = ema_series(&self.data_27m, self.config.anchor_ema_period);
        let mut anchor_expansion: HashMap<i64, bool> = HashMap::new();
        if self.config.anchor_range_lookback > 0 {
            for idx in 0..self.data_27m.len() {
                let range = self.data_27m[idx].high.0 - self.data_27m[idx].low.0;
                let start = idx.saturating_sub(self.config.anchor_range_lookback);
                let mut sum = Decimal::ZERO;
                let mut count = 0usize;
                for j in start..idx {
                    sum += self.data_27m[j].high.0 - self.data_27m[j].low.0;
                    count += 1;
                }
                let ok = if count == 0 {
                    true
                } else {
                    let avg = sum / Decimal::from_usize(count).unwrap_or(Decimal::ONE);
                    range >= avg * self.config.anchor_range_min_mult
                };
                anchor_expansion.insert(anchor_bucket(self.data_27m[idx].open_time), ok);
            }
        }

        let mut trades = Vec::new();
        let mut swings: VecDeque<SwingPoint> = VecDeque::new();
        let mut active_position: Option<Position> = None;
        let mut pending_setup: Option<PendingSetup> = None;
        let mut pending_entry: Option<PendingEntry> = None;
        let mut bars_in_position = 0usize;
        let mut last_anchor_bucket = None;
        let mut traded_anchor: Option<i64> = None;

        let mut fast_1m_idx = 0usize;
        let mut slow_1m_idx = 0usize;
        let mut fast_3m_idx = 0usize;
        let mut slow_3m_idx = 0usize;
        let mut anchor_idx = 0usize;

        for i in 2..self.data_1m.len().saturating_sub(1) {
            let candle = self.data_1m[i];
            let current_anchor = anchor_bucket(candle.open_time);

            if last_anchor_bucket != Some(current_anchor) {
                swings.clear();
                pending_setup = None;
                pending_entry = None;
                last_anchor_bucket = Some(current_anchor);
            }

            let fast_now = latest_ema(&fast_1m, &mut fast_1m_idx, candle.close_time);
            let slow_now = latest_ema(&slow_1m, &mut slow_1m_idx, candle.close_time);
            let fast_3m_now = latest_ema(&fast_3m, &mut fast_3m_idx, candle.close_time);
            let slow_3m_now = latest_ema(&slow_3m, &mut slow_3m_idx, candle.close_time);
            let anchor_ema_now = latest_ema(&anchor_27m, &mut anchor_idx, candle.close_time);

            if let Some(entry) = pending_entry {
                if entry.entry_index == i {
                    let entry_price = candle.open;
                    let risk = match entry.direction {
                        PositionDirection::Long => entry_price.0 - entry.stop_level.0,
                        PositionDirection::Short => entry.stop_level.0 - entry_price.0,
                    };
                    if risk > self.config.tick_size {
                        active_position = Some(Position {
                            direction: entry.direction,
                            open_time: candle.open_time,
                            entry: entry_price,
                            sl: entry.stop_level,
                            tp: entry.target_level,
                            at_break_even: false,
                        });
                        bars_in_position = 0;
                        traded_anchor = Some(current_anchor);
                    }
                    pending_entry = None;
                }
            }

            if let Some(position) = active_position {
                bars_in_position += 1;

                let exit_price = match position.direction {
                    PositionDirection::Long => {
                        if candle.open <= position.sl {
                            Some(candle.open)
                        } else if candle.open >= position.tp {
                            Some(candle.open)
                        } else if candle.low <= position.sl {
                            Some(position.sl)
                        } else if candle.high >= position.tp {
                            Some(position.tp)
                        } else if bars_in_position >= self.config.max_hold_bars {
                            Some(candle.close)
                        } else {
                            None
                        }
                    }
                    PositionDirection::Short => {
                        if candle.open >= position.sl {
                            Some(candle.open)
                        } else if candle.open <= position.tp {
                            Some(candle.open)
                        } else if candle.high >= position.sl {
                            Some(position.sl)
                        } else if candle.low <= position.tp {
                            Some(position.tp)
                        } else if bars_in_position >= self.config.max_hold_bars {
                            Some(candle.close)
                        } else {
                            None
                        }
                    }
                };

                if let Some(exit) = exit_price {
                    let commission = self.config.commission_per_side_points * Decimal::from(2);
                    let slippage = self.config.tick_size
                        * Decimal::from_i32(self.config.slippage_ticks_per_side)
                            .unwrap_or(Decimal::ZERO)
                        * Decimal::from(2);

                    trades.push(Trade {
                        direction: position.direction,
                        open_time: position.open_time,
                        close_time: candle.close_time,
                        entry: position.entry,
                        sl: position.sl,
                        tp: exit,
                        result: trade_result(position.direction, position.entry, exit),
                        commission,
                        slippage,
                        fees: Decimal::ZERO,
                    });
                    active_position = None;
                    bars_in_position = 0;
                }
                continue;
            }

            let strength = self.config.pivot_strength.max(1);
            if i <= strength * 2 {
                continue;
            }

            let pivot_index = i - strength;
            let mid = self.data_1m[pivot_index];
            let mut is_swing_high = true;
            let mut is_swing_low = true;
            for offset in 1..=strength {
                let left = self.data_1m[pivot_index - offset];
                let right = self.data_1m[pivot_index + offset];
                if mid.high <= left.high || mid.high <= right.high {
                    is_swing_high = false;
                }
                if mid.low >= left.low || mid.low >= right.low {
                    is_swing_low = false;
                }
            }

            if is_swing_high {
                push_swing(
                    &mut swings,
                    SwingPoint {
                        index: pivot_index,
                        price: mid.high,
                        kind: SwingKind::High,
                    },
                );
            }
            if is_swing_low {
                push_swing(
                    &mut swings,
                    SwingPoint {
                        index: pivot_index,
                        price: mid.low,
                        kind: SwingKind::Low,
                    },
                );
            }

            if pending_setup.is_none() {
                pending_setup = build_setup(&swings, &self.data_1m, i, &self.config);
            }

            let Some(mut setup) = pending_setup else {
                continue;
            };

            if traded_anchor == Some(current_anchor) {
                continue;
            }

            if i > setup.trigger_expires_index || !in_session(candle.open_time, &self.config) {
                pending_setup = None;
                continue;
            }

            let (Some(fast), Some(slow), Some(fast3), Some(slow3)) =
                (fast_now, slow_now, fast_3m_now, slow_3m_now)
            else {
                continue;
            };

            let anchor_ok = if self.config.require_anchor_bias {
                match anchor_ema_now {
                    Some(anchor_ema) => match setup.direction {
                        PositionDirection::Long => candle.close.0 > anchor_ema,
                        PositionDirection::Short => candle.close.0 < anchor_ema,
                    },
                    None => false,
                }
            } else {
                true
            };

            if !anchor_ok {
                continue;
            }

            if self.config.require_anchor_expansion {
                let exp_ok = anchor_expansion
                    .get(&current_anchor)
                    .copied()
                    .unwrap_or(false);
                if !exp_ok {
                    continue;
                }
            }

            if !setup.bag_confirmed {
                let body_ticks = candle_body_ticks(candle, self.config.tick_size);
                let range = (candle.high.0 - candle.low.0).max(self.config.tick_size);
                let close_pos = match setup.direction {
                    PositionDirection::Long => (candle.close.0 - candle.low.0) / range,
                    PositionDirection::Short => (candle.high.0 - candle.close.0) / range,
                };
                setup.bag_confirmed = match setup.direction {
                    PositionDirection::Long => {
                        candle.close > setup.bag_inversion_level
                            && body_ticks
                                >= Decimal::from_i32(self.config.inversion_min_body_ticks.max(1))
                                    .unwrap_or(Decimal::ONE)
                            && close_pos >= self.config.inversion_close_pct
                    }
                    PositionDirection::Short => {
                        candle.close < setup.bag_inversion_level
                            && body_ticks
                                >= Decimal::from_i32(self.config.inversion_min_body_ticks.max(1))
                                    .unwrap_or(Decimal::ONE)
                            && close_pos >= self.config.inversion_close_pct
                    }
                };
                if setup.bag_confirmed {
                    setup.bag_confirmed_index = i;
                }
                pending_setup = Some(setup);
                if !setup.bag_confirmed {
                    continue;
                }
            }

            if i.saturating_sub(setup.bag_confirmed_index) > self.config.max_bars_after_bag_confirm
            {
                pending_setup = None;
                continue;
            }

            let trend_ok = match setup.direction {
                PositionDirection::Long => fast > slow && fast3 > slow3,
                PositionDirection::Short => fast < slow && fast3 < slow3,
            };
            if !trend_ok {
                pending_setup = Some(setup);
                continue;
            }

            let ema_reclaim_ok = match setup.direction {
                PositionDirection::Long => candle.low.0 <= fast && candle.close.0 > fast,
                PositionDirection::Short => candle.high.0 >= fast && candle.close.0 < fast,
            };

            let breakout_entry = match setup.direction {
                PositionDirection::Long => candle.close > setup.explosion_level,
                PositionDirection::Short => candle.close < setup.explosion_level,
            };

            let rmb_touch = match setup.direction {
                PositionDirection::Long => {
                    let tol = self.config.tick_size
                        * Decimal::from_i32(self.config.rmb_edge_tolerance_ticks.max(1))
                            .unwrap_or(Decimal::ONE);
                    candle.low <= DecimalVec(setup.rmb_low.0 + tol)
                        && candle.close.0 > setup.rmb_low.0
                }
                PositionDirection::Short => {
                    let tol = self.config.tick_size
                        * Decimal::from_i32(self.config.rmb_edge_tolerance_ticks.max(1))
                            .unwrap_or(Decimal::ONE);
                    candle.high >= DecimalVec(setup.rmb_high.0 - tol)
                        && candle.close.0 < setup.rmb_high.0
                }
            };

            let entry_gate = match self.config.entry_variant {
                EntryVariant::BreakoutOnly => breakout_entry,
                EntryVariant::RmbRetestOnly => rmb_touch,
                EntryVariant::BreakoutOrRmbRetest => breakout_entry || rmb_touch,
            };

            let should_enter = ema_reclaim_ok && entry_gate;

            if should_enter && i + 1 < self.data_1m.len() {
                let next_open = self.data_1m[i + 1].open;
                let risk = match setup.direction {
                    PositionDirection::Long => next_open.0 - setup.stop_level.0,
                    PositionDirection::Short => setup.stop_level.0 - next_open.0,
                };
                if risk > self.config.tick_size {
                    let target_level = match setup.direction {
                        PositionDirection::Long => DecimalVec(
                            next_open.0 + risk * self.config.rr_target.trunc_with_scale(4),
                        ),
                        PositionDirection::Short => DecimalVec(
                            next_open.0 - risk * self.config.rr_target.trunc_with_scale(4),
                        ),
                    };
                    pending_entry = Some(PendingEntry {
                        direction: setup.direction,
                        entry_index: i + 1,
                        stop_level: setup.stop_level,
                        target_level,
                    });
                    pending_setup = None;
                } else {
                    pending_setup = Some(setup);
                }
            } else {
                pending_setup = Some(setup);
            }
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}
