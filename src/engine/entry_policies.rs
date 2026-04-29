use crate::engine::types::EntryPolicy;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position_direction::PositionDirection;
use rust_decimal::Decimal;

pub fn resolve_entry_policy(
    policy: EntryPolicy,
    direction: PositionDirection,
    signal: CandleStick,
    previous: CandleStick,
) -> DecimalVec {
    match policy {
        EntryPolicy::Price(px) => px,
        EntryPolicy::PrevOpen | EntryPolicy::ObPrevOpen => previous.open,
        EntryPolicy::PairMidpoint | EntryPolicy::ObPairMidpoint => {
            let pair_high = if signal.high > previous.high {
                signal.high
            } else {
                previous.high
            };
            let pair_low = if signal.low < previous.low {
                signal.low
            } else {
                previous.low
            };
            DecimalVec((pair_high.0 + pair_low.0) / Decimal::from(2))
        }
        EntryPolicy::PairExtreme | EntryPolicy::ObPairExtreme => match direction {
            PositionDirection::Long => {
                if signal.low < previous.low {
                    signal.low
                } else {
                    previous.low
                }
            }
            PositionDirection::Short => {
                if signal.high > previous.high {
                    signal.high
                } else {
                    previous.high
                }
            }
        },
        EntryPolicy::FvgMidpoint { low, high } | EntryPolicy::OteMidpoint { low, high } => {
            DecimalVec((low.0 + high.0) / Decimal::from(2))
        }
    }
}
