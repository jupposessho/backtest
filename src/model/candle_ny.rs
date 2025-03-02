use core::fmt;

use chrono::DateTime;
use chrono_tz::Tz;

use super::decimal::DecimalVec;

#[derive(Clone, PartialEq)]
pub struct CandleNY {
    pub open_time: DateTime<Tz>,
    pub open: DecimalVec,
    pub high: DecimalVec,
    pub low: DecimalVec,
    pub close: DecimalVec,
}

impl CandleNY {
    pub fn bullish(self) -> bool {
        self.close >= self.open
    }
    pub fn bearish(self) -> bool {
        !self.bullish()
    }

    // pub fn from_candle_stick(candle: CandleStick) -> Self {

    // }
}

impl fmt::Debug for CandleNY {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // let o = to_new_york_time(self.open_time.clone())
        let o = self
            .open_time
            .clone()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        f.debug_struct("CandleStick")
            .field("open_time", &o)
            .field("open", &self.open)
            .field("close", &self.close)
            .field("high", &self.high)
            .field("low", &self.low)
            .finish()
    }
}
