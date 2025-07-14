use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::model::candle_stick::CandleStick;

use super::decimal::DecimalVec;

#[derive(Debug, Deserialize)]
pub struct BarChartCandle {
    hd: Header,
    open: String,
    high: String,
    low: String,
    close: String,
}

#[derive(Debug, Deserialize)]
struct Header {
    ts_event: String,
}

// Convert from BarChartCandle to CandleStick
impl TryFrom<BarChartCandle> for CandleStick {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: BarChartCandle) -> Result<Self, Self::Error> {
        let dt: DateTime<Utc> = raw.hd.ts_event.parse()?;
        let timestamp = dt.timestamp(); // Unix timestamp in seconds
        Ok(CandleStick {
            open_time: timestamp,
            open: DecimalVec::from_str(&raw.open),
            high: DecimalVec::from_str(&raw.high),
            low: DecimalVec::from_str(&raw.low),
            close: DecimalVec::from_str(&raw.close),
            close_time: timestamp, // TODO: fix
        })
    }
}
