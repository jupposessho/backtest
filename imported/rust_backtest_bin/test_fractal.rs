extern crate rust_decimal;

use chrono::NaiveTime;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, fee_config::FeeConfig, trade::Trade, trade_result::TradeResult},
    strategies::ttrades_fractal::{FractalConfig, TTradesFractal},
};

fn load_btc_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_5m.json"))
}

fn load_btc_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_15m.json"))
}

fn load_eth_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_ETHUSDT_5m.json"))
}

fn load_eth_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_ETHUSDT_15m.json"))
}

fn load_sol_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_SOLUSDT_5m.json"))
}

fn load_sol_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_SOLUSDT_15m.json"))
}

#[derive(Debug, Clone)]
struct Stats {
    asset: &'static str,
    timeframe: &'static str,
    config_name: &'static str,
    trades: usize,
    winners: usize,
    losers: usize,
    break_evens: usize,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    profit_factor: Decimal,
    final_balance: Decimal,
    total_fees: Decimal,
}

fn compute_stats(asset: &'static str, timeframe: &'static str, config_name: &'static str, result: &BacktestResult) -> Stats {
    let trades = &result.trades;
    let total = trades.len();

    let winners = trades.iter().filter(|t| t.result == TradeResult::Winner).count();
    let losers = trades.iter().filter(|t| t.result == TradeResult::Expense).count();
    let break_evens = trades.iter().filter(|t| t.result == TradeResult::BreakEven).count();

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
        asset,
        timeframe,
        config_name,
        trades: total,
        winners,
        losers,
        break_evens,
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

fn run_case(asset: &'static str, timeframe: &'static str, config_name: &'static str, data: &Vec<CandleStick>, config: FractalConfig) -> Stats {
    let trading_model = TTradesFractal {
        data: data.clone(),
        config,
    };
    let result: BacktestResult = execute(trading_model);
    compute_stats(asset, timeframe, config_name, &result)
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        TTRADES FRACTAL MODEL - MULTI-ASSET ANALYSIS          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("Strategy: TTrades Fractal Model");
    println!("Based on: https://ttrades.com/the-only-trading-strategy-you-need-for-2026/\n");

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
        ("no_fvg_2r", FractalConfig {
            rr_target: Decimal::from(2),
            fee_config: FeeConfig::binance_standard(),
            use_fvg: false,
            lookback_candles: 20,
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

    // Load all datasets
    let datasets = vec![
        ("BTC", "15m", load_btc_15m()),
        ("BTC", "5m", load_btc_5m()),
        ("ETH", "15m", load_eth_15m()),
        ("ETH", "5m", load_eth_5m()),
        ("SOL", "15m", load_sol_15m()),
        ("SOL", "5m", load_sol_5m()),
    ];

    let mut all_results: Vec<Stats> = vec![];

    // Run tests
    for (asset, timeframe, data) in &datasets {
        println!("Testing {} {}...", asset, timeframe);
        for (config_name, config) in &configs {
            let stats = run_case(asset, timeframe, config_name, data, config.clone());
            all_results.push(stats);
        }
    }

    // Display results by asset/timeframe
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                         RESULTS BY ASSET                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    for (asset, timeframe, _) in &datasets {
        println!("\n{} {} - TTrades Fractal Model", asset, timeframe);
        println!("{}", "=".repeat(110));
        println!("{:<20} {:>7} {:>10} {:>8} {:>12} {:>13} {:>12} {:>12}",
            "config", "trades", "win_rate", "max_dd%", "profit_factor", "balance", "gain", "fees"
        );
        println!("{}", "-".repeat(110));

        let mut asset_results: Vec<&Stats> = all_results.iter()
            .filter(|s| s.asset == *asset && s.timeframe == *timeframe)
            .collect();
        asset_results.sort_by(|a, b| b.final_balance.cmp(&a.final_balance));

        for stats in asset_results {
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
    }

    // Find overall best
    println!("\n\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                      BEST CONFIGURATIONS                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let mut sorted = all_results.clone();
    sorted.sort_by(|a, b| b.final_balance.cmp(&a.final_balance));

    println!("🏆 TOP 10 BY BALANCE:");
    for (i, stats) in sorted.iter().take(10).enumerate() {
        println!("  {}. {} {} - {} → ${:.2} ({:.1}x) | {} trades | WR: {}%",
            i + 1, stats.asset, stats.timeframe, stats.config_name,
            stats.final_balance,
            stats.final_balance / Decimal::from(1000),
            stats.trades,
            stats.win_rate,
        );
    }

    println!("\n💰 TOP 5 BY PROFIT FACTOR (min 50 trades):");
    let mut by_pf: Vec<&Stats> = all_results.iter().filter(|s| s.trades >= 50).collect();
    by_pf.sort_by(|a, b| b.profit_factor.cmp(&a.profit_factor));
    for (i, stats) in by_pf.iter().take(5).enumerate() {
        println!("  {}. {} {} - {} → PF: {}, ${:.2}",
            i + 1, stats.asset, stats.timeframe, stats.config_name,
            stats.profit_factor, stats.final_balance
        );
    }

    println!("\n📊 BEST WIN RATE (min 50 trades):");
    let mut by_wr: Vec<&Stats> = all_results.iter().filter(|s| s.trades >= 50).collect();
    by_wr.sort_by(|a, b| b.win_rate.cmp(&a.win_rate));
    for (i, stats) in by_wr.iter().take(5).enumerate() {
        println!("  {}. {} {} - {} → WR: {}%, ${:.2}",
            i + 1, stats.asset, stats.timeframe, stats.config_name,
            stats.win_rate, stats.final_balance
        );
    }

    // Summary by config
    println!("\n\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                    AVERAGE BY CONFIGURATION                    ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    for (config_name, _) in &configs {
        let config_results: Vec<&Stats> = all_results.iter()
            .filter(|s| s.config_name == *config_name)
            .collect();

        let avg_balance: Decimal = config_results.iter()
            .map(|s| s.final_balance)
            .sum::<Decimal>() / Decimal::from(config_results.len());

        let avg_trades: usize = config_results.iter()
            .map(|s| s.trades)
            .sum::<usize>() / config_results.len();

        let avg_wr: Decimal = config_results.iter()
            .map(|s| s.win_rate)
            .sum::<Decimal>() / Decimal::from(config_results.len());

        let avg_fees: Decimal = config_results.iter()
            .map(|s| s.total_fees)
            .sum::<Decimal>() / Decimal::from(config_results.len());

        println!("{:<20} Avg Balance: ${:>9.2}  Avg Trades: {:>5}  Avg WR: {:>5.1}%  Avg Fees: ${:>8.2}",
            config_name, avg_balance, avg_trades, avg_wr, avg_fees
        );
    }

    println!("\n\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                         STRATEGY NOTES                         ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("📖 TTrades Fractal Model Key Principles:");
    println!("   1. Daily Bias - Direction determined by daily candle close vs previous day");
    println!("   2. Points of Interest - Fair Value Gaps and swing highs/lows");
    println!("   3. CISD - Change in State of Delivery for confirmation");
    println!("   4. Continuation Order Blocks - Entry after sweep and reversal");
    println!("   5. Multi-timeframe Alignment - All timeframes must agree\n");

    println!("⚠️  Important Notes:");
    println!("   • This is a lower-frequency strategy by design");
    println!("   • Requires patience - waits for proper setup alignment");
    println!("   • Avoids choppy/range-bound markets");
    println!("   • Default target is 2R with protective stops");
    println!("   • Best suited for trending market conditions\n");

    println!("✅ Analysis complete!\n");
}
