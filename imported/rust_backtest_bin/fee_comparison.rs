extern crate rust_decimal;

use chrono::NaiveTime;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, fee_config::FeeConfig, trade::Trade, trade_result::TradeResult},
    strategies::mc::{
        EntryMode, EngulfingConfig, FvgConfig, LevelFilters, Mc, McConfig, McMode, SignalPattern, TimeWindow,
        TrailingStopConfig, TrailingStopMode, TrendFilter,
    },
};

fn load_btc_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_15m.json"))
}

#[derive(Debug, Clone)]
struct Stats {
    fee_name: &'static str,
    trades: usize,
    winners: usize,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    profit_factor: Decimal,
    final_balance: Decimal,
    total_fees_paid: Decimal,
}

fn compute_stats(fee_name: &'static str, result: &BacktestResult) -> Stats {
    let trades = &result.trades;
    let total = trades.len();

    let winners = trades.iter().filter(|t| t.result == TradeResult::Winner).count();

    let win_rate = if total > 0 {
        (Decimal::from(winners) / Decimal::from(total) * Decimal::from(100)).trunc_with_scale(2)
    } else {
        Decimal::ZERO
    };

    let (final_balance, max_drawdown_pct, gross_profit, gross_loss) =
        equity_metrics(trades, Decimal::from(1000));

    let profit_factor = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).trunc_with_scale(2)
    } else {
        Decimal::from(999)
    };

    let total_fees_paid: Decimal = trades.iter()
        .map(|t| t.total_commission().0)
        .sum();

    Stats {
        fee_name,
        trades: total,
        winners,
        win_rate,
        max_drawdown_pct,
        profit_factor,
        final_balance,
        total_fees_paid,
    }
}

fn equity_metrics(trades: &[Trade], starting_capital: Decimal) -> (Decimal, Decimal, Decimal, Decimal) {
    let risk_per_trade_pct = Decimal::from_f32(0.01).unwrap();
    let mut balance = starting_capital;
    let mut peak = starting_capital;
    let mut max_dd_pct = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;

    for trade in trades {
        let risk_amount = balance * risk_per_trade_pct;
        let risk_distance = (trade.sl.0 - trade.entry.0).abs();

        if risk_distance <= Decimal::ZERO {
            continue;
        }

        let position_size = risk_amount / risk_distance;
        let entry_value = position_size * trade.entry.0;
        let entry_commission = trade.entry_commission.0 * position_size;

        let exit_price = match trade.result {
            TradeResult::Winner => trade.tp.0,
            TradeResult::Expense => trade.sl.0,
            TradeResult::BreakEven => trade.entry.0,
        };

        let exit_value = position_size * exit_price;
        let exit_commission = trade.exit_commission.0 * position_size;

        let pnl = match trade.direction {
            backtest::model::position_direction::PositionDirection::Long => {
                exit_value - entry_value - entry_commission - exit_commission
            }
            backtest::model::position_direction::PositionDirection::Short => {
                entry_value - exit_value - entry_commission - exit_commission
            }
        };

        balance += pnl;

        if pnl > Decimal::ZERO {
            gross_profit += pnl;
        } else {
            gross_loss += pnl.abs();
        }

        if balance > peak {
            peak = balance;
        }

        if peak > Decimal::ZERO {
            let dd_pct = ((peak - balance) / peak * Decimal::from(100)).trunc_with_scale(2);
            if dd_pct > max_dd_pct {
                max_dd_pct = dd_pct;
            }
        }
    }

    (balance, max_dd_pct, gross_profit, gross_loss)
}

fn create_config(fee_config: FeeConfig) -> McConfig {
    McConfig {
        mode: McMode::ContinuationEma200,
        pattern: SignalPattern::Engulfing,
        entry_mode: EntryMode::PrevOpen,
        rr_target: Decimal::from(2),
        trade_window: Some(TimeWindow::default()),
        prev_open_fill_window_candles: 3,
        level_filters: LevelFilters {
            enabled: false,
            ..LevelFilters::default()
        },
        trend_filter: TrendFilter::Ema { fast: 50, slow: 200 },
        fvg_filter: FvgConfig {
            enabled: false,
            ..FvgConfig::default()
        },
        daily_open_time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        trailing_stop: TrailingStopConfig { mode: TrailingStopMode::None },
        engulfing_config: EngulfingConfig::default(),  // No filters - any engulfing pattern
        fee_config,
    }
}

