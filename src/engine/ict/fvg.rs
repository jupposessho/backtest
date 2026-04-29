use crate::engine::context::MarketContext;
use crate::engine::detection::SetupDetector;
use crate::engine::types::SetupCandidate;
use crate::model::candle_stick::CandleStick;

pub struct FvgDetector;

impl SetupDetector for FvgDetector {
    fn detect(
        &self,
        _index: usize,
        _candles: &[CandleStick],
        _ctx: &MarketContext,
    ) -> Vec<SetupCandidate> {
        vec![]
    }
}
