use rust_decimal::Decimal;

use crate::engine::types::ExecutionConfig;
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trading_model::TradingModel;
use crate::model::trigger_type::TriggerType;
use crate::strategies::lib::{first_swing, is_swing_low};

use super::lib::{
    add_to_swings, find_candle, find_sfp_high, find_sfp_low, is_swing_high, trigger_mayne,
};

pub struct Mayne {
    pub rr_threshold: Decimal,
    pub trigger_type: TriggerType,
    pub htf_data: Vec<CandleStick>,
    pub ltf_data: Vec<CandleStick>,
    pub execution: ExecutionConfig,
}

impl TradingModel for Mayne {
    fn execute(&self) -> BacktestResult {
        let mut swing_lows: Vec<CandleStick> = vec![];
        let mut swing_highs: Vec<CandleStick> = vec![];
        let mut trades: Vec<Trade> = vec![];

        let mut ind = 0;
        while ind < self.htf_data.len() {
            if ind > 0 && ind < self.htf_data.len() - 1 {
                let actual = self.htf_data[ind];
                let previous = self.htf_data[ind - 1];
                let next = self.htf_data[ind + 1];

                // if there is a valid target - previous swing low
                if let Some(prev_swing_low) = swing_lows.iter().last() {
                    // if first trigger: - SFP on a higher timeframe
                    if find_sfp_high(actual, &swing_highs).is_some() {
                        // 1. find the candle on the ltf where the htf sfp formed
                        let ltf_sfp_candle =
                            find_candle(actual, &self.ltf_data, |a, b| a.high == b.high);
                        let (mut previous_candles, next_candles): (Vec<_>, Vec<_>) = self
                            .ltf_data
                            .clone()
                            .into_iter()
                            .partition(|x| x.open_time < ltf_sfp_candle.open_time);
                        previous_candles.reverse();
                        // 2. find the previous swing low on ltf - trigger level
                        if let Some(prev_ltf_swing_low) =
                            first_swing(previous_candles, is_swing_low)
                        {
                            // 3. on the ltf look for:
                            //  - close or wick below(trigger type) trigger level
                            //      -> trade if there is a valid entry
                            //  - wick above invalidation level -> no trade
                            trigger_mayne(
                                PositionDirection::Short,
                                self.trigger_type,
                                prev_ltf_swing_low.low,
                                actual.high,
                                prev_swing_low.low,
                                self.rr_threshold,
                                next_candles,
                                &mut trades,
                                &self.execution,
                            );
                        }
                    }
                }

                if let Some(prev_swing_high) = swing_highs.iter().last() {
                    if find_sfp_low(actual, &swing_lows).is_some() {
                        let ltf_sfp_candle =
                            find_candle(actual, &self.ltf_data, |a, b| a.low == b.low);
                        // TODO: move this to trigger_mayne
                        let (mut previous_candles, next_candles): (Vec<_>, Vec<_>) = self
                            .ltf_data
                            .clone()
                            .into_iter()
                            .partition(|x| x.open_time < ltf_sfp_candle.open_time);
                        previous_candles.reverse();
                        if let Some(prev_ltf_swing_high) =
                            first_swing(previous_candles, is_swing_high)
                        {
                            trigger_mayne(
                                PositionDirection::Long,
                                self.trigger_type,
                                prev_ltf_swing_high.high,
                                actual.low,
                                prev_swing_high.high,
                                self.rr_threshold,
                                next_candles,
                                &mut trades,
                                &self.execution,
                            );
                        }
                    }
                }

                add_to_swings(&mut swing_lows, &mut swing_highs, actual, previous, next)
            }
            ind = ind + 1;
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}
