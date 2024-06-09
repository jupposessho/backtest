use rust_decimal::Decimal;

use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::{CandleStick, DecimalVec};
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;

pub struct Sfp {
    pub(crate) data: Vec<CandleStick>,
}

impl TradingModel for Sfp {
    fn execute(&self) -> BacktestResult {
        let mut swing_lows: Vec<CandleStick> = vec![];
        let mut swing_highs: Vec<CandleStick> = vec![];
        let mut position: Option<Position> = None;
        let mut trades: Vec<Trade> = vec![];

        let mut ind = 0;
        while ind < self.data.len() {
            if ind > 0 && ind < self.data.len() - 1 {
                let actual = self.data[ind];
                let previous = self.data[ind - 1];
                let next = self.data[ind + 1];

                if position.is_some() {
                    // we are in a trade
                    let trade = position.unwrap();
                    match trade.direction {
                        PositionDirection::Short => {
                            if trade.sl < actual.high {
                                // TODO: handle BE
                                trades.push(Trade::from_position(
                                    trade,
                                    actual.close_time,
                                    TradeResult::Expense,
                                ));
                                position = None;
                            }
                            if trade.tp > actual.low {
                                trades.push(Trade::from_position(
                                    trade,
                                    actual.close_time,
                                    TradeResult::Winner,
                                ));
                                position = None;
                            }
                        }
                        PositionDirection::Long => {
                            if trade.sl > actual.low {
                                trades.push(Trade::from_position(
                                    trade,
                                    actual.close_time,
                                    TradeResult::Expense,
                                ));
                                position = None;
                            }
                            if trade.tp < actual.high {
                                trades.push(Trade::from_position(
                                    trade,
                                    actual.close_time,
                                    TradeResult::Winner,
                                ));
                                position = None;
                            }
                        }
                    }
                }

                // trade
                let sfp_high = swing_highs
                    .iter()
                    .find(|x| {
                        x.close_time < actual.close_time
                            && x.high < actual.high
                            && x.high > actual.close
                    })
                    .is_some();
                let prev_low = swing_lows.iter().last();
                if sfp_high && position.is_none() && prev_low.is_some() {
                    let position_candidate = Position {
                        direction: PositionDirection::Short,
                        open_time: actual.close_time,
                        entry: actual.close,
                        sl: actual.high,
                        tp: prev_low.unwrap().low,
                    };
                    if position_candidate.rr().0 >= Decimal::from(2) {
                        position = Some(position_candidate);
                    }
                }

                let sfp_low = swing_lows
                    .iter()
                    .find(|x| {
                        x.close_time < actual.close_time
                            && x.low > actual.low
                            && x.low < actual.close
                    })
                    .is_some();
                let prev_high = swing_highs.iter().last();
                if sfp_low && position.is_none() && prev_high.is_some() {
                    let position_candidate = Position {
                        direction: PositionDirection::Long,
                        open_time: actual.close_time,
                        entry: actual.close,
                        sl: actual.low,
                        tp: prev_high.unwrap().high,
                    };
                    if position_candidate.rr().0 >= Decimal::from(2) {
                        position = Some(position_candidate);
                    }
                }

                // swings
                if actual.high > previous.high && actual.high > next.high {
                    // remove previous highs with lower high
                    swing_highs.retain(|&c: &CandleStick| c.high >= actual.high);
                    swing_highs.push(actual);
                }
                if actual.low < previous.low && actual.low < next.low {
                    // remove previous lows with higher lows
                    swing_lows.retain(|&c: &CandleStick| c.low <= actual.low);
                    swing_lows.push(actual);
                }
            }
            ind = ind + 1;
        }

        BacktestResult { trades }
    }
}
