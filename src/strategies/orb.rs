use chrono::{NaiveDate, NaiveTime};
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

// ── Config ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub enum OrbDuration {
    Minutes15,
    Minutes30,
}

#[derive(Clone, Debug)]
pub enum OrbSlType {
    /// SL at the far side of the opening range (opposite of entry direction)
    OppositeRange,
    /// SL = entry ± (range_size × pct / 100)
    RangePct(Decimal),
}

#[derive(Clone, Debug)]
pub struct OrbConfig {
    pub duration: OrbDuration,
    pub sl_type: OrbSlType,
    /// TP = entry ± rr_target × risk
    pub rr_target: Decimal,
    /// Force-close any open position at 16:30 ET
    pub eod_close: bool,
    pub fee_config: FeeConfig,
}

impl Default for OrbConfig {
    fn default() -> Self {
        Self {
            duration: OrbDuration::Minutes30,
            sl_type: OrbSlType::OppositeRange,
            rr_target: Decimal::from(2),
            eod_close: true,
            fee_config: FeeConfig::default(),
        }
    }
}

// ── Strategy struct ──────────────────────────────────────────────────────────

pub struct Orb {
    pub data: Vec<CandleStick>,
    pub config: OrbConfig,
}

// ── Helper ───────────────────────────────────────────────────────────────────

/// End-of-OR window time in NY (exclusive: the first candle >= or_end is after the OR)
fn or_end_time(duration: OrbDuration) -> NaiveTime {
    match duration {
        OrbDuration::Minutes15 => NaiveTime::from_hms_opt(9, 45, 0).unwrap(),
        OrbDuration::Minutes30 => NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
    }
}

/// NYSE open time (NY)
const SESSION_OPEN: (u32, u32) = (9, 30);
/// EOD close time (NY) — check at candle open >= 16:30
const EOD_CLOSE_HOUR: u32 = 16;
const EOD_CLOSE_MIN: u32 = 30;

fn compute_sl(
    entry: Decimal,
    direction: PositionDirection,
    session_high: Decimal,
    session_low: Decimal,
    sl_type: &OrbSlType,
) -> Decimal {
    match sl_type {
        OrbSlType::OppositeRange => match direction {
            PositionDirection::Long => session_low,
            PositionDirection::Short => session_high,
        },
        OrbSlType::RangePct(pct) => {
            let range = session_high - session_low;
            let offset = range * pct / Decimal::from(100);
            match direction {
                PositionDirection::Long => entry - offset,
                PositionDirection::Short => entry + offset,
            }
        }
    }
}

// ── TradingModel implementation ───────────────────────────────────────────────

