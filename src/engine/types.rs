use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position_direction::PositionDirection;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

#[derive(Clone, Copy)]
pub enum EntryPolicy {
    Price(DecimalVec),
    PrevOpen,
    PairMidpoint,
    PairExtreme,
    ObPrevOpen,
    ObPairMidpoint,
    ObPairExtreme,
    FvgMidpoint { low: DecimalVec, high: DecimalVec },
    OteMidpoint { low: DecimalVec, high: DecimalVec },
}

#[derive(Clone, Copy)]
pub enum EntryModel {
    SignalClose,
    MarketClose,
    NextBarOpen,
    LimitTouch {
        price: DecimalVec,
        expiry_bars: usize,
    },
    LimitByPolicy {
        policy: EntryPolicy,
        expiry_bars: usize,
    },
}

#[derive(Clone, Copy)]
pub enum StopModel {
    FixedPrice(DecimalVec),
}

#[derive(Clone, Copy)]
pub enum TargetModel {
    FixedPrice(DecimalVec),
    FixedR(Decimal),
    FixedPoints(Decimal),
}

#[derive(Clone, Copy)]
pub enum TrailingModel {
    None,
    BreakEvenAtR(Decimal),
    StepAtR {
        trigger_r: Decimal,
        lock_r: Decimal,
    },
    ProgressiveHalfR {
        start_r: Decimal,
        step_r: Decimal,
    },
}

#[derive(Clone, Copy)]
pub struct ExecutionConfig {
    pub commission_rate_per_side: Decimal,
    pub fee_rate_per_side: Decimal,
    pub slippage_ticks_per_side: i32,
    pub tick_size: Decimal,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            commission_rate_per_side: Decimal::ZERO,
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 0,
            tick_size: Decimal::new(1, 2),
        }
    }
}

#[derive(Clone, Copy)]
pub struct SetupCandidate {
    pub direction: PositionDirection,
    pub signal_index: usize,
    pub entry: EntryModel,
    pub stop: StopModel,
    pub target: TargetModel,
    pub trailing: TrailingModel,
}

#[derive(Clone, Copy)]
pub struct OpenPosition {
    pub direction: PositionDirection,
    pub open_time: i64,
    pub entry: DecimalVec,
    pub stop: DecimalVec,
    pub current_stop: DecimalVec,
    pub target: DecimalVec,
    pub initial_risk: Decimal,
    pub trailing: TrailingModel,
}

pub fn risk_amount(direction: PositionDirection, entry: DecimalVec, stop: DecimalVec) -> Decimal {
    match direction {
        PositionDirection::Long => entry.0 - stop.0,
        PositionDirection::Short => stop.0 - entry.0,
    }
}

pub fn slippage_value(cfg: &ExecutionConfig) -> Decimal {
    Decimal::from_i32(cfg.slippage_ticks_per_side).unwrap_or(Decimal::ZERO) * cfg.tick_size
}

pub fn apply_entry_slippage(
    direction: PositionDirection,
    px: DecimalVec,
    cfg: &ExecutionConfig,
) -> DecimalVec {
    let s = slippage_value(cfg);
    match direction {
        PositionDirection::Long => DecimalVec(px.0 + s),
        PositionDirection::Short => DecimalVec(px.0 - s),
    }
}

pub fn apply_exit_slippage(
    direction: PositionDirection,
    px: DecimalVec,
    cfg: &ExecutionConfig,
) -> DecimalVec {
    let s = slippage_value(cfg);
    match direction {
        PositionDirection::Long => DecimalVec(px.0 - s),
        PositionDirection::Short => DecimalVec(px.0 + s),
    }
}

pub fn stop_hit(c: CandleStick, position: &OpenPosition) -> bool {
    match position.direction {
        PositionDirection::Long => c.low <= position.current_stop,
        PositionDirection::Short => c.high >= position.current_stop,
    }
}

pub fn target_hit(c: CandleStick, position: &OpenPosition) -> bool {
    match position.direction {
        PositionDirection::Long => c.high >= position.target,
        PositionDirection::Short => c.low <= position.target,
    }
}
