use crate::model::{
    barchart::BarChartCandle, binance_klines_item::BinanceKlinesItem, candle_stick::CandleStick,
    decimal::DecimalVec,
};
use crate::{parse_datetime, parse_decimal};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::Field;
use rust_decimal::Decimal;
use std::io::{self, BufRead, BufReader};
use std::str::FromStr;
use std::{error::Error, fs::File, path::Path};

pub struct CandleStickLoader {}

pub enum CandleDataSource<'a> {
    BinanceJsonStr(&'a str),
    JsonStr(&'a str),
    CsvPath(&'a str),
    BarChartPath(&'a str),
    ParquetPath(&'a str),
}

impl CandleStickLoader {
    pub fn load_source(source: CandleDataSource<'_>) -> Result<Vec<CandleStick>, Box<dyn Error>> {
        match source {
            CandleDataSource::BinanceJsonStr(s) => Ok(Self::load_binance(s)),
            CandleDataSource::JsonStr(s) => Ok(Self::load(s)),
            CandleDataSource::CsvPath(p) => Self::load_csv(p),
            CandleDataSource::BarChartPath(p) => Self::load_bar_chart(p),
            CandleDataSource::ParquetPath(p) => Self::load_parquet(p),
        }
    }

    pub fn load_binance(file_path: &str) -> Vec<CandleStick> {
        let raw_data: Vec<BinanceKlinesItem> = serde_json::from_str(file_path).unwrap();

        raw_data
            .iter()
            .enumerate()
            .map(|(_, v)| CandleStick {
                open_time: v.open_time as i64 / 1000,
                open: DecimalVec(Decimal::from_str(v.open.as_str()).unwrap()),
                high: DecimalVec(Decimal::from_str(v.high.as_str()).unwrap()),
                low: DecimalVec(Decimal::from_str(v.low.as_str()).unwrap()),
                close: DecimalVec(Decimal::from_str(v.close.as_str()).unwrap()),
                close_time: v.close_time as i64 / 1000,
            })
            .collect::<Vec<_>>()
    }

    pub fn load(file_path: &str) -> Vec<CandleStick> {
        serde_json::from_str(file_path).unwrap()
    }

    pub fn load_csv(file_path: &str) -> Result<Vec<CandleStick>, Box<dyn Error>> {
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

            let candlestick = CandleStick {
                open_time: parse_datetime(fields[0])?.timestamp(),
                close_time: parse_datetime(fields[0])?.timestamp(), // TODO: fix
                open: parse_decimal(fields[1])?,
                high: parse_decimal(fields[2])?,
                low: parse_decimal(fields[3])?,
                close: parse_decimal(fields[4])?,
            };

            candlesticks.push(candlestick);
        }

        Ok(candlesticks)
    }

    pub fn load_bar_chart(file_path: &str) -> Result<Vec<CandleStick>, Box<dyn Error>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut candles = Vec::new();

        for line in reader.lines() {
            let line = line?;
            println!("{}", line);
            let raw: BarChartCandle = serde_json::from_str(&line)?;
            let candle: CandleStick = raw.try_into()?;
            candles.push(candle);
        }

        Ok(candles)
    }

    pub fn load_parquet(file_path: &str) -> Result<Vec<CandleStick>, Box<dyn Error>> {
        let file = File::open(file_path)?;
        let reader = SerializedFileReader::new(file)?;
        let iter = reader.get_row_iter(None)?;

        let mut candles = Vec::new();

        for row in iter {
            let row = row?;
            let open_time = find_i64(&row, &["open_time", "timestamp", "ts", "time", "datetime"])
                .ok_or("missing timestamp column")?;

            let open = find_decimal(&row, &["open", "o"]).ok_or("missing open column")?;
            let high = find_decimal(&row, &["high", "h"]).ok_or("missing high column")?;
            let low = find_decimal(&row, &["low", "l"]).ok_or("missing low column")?;
            let close = find_decimal(&row, &["close", "c"]).ok_or("missing close column")?;

            let close_time = find_i64(&row, &["close_time", "close_ts", "close_timestamp"])
                .unwrap_or(open_time + 60);

            let open_time_sec = normalize_epoch(open_time);
            let close_time_sec = normalize_epoch(close_time);

            candles.push(CandleStick {
                open_time: open_time_sec,
                open: DecimalVec(open),
                high: DecimalVec(high),
                low: DecimalVec(low),
                close: DecimalVec(close),
                close_time: close_time_sec,
            });
        }

        Ok(candles)
    }
}

fn normalize_epoch(ts: i64) -> i64 {
    let mut out = ts;
    while out > 10_000_000_000 {
        out /= 1000;
    }
    out
}

fn find_decimal(row: &parquet::record::Row, names: &[&str]) -> Option<Decimal> {
    for (name, field) in row.get_column_iter() {
        let lower = name.to_ascii_lowercase();
        if names.iter().any(|n| *n == lower.as_str()) {
            if let Some(v) = field_to_decimal(field) {
                return Some(v);
            }
        }
    }
    None
}

fn find_i64(row: &parquet::record::Row, names: &[&str]) -> Option<i64> {
    for (name, field) in row.get_column_iter() {
        let lower = name.to_ascii_lowercase();
        if names.iter().any(|n| *n == lower.as_str()) {
            if let Some(v) = field_to_i64(field) {
                return Some(v);
            }
        }
    }
    None
}

fn field_to_decimal(field: &Field) -> Option<Decimal> {
    match field {
        Field::Double(v) => Decimal::from_f64_retain(*v),
        Field::Float(v) => Decimal::from_f32_retain(*v),
        Field::Int(v) => Some(Decimal::from(*v)),
        Field::Long(v) => Some(Decimal::from(*v)),
        Field::Str(s) => Decimal::from_str(s).ok(),
        _ => None,
    }
}

fn field_to_i64(field: &Field) -> Option<i64> {
    match field {
        Field::Int(v) => Some(*v as i64),
        Field::Long(v) => Some(*v),
        Field::Str(s) => s.parse::<i64>().ok(),
        _ => None,
    }
}
