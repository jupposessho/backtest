use chrono::{Datelike, Timelike};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::fee_config::FeeConfig;
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;

pub struct MomentumRider {
    pub data_15m: Vec<CandleStick>,
    pub volume_15m: Vec<Decimal>,
    pub data_1h: Vec<CandleStick>,
    pub config: MomentumRiderConfig,
}

/// Where to place the stop loss.
///
/// `CandleLow` (spec default): stop = candle_low − stop_atr_mult × ATR.
/// The risk from entry(=close) to stop includes the candle body (close−low),
/// so risk ≈ 1.5–2.5 × ATR; TP1 lands well below 1.5 R.
///
/// `EntryBased`: stop = entry − stop_atr_mult × ATR.
/// Risk = exactly stop_atr_mult × ATR; TP1 at tp1_atr_mult / stop_atr_mult R.
/// With defaults (1.0 / 1.5), TP1 = exactly 1.5 R.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StopMode {
    CandleLow,
    EntryBased,
}

/// Optional entry-time filter.  Empty Vec means "all allowed".
#[derive(Clone, Debug, Default)]
pub struct MomentumTimeWindow {
    /// NY hours (0–23) during which new entries are permitted.
    pub hours: Vec<u32>,
    /// Days of week (0 = Mon … 6 = Sun) during which entries are permitted.
    pub days: Vec<u32>,
}

impl MomentumTimeWindow {
    pub fn new(hours: Vec<u32>, days: Vec<u32>) -> Self {
        Self { hours, days }
    }
    fn allows(&self, candle: &CandleStick) -> bool {
        let t = to_new_york_time(candle.close_time);
        let h = t.hour();
        let d = t.weekday().num_days_from_monday();
        (self.hours.is_empty() || self.hours.contains(&h))
            && (self.days.is_empty() || self.days.contains(&d))
    }
}

pub struct MomentumRiderConfig {
    pub htf_ema_period: usize,
    pub rsi_period: usize,
    pub rsi_long_min: Decimal,
    pub rsi_long_max: Decimal,
    pub rsi_short_min: Decimal,
    pub rsi_short_max: Decimal,
    pub breakout_lookback: usize,
    pub volume_multiplier: Decimal,
    pub atr_period: usize,
    pub atr_sma_period: usize,
    pub stop_atr_mult: Decimal,
    pub tp1_atr_mult: Decimal,
    pub trail_atr_mult: Decimal,
    pub stop_mode: StopMode,
    pub time_window: Option<MomentumTimeWindow>,
    pub enable_session_filter: bool,
    pub fee_config: FeeConfig,
}

impl Default for MomentumRiderConfig {
    fn default() -> Self {
        Self {
            htf_ema_period: 50,
            rsi_period: 14,
            rsi_long_min: Decimal::from(52),
            rsi_long_max: Decimal::from(75),
            rsi_short_min: Decimal::from(25),
            rsi_short_max: Decimal::from(48),
            breakout_lookback: 3,
            volume_multiplier: Decimal::from_f32(1.5).unwrap(),
            atr_period: 14,
            atr_sma_period: 20,
            stop_atr_mult: Decimal::ONE,
            tp1_atr_mult: Decimal::from_f32(1.5).unwrap(),
            trail_atr_mult: Decimal::ONE,
            stop_mode: StopMode::CandleLow,
            time_window: None,
            enable_session_filter: true,
            fee_config: FeeConfig::default(),
        }
    }
}

// ── Indicator helpers ──────────────────────────────────────────────────────────

fn compute_ema(candles: &[CandleStick], period: usize) -> Vec<Option<Decimal>> {
    let mut result = vec![None; candles.len()];
    if candles.len() < period {
        return result;
    }
    // SMA seed
    let seed: Decimal = candles[..period].iter().map(|c| c.close.0).sum::<Decimal>()
        / Decimal::from(period as u32);
    result[period - 1] = Some(seed);
    let k = Decimal::from(2) / Decimal::from((period + 1) as u32);
    for i in period..candles.len() {
        let prev = result[i - 1].unwrap();
        result[i] = Some(candles[i].close.0 * k + prev * (Decimal::ONE - k));
    }
    result
}

