use std::sync::Arc;

use chrono::{Datelike, NaiveTime, Weekday};
use rayon::prelude::*;
use rust_decimal::Decimal;

use crate::engine::types::{apply_entry_slippage, apply_exit_slippage, ExecutionConfig};
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;

#[derive(Clone, Copy)]
pub enum DayDirection {
    LongOnly,
    LongShort,
}

#[derive(Clone, Copy)]
pub struct DayParams {
    pub tp_pct: Decimal,
    pub sl_pct: Decimal,
    pub min_engulf_pct: Decimal,
    pub max_engulf_pct: Decimal,
    pub direction: DayDirection,
    pub entry_cutoff: NaiveTime,
}

#[derive(Clone)]
pub struct WeekdayEngulfingConfig {
    pub tick_size: Decimal,
    pub point_value_usd: Decimal,
    pub contracts: i32,
    pub max_loss_usd_per_trade: Decimal,
    pub session_start: NaiveTime,
    pub session_end: NaiveTime,
    pub monday: DayParams,
    pub tuesday: DayParams,
    pub wednesday: DayParams,
    pub thursday: DayParams,
    pub friday: DayParams,
    pub execution: ExecutionConfig,
}

impl Default for WeekdayEngulfingConfig {
    fn default() -> Self {
        Self {
            tick_size: Decimal::new(25, 2),
            point_value_usd: Decimal::from(2),
            contracts: 5,
            max_loss_usd_per_trade: Decimal::from(250),
            session_start: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            session_end: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            monday: DayParams {
                tp_pct: Decimal::from(25),
                sl_pct: Decimal::from(50),
                min_engulf_pct: Decimal::ZERO,
                max_engulf_pct: Decimal::from(300),
                direction: DayDirection::LongOnly,
                entry_cutoff: NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
            },
            tuesday: DayParams {
                tp_pct: Decimal::from(50),
                sl_pct: Decimal::from(125),
                min_engulf_pct: Decimal::from(200),
                max_engulf_pct: Decimal::from(300),
                direction: DayDirection::LongOnly,
                entry_cutoff: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            },
            wednesday: DayParams {
                tp_pct: Decimal::from(75),
                sl_pct: Decimal::from(125),
                min_engulf_pct: Decimal::ZERO,
                max_engulf_pct: Decimal::from(300),
                direction: DayDirection::LongOnly,
                entry_cutoff: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
            },
            thursday: DayParams {
                tp_pct: Decimal::from(50),
                sl_pct: Decimal::from(75),
                min_engulf_pct: Decimal::ZERO,
                max_engulf_pct: Decimal::new(5, 1),
                direction: DayDirection::LongOnly,
                entry_cutoff: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            },
            friday: DayParams {
                tp_pct: Decimal::from(25),
                sl_pct: Decimal::from(40),
                min_engulf_pct: Decimal::ZERO,
                max_engulf_pct: Decimal::new(5, 1),
                direction: DayDirection::LongShort,
                entry_cutoff: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            },
            execution: ExecutionConfig::default(),
        }
    }
}

pub struct WeekdayEngulfing {
    pub data: Arc<Vec<CandleStick>>,
    pub config: WeekdayEngulfingConfig,
}

#[derive(Clone)]
pub struct OptimizationRow {
    pub label: String,
    pub cfg: WeekdayEngulfingConfig,
    pub result: BacktestResult,
}

impl WeekdayEngulfingConfig {
    pub fn day_params(&self, weekday: Weekday) -> DayParams {
        match weekday {
            Weekday::Mon => self.monday,
            Weekday::Tue => self.tuesday,
            Weekday::Wed => self.wednesday,
            Weekday::Thu => self.thursday,
            Weekday::Fri => self.friday,
            Weekday::Sat | Weekday::Sun => self.monday,
        }
    }
}

fn in_session(candle: CandleStick, cfg: &WeekdayEngulfingConfig) -> bool {
    let ny = to_new_york_time(candle.open_time);
    let t = ny.time();
    t >= cfg.session_start && t < cfg.session_end
}

fn bullish_engulfing(prev: CandleStick, cur: CandleStick) -> bool {
    prev.close < prev.open
        && cur.close > cur.open
        && cur.open <= prev.close
        && cur.close >= prev.open
}

fn bearish_engulfing(prev: CandleStick, cur: CandleStick) -> bool {
    prev.close > prev.open
        && cur.close < cur.open
        && cur.open >= prev.close
        && cur.close <= prev.open
}

fn engulf_pct(prev: CandleStick, cur: CandleStick) -> Decimal {
    let prev_body = (prev.close.0 - prev.open.0).abs();
    if prev_body <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let cur_body = (cur.close.0 - cur.open.0).abs();
    (cur_body / prev_body) * Decimal::from(100)
}

fn risk_usd(entry: Decimal, stop: Decimal, cfg: &WeekdayEngulfingConfig) -> Decimal {
    let points = (entry - stop).abs();
    points * cfg.point_value_usd * Decimal::from(cfg.contracts)
}

