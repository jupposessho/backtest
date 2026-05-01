extern crate rust_decimal;

use clap::{Arg, Command};
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, trade_result::TradeResult},
    strategies::orb_london_reversal::{OrbLondonReversal, OrbLondonReversalConfig},
};

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path)).expect("failed loading parquet")
}

fn print_stats(name: &str, result: &BacktestResult) {
    let winners = result.result(TradeResult::Winner);
    let losers = result.result(TradeResult::Expense);
    let break_evens = result.result(TradeResult::BreakEven);
    let total = result.number_of_trades();
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as u32) / Decimal::from(total as u32) * Decimal::from(100)
    };

    println!("\n=== {name} ===");
    println!("trades      : {}", total);
    println!("winners     : {}", winners);
    println!("losers      : {}", losers);
    println!("break_evens : {}", break_evens);
    println!("win_rate%   : {}", win_rate.round_dp(2));
    println!("profit_r    : {}", result.profit_in_r());
    println!("points      : {}", result.profit_in_points());
    println!("costs_total : {}", result.costs_total());
    println!("pnl%        : {}", result.pnl());
}

fn main() {
    let matches = Command::new("ORB London Reversal Runner")
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

    let bars = data.len();
    let model = OrbLondonReversal {
        data,
        config: OrbLondonReversalConfig::default(),
    };
    let result = execute(model);

    println!("ORB London Reversal");
    println!("source: {source}");
    println!("bars: {}", bars);
    print_stats("base", &result);
}
