use crate::engine::context::MarketContext;
use crate::engine::types::SetupCandidate;
use crate::model::candle_stick::CandleStick;

pub trait SetupDetector {
    fn detect(
        &self,
        index: usize,
        candles: &[CandleStick],
        ctx: &MarketContext,
    ) -> Vec<SetupCandidate>;
}