fn compute_rsi(candles: &[CandleStick], period: usize) -> Vec<Option<Decimal>> {
    let mut result = vec![None; candles.len()];
    if candles.len() < period + 1 {
        return result;
    }
    // Seed: SMA of first `period` changes
    let mut gains = Decimal::ZERO;
    let mut losses = Decimal::ZERO;
    for i in 1..=period {
        let diff = candles[i].close.0 - candles[i - 1].close.0;
        if diff > Decimal::ZERO {
            gains += diff;
        } else {
            losses -= diff;
        }
    }
    let period_d = Decimal::from(period as u32);
    let mut avg_gain = gains / period_d;
    let mut avg_loss = losses / period_d;
    let rsi_val = |ag: Decimal, al: Decimal| -> Decimal {
        if al == Decimal::ZERO {
            return Decimal::from(100);
        }
        Decimal::from(100) - Decimal::from(100) / (Decimal::ONE + ag / al)
    };
    result[period] = Some(rsi_val(avg_gain, avg_loss));
    for i in (period + 1)..candles.len() {
        let diff = candles[i].close.0 - candles[i - 1].close.0;
        let gain = if diff > Decimal::ZERO { diff } else { Decimal::ZERO };
        let loss = if diff < Decimal::ZERO { -diff } else { Decimal::ZERO };
        avg_gain = (avg_gain * (period_d - Decimal::ONE) + gain) / period_d;
        avg_loss = (avg_loss * (period_d - Decimal::ONE) + loss) / period_d;
        result[i] = Some(rsi_val(avg_gain, avg_loss));
    }
    result
}

fn compute_atr(candles: &[CandleStick], period: usize) -> Vec<Option<Decimal>> {
    let mut result = vec![None; candles.len()];
    if candles.len() < period + 1 {
        return result;
    }
    let tr = |i: usize| -> Decimal {
        let h = candles[i].high.0;
        let l = candles[i].low.0;
        let pc = candles[i - 1].close.0;
        (h - l).abs().max((h - pc).abs()).max((l - pc).abs())
    };
    let mut sum = Decimal::ZERO;
    for i in 1..=period {
        sum += tr(i);
    }
    let period_d = Decimal::from(period as u32);
    let mut atr = sum / period_d;
    result[period] = Some(atr);
    for i in (period + 1)..candles.len() {
        atr = (atr * (period_d - Decimal::ONE) + tr(i)) / period_d;
        result[i] = Some(atr);
    }
    result
}

fn compute_vol_sma(volumes: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    let mut result = vec![None; volumes.len()];
    if volumes.len() < period {
        return result;
    }
    for i in (period - 1)..volumes.len() {
        let sum: Decimal = volumes[(i + 1 - period)..=i].iter().copied().sum();
        result[i] = Some(sum / Decimal::from(period as u32));
    }
    result
}

fn compute_atr_sma(atrs: &[Option<Decimal>], period: usize) -> Vec<Option<Decimal>> {
    let mut result = vec![None; atrs.len()];
    for i in 0..atrs.len() {
        if i + 1 < period {
            continue;
        }
        let window: Vec<Decimal> = atrs[(i + 1 - period)..=i]
            .iter()
            .filter_map(|&v| v)
            .collect();
        if window.len() == period {
            let sum: Decimal = window.iter().copied().sum();
            result[i] = Some(sum / Decimal::from(period as u32));
        }
    }
    result
}

// ── Session filter ─────────────────────────────────────────────────────────────

/// Returns true if this candle is in the 00:00–00:14 NY open buffer (skip entry).
fn in_open_buffer(candle: &CandleStick) -> bool {
    let t = to_new_york_time(candle.close_time);
    let h = t.hour();
    let m = t.minute();
    h == 0 && m < 15
}

/// Returns true if we should force-close by time: 23:30–00:00 NY.
fn in_time_exit(candle: &CandleStick) -> bool {
    let t = to_new_york_time(candle.close_time);
    let h = t.hour();
    h == 23 && t.minute() >= 30
}

