use super::position_direction::PositionDirection;
use crate::model::decimal::DecimalVec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub direction: PositionDirection,
    pub open_time: i64,
    pub entry: DecimalVec,
    pub sl: DecimalVec,
    pub tp: DecimalVec,
    pub at_break_even: bool,
}

impl Position {
    pub fn rr(&self) -> DecimalVec {
        self.actual_rr(self.tp)
    }

    pub fn actual_rr(&self, actual_price: DecimalVec) -> DecimalVec {
        match self.direction {
            PositionDirection::Short => (self.entry - actual_price) / (self.sl - self.entry),
            PositionDirection::Long => (actual_price - self.entry) / (self.entry - self.sl),
        }
    }

    pub fn move_to_break_even(&mut self) {
        self.at_break_even = true;
    }
}
