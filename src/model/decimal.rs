use std::ops::{Add, Div, Sub};

use charming::datatype::NumericValue;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialOrd, PartialEq)]
pub struct DecimalVec(pub Decimal);

impl DecimalVec {
    pub fn new(v: i32) -> Self {
        DecimalVec(rust_decimal::Decimal::from(v))
    }
}

impl Add for DecimalVec {
    type Output = DecimalVec;

    fn add(self, rhs: Self) -> Self::Output {
        DecimalVec(self.0 + rhs.0)
    }
}

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
