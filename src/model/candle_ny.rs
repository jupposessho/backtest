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
}
