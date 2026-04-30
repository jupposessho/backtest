extern crate rust_decimal;

use clap::{Arg, Command};
use rust_decimal::Decimal;
use std::sync::Arc;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, trade_result::TradeResult},
    strategies::ttrades_fractal_mtf::{FractalMTFConfig, TTradesFractalMTF},
};

fn load_binance_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_5m.json"))
}

fn load_binance_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_15m.json"))
}

fn load_binance_1h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_1h.json"))
}

fn load_binance_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_4h.json"))
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
    let matches = Command::new("TTrades Fractal MTF Runner")
        .arg(
            Arg::new("source")
                .long("source")
                .value_parser(["btc_5m_1h", "btc_15m_4h"])
                .default_value("btc_15m_4h"),
        )
        .arg(
            Arg::new("rr")
                .long("rr")
                .value_parser(clap::value_parser!(u32))
                .default_value("2"),
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
        .unwrap_or("btc_15m_4h");
    let rr_raw = *matches.get_one::<u32>("rr").unwrap_or(&2);
    let max_bars = matches.get_one::<usize>("max-bars").copied();

    let (mut ltf_data, mut htf_data, htf_name, ltf_name) = match source {
        "btc_5m_1h" => (load_binance_5m(), load_binance_1h(), "1h", "5m"),
        "btc_15m_4h" => (load_binance_15m(), load_binance_4h(), "4h", "15m"),
        _ => unreachable!(),
    };

    if let Some(limit) = max_bars {
        if ltf_data.len() > limit {
            ltf_data = ltf_data.into_iter().take(limit).collect();
        }
    }

    if let Some(last_ltf_open) = ltf_data.last().map(|c| c.open_time) {
        htf_data.retain(|c| c.open_time <= last_ltf_open);
    }

    let mut config = FractalMTFConfig::default();
    config.rr_target = Decimal::from(rr_raw);
    config.htf_name = htf_name;
    config.ltf_name = ltf_name;
    config.log_progress = true;

    let ltf_bars = ltf_data.len();
    let htf_bars = htf_data.len();

    let model = TTradesFractalMTF {
        htf_data: Arc::new(htf_data),
        ltf_data: Arc::new(ltf_data),
        config,
    };
    let result = execute(model);

    println!("TTrades Fractal MTF");
    println!("source: {source}");
    println!("rr_target: {}R", rr_raw);
    println!("htf_bars: {}", htf_bars);
    println!("ltf_bars: {}", ltf_bars);
    print_stats("base", &result);
}
