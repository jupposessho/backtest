use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::{America::New_York, Tz};
use model::backtest_result::BacktestResult;
use model::decimal::DecimalVec;
use model::trading_model::TradingModel;
use rust_decimal::Decimal;
use std::error::Error;

pub mod candle_stick_loader;
pub mod chart;
pub mod engine;
pub mod model;
pub mod strategies;

pub fn to_new_york_time(timestamp: i64) -> DateTime<Tz> {
    DateTime::from_timestamp(timestamp, 0)
        .unwrap()
        .with_timezone(&New_York)
}

fn parse_decimal(s: &str) -> Result<DecimalVec, Box<dyn Error>> {
    Ok(DecimalVec(s.parse::<Decimal>()?))
}
pub fn parse_datetime(s: &str) -> Result<DateTime<Tz>, Box<dyn Error>> {
    let naive_datetime = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| format!("Error converting datetime:{}", s))?;
    let ny_datetime = New_York
        .from_local_datetime(&naive_datetime)
        .single()
        .expect("Failed to convert to New York time");
    Ok(ny_datetime)
}

pub fn execute<T: TradingModel>(model: T) -> BacktestResult {
    model.execute()
}
