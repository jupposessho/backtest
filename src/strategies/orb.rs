use chrono::{NaiveDate, NaiveTime};
use rust_decimal::Decimal;

use crate::engine::types::{apply_entry_slippage, apply_exit_slippage, ExecutionConfig};
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

#[derive(Clone, Copy, Debug)]
pub enum OrbEntryModel {
    Boundary,
    NextBarOpen,
}

#[derive(Clone, Debug)]
pub struct OrbConfig {
    pub duration: OrbDuration,
    pub active_window_minutes: usize,
    pub sl_type: OrbSlType,
    /// TP = entry ± rr_target × risk
    pub rr_target: Decimal,
    /// Force-close any open position at 16:30 ET
    pub eod_close: bool,
    pub max_hold_bars: Option<usize>,
    pub retest_mode: bool,
    pub retest_max_bars: usize,
    pub entry_model: OrbEntryModel,
    pub conservative_intrabar: bool,
    pub execution: ExecutionConfig,
    pub fee_config: FeeConfig,
}

impl Default for OrbConfig {
    fn default() -> Self {
        Self {
            duration: OrbDuration::Minutes30,
            active_window_minutes: 6 * 60,
            sl_type: OrbSlType::OppositeRange,
            rr_target: Decimal::from(2),
            eod_close: true,
            max_hold_bars: None,
            retest_mode: false,
            retest_max_bars: 12,
            entry_model: OrbEntryModel::Boundary,
            conservative_intrabar: false,
            execution: ExecutionConfig::default(),
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

fn fee_rate_pct_to_rate(pct: Decimal) -> Decimal {
    pct / Decimal::from(100)
}

fn build_trade_with_exit(
    position: Position,
    close_time: i64,
    exit: DecimalVec,
    result: TradeResult,
    cfg: &OrbConfig,
) -> Trade {
    let entry_notional = position.entry.0;
    let exit_notional = exit.0;
    let maker_rate = fee_rate_pct_to_rate(cfg.fee_config.maker_fee_pct);
    let taker_rate = fee_rate_pct_to_rate(cfg.fee_config.taker_fee_pct);
    let commission = entry_notional * maker_rate + exit_notional * taker_rate;
    let slippage = (cfg.execution.tick_size
        * Decimal::from(cfg.execution.slippage_ticks_per_side).abs())
    .abs()
        * Decimal::from(2);
    Trade {
        direction: position.direction,
        open_time: position.open_time,
        close_time,
        entry: position.entry,
        sl: position.sl,
        tp: exit,
        result,
        commission,
        slippage,
        fees: Decimal::ZERO,
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
        let mut active_position_open_index: Option<usize> = None;
        let mut index_in_day: usize = 0;
        let mut break_direction: Option<PositionDirection> = None;
        let mut break_index_in_day: Option<usize> = None;
        let mut pending_entry: Option<(PositionDirection, Decimal, Decimal)> = None;

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
                active_position_open_index = None;
                index_in_day = 0;
                break_direction = None;
                break_index_in_day = None;
                pending_entry = None;
                // NOTE: active_position carries over to the new day only if eod_close was off.
                // With eod_close = true it will have been closed before midnight.
            }

            // ── 2. EOD close ─────────────────────────────────────────────
            if self.config.eod_close && ny_time >= eod_time && active_position.is_some() {
                let pos = active_position.take().unwrap();
                let exit_price =
                    apply_exit_slippage(pos.direction, candle.open, &self.config.execution).0;

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

                let trade = build_trade_with_exit(
                    closed_pos,
                    candle.open_time,
                    DecimalVec(exit_price),
                    result,
                    &self.config,
                );
                trades.push(trade);
                active_position_open_index = None;
                continue;
            }

            if active_position.is_none() {
                if let Some((dir, hi, lo)) = pending_entry.take() {
                    let entry = apply_entry_slippage(dir, candle.open, &self.config.execution).0;
                    let sl_raw = compute_sl(entry, dir, hi, lo, &self.config.sl_type);
                    let risk = (entry - sl_raw).abs();
                    if risk > Decimal::ZERO {
                        let tp = match dir {
                            PositionDirection::Long => entry + self.config.rr_target * risk,
                            PositionDirection::Short => entry - self.config.rr_target * risk,
                        };
                        active_position = Some(Position {
                            direction: dir,
                            open_time: candle.open_time,
                            entry: DecimalVec(entry),
                            sl: DecimalVec(sl_raw),
                            tp: DecimalVec(tp),
                            at_break_even: false,
                        });
                        active_position_open_index = Some(index_in_day);
                        traded_today = true;
                    }
                }
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
                if let Some(max_hold) = self.config.max_hold_bars {
                    if let Some(open_i) = active_position_open_index {
                        if index_in_day.saturating_sub(open_i) >= max_hold {
                            let exit_price = apply_exit_slippage(
                                pos.direction,
                                candle.close,
                                &self.config.execution,
                            )
                            .0;
                            let profitable = match pos.direction {
                                PositionDirection::Long => exit_price > pos.entry.0,
                                PositionDirection::Short => exit_price < pos.entry.0,
                            };
                            let mut closed_pos = pos;
                            let result = if profitable {
                                closed_pos.tp = DecimalVec(exit_price);
                                TradeResult::Winner
                            } else {
                                closed_pos.sl = DecimalVec(exit_price);
                                TradeResult::Expense
                            };
                            let trade = build_trade_with_exit(
                                closed_pos,
                                candle.close_time,
                                DecimalVec(exit_price),
                                result,
                                &self.config,
                            );
                            trades.push(trade);
                            active_position = None;
                            active_position_open_index = None;
                            index_in_day += 1;
                            continue;
                        }
                    }
                }
                let closed = match pos.direction {
                    PositionDirection::Long => {
                        if candle.low.0 <= pos.sl.0 && self.config.conservative_intrabar {
                            let stop_fill = if candle.open.0 <= pos.sl.0 {
                                apply_exit_slippage(
                                    pos.direction,
                                    candle.open,
                                    &self.config.execution,
                                )
                            } else {
                                apply_exit_slippage(pos.direction, pos.sl, &self.config.execution)
                            };
                            let trade = build_trade_with_exit(
                                pos,
                                candle.close_time,
                                stop_fill,
                                TradeResult::Expense,
                                &self.config,
                            );
                            trades.push(trade);
                            true
                        } else if candle.high.0 >= pos.tp.0 {
                            let tp_fill =
                                apply_exit_slippage(pos.direction, pos.tp, &self.config.execution);
                            let trade = build_trade_with_exit(
                                pos,
                                candle.close_time,
                                tp_fill,
                                TradeResult::Winner,
                                &self.config,
                            );
                            trades.push(trade);
                            true
                        } else if candle.low.0 <= pos.sl.0 {
                            let stop_fill = if candle.open.0 <= pos.sl.0 {
                                apply_exit_slippage(
                                    pos.direction,
                                    candle.open,
                                    &self.config.execution,
                                )
                            } else {
                                apply_exit_slippage(pos.direction, pos.sl, &self.config.execution)
                            };
                            let trade = build_trade_with_exit(
                                pos,
                                candle.close_time,
                                stop_fill,
                                TradeResult::Expense,
                                &self.config,
                            );
                            trades.push(trade);
                            true
                        } else {
                            false
                        }
                    }
                    PositionDirection::Short => {
                        if candle.high.0 >= pos.sl.0 && self.config.conservative_intrabar {
                            let stop_fill = if candle.open.0 >= pos.sl.0 {
                                apply_exit_slippage(
                                    pos.direction,
                                    candle.open,
                                    &self.config.execution,
                                )
                            } else {
                                apply_exit_slippage(pos.direction, pos.sl, &self.config.execution)
                            };
                            let trade = build_trade_with_exit(
                                pos,
                                candle.close_time,
                                stop_fill,
                                TradeResult::Expense,
                                &self.config,
                            );
                            trades.push(trade);
                            true
                        } else if candle.low.0 <= pos.tp.0 {
                            let tp_fill =
                                apply_exit_slippage(pos.direction, pos.tp, &self.config.execution);
                            let trade = build_trade_with_exit(
                                pos,
                                candle.close_time,
                                tp_fill,
                                TradeResult::Winner,
                                &self.config,
                            );
                            trades.push(trade);
                            true
                        } else if candle.high.0 >= pos.sl.0 {
                            let stop_fill = if candle.open.0 >= pos.sl.0 {
                                apply_exit_slippage(
                                    pos.direction,
                                    candle.open,
                                    &self.config.execution,
                                )
                            } else {
                                apply_exit_slippage(pos.direction, pos.sl, &self.config.execution)
                            };
                            let trade = build_trade_with_exit(
                                pos,
                                candle.close_time,
                                stop_fill,
                                TradeResult::Expense,
                                &self.config,
                            );
                            trades.push(trade);
                            true
                        } else {
                            false
                        }
                    }
                };

                if closed {
                    active_position = None;
                    active_position_open_index = None;
                }
                // Position managed — don't look for entry this candle
                index_in_day += 1;
                continue;
            }

            // ── 6. Entry signal ──────────────────────────────────────────
            if range_complete && !traded_today {
                let minutes_after_or = ny_time.signed_duration_since(or_end).num_minutes();
                if minutes_after_or < 0
                    || minutes_after_or as usize > self.config.active_window_minutes
                {
                    index_in_day += 1;
                    continue;
                }
                let hi = match session_high {
                    Some(h) => h,
                    None => {
                        index_in_day += 1;
                        continue;
                    }
                };
                let lo = match session_low {
                    Some(l) => l,
                    None => {
                        index_in_day += 1;
                        continue;
                    }
                };

                let direction = if self.config.retest_mode {
                    match break_direction {
                        None => {
                            if candle.close.0 > hi {
                                break_direction = Some(PositionDirection::Long);
                                break_index_in_day = Some(index_in_day);
                            } else if candle.close.0 < lo {
                                break_direction = Some(PositionDirection::Short);
                                break_index_in_day = Some(index_in_day);
                            }
                            None
                        }
                        Some(PositionDirection::Long) => {
                            let within = break_index_in_day
                                .map(|b| {
                                    index_in_day.saturating_sub(b) <= self.config.retest_max_bars
                                })
                                .unwrap_or(false);
                            if !within {
                                break_direction = None;
                                break_index_in_day = None;
                                None
                            } else if candle.low.0 <= hi {
                                Some(PositionDirection::Long)
                            } else {
                                None
                            }
                        }
                        Some(PositionDirection::Short) => {
                            let within = break_index_in_day
                                .map(|b| {
                                    index_in_day.saturating_sub(b) <= self.config.retest_max_bars
                                })
                                .unwrap_or(false);
                            if !within {
                                break_direction = None;
                                break_index_in_day = None;
                                None
                            } else if candle.high.0 >= lo {
                                Some(PositionDirection::Short)
                            } else {
                                None
                            }
                        }
                    }
                } else if candle.close.0 > hi {
                    Some(PositionDirection::Long)
                } else if candle.close.0 < lo {
                    Some(PositionDirection::Short)
                } else {
                    None
                };

                if let Some(dir) = direction {
                    match self.config.entry_model {
                        OrbEntryModel::Boundary => {
                            let boundary_entry = match dir {
                                PositionDirection::Long => DecimalVec(hi),
                                PositionDirection::Short => DecimalVec(lo),
                            };
                            let entry =
                                apply_entry_slippage(dir, boundary_entry, &self.config.execution).0;
                            let sl_raw = compute_sl(entry, dir, hi, lo, &self.config.sl_type);
                            let risk = (entry - sl_raw).abs();
                            if risk <= Decimal::ZERO {
                                index_in_day += 1;
                                continue;
                            }
                            let tp = match dir {
                                PositionDirection::Long => entry + self.config.rr_target * risk,
                                PositionDirection::Short => entry - self.config.rr_target * risk,
                            };
                            active_position = Some(Position {
                                direction: dir,
                                open_time: candle.close_time,
                                entry: DecimalVec(entry),
                                sl: DecimalVec(sl_raw),
                                tp: DecimalVec(tp),
                                at_break_even: false,
                            });
                            active_position_open_index = Some(index_in_day);
                            traded_today = true;
                        }
                        OrbEntryModel::NextBarOpen => {
                            pending_entry = Some((dir, hi, lo));
                        }
                    }
                }
            }
            index_in_day += 1;
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}
