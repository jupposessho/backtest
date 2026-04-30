extern crate rust_decimal;

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use clap::Parser;
use chrono::NaiveDate;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    model::{
        backtest_result::BacktestResult,
        candle_stick::CandleStick,
        fee_config::FeeConfig,
        trade::Trade,
        trade_result::TradeResult,
    },
    strategies::orb::{Orb, OrbConfig, OrbDuration, OrbSlType},
    execute,
};

// ── Data loaders ─────────────────────────────────────────────────────────────

fn load_btc() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_15m.json"))
}

fn load_eth() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_ETHUSDT_15m.json"))
}

fn load_sol() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_SOLUSDT_15m.json"))
}

// ── Metrics ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Stats {
    label: String,
    asset: String,
    trades: usize,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    profit_factor: Decimal,
    final_balance: Decimal,
}

fn equity_metrics(
    trades: &[Trade],
    start_capital: Decimal,
) -> (Decimal, Decimal, Decimal, Decimal) {
    let r = Decimal::from_f32(0.01).unwrap();
    let mut capital = start_capital;
    let mut peak = start_capital;
    let mut max_dd = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;

    for t in trades {
        let trade_pnl = match t.result {
            TradeResult::Winner => capital * r * t.rr().0.trunc_with_scale(2),
            TradeResult::Expense => -capital * r,
            TradeResult::BreakEven => Decimal::ZERO,
        };

        // Correct commission formula: position_size × fee_per_unit / risk_per_unit
        // position_size = capital * r  (in $ at 1% risk)
        // fee_per_unit = entry_commission + exit_commission (price × fee%)
        // risk_per_unit = |entry - sl|
        let risk = match t.direction {
            backtest::model::position_direction::PositionDirection::Long => t.entry.0 - t.sl.0,
            backtest::model::position_direction::PositionDirection::Short => t.sl.0 - t.entry.0,
        };
        let commission_cost = if risk > Decimal::ZERO {
            capital * r * (t.entry_commission.0 + t.exit_commission.0) / risk
        } else {
            Decimal::ZERO
        };

        let change = trade_pnl - commission_cost;

        if trade_pnl > Decimal::ZERO {
            gross_profit += trade_pnl;
        } else if trade_pnl < Decimal::ZERO {
            gross_loss += -trade_pnl;
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

    (capital.trunc_with_scale(2), max_dd, gross_profit, gross_loss)
}

fn compute_stats(label: String, asset: String, result: &BacktestResult) -> Stats {
    let trades = &result.trades;
    let total = trades.len();
    let winner_count = trades.iter().filter(|t| t.result == TradeResult::Winner).count();

    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_i32(winner_count as i32).unwrap()
            / Decimal::from_i32(total as i32).unwrap()
            * Decimal::from(100))
        .trunc_with_scale(1)
    };

    let (final_balance, max_dd, gross_profit, gross_loss) =
        equity_metrics(trades, Decimal::from(1000));

    let profit_factor = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).trunc_with_scale(2)
    } else {
        Decimal::ZERO
    };

    Stats {
        label,
        asset,
        trades: total,
        win_rate,
        max_drawdown_pct: max_dd,
        profit_factor,
        final_balance,
    }
}

fn run_case(label: &str, asset: &str, data: &[CandleStick], config: OrbConfig) -> Stats {
    let strategy = Orb {
        data: data.to_vec(),
        config,
    };
    let result = execute(strategy);
    compute_stats(label.to_string(), asset.to_string(), &result)
}

// ── Args & Filtering ─────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(about = "ORB strategy backtest")]
struct Args {
    /// Start date in YYYY-MM-DD format (candles before this date are excluded)
    #[arg(long)]
    from: Option<String>,
}

