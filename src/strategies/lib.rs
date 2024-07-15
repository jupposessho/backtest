use chrono::DateTime;
use chrono_tz::Tz;
use itertools::Itertools;
use rust_decimal::Decimal;

use crate::model::{
    candle_stick::CandleStick, decimal::DecimalVec, position::Position,
    position_direction::PositionDirection, session::Session, trade::Trade,
    trade_result::TradeResult, trigger_type::TriggerType,
};

pub fn is_swing_low(actual: CandleStick, previous: CandleStick, next: CandleStick) -> bool {
    actual.low < previous.low && actual.low < next.low
}

pub fn is_swing_high(actual: CandleStick, previous: CandleStick, next: CandleStick) -> bool {
    actual.high > previous.high && actual.high > next.high
}

pub fn add_to_swings(
    swing_lows: &mut Vec<CandleStick>,
    swing_highs: &mut Vec<CandleStick>,
    actual: CandleStick,
    previous: CandleStick,
    next: CandleStick,
) {
    if is_swing_high(actual, previous, next) {
        // remove previous highs with lower high
        swing_highs.retain(|&c: &CandleStick| c.high >= actual.high);
        swing_highs.push(actual);
    }
    if is_swing_low(actual, previous, next) {
        // remove previous lows with higher lows
        swing_lows.retain(|&c: &CandleStick| c.low <= actual.low);
        swing_lows.push(actual);
    }
}

pub fn first_swing(
    candles: Vec<CandleStick>,
    p: fn(CandleStick, CandleStick, CandleStick) -> bool,
) -> Option<CandleStick> {
    let mut ind = 0;
    while ind < candles.len() {
        if ind > 0 && ind < candles.len() - 1 {
            let actual = candles[ind];
            let previous = candles[ind - 1];
            let next = candles[ind + 1];
            if p(actual, previous, next) {
                return Some(actual);
            }
        }
        ind = ind + 1;
    }
    None
}

// TODO: test these
pub fn find_sfp_high(actual: CandleStick, swing_highs: &Vec<CandleStick>) -> Option<&CandleStick> {
    swing_highs
        .iter()
        .find(|x| x.close_time < actual.close_time && x.high < actual.high && x.high > actual.close)
}

pub fn find_sfp_low(actual: CandleStick, swing_lows: &Vec<CandleStick>) -> Option<&CandleStick> {
    swing_lows
        .iter()
        .find(|x| x.close_time < actual.close_time && x.low > actual.low && x.low < actual.close)
}

pub fn find_candle(
    candle: CandleStick,
    data: &Vec<CandleStick>,
    p: fn(CandleStick, CandleStick) -> bool,
) -> &CandleStick {
    data.iter()
        .find(|x| {
            x.open_time >= candle.open_time && x.close_time <= candle.close_time && p(**x, candle)
        })
        .expect("Failed to find the matching candle in ltf_data")
}

pub fn trigger_or_invalidation(
    candles: Vec<CandleStick>,
    direction: PositionDirection,
    trigger_level: DecimalVec,
    invalidation_level: DecimalVec,
    trigger_type: TriggerType,
) -> Option<CandleStick> {
    for actual in candles {
        match direction {
            PositionDirection::Short => {
                let trigger_comparision = match trigger_type {
                    TriggerType::Close => actual.close,
                    TriggerType::Wick => actual.low,
                };
                if trigger_comparision < trigger_level {
                    return Some(actual);
                }

                if actual.high >= invalidation_level {
                    return None;
                }
            }
            PositionDirection::Long => {
                let trigger_comparision = match trigger_type {
                    TriggerType::Close => actual.close,
                    TriggerType::Wick => actual.high,
                };
                if trigger_comparision > trigger_level {
                    return Some(actual);
                }
                if actual.low <= invalidation_level {
                    return None;
                }
            }
        }
    }
    return None;
}

pub fn trigger_mayne(
    direction: PositionDirection,
    trigger_type: TriggerType,
    trigger_level: DecimalVec,
    sl: DecimalVec,
    tp: DecimalVec,
    rr_threshold: Decimal,
    candles: Vec<CandleStick>,
    trades: &mut Vec<Trade>,
) {
    let trigger_candle =
        trigger_or_invalidation(candles.clone(), direction, trigger_level, sl, trigger_type);
    if let Some(tc) = trigger_candle {
        let position = Position {
            direction,
            open_time: tc.close_time,
            entry: tc.close,
            sl, // TODO: can we refine this? eg: previous swing high on ltf
            tp,
        };

        if position.rr().0 >= rr_threshold {
            let candles_after_entry = candles
                .iter()
                .skip_while(|x| x.open_time <= tc.open_time)
                .collect_vec();
            let trade = run_trade(position, candles_after_entry);
            if let Some(t) = trade {
                trades.push(t)
            }
        }
    }
}
// pub fn look_for_entry(candles: Vec<CandleStick>) {}

