use std::fmt;

use rust_decimal::Decimal;

use crate::model::candle_stick::DecimalVec;
use crate::to_new_york_time;

use super::{position::Position, position_direction::PositionDirection, trade_result::TradeResult};

#[derive(Clone, Copy)]
pub struct Trade {
    direction: PositionDirection,
    open_time: i64,
    close_time: i64,
    entry: DecimalVec,
    sl: DecimalVec,
    tp: DecimalVec,
    pub result: TradeResult,
}

impl Trade {
    pub fn rr(&self) -> DecimalVec {
        match self.direction {
            PositionDirection::Short => (self.entry - self.tp) / (self.sl - self.entry),
            PositionDirection::Long => (self.tp - self.entry) / (self.entry - self.sl),
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
