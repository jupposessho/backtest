use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::{America::New_York, Tz};
use model::candle_ny::CandleNY;
use model::decimal::DecimalVec;
use rust_decimal::Decimal;
use std::io::{self, BufRead};
use std::{error::Error, fs::File, path::Path};

pub mod chart;
pub mod model;
mod strategies;

pub fn to_new_york_time(timestamp: i64) -> DateTime<Tz> {
    DateTime::from_timestamp(timestamp, 0)
        .unwrap()
        .with_timezone(&New_York)
}

fn parse_decimal(s: &str) -> Result<DecimalVec, Box<dyn Error>> {
    Ok(DecimalVec(s.parse::<Decimal>()?))
}
fn parse_datetime(s: &str) -> Result<DateTime<Tz>, Box<dyn Error>> {
    let naive_datetime = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| format!("Error converting datetime:{}", s))?;
    let ny_datetime = New_York
        .from_local_datetime(&naive_datetime)
        .single()
        .expect("Failed to convert to New York time");
    Ok(ny_datetime)
}

fn read_csv(file_path: &str) -> Result<Vec<CandleNY>, Box<dyn Error>> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);

    let mut candlesticks = Vec::new();

    for (_, line) in reader.lines().enumerate() {
        let line = line?;

        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 5 {
            return Err(Box::from("Invalid CSV format"));
        }

        let candlestick = CandleNY {
            open_time: parse_datetime(fields[0])?,
            open: parse_decimal(fields[1])?,
            high: parse_decimal(fields[2])?,
            low: parse_decimal(fields[3])?,
            close: parse_decimal(fields[4])?,
        };

        candlesticks.push(candlestick);
    }

    Ok(candlesticks)
}
