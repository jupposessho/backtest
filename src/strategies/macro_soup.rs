use chrono::Duration;
use itertools::Itertools;
use rust_decimal::Decimal;

use crate::model::backtest_result::BacktestResult;
use crate::model::candle_ny::CandleNY;
use crate::model::decimal::DecimalVec;
use crate::model::position::Position;
use crate::model::session::Session;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;

use super::lib::in_session;
use crate::model::position_direction::PositionDirection;

pub struct MacroSoup {
    pub rr_threshold: Decimal,
    pub session: Session,
    pub candles: Vec<CandleNY>,
    pub max_duration_min: i64,
    pub be_threshold: Option<DecimalVec>,
}

impl MacroSoup {
    // looking for candles out of the range
    pub fn trigger_or_invalidation(
        candles: Vec<&CandleNY>,
        session_high: DecimalVec,
        session_low: DecimalVec,
        max_duration_min: i64,
    ) -> Option<Position> {
        if candles.len() < 1 {
            return None;
        }
        let time_threshold =
            candles.first().unwrap().open_time + Duration::minutes(max_duration_min);
        let mut out_max: Option<DecimalVec> = None;
        let mut out_min: Option<DecimalVec> = None;
        // let out_direction_up = None;
        for actual in candles.clone() {
            if actual.open_time >= time_threshold {
                return None;
            }

            // raid to the upside
            if actual.high > session_high {
                out_max = out_max
                    .map(|max| if actual.high > max { actual.high } else { max })
                    .or(Some(actual.high));
            }

            if let Some(max) = out_max {
                // TODO: check w/ and w/o actual.bearish check
                if actual.close < session_high && actual.clone().bearish() {
                    return Some(Position {
                        direction: PositionDirection::Short,
                        open_time: actual.open_time.timestamp(), // TODO: check with proper timezone
                        entry: actual.close,
                        sl: max,
                        tp: session_low - (session_high - session_low), // stdv1,
                        at_break_even: false,
                    });
                }
            }

            // raid to the downside
            if actual.low < session_low {
                out_min = out_min
                    .map(|min| if actual.low < min { actual.low } else { min })
                    .or(Some(actual.low));
            }

            if let Some(min) = out_min {
                // TODO: check w/ and w/o actual.bearish check
                if actual.close > session_low && actual.clone().bullish() {
                    return Some(Position {
                        direction: PositionDirection::Long,
                        open_time: actual.open_time.timestamp(), // TODO: check with proper timezone
                        entry: actual.close,
                        sl: min,
                        tp: session_high + (session_high - session_low), // stdv1
                        at_break_even: false,
                    });
                }
            }
        }
        None
    }

    // TODO: test
    pub fn run_trade(
        position: Position,
        candles: Vec<&CandleNY>,
        be_threshold: Option<DecimalVec>,
    ) -> Option<Trade> {
        let mut p = position.clone();
        for actual in candles {
            match p.direction {
                PositionDirection::Short => {
                    if p.sl < actual.high {
                        return Some(Trade::from_position(
                            p,
                            actual.open_time.timestamp(),
                            if p.at_break_even {
                                TradeResult::BreakEven
                            } else {
                                TradeResult::Expense
                            },
                        ));
                    }
                    if p.tp > actual.low {
                        return Some(Trade::from_position(
                            p,
                            actual.open_time.timestamp(),
                            TradeResult::Winner,
                        ));
                    }
                    if let Some(bet) = be_threshold {
                        if actual.low < p.entry && p.actual_rr(actual.low) > bet {
                            p.move_to_break_even();
                        }
                    }
                }
                PositionDirection::Long => {
                    if p.sl > actual.low {
                        return Some(Trade::from_position(
                            p,
                            actual.open_time.timestamp(),
                            if p.at_break_even {
                                TradeResult::BreakEven
                            } else {
                                TradeResult::Expense
                            },
                        ));
                    }
                    if p.tp < actual.high {
                        return Some(Trade::from_position(
                            p,
                            actual.open_time.timestamp(),
                            TradeResult::Winner,
                        ));
                    }
                    if let Some(bet) = be_threshold {
                        if actual.high > p.entry && p.actual_rr(actual.high) > bet {
                            p.move_to_break_even();
                        }
                    }
                }
            }
        }
        return None;
    }
}

