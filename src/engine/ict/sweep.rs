use crate::engine::context::MarketContext;
use crate::engine::detection::SetupDetector;
use crate::engine::types::SetupCandidate;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position_direction::PositionDirection;

#[derive(Clone, Copy)]
pub struct SweepSignal {
    pub direction: PositionDirection,
    pub swept_level: DecimalVec,
    pub sweep_extreme: DecimalVec,
    pub index: usize,
}

pub struct SweepDetector {
    pub lookback: usize,
}

impl SweepDetector {
    pub fn detect_signal(&self, index: usize, candles: &[CandleStick]) -> Option<SweepSignal> {
        if index < self.lookback || index == 0 {
            return None;
        }
        let actual = candles[index];
        let mut highest = candles[index - 1].high;
        let mut lowest = candles[index - 1].low;

        let mut i = index - self.lookback;
        while i < index {
            let c = candles[i];
            if c.high > highest {
                highest = c.high;
            }
            if c.low < lowest {
                lowest = c.low;
            }
            i += 1;
        }

        if actual.high > highest && actual.close < highest {
            return Some(SweepSignal {
                direction: PositionDirection::Short,
                swept_level: highest,
                sweep_extreme: actual.high,
                index,
            });
        }

        if actual.low < lowest && actual.close > lowest {
            return Some(SweepSignal {
                direction: PositionDirection::Long,
                swept_level: lowest,
                sweep_extreme: actual.low,
                index,
            });
        }

        None
    }
}

impl SetupDetector for SweepDetector {
    fn detect(
        &self,
        index: usize,
        candles: &[CandleStick],
        _ctx: &MarketContext,
    ) -> Vec<SetupCandidate> {
        let _ = self.detect_signal(index, candles);
        vec![]
    }
}
