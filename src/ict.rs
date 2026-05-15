extern crate rust_decimal;

use clap::{Arg, Command};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    engine::types::ExecutionConfig,
    execute,
    model::{
        backtest_result::BacktestResult, candle_stick::CandleStick, trade_result::TradeResult,
    },
    strategies::ict_composed::{IctComposed, IctEntryChoice},
};

#[derive(Clone)]
struct Stats {
    label: &'static str,
    trades: usize,
    winners: usize,
    losers: usize,
    break_evens: usize,
    win_rate: Decimal,
    profit_factor: Decimal,
    final_balance: Decimal,
    max_drawdown_pct: Decimal,
    total_costs: Decimal,
}

fn load_binance_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_5m.json"))
}

fn load_binance_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_15m.json"))
}

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .expect("failed loading parquet")
}

fn equity_metrics(
    trades: &[backtest::model::trade::Trade],
    start_capital: Decimal,
) -> (Decimal, Decimal, Decimal, Decimal) {
    let r = Decimal::from_f32(0.01).unwrap();
    let mut capital = start_capital;
    let mut peak = start_capital;
    let mut max_dd = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;

    for t in trades {
        let change = capital * r * t.gross_r().trunc_with_scale(4) - t.total_costs();
        if change > Decimal::ZERO {
            gross_profit += change;
        } else if change < Decimal::ZERO {
            gross_loss += -change;
        }
        capital += change;

        if capital > peak {
            peak = capital;
        }
        if peak > Decimal::ZERO {
            let dd = ((peak - capital) / peak * Decimal::from(100)).trunc_with_scale(2);
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }

    (
        capital.trunc_with_scale(2),
        max_dd,
        gross_profit,
        gross_loss,
    )
}

fn compute_stats(label: &'static str, result: &BacktestResult) -> Stats {
    let trades = &result.trades;
    let total = trades.len();
    let winners = trades
        .iter()
        .filter(|t| t.result == TradeResult::Winner)
        .count();
    let losers = trades
        .iter()
        .filter(|t| t.result == TradeResult::Expense)
        .count();
    let break_evens = trades
        .iter()
        .filter(|t| t.result == TradeResult::BreakEven)
        .count();

    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_i32(winners as i32).unwrap() / Decimal::from_i32(total as i32).unwrap()
            * Decimal::from(100))
        .trunc_with_scale(2)
    };

    let (final_balance, max_drawdown_pct, gross_profit, gross_loss) =
        equity_metrics(trades, Decimal::from(1000));

    let profit_factor = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).trunc_with_scale(2)
    } else {
        Decimal::ZERO
    };

    let total_costs = trades
        .iter()
        .map(|t| t.total_costs())
        .sum::<Decimal>()
        .trunc_with_scale(2);

    Stats {
        label,
        trades: total,
        winners,
        losers,
        break_evens,
        win_rate,
        profit_factor,
        final_balance,
        max_drawdown_pct,
        total_costs,
    }
}

fn run_case(label: &'static str, data: &Vec<CandleStick>, entry_choice: IctEntryChoice) -> Stats {
    let model = IctComposed {
        data: data.clone(),
        rr_target: Decimal::from(2),
        sweep_lookback: 24,
        mss_confirm_window: 24,
        entry_choice,
        entry_expiry_bars: 24,
        execution: ExecutionConfig {
            commission_rate_per_side: Decimal::from_f32(0.001).unwrap(),
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 1,
            tick_size: Decimal::from_f32(0.01).unwrap(),
        },
    };
    let result = execute(model);
    compute_stats(label, &result)
}

fn print_table(title: &str, data: &Vec<CandleStick>) {
    let results = vec![
        run_case("ob_prev_open", data, IctEntryChoice::ObPrevOpen),
        run_case("ob_pair_midpoint", data, IctEntryChoice::ObPairMidpoint),
        run_case("ote_midpoint", data, IctEntryChoice::OteMidpoint),
    ];

    println!("\n=== {title} ===");
    println!(
        "{:<18} {:>7} {:>8} {:>8} {:>8} {:>8} {:>10} {:>12} {:>10} {:>12}",
        "entry", "trades", "wins", "losses", "b/e", "win%", "pf", "balance", "max_dd%", "costs"
    );

    for s in results {
        println!(
            "{:<18} {:>7} {:>8} {:>8} {:>8} {:>8} {:>10} {:>12.2} {:>10} {:>12.2}",
            s.label,
            s.trades,
            s.winners,
            s.losers,
            s.break_evens,
            s.win_rate,
            s.profit_factor,
            s.final_balance,
            s.max_drawdown_pct,
            s.total_costs,
        );
    }
}

fn main() {
    let matches = Command::new("ICT entry comparison")
        .arg(
            Arg::new("source")
                .long("source")
                .value_parser(["binance5m", "binance15m", "gold", "mnq", "mes"])
                .default_value("binance5m"),
        )
        .get_matches();

    let source = matches
        .get_one::<String>("source")
        .map(|s| s.as_str())
        .unwrap_or("binance5m");

    println!("ICT composed strategy entry comparison");
    println!("Costs: 0.1% per side, slippage: 1 tick per side\n");

    match source {
        "binance5m" => {
            let data = load_binance_5m();
            print_table("BTCUSDT 5m", &data);
        }
        "binance15m" => {
            let data = load_binance_15m();
            print_table("BTCUSDT 15m", &data);
        }
        "gold" => {
            let data = load_parquet("assets/gold_1m_cont.parquet");
            print_table("GOLD 1m parquet", &data);
        }
        "mnq" => {
            let data = load_parquet("assets/mnq_1m_cont.parquet");
            print_table("MNQ 1m parquet", &data);
        }
        "mes" => {
            let data = load_parquet("assets/mes_1m_cont.parquet");
            print_table("MES 1m parquet", &data);
        }
        _ => unreachable!(),
    }
}
