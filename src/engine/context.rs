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

#[derive(Clone, Copy)]
pub struct MssSignal {
    pub direction: PositionDirection,
    pub index: usize,
}

#[derive(Default)]
pub struct MarketContext {
    pub trading_day_index: usize,
    pub ny_hour: u32,
    pub ny_minute: u32,
    pub marker: Option<CandleStick>,
    pub sweep: Option<SweepSignal>,
    pub mss: Option<MssSignal>,
}
