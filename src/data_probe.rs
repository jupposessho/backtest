use clap::{Arg, Command};

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};

fn main() {
    let matches = Command::new("Data source probe")
        .about("Validate source parsing into CandleStick")
        .arg(
            Arg::new("source")
                .long("source")
                .value_parser(["gold", "mnq", "mes", "binance5m", "binance15m"])
                .required(true),
        )
        .get_matches();

    let source = matches
        .get_one::<String>("source")
        .map(|s| s.as_str())
        .unwrap_or("gold");

    let loaded = match source {
        "gold" => CandleStickLoader::load_source(CandleDataSource::ParquetPath(
            "assets/gold_1m_cont.parquet",
        )),
        "mnq" => CandleStickLoader::load_source(CandleDataSource::ParquetPath(
            "assets/mnq_1m_cont.parquet",
        )),
        "mes" => CandleStickLoader::load_source(CandleDataSource::ParquetPath(
            "assets/mes_1m_cont.parquet",
        )),
        "binance5m" => CandleStickLoader::load_source(CandleDataSource::BinanceJsonStr(
            include_str!("../assets/binance_BTCUSDT_5m.json"),
        )),
        "binance15m" => CandleStickLoader::load_source(CandleDataSource::BinanceJsonStr(
            include_str!("../assets/binance_BTCUSDT_15m.json"),
        )),
        _ => unreachable!(),
    };

    let candles = match loaded {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to load source {source}: {e}");
            std::process::exit(1);
        }
    };

    if candles.is_empty() {
        println!("source={source} rows=0");
        return;
    }

    let first = candles.first().unwrap();
    let last = candles.last().unwrap();

    let mut invalid_ohlc = 0usize;
    let mut non_monotonic = 0usize;
    let mut prev_open_time = first.open_time;
    for c in &candles {
        let max_oc = if c.open > c.close { c.open } else { c.close };
        let min_oc = if c.open < c.close { c.open } else { c.close };
        if !(c.low <= min_oc && c.high >= max_oc && c.low <= c.high) {
            invalid_ohlc += 1;
        }
        if c.open_time < prev_open_time {
            non_monotonic += 1;
        }
        prev_open_time = c.open_time;
    }

    println!("source={source}");
    println!("rows={}", candles.len());
    println!("first_open_time={}", first.open_time);
    println!("last_open_time={}", last.open_time);
    println!("invalid_ohlc_rows={}", invalid_ohlc);
    println!("non_monotonic_open_time_rows={}", non_monotonic);
}
