use super::position_direction::PositionDirection;
use crate::model::decimal::DecimalVec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub direction: PositionDirection,
    pub open_time: i64,
    pub entry: DecimalVec,
    pub sl: DecimalVec,
    pub tp: DecimalVec,
}

impl Position {
    pub fn rr(&self) -> DecimalVec {
        match self.direction {
            PositionDirection::Short => (self.entry - self.tp) / (self.sl - self.entry),
            PositionDirection::Long => (self.tp - self.entry) / (self.entry - self.sl),
        }
    }
}