// ── State machine ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum MomentumStage {
    PreTp1 {
        position: Position,
        tp1_price: Decimal,
        atr: Decimal,
    },
    PostTp1 {
        position: Position,
        tp1_price: Decimal,
        trail_stop: Decimal,
        atr: Decimal,
    },
}

// ── TradingModel impl ──────────────────────────────────────────────────────────

impl TradingModel for MomentumRider {
    fn execute(&self) -> BacktestResult {
        let cfg = &self.config;

        // Precompute 1H indicators
        let ema50_1h = compute_ema(&self.data_1h, cfg.htf_ema_period);

        // Precompute 15m indicators
        let rsi_15m = compute_rsi(&self.data_15m, cfg.rsi_period);
        let atr_15m = compute_atr(&self.data_15m, cfg.atr_period);
        let vol_sma_15m = compute_vol_sma(&self.volume_15m, cfg.atr_sma_period);
        let atr_sma_15m = compute_atr_sma(&atr_15m, cfg.atr_sma_period);

        let mut trades: Vec<Trade> = Vec::new();
        let mut stage: Option<MomentumStage> = None;
        let mut htf_idx: usize = 0;

        let two = Decimal::from(2);
        let half = Decimal::ONE / two;

        for i in 0..self.data_15m.len() {
            let candle = &self.data_15m[i];

            // Advance 1H pointer: find last completed 1H candle whose close_time <= ltf open_time
            while htf_idx + 1 < self.data_1h.len()
                && self.data_1h[htf_idx + 1].close_time <= candle.open_time
            {
                htf_idx += 1;
            }

            // ── Handle open position ───────────────────────────────────────────
            if let Some(st) = stage {
                match st {
                    MomentumStage::PreTp1 { position, tp1_price, atr } => {
                        let sl_hit = match position.direction {
                            PositionDirection::Long => candle.low.0 <= position.sl.0,
                            PositionDirection::Short => candle.high.0 >= position.sl.0,
                        };
                        let tp1_hit = match position.direction {
                            PositionDirection::Long => candle.high.0 >= tp1_price,
                            PositionDirection::Short => candle.low.0 <= tp1_price,
                        };
                        // Session time-exit
                        let force_exit = cfg.enable_session_filter && in_time_exit(candle);

                        if force_exit {
                            // Exit at close, treat as break-even-ish; emit Expense if loss, Winner if gain
                            let exit_price = candle.close.0;
                            let is_winner = match position.direction {
                                PositionDirection::Long => exit_price > position.entry.0,
                                PositionDirection::Short => exit_price < position.entry.0,
                            };
                            let synthetic = Position {
                                tp: DecimalVec(exit_price),
                                ..position
                            };
                            let result = if is_winner { TradeResult::Winner } else { TradeResult::Expense };
                            trades.push(Trade::from_position(synthetic, candle.close_time, result, &cfg.fee_config));
                            stage = None;
                        } else if sl_hit && !tp1_hit {
                            trades.push(Trade::from_position(position, candle.close_time, TradeResult::Expense, &cfg.fee_config));
                            stage = None;
                        } else if tp1_hit {
                            // Transition to PostTp1; trail_stop starts at break-even (entry)
                            let trail_stop = position.entry.0;
                            stage = Some(MomentumStage::PostTp1 { position, tp1_price, trail_stop, atr });
                        }
                        // else: still in trade
                    }
                    MomentumStage::PostTp1 { position, tp1_price, mut trail_stop, atr } => {
                        // Advance trail stop based on candle close
                        let new_trail = match position.direction {
                            PositionDirection::Long => candle.close.0 - cfg.trail_atr_mult * atr,
                            PositionDirection::Short => candle.close.0 + cfg.trail_atr_mult * atr,
                        };
                        // Only advance in favour direction
                        trail_stop = match position.direction {
                            PositionDirection::Long => trail_stop.max(new_trail),
                            PositionDirection::Short => trail_stop.min(new_trail),
                        };

                        let trail_hit = match position.direction {
                            PositionDirection::Long => candle.low.0 <= trail_stop,
                            PositionDirection::Short => candle.high.0 >= trail_stop,
                        };
                        let force_exit = cfg.enable_session_filter && in_time_exit(candle);

                        if trail_hit || force_exit {
                            let trail_exit = if force_exit { candle.close.0 } else { trail_stop };
                            // Blended exit: 50% at TP1, 50% at trail exit
                            let blended = (tp1_price + trail_exit) * half;
                            let synthetic = Position {
                                tp: DecimalVec(blended),
                                ..position
                            };
                            trades.push(Trade::from_position(synthetic, candle.close_time, TradeResult::Winner, &cfg.fee_config));
                            stage = None;
                        } else {
                            // Update trail stop
                            stage = Some(MomentumStage::PostTp1 { position, tp1_price, trail_stop, atr });
                        }
                    }
                }
                // Don't look for new entries while in a trade
                continue;
            }

            // ── Check indicators available ────────────────────────────────────
            if i < cfg.breakout_lookback {
                continue;
            }
            let rsi = match rsi_15m[i] { Some(v) => v, None => continue };
            let atr = match atr_15m[i] { Some(v) => v, None => continue };
            let vol_sma = match vol_sma_15m[i] { Some(v) => v, None => continue };
            let atr_sma = match atr_sma_15m[i] { Some(v) => v, None => continue };
            let ema50 = match ema50_1h[htf_idx] { Some(v) => v, None => continue };
            let htf_close = self.data_1h[htf_idx].close.0;
            let vol = self.volume_15m[i];

            // Session filter: skip open buffer
            if cfg.enable_session_filter && in_open_buffer(candle) {
                continue;
            }

            // Time window filter
            if let Some(ref tw) = cfg.time_window {
                if !tw.allows(candle) {
                    continue;
                }
            }

            // Volume and ATR expansion filter
            let vol_ok = vol >= cfg.volume_multiplier * vol_sma;
            let atr_ok = atr > atr_sma;
            if !vol_ok || !atr_ok {
                continue;
            }

            let close = candle.close.0;

            // Stop placement: candle-low or entry-based
            let (stop_long, stop_short) = match cfg.stop_mode {
                StopMode::CandleLow  => (candle.low.0  - cfg.stop_atr_mult * atr,
                                         candle.high.0 + cfg.stop_atr_mult * atr),
                StopMode::EntryBased => (close - cfg.stop_atr_mult * atr,
                                         close + cfg.stop_atr_mult * atr),
            };

            // ── Long entry ────────────────────────────────────────────────────
            let trend_long = htf_close > ema50;
            let rsi_long = rsi >= cfg.rsi_long_min && rsi <= cfg.rsi_long_max;
            if trend_long && rsi_long {
                let prev_highs_max = (1..=cfg.breakout_lookback)
                    .map(|k| self.data_15m[i - k].high.0)
                    .fold(Decimal::MIN, |a, b| a.max(b));
                if close > prev_highs_max {
                    let tp1 = close + cfg.tp1_atr_mult * atr;
                    let position = Position {
                        direction: PositionDirection::Long,
                        open_time: candle.close_time,
                        entry: DecimalVec(close),
                        sl: DecimalVec(stop_long),
                        tp: DecimalVec(tp1),
                        at_break_even: false,
                    };
                    stage = Some(MomentumStage::PreTp1 { position, tp1_price: tp1, atr });
                    continue;
                }
            }

            // ── Short entry ───────────────────────────────────────────────────
            let trend_short = htf_close < ema50;
            let rsi_short = rsi >= cfg.rsi_short_min && rsi <= cfg.rsi_short_max;
            if trend_short && rsi_short {
                let prev_lows_min = (1..=cfg.breakout_lookback)
                    .map(|k| self.data_15m[i - k].low.0)
                    .fold(Decimal::MAX, |a, b| a.min(b));
                if close < prev_lows_min {
                    let tp1 = close - cfg.tp1_atr_mult * atr;
                    let position = Position {
                        direction: PositionDirection::Short,
                        open_time: candle.close_time,
                        entry: DecimalVec(close),
                        sl: DecimalVec(stop_short),
                        tp: DecimalVec(tp1),
                        at_break_even: false,
                    };
                    stage = Some(MomentumStage::PreTp1 { position, tp1_price: tp1, atr });
                }
            }
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}
