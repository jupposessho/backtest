use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::{America::New_York, Tz};

pub mod chart;
mod model;
mod strategies;

pub fn to_new_york_time(timestamp: i64) -> DateTime<Tz> {
    let naive = NaiveDateTime::from_timestamp_millis(timestamp).unwrap();
    Utc.from_utc_datetime(&naive).with_timezone(&New_York)
}