impl TradingModel for Orb {
    fn execute(&self) -> BacktestResult {
        let or_end = or_end_time(self.config.duration);
        let session_open = NaiveTime::from_hms_opt(SESSION_OPEN.0, SESSION_OPEN.1, 0).unwrap();
        let eod_time = NaiveTime::from_hms_opt(EOD_CLOSE_HOUR, EOD_CLOSE_MIN, 0).unwrap();

        let mut trades: Vec<Trade> = Vec::new();

        // Per-day state
        let mut current_date: Option<NaiveDate> = None;
        let mut session_high: Option<Decimal> = None;
        let mut session_low: Option<Decimal> = None;
        let mut range_complete = false;
        let mut traded_today = false;
        let mut active_position: Option<Position> = None;

        for candle in &self.data {
            let ny_open = to_new_york_time(candle.open_time);
            let ny_date = ny_open.date_naive();
            let ny_time = ny_open.time();

            // ── 1. New day reset ─────────────────────────────────────────
            if current_date != Some(ny_date) {
                current_date = Some(ny_date);
                session_high = None;
                session_low = None;
                range_complete = false;
                traded_today = false;
                // NOTE: active_position carries over to the new day only if eod_close was off.
                // With eod_close = true it will have been closed before midnight.
            }

            // ── 2. EOD close ─────────────────────────────────────────────
            if self.config.eod_close
                && ny_time >= eod_time
                && active_position.is_some()
            {
                let pos = active_position.take().unwrap();
                let exit_price = candle.open.0;

                let profitable = match pos.direction {
                    PositionDirection::Long => exit_price > pos.entry.0,
                    PositionDirection::Short => exit_price < pos.entry.0,
                };

                // Construct a modified position with tp/sl set to the actual exit price
                // so that commission and rr are calculated from the real exit.
                let mut closed_pos = pos;
                let result = if profitable {
                    closed_pos.tp = DecimalVec(exit_price);
                    TradeResult::Winner
                } else {
                    closed_pos.sl = DecimalVec(exit_price);
                    TradeResult::Expense
                };

                let trade = Trade::from_position(closed_pos, candle.open_time, result, &self.config.fee_config);
                trades.push(trade);
                continue;
            }

            // ── 3. Build opening range ────────────────────────────────────
            if ny_time >= session_open && ny_time < or_end {
                session_high = Some(match session_high {
                    None => candle.high.0,
                    Some(h) => h.max(candle.high.0),
                });
                session_low = Some(match session_low {
                    None => candle.low.0,
                    Some(l) => l.min(candle.low.0),
                });
            }

            // ── 4. Mark range complete (once per day) ────────────────────
            if !range_complete && ny_time >= or_end && session_high.is_some() {
                range_complete = true;
            }

            // ── 5. Manage active position ────────────────────────────────
            if let Some(pos) = active_position {
                let closed = match pos.direction {
                    PositionDirection::Long => {
                        if candle.high.0 >= pos.tp.0 {
                            let trade = Trade::from_position(pos, candle.close_time, TradeResult::Winner, &self.config.fee_config);
                            trades.push(trade);
                            true
                        } else if candle.low.0 <= pos.sl.0 {
                            let trade = Trade::from_position(pos, candle.close_time, TradeResult::Expense, &self.config.fee_config);
                            trades.push(trade);
                            true
                        } else {
                            false
                        }
                    }
                    PositionDirection::Short => {
                        if candle.low.0 <= pos.tp.0 {
                            let trade = Trade::from_position(pos, candle.close_time, TradeResult::Winner, &self.config.fee_config);
                            trades.push(trade);
                            true
                        } else if candle.high.0 >= pos.sl.0 {
                            let trade = Trade::from_position(pos, candle.close_time, TradeResult::Expense, &self.config.fee_config);
                            trades.push(trade);
                            true
                        } else {
                            false
                        }
                    }
                };

                if closed {
                    active_position = None;
                }
                // Position managed — don't look for entry this candle
                continue;
            }

            // ── 6. Entry signal ──────────────────────────────────────────
            if range_complete && !traded_today {
                let hi = match session_high {
                    Some(h) => h,
                    None => continue,
                };
                let lo = match session_low {
                    Some(l) => l,
                    None => continue,
                };

                let direction = if candle.close.0 > hi {
                    Some(PositionDirection::Long)
                } else if candle.close.0 < lo {
                    Some(PositionDirection::Short)
                } else {
                    None
                };

                if let Some(dir) = direction {
                    let (entry, sl_raw) = match dir {
                        PositionDirection::Long => {
                            let e = hi;
                            let s = compute_sl(e, dir, hi, lo, &self.config.sl_type);
                            (e, s)
                        }
                        PositionDirection::Short => {
                            let e = lo;
                            let s = compute_sl(e, dir, hi, lo, &self.config.sl_type);
                            (e, s)
                        }
                    };

                    let risk = (entry - sl_raw).abs();
                    if risk <= Decimal::ZERO {
                        continue;
                    }

                    let tp = match dir {
                        PositionDirection::Long => entry + self.config.rr_target * risk,
                        PositionDirection::Short => entry - self.config.rr_target * risk,
                    };

                    let position = Position {
                        direction: dir,
                        open_time: candle.close_time,
                        entry: DecimalVec(entry),
                        sl: DecimalVec(sl_raw),
                        tp: DecimalVec(tp),
                        at_break_even: false,
                    };

                    active_position = Some(position);
                    traded_today = true;
                }
            }
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}
