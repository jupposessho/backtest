use crate::engine::context::MarketContext;
use crate::engine::detection::SetupDetector;
use crate::engine::types::SetupCandidate;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position_direction::PositionDirection;

#[derive(Clone, Copy)]
pub struct IfvgSignal {
    pub direction: PositionDirection,
    pub index: usize,
    pub level: DecimalVec,
}

pub struct IfvgDetector {
    pub lookback: usize,
}

impl IfvgDetector {
    pub fn detect_signal(
        &self,
        index: usize,
        candles: &[CandleStick],
        expected_direction: PositionDirection,
    ) -> Option<IfvgSignal> {
        if index < 3 || index >= candles.len() {
            return None;
        }
        let start = index.saturating_sub(self.lookback.max(3));
        let current = candles[index];
        let previous = candles[index - 1];

        match expected_direction {
            PositionDirection::Long => {
                for i in start..index.saturating_sub(1) {
                    if i + 2 >= candles.len() {
                        break;
                    }
                    let c1 = candles[i];
                    let c3 = candles[i + 2];
                    if c3.high < c1.low {
                        let level = c1.low;
                        if previous.close <= level && current.close > level {
                            return Some(IfvgSignal {
                                direction: PositionDirection::Long,
                                index,
                                level,
                            });
                        }
                    }
                }
                None
            }
            PositionDirection::Short => {
                for i in start..index.saturating_sub(1) {
                    if i + 2 >= candles.len() {
                        break;
                    }
                    let c1 = candles[i];
                    let c3 = candles[i + 2];
                    if c3.low > c1.high {
                        let level = c1.high;
                        if previous.close >= level && current.close < level {
                            return Some(IfvgSignal {
                                direction: PositionDirection::Short,
                                index,
                                level,
                            });
                        }
                    }
                }
                None
            }
        }
    }
}

impl SetupDetector for IfvgDetector {
    fn detect(
        &self,
        _index: usize,
        _candles: &[CandleStick],
        _ctx: &MarketContext,
    ) -> Vec<SetupCandidate> {
        vec![]
    }
}
