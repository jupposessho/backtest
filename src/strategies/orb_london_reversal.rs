use chrono::NaiveTime;
use chrono_tz::Europe::London;
use rust_decimal::Decimal;

use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;

#[derive(Clone, Debug)]
pub struct OrbLondonReversalConfig {
    pub orb_start: NaiveTime,
    pub orb_end: NaiveTime,
    pub session_end: NaiveTime,
    pub eod_close: bool,
    pub min_first_break_excursion_pct_of_orb: Decimal,
    pub max_bars_to_reenter: Option<usize>,
    pub breakeven_at_r: Option<Decimal>,
    pub time_stop_bars: Option<usize>,
}

impl Default for OrbLondonReversalConfig {
    fn default() -> Self {
        Self {
            orb_start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            orb_end: NaiveTime::from_hms_opt(8, 15, 0).unwrap(),
            session_end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            eod_close: true,
            min_first_break_excursion_pct_of_orb: Decimal::from(20),
            max_bars_to_reenter: Some(24),
            breakeven_at_r: Some(Decimal::new(6, 1)),
            time_stop_bars: Some(36),
        }
    }
}

pub struct OrbLondonReversal {
    pub data: Vec<CandleStick>,
    pub config: OrbLondonReversalConfig,
}

#[derive(Clone, Copy)]
enum FirstBreak {
    High,
    Low,
}