impl TradingModel for MacroSoup {
    fn execute(&self) -> BacktestResult {
        let mut trades: Vec<Trade> = vec![];
        let mut session_high: Option<DecimalVec> = None;
        let mut session_low: Option<DecimalVec> = None;
        let mut last_candle_in_session = false;

        for actual in self.candles.clone() {
            if in_session(&self.session, actual.open_time) {
                match session_low {
                    Some(s) => {
                        if s > actual.low {
                            session_low = Some(actual.low)
                        }
                    }
                    None => session_low = Some(actual.low),
                }
                match session_high {
                    Some(s) => {
                        if s < actual.high {
                            session_high = Some(actual.high)
                        }
                    }
                    None => session_high = Some(actual.high),
                }
                last_candle_in_session = true;
            } else if last_candle_in_session {
                let c = self.candles.clone();
                let candles_after_session = c
                    .iter()
                    .skip_while(|x| x.open_time <= actual.open_time)
                    .collect_vec();
                // this is the first candle after the session ended
                // find trigger + run trade
                if let Some(position) = Self::trigger_or_invalidation(
                    candles_after_session,
                    session_high.unwrap(),
                    session_low.unwrap(),
                    self.max_duration_min,
                ) {
                    if position.rr().0 >= self.rr_threshold {
                        let c = self.candles.clone();
                        let candles_after_entry = c
                            .iter()
                            .skip_while(|x| x.open_time.timestamp() <= position.open_time)
                            .collect_vec();
                        let trade =
                            Self::run_trade(position, candles_after_entry, self.be_threshold);
                        if let Some(t) = trade {
                            trades.push(t)
                        }
                    }
                }
                last_candle_in_session = false;
                session_low = None;
                session_high = None;
            }
        }

        BacktestResult { trades }
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;

    use super::*;
    use crate::parse_datetime;
    use rust_decimal::Decimal;

    fn date(date_time: &str) -> chrono::DateTime<chrono_tz::Tz> {
        parse_datetime(date_time).unwrap()
    }

    fn candlestick(duration: i64, open: i32, high: i32, low: i32, close: i32) -> CandleNY {
        CandleNY {
            open_time: date("2022-09-30 08:50:00") + Duration::minutes(duration),
            open: DecimalVec(Decimal::from(open)),
            high: DecimalVec(Decimal::from(high)),
            low: DecimalVec(Decimal::from(low)),
            close: DecimalVec(Decimal::from(close)),
        }
    }

    lazy_static! {
        static ref SESSION_HIGH: DecimalVec = DecimalVec(Decimal::from(100));
        static ref SESSION_LOW: DecimalVec = DecimalVec(Decimal::from(60));
    }

    fn trigger(candles: Vec<&CandleNY>) -> Option<Position> {
        MacroSoup::trigger_or_invalidation(candles, *SESSION_HIGH, *SESSION_LOW, 4)
    }

    #[test]
    fn test_trigger_or_invalidation_empty_candles() {
        assert!(trigger(vec![]).is_none());
    }

    #[test]
    fn test_trigger_or_invalidation_inside_candles() {
        assert!(trigger(vec![
            &candlestick(0, 90, 95, 80, 85),
            &candlestick(1, 90, 100, 80, 85),
            &candlestick(2, 90, 85, 80, 85),
            &candlestick(3, 90, 99, 90, 99),
            &candlestick(4, 90, 110, 80, 85),
            &candlestick(5, 90, 95, 50, 55),
        ])
        .is_none());
    }

    #[test]
    fn test_trigger_or_invalidation_first_candle_wick_up_bearish() {
        let result = trigger(vec![&candlestick(0, 90, 110, 80, 85)]);
        let expected = Position {
            direction: PositionDirection::Short,
            open_time: date("2022-09-30 08:50:00").timestamp(),
            entry: DecimalVec::new(85),
            sl: DecimalVec::new(110),
            tp: DecimalVec::new(20),
            at_break_even: false,
        };
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_trigger_or_invalidation_first_candle_wick_up_bullish() {
        assert!(trigger(vec![&candlestick(0, 90, 110, 80, 95)]).is_none());
    }

    #[test]
    fn test_trigger_or_invalidation_deviation_up_with_downclose_inside() {
        let result = trigger(vec![
            &candlestick(0, 90, 95, 80, 85),
            &candlestick(1, 85, 110, 90, 105),
            &candlestick(2, 105, 120, 95, 100),
            &candlestick(3, 100, 105, 90, 95),
        ]);
        let expected = Position {
            direction: PositionDirection::Short,
            open_time: date("2022-09-30 08:53:00").timestamp(),
            entry: DecimalVec::new(95),
            sl: DecimalVec::new(120),
            tp: DecimalVec::new(20),
            at_break_even: false,
        };
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_trigger_or_invalidation_deviation_up_wick_inside() {
        let result = trigger(vec![
            &candlestick(0, 90, 95, 80, 85),
            &candlestick(1, 85, 110, 90, 105),
            &candlestick(2, 105, 120, 95, 100),
            &candlestick(3, 100, 105, 90, 105),
            &candlestick(4, 90, 110, 80, 85),
        ]);
        assert!(result.is_none());
    }

    #[test]
    fn test_trigger_or_invalidation_first_candle_wick_down_bullish() {
        let result = trigger(vec![&candlestick(0, 70, 75, 50, 80)]);
        let expected = Position {
            direction: PositionDirection::Long,
            open_time: date("2022-09-30 08:50:00").timestamp(),
            entry: DecimalVec::new(80),
            sl: DecimalVec::new(50),
            tp: DecimalVec::new(140),
            at_break_even: false,
        };
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_trigger_or_invalidation_first_candle_wick_down_bearish() {
        assert!(trigger(vec![&candlestick(0, 70, 75, 50, 65)]).is_none());
    }

    #[test]
    fn test_trigger_or_invalidation_deviation_down_with_upclose_inside() {
        let result = trigger(vec![
            &candlestick(0, 90, 95, 50, 55),
            &candlestick(1, 55, 65, 50, 50),
            &candlestick(2, 50, 60, 45, 60),
            &candlestick(3, 60, 70, 55, 65),
        ]);
        let expected = Position {
            direction: PositionDirection::Long,
            open_time: date("2022-09-30 08:53:00").timestamp(),
            entry: DecimalVec::new(65),
            sl: DecimalVec::new(45),
            tp: DecimalVec::new(140),
            at_break_even: false,
        };
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_trigger_or_invalidation_deviation_down_with_wick_inside() {
        let result = trigger(vec![
            &candlestick(0, 90, 95, 50, 55),
            &candlestick(1, 55, 65, 50, 50),
            &candlestick(2, 50, 60, 45, 60),
            &candlestick(3, 60, 70, 55, 60),
        ]);
        assert!(result.is_none());
    }
}