fn run_with_fees(fee_name: &'static str, data: &Vec<CandleStick>, fee_config: FeeConfig) -> Stats {
    let config = create_config(fee_config);
    let trading_model = Mc {
        data: data.clone(),
        config,
    };
    let result: BacktestResult = execute(trading_model);
    compute_stats(fee_name, &result)
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          FEE STRUCTURE COMPARISON - BTC 15m                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("Strategy: Continuation EMA200 + Engulfing (default filters)");
    println!("RR Target: 2:1");
    println!("Entry: Previous Daily Open\n");

    let data = load_btc_15m();

    // Test different fee structures
    let fee_configs = vec![
        ("Zero Fees", FeeConfig::zero()),
        ("Legacy (0%/0.005%)", FeeConfig::legacy()),
        ("Binance Standard (0.02%/0.06%)", FeeConfig::binance_standard()),
        ("Binance VIP1 (0.016%/0.04%)", FeeConfig::binance_vip1()),
        ("Binance VIP2 (0.014%/0.035%)", FeeConfig::binance_vip2()),
        ("Conservative (0.05%/0.1%)", FeeConfig::conservative()),
    ];

    let mut results: Vec<Stats> = vec![];

    for (name, fee_config) in fee_configs {
        let stats = run_with_fees(name, &data, fee_config);
        results.push(stats);
    }

    // Display results
    println!("╔════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                    RESULTS                                             ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("{:<30} {:>7} {:>10} {:>8} {:>12} {:>13} {:>12}",
        "Fee Structure", "trades", "win_rate", "max_dd%", "profit_factor", "balance", "fees_paid"
    );
    println!("{}", "=".repeat(110));

    for stats in &results {
        println!(
            "{:<30} {:>7} {:>9}% {:>7}% {:>12} {:>13.2} {:>12.2}",
            stats.fee_name,
            stats.trades,
            stats.win_rate,
            stats.max_drawdown_pct,
            stats.profit_factor,
            stats.final_balance,
            stats.total_fees_paid,
        );
    }

    // Calculate impact
    println!("\n╔════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                  FEE IMPACT ANALYSIS                                   ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════╝\n");

    if let Some(zero_fees) = results.first() {
        println!("Baseline (Zero Fees): ${:.2}\n", zero_fees.final_balance);

        for stats in results.iter().skip(1) {
            let difference = zero_fees.final_balance - stats.final_balance;
            let pct_impact = if zero_fees.final_balance > Decimal::ZERO {
                (difference / zero_fees.final_balance * Decimal::from(100)).trunc_with_scale(2)
            } else {
                Decimal::ZERO
            };

            println!(
                "{:<30} Impact: ${:>8.2} ({:>5.2}%) | Fees Paid: ${:>8.2}",
                stats.fee_name,
                difference,
                pct_impact,
                stats.total_fees_paid
            );
        }
    }

    println!("\n\n╔════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                          KEY FINDINGS - ENGULFING DISCREPANCY                          ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("⚠️  IMPORTANT DISCOVERY:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("The discrepancy between 'cargo run --bin mc' and 'cargo run --bin test_engulfing'");
    println!("occurs because they use DIFFERENT engulfing configurations:\n");

    println!("📊 mc.rs (showing profitable results):");
    println!("   └─ Uses: EngulfingConfig::default()");
    println!("   └─ Filters: ALL DISABLED");
    println!("      • require_sweep: false");
    println!("      • require_body_size_ratio: None");
    println!("      • require_close_above_prev_high: false");
    println!("   └─ Effect: Accepts ANY engulfing pattern (very loose)\n");

    println!("📊 test_engulfing.rs (testing different configurations):");
    println!("   └─ Tests 4 configurations with STRICT filters:");
    println!("      • baseline: No filters (same as mc.rs)");
    println!("      • sweep_only: Requires liquidity sweep");
    println!("      • sweep_body_1.5x: Sweep + body 1.5x larger");
    println!("      • all_filters_2.0x: All filters + body 2x larger\n");

    println!("💡 CONCLUSION:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("The profitable results in mc.rs come from accepting ALL engulfing patterns,");
    println!("not just high-quality ones. The test_engulfing.rs with stricter filters shows");
    println!("that filtered engulfing patterns don't perform as well.\n");

    println!("📋 RECOMMENDATION:");
    println!("Run test_engulfing.rs and look at the 'baseline' configuration results.");
    println!("That will match the mc.rs results for engulfing patterns.\n");

    println!("✅ Fee structure is now configurable - you can test with different fee tiers!");
    println!();
}
