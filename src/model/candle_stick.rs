use crate::to_new_york_time;
use charming::datatype::NumericValue;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::fmt;
use std::ops::{Div, Sub};

// #[derive(Clone, Copy)]
#[derive(Clone, Copy, Deserialize)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct CandleStick {
    pub open_time: i64,
    pub open: DecimalVec,
    pub high: DecimalVec,
    pub low: DecimalVec,
    pub close: DecimalVec,
    pub close_time: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialOrd, PartialEq)]
pub struct DecimalVec(pub Decimal);

impl Sub for DecimalVec {
    type Output = DecimalVec;

    fn sub(self, rhs: Self) -> Self::Output {
        DecimalVec(self.0 - rhs.0)
    }
}

impl Div for DecimalVec {
    type Output = DecimalVec;

    fn div(self, rhs: Self) -> Self::Output {
        DecimalVec(self.0 / rhs.0)
    }
}

impl From<DecimalVec> for NumericValue {
    fn from(n: DecimalVec) -> Self {
        NumericValue::Float(n.0.to_f64().unwrap())
    }
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
