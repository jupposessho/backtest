extern crate rust_decimal;

use chrono::NaiveTime;
use clap::{Arg, Command};

use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, trade_result::TradeResult},
    strategies::orb_london_reversal::{OrbLondonReversal, OrbLondonReversalConfig},
};

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path)).expect("failed loading parquet")
}

fn run_case(data: &[CandleStick], config: OrbLondonReversalConfig) -> BacktestResult {
    execute(OrbLondonReversal {
        data: data.to_vec(),
        config,
    })
}

fn main() {
    let matches = Command::new("ORB London Reversal Sweep")
        .arg(
            Arg::new("source")
                .long("source")
                .value_parser(["gold", "mnq", "mes"])
                .default_value("mnq"),
        )
        .arg(
            Arg::new("max-bars")
                .long("max-bars")
                .value_parser(clap::value_parser!(usize))
                .required(false),
        )
        .get_matches();

    let source = matches
        .get_one::<String>("source")
        .map(|s| s.as_str())
        .unwrap_or("mnq");
    let max_bars = matches.get_one::<usize>("max-bars").copied();

    let mut data = match source {
        "gold" => load_parquet("assets/gold_1m_cont.parquet"),
        "mnq" => load_parquet("assets/mnq_1m_cont.parquet"),
        "mes" => load_parquet("assets/mes_1m_cont.parquet"),
        _ => unreachable!(),
    };

    if let Some(limit) = max_bars {
        if data.len() > limit {
            data = data.into_iter().take(limit).collect();
        }
    }

    let orb_windows = [15u32, 30u32];
    let session_ends = [(12u32, 0u32), (14u32, 0u32), (17u32, 0u32)];

    println!("\nORB London Reversal Sweep");
    println!("source: {}", source);
    println!("bars: {}", data.len());
    println!("\n{:<10} {:<8} {:>8} {:>8} {:>9} {:>9}", "orb", "close", "trades", "win%", "profit_r", "pnl%");
    println!("{}", "-".repeat(62));

    for window in orb_windows {
        for (end_h, end_m) in session_ends {
            let config = OrbLondonReversalConfig {
                orb_start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                orb_end: NaiveTime::from_hms_opt(8, window, 0).unwrap(),
                session_end: NaiveTime::from_hms_opt(end_h, end_m, 0).unwrap(),
                eod_close: true,
                ..OrbLondonReversalConfig::default()
            };

            let result = run_case(&data, config);
            let winners = result.result(TradeResult::Winner);
            let total = result.number_of_trades();
            let win_rate = if total == 0 {
                0.0
            } else {
                (winners as f64 / total as f64) * 100.0
            };

            println!(
                "{:<10} {:<8} {:>8} {:>7.2} {:>9} {:>9}",
                format!("{}m", window),
                format!("{:02}:{:02}", end_h, end_m),
                total,
                win_rate,
                result.profit_in_r(),
                result.pnl(),
            );
        }
    }
}
