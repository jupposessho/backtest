use crate::model::{
    barchart::BarChartCandle, binance_klines_item::BinanceKlinesItem, candle_stick::CandleStick,
    decimal::DecimalVec,
};
use crate::{parse_datetime, parse_decimal};
use rust_decimal::Decimal;
use std::io::{self, BufRead, BufReader};
use std::str::FromStr;
use std::{error::Error, fs::File, path::Path};

pub struct CandleStickLoader {}

impl CandleStickLoader {
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
}
