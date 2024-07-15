use crate::to_new_york_time;
use serde::Deserialize;
use std::fmt;

use super::decimal::DecimalVec;

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct CandleStick {
    pub open_time: i64,
    pub open: DecimalVec,
    pub high: DecimalVec,
    pub low: DecimalVec,
    pub close: DecimalVec,
    pub close_time: i64,
}

impl fmt::Debug for CandleStick {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let o = to_new_york_time(self.open_time.clone())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let c = to_new_york_time(self.close_time.clone())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        f.debug_struct("CandleStick")
            .field("open_time", &o)
            .field("close_time", &c)
            .field("open", &self.open)
            .field("close", &self.close)
            .field("high", &self.high)
            .field("low", &self.low)
            .finish()
    }
}
