use crate::engine::context::MarketContext;
use crate::engine::detection::SetupDetector;
use crate::engine::ict::sweep::SweepSignal;
use crate::engine::types::SetupCandidate;
use crate::model::candle_stick::CandleStick;
use crate::model::position_direction::PositionDirection;

#[derive(Clone, Copy)]
pub struct MssSignal {
    pub direction: PositionDirection,
    pub index: usize,
}

pub struct MssDetector {
    pub confirm_window: usize,
}

impl MssDetector {
    pub fn detect_signal(
        &self,
        index: usize,
        candles: &[CandleStick],
        sweep: SweepSignal,
    ) -> Option<MssSignal> {
        if index <= sweep.index || index > sweep.index + self.confirm_window || index == 0 {
            return None;
        }
        let actual = candles[index];
        let previous = candles[index - 1];

        match sweep.direction {
            PositionDirection::Long => {
                if actual.close > previous.high {
                    return Some(MssSignal {
                        direction: PositionDirection::Long,
                        index,
                    });
                }
            }
            PositionDirection::Short => {
                if actual.close < previous.low {
                    return Some(MssSignal {
                        direction: PositionDirection::Short,
                        index,
                    });
                }
            }
        }
        None
    }
}

impl SetupDetector for MssDetector {
    fn detect(
        &self,
        _index: usize,
        _candles: &[CandleStick],
        _ctx: &MarketContext,
    ) -> Vec<SetupCandidate> {
        vec![]
    }
}
