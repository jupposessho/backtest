extern crate rust_decimal;

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, fee_config::FeeConfig, trade::Trade, trade_result::TradeResult},
    strategies::ttrades_fractal::{FractalConfig, TTradesFractal},
};

fn load_btc_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_15m.json"))
}

#[derive(Debug, Clone)]
struct Stats {
    config_name: &'static str,
    trades: usize,
    winners: usize,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    profit_factor: Decimal,
    final_balance: Decimal,
    total_fees: Decimal,
}

fn compute_stats(config_name: &'static str, result: &BacktestResult) -> Stats {
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

    let total_fees: Decimal = trades.iter()
        .map(|t| t.total_commission().0)
        .sum();

    Stats {
        config_name,
        trades: total,
        winners,
        win_rate,
        max_drawdown_pct,
        profit_factor,
        final_balance,
        total_fees,
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

fn run_case(config_name: &'static str, data: &Vec<CandleStick>, config: FractalConfig) -> Stats {
    let trading_model = TTradesFractal {
        data: data.clone(),
        config,
    };
    let result: BacktestResult = execute(trading_model);
    compute_stats(config_name, &result)
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        TTRADES FRACTAL MODEL - QUICK TEST (BTC 15m)          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("Strategy: TTrades Fractal Model");
    println!("Based on: https://ttrades.com/the-only-trading-strategy-you-need-for-2026/\n");

    println!("Loading BTC 15m data...");
    let data = load_btc_15m();
    println!("Loaded {} candles\n", data.len());

    // Test configurations
    let configs = vec![
        ("standard_2r", FractalConfig {
            rr_target: Decimal::from(2),
            fee_config: FeeConfig::binance_standard(),
            use_fvg: true,
            lookback_candles: 20,
            require_cisd: true,
        }),
        ("aggressive_1.5r", FractalConfig {
            rr_target: Decimal::from_f32(1.5).unwrap(),
            fee_config: FeeConfig::binance_standard(),
            use_fvg: true,
            lookback_candles: 20,
            require_cisd: false,
        }),
        ("conservative_3r", FractalConfig {
            rr_target: Decimal::from(3),
            fee_config: FeeConfig::binance_standard(),
            use_fvg: true,
            lookback_candles: 30,
            require_cisd: true,
        }),
        ("zero_fees_2r", FractalConfig {
            rr_target: Decimal::from(2),
            fee_config: FeeConfig::zero(),
            use_fvg: true,
            lookback_candles: 20,
            require_cisd: true,
        }),
    ];

    let mut results: Vec<Stats> = vec![];

    // Run tests
    println!("Running tests...\n");
    for (config_name, config) in &configs {
        print!("  Testing {}...", config_name);
        let stats = run_case(config_name, &data, config.clone());
        println!(" {} trades", stats.trades);
        results.push(stats);
    }

    // Display results
    println!("\n╔════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                    RESULTS - BTC 15m                                   ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("{:<20} {:>7} {:>10} {:>8} {:>12} {:>13} {:>12} {:>12}",
        "config", "trades", "win_rate", "max_dd%", "profit_factor", "balance", "gain", "fees"
    );
    println!("{}", "=".repeat(110));

    for stats in &results {
        let gain_x = if stats.final_balance > Decimal::ZERO {
            (stats.final_balance / Decimal::from(1000)).trunc_with_scale(1)
        } else {
            Decimal::ZERO
        };

        println!(
            "{:<20} {:>7} {:>9}% {:>7}% {:>12} {:>13.2} {:>11}x {:>12.2}",
            stats.config_name,
            stats.trades,
            stats.win_rate,
            stats.max_drawdown_pct,
            stats.profit_factor,
            stats.final_balance,
            gain_x,
            stats.total_fees,
        );
    }

    println!("\n\n╔════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         STRATEGY ANALYSIS                                              ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("📖 TTrades Fractal Model Key Principles:");
    println!("   1. Daily Bias - Direction from daily candle close vs previous day");
    println!("   2. Points of Interest - Fair Value Gaps and swing highs/lows");
    println!("   3. CISD - Change in State of Delivery for confirmation");
    println!("   4. Continuation Order Blocks - Entry after sweep and reversal");
    println!("   5. Multi-timeframe Alignment - All timeframes must agree\n");

    // Find best configuration
    let best = results.iter().max_by(|a, b| a.final_balance.cmp(&b.final_balance));
    if let Some(best_config) = best {
        println!("🏆 BEST CONFIGURATION: {}", best_config.config_name);
        println!("   Balance: ${:.2} ({:.1}x)", best_config.final_balance, best_config.final_balance / Decimal::from(1000));
        println!("   Trades: {}", best_config.trades);
        println!("   Win Rate: {}%", best_config.win_rate);
        println!("   Profit Factor: {}", best_config.profit_factor);
        println!("   Max Drawdown: {}%", best_config.max_drawdown_pct);
        println!("   Fees Paid: ${:.2}\n", best_config.total_fees);
    }

    // Compare fees impact
    let with_fees = results.iter().find(|s| s.config_name == "standard_2r");
    let without_fees = results.iter().find(|s| s.config_name == "zero_fees_2r");

    if let (Some(with), Some(without)) = (with_fees, without_fees) {
        println!("💰 FEE IMPACT ANALYSIS:");
        println!("   Zero Fees:     ${:.2}", without.final_balance);
        println!("   With Fees:     ${:.2}", with.final_balance);
        println!("   Fee Cost:      ${:.2}", without.final_balance - with.final_balance);
        let impact_pct = if without.final_balance > Decimal::ZERO {
            ((without.final_balance - with.final_balance) / without.final_balance * Decimal::from(100)).trunc_with_scale(2)
        } else {
            Decimal::ZERO
        };
        println!("   Impact:        {}%\n", impact_pct);
    }

    println!("⚠️  Important Notes:");
    println!("   • Lower-frequency strategy - fewer trades by design");
    println!("   • Requires patience for proper setup alignment");
    println!("   • Avoids choppy/range-bound markets automatically");
    println!("   • Default 2R target with protective stops");
    println!("   • Best in trending market conditions\n");

    println!("✅ Quick test complete!\n");
}