impl TradingModel for WeekdayEngulfing {
    fn execute(&self) -> BacktestResult {
        let mut trades = Vec::new();
        if self.data.len() < 3 {
            return BacktestResult {
                trades,
                capital: Decimal::from(1000),
            };
        }

        let mut i = 1usize;
        while i + 1 < self.data.len() {
            let prev = self.data[i - 1];
            let sig = self.data[i];
            let entry_candle = self.data[i + 1];
            if !in_session(sig, &self.config) || !in_session(entry_candle, &self.config) {
                i += 1;
                continue;
            }

            let weekday = to_new_york_time(sig.open_time).weekday();
            if matches!(weekday, Weekday::Sat | Weekday::Sun) {
                i += 1;
                continue;
            }

            let params = self.config.day_params(weekday);
            let engulf = engulf_pct(prev, sig);
            if engulf < params.min_engulf_pct || engulf > params.max_engulf_pct {
                i += 1;
                continue;
            }

            let sig_time = to_new_york_time(sig.open_time).time();
            if sig_time >= params.entry_cutoff {
                i += 1;
                continue;
            }

            let mut direction: Option<PositionDirection> = None;
            if bullish_engulfing(prev, sig) {
                direction = Some(PositionDirection::Long);
            } else if bearish_engulfing(prev, sig)
                && matches!(params.direction, DayDirection::LongShort)
            {
                direction = Some(PositionDirection::Short);
            }

            let Some(direction) = direction else {
                i += 1;
                continue;
            };

            let entry = apply_entry_slippage(direction, entry_candle.open, &self.config.execution);
            let sig_range = sig.high.0 - sig.low.0;
            if sig_range <= Decimal::ZERO {
                i += 1;
                continue;
            }

            let sl_distance = sig_range * (params.sl_pct / Decimal::from(100));
            let tp_distance = sig_range * (params.tp_pct / Decimal::from(100));
            if sl_distance <= Decimal::ZERO || tp_distance <= Decimal::ZERO {
                i += 1;
                continue;
            }

            let stop = match direction {
                PositionDirection::Long => DecimalVec(entry.0 - sl_distance),
                PositionDirection::Short => DecimalVec(entry.0 + sl_distance),
            };

            if risk_usd(entry.0, stop.0, &self.config) > self.config.max_loss_usd_per_trade {
                i += 1;
                continue;
            }

            let target = match direction {
                PositionDirection::Long => DecimalVec(entry.0 + tp_distance),
                PositionDirection::Short => DecimalVec(entry.0 - tp_distance),
            };

            let position = Position {
                direction,
                open_time: entry_candle.open_time,
                entry,
                sl: stop,
                tp: target,
                at_break_even: false,
            };

            let mut closed_trade: Option<Trade> = None;
            let mut j = i + 1;
            while j < self.data.len() {
                let c = self.data[j];
                if !in_session(c, &self.config) {
                    let exit = apply_exit_slippage(direction, c.open, &self.config.execution);
                    let profitable = match direction {
                        PositionDirection::Long => exit.0 > entry.0,
                        PositionDirection::Short => exit.0 < entry.0,
                    };
                    let result = if profitable {
                        TradeResult::Winner
                    } else {
                        TradeResult::Expense
                    };
                    closed_trade = Some(Trade::from_position_with_exit(
                        position,
                        c.open_time,
                        exit,
                        result,
                        &self.config.execution,
                    ));
                    break;
                }

                let (sl_hit, tp_hit) = match direction {
                    PositionDirection::Long => (c.low.0 <= stop.0, c.high.0 >= target.0),
                    PositionDirection::Short => (c.high.0 >= stop.0, c.low.0 <= target.0),
                };

                if sl_hit || tp_hit {
                    let (exit_price, result) = if tp_hit && !sl_hit {
                        (target, TradeResult::Winner)
                    } else if sl_hit && !tp_hit {
                        (stop, TradeResult::Expense)
                    } else {
                        (stop, TradeResult::Expense)
                    };
                    let slipped_exit = apply_exit_slippage(direction, exit_price, &self.config.execution);
                    closed_trade = Some(Trade::from_position_with_exit(
                        position,
                        c.close_time,
                        slipped_exit,
                        result,
                        &self.config.execution,
                    ));
                    break;
                }
                j += 1;
            }

            if let Some(trade) = closed_trade {
                trades.push(trade);
                i = j;
            } else {
                break;
            }
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}

pub fn resample_from_1m(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if minutes <= 1 || data.is_empty() {
        return data.to_vec();
    }
    let bucket = minutes * 60;
    let mut out = Vec::new();
    let mut cur = data[0];
    let mut current_bucket = cur.open_time / bucket;
    for candle in data.iter().copied().skip(1) {
        let b = candle.open_time / bucket;
        if b != current_bucket {
            out.push(cur);
            cur = candle;
            current_bucket = b;
        } else {
            if candle.high > cur.high {
                cur.high = candle.high;
            }
            if candle.low < cur.low {
                cur.low = candle.low;
            }
            cur.close = candle.close;
            cur.close_time = candle.close_time;
        }
    }
    out.push(cur);
    out
}

pub fn optimize_configs(
    data: Arc<Vec<CandleStick>>,
    labeled_configs: Vec<(String, WeekdayEngulfingConfig)>,
) -> Vec<OptimizationRow> {
    labeled_configs
        .into_par_iter()
        .map(|(label, cfg)| {
            let result = WeekdayEngulfing {
                data: Arc::clone(&data),
                config: cfg.clone(),
            }
            .execute();
            OptimizationRow { label, cfg, result }
        })
        .collect()
}