pub fn run_trade(position: Position, candles: Vec<&CandleStick>) -> Option<Trade> {
    for actual in candles {
        match position.direction {
            PositionDirection::Short => {
                if position.sl < actual.high {
                    return Some(Trade::from_position(
                        position,
                        actual.close_time,
                        TradeResult::Expense,
                    ));
                }
                if position.tp > actual.low {
                    return Some(Trade::from_position(
                        position,
                        actual.close_time,
                        TradeResult::Winner,
                    ));
                }
            }
            PositionDirection::Long => {
                if position.sl > actual.low {
                    return Some(Trade::from_position(
                        position,
                        actual.close_time,
                        TradeResult::Expense,
                    ));
                }
                if position.tp < actual.high {
                    return Some(Trade::from_position(
                        position,
                        actual.close_time,
                        TradeResult::Winner,
                    ));
                }
            }
        }
    }
    return None;
}

pub fn in_session(session: &Session, open_time: DateTime<Tz>) -> bool {
    open_time.time() >= session.start && open_time.time() < session.end
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;
    use lazy_static::lazy_static;

    use super::*;
    use crate::{model::candle_stick::CandleStick, parse_datetime};
    use rust_decimal::{prelude::FromPrimitive, Decimal};

    fn candlestick(high: i32, low: i32) -> CandleStick {
        CandleStick {
            open_time: 0,
            open: DecimalVec(Decimal::from(0)),
            high: DecimalVec(Decimal::from(high)),
            low: DecimalVec(Decimal::from(low)),
            close: DecimalVec(Decimal::from(0)),
            close_time: 0,
        }
    }

    fn candlestick_high_close(close_time: i64, high: f32, close: f32) -> CandleStick {
        CandleStick {
            open_time: 0,
            open: DecimalVec(Decimal::from(0)),
            high: DecimalVec(Decimal::from_f32(high).unwrap()),
            low: DecimalVec(Decimal::from(0)),
            close: DecimalVec(Decimal::from_f32(close).unwrap()),
            close_time,
        }
    }

    #[test]
    fn test_swing_low() {
        let actual = candlestick(0, 1);
        let previous = candlestick(0, 3);
        let next = candlestick(0, 2);
        assert!(is_swing_low(actual, previous, next));
    }

    #[test]
    fn test_not_swing_low_previous_lower() {
        let actual = candlestick(0, 2);
        let previous = candlestick(0, 1);
        let next = candlestick(0, 3);
        assert!(!is_swing_low(actual, previous, next));
    }

    #[test]
    fn test_not_swing_low_next_lower() {
        let actual = candlestick(0, 2);
        let previous = candlestick(0, 3);
        let next = candlestick(0, 1);
        assert!(!is_swing_low(actual, previous, next));
    }

    #[test]
    fn test_not_swing_low_both_lower() {
        let actual = candlestick(0, 2);
        let previous = candlestick(0, 1);
        let next = candlestick(0, 1);
        assert!(!is_swing_low(actual, previous, next));
    }

    #[test]
    fn test_not_swing_low_equal_previous() {
        let actual = candlestick(0, 1);
        let previous = candlestick(0, 1);
        let next = candlestick(0, 2);
        assert!(!is_swing_low(actual, previous, next));
    }

    #[test]
    fn test_not_swing_low_equal_next() {
        let actual = candlestick(0, 1);
        let previous = candlestick(0, 2);
        let next = candlestick(0, 1);
        assert!(!is_swing_low(actual, previous, next));
    }

    #[test]
    fn test_not_swing_low_all_equal() {
        let actual = candlestick(0, 1);
        let previous = candlestick(0, 1);
        let next = candlestick(0, 1);
        assert!(!is_swing_low(actual, previous, next));
    }

    #[test]
    fn test_swing_high() {
        let actual = candlestick(3, 0);
        let previous = candlestick(1, 0);
        let next = candlestick(2, 0);
        assert!(is_swing_high(actual, previous, next));
    }

    #[test]
    fn test_not_swing_high_previous_higher() {
        let actual = candlestick(2, 0);
        let previous = candlestick(3, 0);
        let next = candlestick(1, 0);
        assert!(!is_swing_high(actual, previous, next));
    }

    #[test]
    fn test_not_swing_high_next_higher() {
        let actual = candlestick(2, 0);
        let previous = candlestick(1, 0);
        let next = candlestick(3, 0);
        assert!(!is_swing_high(actual, previous, next));
    }

    #[test]
    fn test_not_swing_high_both_higher() {
        let actual = candlestick(1, 0);
        let previous = candlestick(2, 0);
        let next = candlestick(3, 0);
        assert!(!is_swing_high(actual, previous, next));
    }

    #[test]
    fn test_not_swing_high_equal_previous() {
        let actual = candlestick(2, 0);
        let previous = candlestick(2, 0);
        let next = candlestick(1, 0);
        assert!(!is_swing_high(actual, previous, next));
    }

    #[test]
    fn test_not_swing_high_equal_next() {
        let actual = candlestick(2, 0);
        let previous = candlestick(1, 0);
        let next = candlestick(2, 0);
        assert!(!is_swing_high(actual, previous, next));
    }

    #[test]
    fn test_not_swing_high_all_equal() {
        let actual = candlestick(2, 0);
        let previous = candlestick(2, 0);
        let next = candlestick(2, 0);
        assert!(!is_swing_high(actual, previous, next));
    }

    #[test]
    fn test_first_swing_found() {
        let candles = vec![
            candlestick(0, 20),
            candlestick(0, 10),
            candlestick(0, 15),
            candlestick(0, 5),
            candlestick(0, 25),
        ];
        let result = first_swing(candles, is_swing_low);
        assert_eq!(result, Some(candlestick(0, 10)));
    }

    #[test]
    fn test_first_swing_not_found() {
        let candles = vec![candlestick(0, 20), candlestick(0, 15), candlestick(0, 10)];
        let result = first_swing(candles, is_swing_low);
        assert!(result.is_none());
    }

    #[test]
    fn test_first_swing_single_element() {
        let candles = vec![candlestick(0, 10)];
        let result = first_swing(candles, is_swing_low);
        assert!(result.is_none());
    }

    #[test]
    fn test_first_swing_two_elements() {
        let candles = vec![candlestick(0, 10), candlestick(0, 20)];
        let result = first_swing(candles, is_swing_low);
        assert!(result.is_none());
    }

    #[test]
    fn test_first_swing_at_end() {
        let candles = vec![
            candlestick(0, 25),
            candlestick(0, 20),
            candlestick(0, 5),
            candlestick(0, 15),
            candlestick(0, 10),
        ];

        let result = first_swing(candles, is_swing_low);
        assert_eq!(result, Some(candlestick(0, 5)));
    }

    #[test]
    fn test_first_swing_multiple_swings() {
        let candles = vec![
            candlestick(0, 25),
            candlestick(0, 10),
            candlestick(0, 15),
            candlestick(0, 5),
            candlestick(0, 25),
            candlestick(0, 3),
            candlestick(0, 20),
        ];
        let result = first_swing(candles, is_swing_low);
        assert_eq!(result, Some(candlestick(0, 10)));
    }

    #[test]
    fn test_first_swing_multiple_swing_highs() {
        let candles = vec![
            candlestick(25, 0),
            candlestick(30, 0),
            candlestick(15, 0),
            candlestick(5, 0),
            candlestick(25, 0),
            candlestick(3, 0),
            candlestick(20, 0),
        ];
        let result = first_swing(candles, is_swing_high);
        assert_eq!(result, Some(candlestick(30, 0)));
    }

    #[test]
    fn test_add_to_swings_high_to_empty() {
        let mut swing_highs = vec![];
        let mut swing_lows = vec![];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(30, 0),
            candlestick(25, 0),
            candlestick(15, 0),
        );
        assert_eq!(swing_lows, vec![]);
        assert_eq!(swing_highs, vec![candlestick(30, 0)]);
    }

    #[test]
    fn test_add_to_swings_high_keep_higher() {
        let mut swing_highs = vec![candlestick(40, 0)];
        let mut swing_lows = vec![];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(30, 0),
            candlestick(25, 0),
            candlestick(15, 0),
        );
        assert_eq!(swing_highs, vec![candlestick(40, 0), candlestick(30, 0)]);
    }

    #[test]
    fn test_add_to_swings_high_remove_lowers() {
        let mut swing_highs = vec![candlestick(10, 0), candlestick(20, 0), candlestick(40, 0)];
        let mut swing_lows = vec![];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(30, 0),
            candlestick(25, 0),
            candlestick(15, 0),
        );
        assert_eq!(swing_highs, vec![candlestick(40, 0), candlestick(30, 0)]);
    }

    #[test]
    fn test_add_to_swings_low_to_empty() {
        let mut swing_highs = vec![];
        let mut swing_lows = vec![];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(0, 5),
            candlestick(0, 10),
            candlestick(0, 15),
        );
        assert_eq!(swing_lows, vec![candlestick(0, 5)]);
        assert_eq!(swing_highs, vec![]);
    }

    #[test]
    fn test_add_to_swings_low_keep_lower() {
        let mut swing_highs = vec![];
        let mut swing_lows = vec![candlestick(0, 4)];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(0, 5),
            candlestick(0, 10),
            candlestick(0, 15),
        );
        assert_eq!(swing_lows, vec![candlestick(0, 4), candlestick(0, 5)]);
    }

    #[test]
    fn test_add_to_swings_low_remove_highers() {
        let mut swing_highs = vec![];
        let mut swing_lows = vec![candlestick(0, 4), candlestick(0, 6), candlestick(0, 7)];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(0, 5),
            candlestick(0, 10),
            candlestick(0, 15),
        );
        assert_eq!(swing_lows, vec![candlestick(0, 4), candlestick(0, 5)]);
    }

    #[test]
    fn test_add_to_swings_no_swing() {
        let mut swing_highs = vec![];
        let mut swing_lows = vec![];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(0, 10),
            candlestick(0, 5),
            candlestick(0, 15),
        );
        assert_eq!(swing_lows, vec![]);
        assert_eq!(swing_highs, vec![]);
    }

    #[test]
    fn test_add_to_swings_both_remove_both() {
        let mut swing_highs = vec![candlestick(10, 8)];
        let mut swing_lows = vec![candlestick(5, 4)];
        let _ = add_to_swings(
            &mut swing_lows,
            &mut swing_highs,
            candlestick(15, 3),
            candlestick(12, 5),
            candlestick(13, 5),
        );
        assert_eq!(swing_lows, vec![candlestick(15, 3)]);
        assert_eq!(swing_highs, vec![candlestick(15, 3)]);
    }

    #[test]
    fn test_find_sfp_high_basic() {
        let actual = candlestick_high_close(10, 110.0, 105.0);

        let swing_highs = vec![
            candlestick_high_close(1, 100.0, 98.0),
            candlestick_high_close(5, 108.0, 107.0),
            candlestick_high_close(8, 109.0, 108.0),
        ];

        let result = find_sfp_high(actual, &swing_highs);
        assert_eq!(result, Some(&swing_highs[2]));
    }

    #[test]
    fn test_find_sfp_high_no_match() {
        let actual = candlestick_high_close(10, 110.0, 105.0);

        let swing_highs = vec![
            candlestick_high_close(1, 112.0, 111.0),
            candlestick_high_close(5, 113.0, 112.0),
        ];

        let result = find_sfp_high(actual, &swing_highs);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_sfp_high_edge_cases() {
        let actual = candlestick_high_close(10, 110.0, 105.0);

        // Test with an empty vector
        let swing_highs: Vec<CandleStick> = vec![];
        let result = find_sfp_high(actual, &swing_highs);
        assert_eq!(result, None);

        // Test with swing high exactly at close time
        let swing_highs = vec![candlestick_high_close(10, 109.0, 108.0)];
        let result = find_sfp_high(actual, &swing_highs);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_sfp_high_multiple_matches() {
        let actual = candlestick_high_close(10, 110.0, 105.0);

        let swing_highs = vec![
            candlestick_high_close(1, 109.0, 108.0),
            candlestick_high_close(5, 109.5, 109.0),
            candlestick_high_close(3, 108.5, 108.0),
        ];

        let result = find_sfp_high(actual, &swing_highs);
        assert_eq!(result, Some(&swing_highs[0]));
    }

    lazy_static! {
        static ref SESSION: Session = Session {
            start: NaiveTime::from_hms_opt(8, 50, 0).unwrap(),
            end: NaiveTime::from_hms_opt(9, 10, 0).unwrap(),
        };
    }

    #[test]
    fn test_in_session_outside() {
        let result = in_session(&SESSION, parse_datetime("2022-09-30 09:30:00").unwrap());
        assert!(!result);
    }

    #[test]
    fn test_in_session_at_start() {
        let result = in_session(&SESSION, parse_datetime("2022-09-30 08:50:00").unwrap());
        assert!(result);
    }
}
