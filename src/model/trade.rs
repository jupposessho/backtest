use std::fmt;

use crate::model::decimal::DecimalVec;
use crate::to_new_york_time;

use super::{position::Position, position_direction::PositionDirection, trade_result::TradeResult};

#[derive(Clone, Copy)]
pub struct Trade {
    pub direction: PositionDirection,
    pub open_time: i64,
    pub close_time: i64,
    pub entry: DecimalVec,
    pub sl: DecimalVec,
    pub tp: DecimalVec,
    pub result: TradeResult,
}

impl Trade {
    pub fn rr(&self) -> DecimalVec {
        match self.direction {
            PositionDirection::Short => (self.entry - self.tp) / (self.sl - self.entry),
            PositionDirection::Long => (self.tp - self.entry) / (self.entry - self.sl),
        }
    }

    pub fn points(&self) -> DecimalVec {
        match self.direction {
            PositionDirection::Short => match self.result {
                TradeResult::Winner => self.entry - self.tp,
                TradeResult::Expense => self.entry - self.sl,
                TradeResult::BreakEven => DecimalVec::new(0),
            },
            PositionDirection::Long => match self.result {
                TradeResult::Winner => self.tp - self.entry,
                TradeResult::Expense => self.sl - self.entry,
                TradeResult::BreakEven => DecimalVec::new(0),
            },
        }
    }
}

impl fmt::Debug for Trade {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let o = to_new_york_time(self.open_time.clone())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let c = to_new_york_time(self.close_time.clone())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        f.debug_struct("Trade")
            .field("direction", &self.direction)
            .field("open_time", &o)
            .field("close_time", &c)
            .field("entry", &self.entry.0)
            .field("sl", &self.sl.0)
            .field("tp", &self.tp.0)
            .field("rr", &self.rr().0)
            .field("result", &self.result)
            .finish()
    }
}

impl Trade {
    pub(crate) fn from_position(position: Position, close_time: i64, result: TradeResult) -> Trade {
        Trade {
            direction: position.direction,
            open_time: position.open_time,
            close_time,
            entry: position.entry,
            sl: position.sl,
            tp: position.tp,
            result,
        }
    }
}