impl TradingModel for OrbLondonReversal {
    fn execute(&self) -> BacktestResult {
        let mut trades: Vec<Trade> = Vec::new();

        let mut current_day = None;
        let mut orb_high: Option<Decimal> = None;
        let mut orb_low: Option<Decimal> = None;
        let mut range_ready = false;
        let mut traded_today = false;

        let mut first_break: Option<FirstBreak> = None;
        let mut first_break_extreme: Option<Decimal> = None;
        let mut bars_since_first_break: usize = 0;

        let mut active_position: Option<Position> = None;
        let mut bars_in_position: usize = 0;

        for candle in &self.data {
            let london_open = chrono::DateTime::from_timestamp(candle.open_time, 0)
                .unwrap()
                .with_timezone(&London);
            let london_date = london_open.date_naive();
            let london_time = london_open.time();

            if current_day != Some(london_date) {
                current_day = Some(london_date);
                orb_high = None;
                orb_low = None;
                range_ready = false;
                traded_today = false;
                first_break = None;
                first_break_extreme = None;
                bars_since_first_break = 0;
                bars_in_position = 0;
            }

            if self.config.eod_close && london_time >= self.config.session_end && active_position.is_some() {
                let mut pos = active_position.take().unwrap();
                let exit_price = candle.open.0;
                let result = match pos.direction {
                    PositionDirection::Long if exit_price > pos.entry.0 => {
                        pos.tp = DecimalVec(exit_price);
                        TradeResult::Winner
                    }
                    PositionDirection::Short if exit_price < pos.entry.0 => {
                        pos.tp = DecimalVec(exit_price);
                        TradeResult::Winner
                    }
                    _ => {
                        pos.sl = DecimalVec(exit_price);
                        TradeResult::Expense
                    }
                };

                trades.push(Trade::from_position(pos, candle.open_time, result));
                continue;
            }

            if london_time >= self.config.orb_start && london_time < self.config.orb_end {
                orb_high = Some(match orb_high {
                    Some(v) => v.max(candle.high.0),
                    None => candle.high.0,
                });
                orb_low = Some(match orb_low {
                    Some(v) => v.min(candle.low.0),
                    None => candle.low.0,
                });
            }

            if !range_ready && london_time >= self.config.orb_end && orb_high.is_some() && orb_low.is_some() {
                range_ready = true;
            }

            if let Some(mut pos) = active_position {
                bars_in_position += 1;

                if let Some(be_r) = self.config.breakeven_at_r {
                    if be_r > Decimal::ZERO {
                        let risk = match pos.direction {
                            PositionDirection::Long => pos.entry.0 - pos.sl.0,
                            PositionDirection::Short => pos.sl.0 - pos.entry.0,
                        };
                        if risk > Decimal::ZERO {
                            let trigger = match pos.direction {
                                PositionDirection::Long => pos.entry.0 + risk * be_r,
                                PositionDirection::Short => pos.entry.0 - risk * be_r,
                            };
                            let should_be = match pos.direction {
                                PositionDirection::Long => candle.high.0 >= trigger,
                                PositionDirection::Short => candle.low.0 <= trigger,
                            };
                            if should_be {
                                pos.at_break_even = true;
                            }
                        }
                    }
                }

                let closed = match pos.direction {
                    PositionDirection::Long => {
                        if candle.high.0 >= pos.tp.0 {
                            trades.push(Trade::from_position(pos, candle.close_time, TradeResult::Winner));
                            true
                        } else if pos.at_break_even && candle.low.0 <= pos.entry.0 {
                            let mut be_pos = pos;
                            be_pos.tp = be_pos.entry;
                            trades.push(Trade::from_position(be_pos, candle.close_time, TradeResult::BreakEven));
                            true
                        } else if candle.low.0 <= pos.sl.0 {
                            trades.push(Trade::from_position(pos, candle.close_time, TradeResult::Expense));
                            true
                        } else {
                            false
                        }
                    }
                    PositionDirection::Short => {
                        if candle.low.0 <= pos.tp.0 {
                            trades.push(Trade::from_position(pos, candle.close_time, TradeResult::Winner));
                            true
                        } else if pos.at_break_even && candle.high.0 >= pos.entry.0 {
                            let mut be_pos = pos;
                            be_pos.tp = be_pos.entry;
                            trades.push(Trade::from_position(be_pos, candle.close_time, TradeResult::BreakEven));
                            true
                        } else if candle.high.0 >= pos.sl.0 {
                            trades.push(Trade::from_position(pos, candle.close_time, TradeResult::Expense));
                            true
                        } else {
                            false
                        }
                    }
                };

                if closed {
                    active_position = None;
                    bars_in_position = 0;
                    continue;
                }

                if let Some(max_bars) = self.config.time_stop_bars {
                    if bars_in_position >= max_bars {
                        let mut timed = pos;
                        let exit = candle.close.0;
                        let result = match timed.direction {
                            PositionDirection::Long if exit > timed.entry.0 => {
                                timed.tp = DecimalVec(exit);
                                TradeResult::Winner
                            }
                            PositionDirection::Short if exit < timed.entry.0 => {
                                timed.tp = DecimalVec(exit);
                                TradeResult::Winner
                            }
                            _ => {
                                timed.sl = DecimalVec(exit);
                                TradeResult::Expense
                            }
                        };
                        trades.push(Trade::from_position(timed, candle.close_time, result));
                        active_position = None;
                        bars_in_position = 0;
                        continue;
                    }
                }

                active_position = Some(pos);
                continue;
            }

            if !range_ready || traded_today {
                continue;
            }

            let hi = match orb_high {
                Some(v) => v,
                None => continue,
            };
            let lo = match orb_low {
                Some(v) => v,
                None => continue,
            };

            if first_break.is_none() {
                if candle.close.0 > hi {
                    first_break = Some(FirstBreak::High);
                    first_break_extreme = Some(candle.high.0);
                    bars_since_first_break = 0;
                } else if candle.close.0 < lo {
                    first_break = Some(FirstBreak::Low);
                    first_break_extreme = Some(candle.low.0);
                    bars_since_first_break = 0;
                }
                continue;
            }

            bars_since_first_break += 1;
            if let Some(max_bars) = self.config.max_bars_to_reenter {
                if bars_since_first_break > max_bars {
                    traded_today = true;
                    continue;
                }
            }

            let range = hi - lo;
            if range <= Decimal::ZERO {
                continue;
            }

            match first_break.unwrap() {
                FirstBreak::High => {
                    first_break_extreme = Some(
                        first_break_extreme
                            .map(|v| v.max(candle.high.0))
                            .unwrap_or(candle.high.0),
                    );

                    if candle.close.0 <= hi && candle.close.0 >= lo {
                        let excursion_pct = ((first_break_extreme.unwrap_or(candle.high.0) - hi) / range)
                            * Decimal::from(100);
                        if excursion_pct < self.config.min_first_break_excursion_pct_of_orb {
                            traded_today = true;
                            continue;
                        }

                        let entry = candle.close.0;
                        let sl = first_break_extreme.unwrap_or(candle.high.0);
                        let tp = lo;
                        let risk = sl - entry;

                        if risk > Decimal::ZERO {
                            active_position = Some(Position {
                                direction: PositionDirection::Short,
                                open_time: candle.close_time,
                                entry: DecimalVec(entry),
                                sl: DecimalVec(sl),
                                tp: DecimalVec(tp),
                                at_break_even: false,
                            });
                            traded_today = true;
                            bars_in_position = 0;
                        }
                    }
                }
                FirstBreak::Low => {
                    first_break_extreme = Some(
                        first_break_extreme
                            .map(|v| v.min(candle.low.0))
                            .unwrap_or(candle.low.0),
                    );

                    if candle.close.0 >= lo && candle.close.0 <= hi {
                        let excursion_pct = ((lo - first_break_extreme.unwrap_or(candle.low.0)) / range)
                            * Decimal::from(100);
                        if excursion_pct < self.config.min_first_break_excursion_pct_of_orb {
                            traded_today = true;
                            continue;
                        }

                        let entry = candle.close.0;
                        let sl = first_break_extreme.unwrap_or(candle.low.0);
                        let tp = hi;
                        let risk = entry - sl;

                        if risk > Decimal::ZERO {
                            active_position = Some(Position {
                                direction: PositionDirection::Long,
                                open_time: candle.close_time,
                                entry: DecimalVec(entry),
                                sl: DecimalVec(sl),
                                tp: DecimalVec(tp),
                                at_break_even: false,
                            });
                            traded_today = true;
                            bars_in_position = 0;
                        }
                    }
                }
            }
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
    use chrono::TimeZone;

    fn ts(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> i64 {
        London
            .with_ymd_and_hms(y, m, d, hh, mm, 0)
            .single()
            .unwrap()
            .timestamp()
    }

    fn candle(y: i32, m: u32, d: u32, hh: u32, mm: u32, o: Decimal, h: Decimal, l: Decimal, c: Decimal) -> CandleStick {
        CandleStick {
            open_time: ts(y, m, d, hh, mm),
            close_time: ts(y, m, d, hh, mm + 1),
            open: DecimalVec(o),
            high: DecimalVec(h),
            low: DecimalVec(l),
            close: DecimalVec(c),
        }
    }

    #[test]
    fn enters_short_after_high_break_reentry_and_hits_orb_low() {
        let data = vec![
            candle(2026, 1, 5, 8, 0, Decimal::from(100), Decimal::from(103), Decimal::from(99), Decimal::from(102)),
            candle(2026, 1, 5, 8, 5, Decimal::from(102), Decimal::from(105), Decimal::from(101), Decimal::from(104)),
            candle(2026, 1, 5, 8, 10, Decimal::from(104), Decimal::from(106), Decimal::from(102), Decimal::from(105)),
            candle(2026, 1, 5, 8, 15, Decimal::from(105), Decimal::from(108), Decimal::from(104), Decimal::from(107)),
            candle(2026, 1, 5, 8, 20, Decimal::from(107), Decimal::from(108), Decimal::from(104), Decimal::from(105)),
            candle(2026, 1, 5, 8, 25, Decimal::from(105), Decimal::from(106), Decimal::from(99), Decimal::from(100)),
        ];

        let strategy = OrbLondonReversal {
            data,
            config: OrbLondonReversalConfig::default(),
        };

        let result = strategy.execute();
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].direction, PositionDirection::Short);
        assert_eq!(result.trades[0].result, TradeResult::Winner);
    }

    #[test]
    fn enters_long_after_low_break_reentry_and_stops_out() {
        let data = vec![
            candle(2026, 1, 6, 8, 0, Decimal::from(200), Decimal::from(204), Decimal::from(199), Decimal::from(203)),
            candle(2026, 1, 6, 8, 5, Decimal::from(203), Decimal::from(205), Decimal::from(201), Decimal::from(204)),
            candle(2026, 1, 6, 8, 10, Decimal::from(204), Decimal::from(206), Decimal::from(202), Decimal::from(205)),
            candle(2026, 1, 6, 8, 15, Decimal::from(205), Decimal::from(206), Decimal::from(198), Decimal::from(198)),
            candle(2026, 1, 6, 8, 20, Decimal::from(198), Decimal::from(202), Decimal::from(197), Decimal::from(200)),
            candle(2026, 1, 6, 8, 25, Decimal::from(200), Decimal::from(201), Decimal::from(196), Decimal::from(197)),
        ];

        let strategy = OrbLondonReversal {
            data,
            config: OrbLondonReversalConfig::default(),
        };

        let result = strategy.execute();
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].direction, PositionDirection::Long);
        assert_eq!(result.trades[0].result, TradeResult::Expense);
    }
}