fn filter_from(candles: &[CandleStick], from_ts: Option<i64>) -> Vec<CandleStick> {
    match from_ts {
        Some(ts) => candles
            .iter()
            .copied()
            .filter(|c| c.open_time >= ts)
            .collect(),
        None => candles.to_vec(),
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let args = Args::parse();
    let from_ts: Option<i64> = args.from.map(|s| {
        NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .unwrap_or_else(|_| panic!("Invalid --from date '{}'. Expected YYYY-MM-DD", s))
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    });

    let btc = filter_from(&load_btc(), from_ts);
    let eth = filter_from(&load_eth(), from_ts);
    let sol = filter_from(&load_sol(), from_ts);

    let assets: &[(&str, &[CandleStick])] = &[
        ("BTC", &btc),
        ("ETH", &eth),
        ("SOL", &sol),
    ];

    let rr2 = Decimal::from(2);
    let rr3 = Decimal::from(3);
    let fee = FeeConfig::binance_standard();
    let no_fee = FeeConfig::zero();

    // Configuration matrix: (label, duration, rr_target)
    let configs: &[(&str, OrbDuration, Decimal)] = &[
        ("15min_rr2", OrbDuration::Minutes15, rr2),
        ("15min_rr3", OrbDuration::Minutes15, rr3),
        ("30min_rr2", OrbDuration::Minutes30, rr2),
        ("30min_rr3", OrbDuration::Minutes30, rr3),
    ];

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║              ORB STRATEGY — Opening Range Breakout (9:30 ET)                    ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("{:<18} {:<5} {:>7} {:>5} {:>8} {:>6} {:>10} {:>7}",
        "label", "asset", "trades", "win%", "max_dd%", "pf", "balance", "gain");
    println!("{}", "─".repeat(72));

    let mut all: Vec<Stats> = Vec::new();

    for (cfg_label, duration, rr) in configs {
        println!();
        for (asset_name, candles) in assets {
            let config = OrbConfig {
                duration: *duration,
                sl_type: OrbSlType::OppositeRange,
                rr_target: *rr,
                eod_close: true,
                fee_config: no_fee,
            };

            let stats = run_case(cfg_label, asset_name, candles, config);

            let gain = (stats.final_balance / Decimal::from(1000)).trunc_with_scale(2);

            println!("{:<18} {:<5} {:>7} {:>5} {:>8} {:>6} {:>10.2} {:>6}x",
                stats.label,
                stats.asset,
                stats.trades,
                stats.win_rate,
                stats.max_drawdown_pct,
                stats.profit_factor,
                stats.final_balance,
                gain,
            );

            all.push(stats);
        }
    }

    // Summary: best config per asset
    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         BEST CONFIG PER ASSET                                   ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("{:<5} {:<18} {:>10} {:>7} {:>7} {:>8}",
        "asset", "label", "balance", "gain", "trades", "win%");
    println!("{}", "═".repeat(60));

    for asset_name in ["BTC", "ETH", "SOL"] {
        let best = all
            .iter()
            .filter(|s| s.asset == asset_name)
            .max_by(|a, b| a.final_balance.cmp(&b.final_balance));

        if let Some(s) = best {
            let gain = (s.final_balance / Decimal::from(1000)).trunc_with_scale(2);
            println!("{:<5} {:<18} {:>10.2} {:>6}x {:>7} {:>8}",
                s.asset, s.label, s.final_balance, gain, s.trades, s.win_rate);
        }
    }

    println!();
    println!("Starting balance : $1,000  |  Risk/trade : 1%  |  Fees : Binance standard (0.02% maker / 0.06% taker)");
    if let Some(ts) = from_ts {
        let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap();
        println!("Date range       : from {} onwards", dt.format("%Y-%m-%d"));
    }
    println!("SL type          : OppositeRange (SL at far side of opening range)");
    println!("EOD close        : 16:30 ET  |  Session open : 09:30 ET");

    // ── Fee impact comparison ─────────────────────────────────────────────────

    let mut all_fee: Vec<Stats> = Vec::new();

    for (cfg_label, duration, rr) in configs {
        for (asset_name, candles) in assets {
            let config = OrbConfig {
                duration: *duration,
                sl_type: OrbSlType::OppositeRange,
                rr_target: *rr,
                eod_close: true,
                fee_config: fee,
            };
            all_fee.push(run_case(cfg_label, asset_name, candles, config));
        }
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║               FEE IMPACT  (Binance Standard 0.02% / 0.06%)                     ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("{:<18} {:<5} {:>12} {:>12} {:>8}", "label", "asset", "no_fee_bal", "fee_bal", "drag%");
    println!("{}", "─".repeat(60));

    for (without_fee, with_fee) in all.iter().zip(all_fee.iter()) {
        let drag_pct = if without_fee.final_balance > Decimal::ZERO {
            ((without_fee.final_balance - with_fee.final_balance)
                / without_fee.final_balance
                * Decimal::from(100))
            .trunc_with_scale(1)
        } else {
            Decimal::ZERO
        };

        println!(
            "{:<18} {:<5} {:>12.2} {:>12.2} {:>7.1}%",
            without_fee.label,
            without_fee.asset,
            without_fee.final_balance,
            with_fee.final_balance,
            drag_pct,
        );
    }

    println!();
    println!("Fee drag = (no_fee_balance - fee_balance) / no_fee_balance × 100");
}
