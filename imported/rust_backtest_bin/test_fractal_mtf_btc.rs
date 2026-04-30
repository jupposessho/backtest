extern crate rust_decimal;

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, fee_config::FeeConfig, trade::Trade, trade_result::TradeResult},
    strategies::ttrades_fractal_mtf::{FractalMTFConfig, TTradesFractalMTF},
};

fn load_btc_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_5m.json"))
}

fn load_btc_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_15m.json"))
}

fn load_btc_1h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_1h.json"))
}

fn load_btc_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_4h.json"))
}

fn load_eth_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_ETHUSDT_5m.json"))
}

fn load_eth_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_ETHUSDT_15m.json"))
}

fn load_eth_1h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_ETHUSDT_1h.json"))
}

fn load_eth_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_ETHUSDT_4h.json"))
}

fn load_sol_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_SOLUSDT_5m.json"))
}

fn load_sol_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_SOLUSDT_15m.json"))
}

fn load_sol_1h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_SOLUSDT_1h.json"))
}

fn load_sol_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_SOLUSDT_4h.json"))
}

#[derive(Debug, Clone)]
struct Stats {
    pair_name: String,
    trades: usize,
    winners: usize,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    profit_factor: Decimal,
    final_balance: Decimal,
    total_fees: Decimal,
}

fn compute_stats(pair_name: &str, result: &BacktestResult) -> Stats {
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
        pair_name: pair_name.to_string(),
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

fn run_case(
    pair_name: &str,
    htf_data: &Vec<CandleStick>,
    ltf_data: &Vec<CandleStick>,
    config: FractalMTFConfig,
) -> Stats {
    let trading_model = TTradesFractalMTF {
        htf_data: htf_data.clone(),
        ltf_data: ltf_data.clone(),
        config,
    };
    let result: BacktestResult = execute(trading_model);
    compute_stats(pair_name, &result)
}

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║   TTRADES FRACTAL MODEL - MULTI-TIMEFRAME (BTC/ETH/SOL)       ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("Strategy: TTrades Fractal Model - Multi-Timeframe Edition");
    println!("Concept: HTF for bias/structure, LTF for entry execution\n");

    // Load all assets/timeframes
    println!("Loading data...");
    let btc_4h = load_btc_4h();
    let btc_1h = load_btc_1h();
    let btc_15m = load_btc_15m();
    let btc_5m = load_btc_5m();

    let eth_4h = load_eth_4h();
    let eth_1h = load_eth_1h();
    let eth_15m = load_eth_15m();
    let eth_5m = load_eth_5m();

    let sol_4h = load_sol_4h();
    let sol_1h = load_sol_1h();
    let sol_15m = load_sol_15m();
    let sol_5m = load_sol_5m();

    println!("  BTC 4h:  {} candles", btc_4h.len());
    println!("  BTC 1h:  {} candles", btc_1h.len());
    println!("  BTC 15m: {} candles", btc_15m.len());
    println!("  BTC 5m:  {} candles", btc_5m.len());

    println!("  ETH 4h:  {} candles", eth_4h.len());
    println!("  ETH 1h:  {} candles", eth_1h.len());
    println!("  ETH 15m: {} candles", eth_15m.len());
    println!("  ETH 5m:  {} candles", eth_5m.len());

    println!("  SOL 4h:  {} candles", sol_4h.len());
    println!("  SOL 1h:  {} candles", sol_1h.len());
    println!("  SOL 15m: {} candles", sol_15m.len());
    println!("  SOL 5m:  {} candles\n", sol_5m.len());

    // Test different timeframe combinations
    let test_pairs = vec![
        ("BTC 4h/15m", &btc_4h, &btc_15m, "4h", "15m"),
        ("BTC 1h/5m", &btc_1h, &btc_5m, "1h", "5m"),
        ("BTC 4h/5m", &btc_4h, &btc_5m, "4h", "5m"),
        ("BTC 1h/15m", &btc_1h, &btc_15m, "1h", "15m"),
        ("ETH 4h/15m", &eth_4h, &eth_15m, "4h", "15m"),
        ("ETH 1h/5m", &eth_1h, &eth_5m, "1h", "5m"),
        ("ETH 4h/5m", &eth_4h, &eth_5m, "4h", "5m"),
        ("ETH 1h/15m", &eth_1h, &eth_15m, "1h", "15m"),
        ("SOL 4h/15m", &sol_4h, &sol_15m, "4h", "15m"),
        ("SOL 1h/5m", &sol_1h, &sol_5m, "1h", "5m"),
        ("SOL 4h/5m", &sol_4h, &sol_5m, "4h", "5m"),
        ("SOL 1h/15m", &sol_1h, &sol_15m, "1h", "15m"),
    ];

    let mut all_results: Vec<Stats> = vec![];

    // Run tests with different R:R targets
    let rr_targets = vec![
        ("1.5R", Decimal::from_f32(1.5).unwrap()),
        ("2R", Decimal::from(2)),
        ("3R", Decimal::from(3)),
    ];

    println!("Running tests...\n");
    for (pair_name, htf_data, ltf_data, htf_label, ltf_label) in &test_pairs {
        println!("Testing {} (HTF: {}, LTF: {})...", pair_name, htf_label, ltf_label);

        for (rr_name, rr_target) in &rr_targets {
            print!("  {} with fees...", rr_name);
            let config_with_fees = FractalMTFConfig {
                rr_target: *rr_target,
                fee_config: FeeConfig::binance_standard(),
                htf_name: htf_label,
                ltf_name: ltf_label,
            };
            let stats = run_case(
                &format!("{} {}", pair_name, rr_name),
                htf_data,
                ltf_data,
                config_with_fees,
            );
            println!(" {} trades", stats.trades);
            all_results.push(stats);

            // Also test with zero fees for comparison
            print!("  {} zero fees...", rr_name);
            let config_no_fees = FractalMTFConfig {
                rr_target: *rr_target,
                fee_config: FeeConfig::zero(),
                htf_name: htf_label,
                ltf_name: ltf_label,
            };
            let stats_no_fees = run_case(
                &format!("{} {} (no fees)", pair_name, rr_name),
                htf_data,
                ltf_data,
                config_no_fees,
            );
            println!(" {} trades", stats_no_fees.trades);
            all_results.push(stats_no_fees);
        }
        println!();
    }

    // Display results
    println!("\n╔════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                  RESULTS - MULTI-ASSET MTF (BTC/ETH/SOL)                                ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("{:<25} {:>7} {:>10} {:>8} {:>12} {:>13} {:>12} {:>12}",
        "timeframe_pair", "trades", "win_rate", "max_dd%", "profit_factor", "balance", "gain", "fees"
    );
    println!("{}", "=".repeat(115));

    // Sort by final balance
    let mut sorted = all_results.clone();
    sorted.sort_by(|a, b| b.final_balance.cmp(&a.final_balance));

    for stats in &sorted {
        let gain_x = if stats.final_balance > Decimal::ZERO {
            (stats.final_balance / Decimal::from(1000)).trunc_with_scale(1)
        } else {
            Decimal::ZERO
        };

        println!(
            "{:<25} {:>7} {:>9}% {:>7}% {:>12} {:>13.2} {:>11}x {:>12.2}",
            stats.pair_name,
            stats.trades,
            stats.win_rate,
            stats.max_drawdown_pct,
            stats.profit_factor,
            stats.final_balance,
            gain_x,
            stats.total_fees,
        );
    }

    // Analysis section
    println!("\n\n╔════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                         ANALYSIS                                                       ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("🏆 TOP 5 CONFIGURATIONS:");
    for (i, stats) in sorted.iter().take(5).enumerate() {
        println!("  {}. {} → ${:.2} ({:.1}x) | {} trades | WR: {}%",
            i + 1,
            stats.pair_name,
            stats.final_balance,
            stats.final_balance / Decimal::from(1000),
            stats.trades,
            stats.win_rate,
        );
    }

    // Compare fee impact for each timeframe pair
    println!("\n💰 FEE IMPACT BY TIMEFRAME PAIR:");
    for (pair_name, _, _, _, _) in &test_pairs {
        let with_fees: Vec<&Stats> = all_results.iter()
            .filter(|s| s.pair_name.starts_with(pair_name) && !s.pair_name.contains("no fees"))
            .collect();

        let without_fees: Vec<&Stats> = all_results.iter()
            .filter(|s| s.pair_name.starts_with(pair_name) && s.pair_name.contains("no fees"))
            .collect();

        if !with_fees.is_empty() && !without_fees.is_empty() {
            let avg_with: Decimal = with_fees.iter().map(|s| s.final_balance).sum::<Decimal>() / Decimal::from(with_fees.len());
            let avg_without: Decimal = without_fees.iter().map(|s| s.final_balance).sum::<Decimal>() / Decimal::from(without_fees.len());
            let impact = avg_without - avg_with;
            let impact_pct = if avg_without > Decimal::ZERO {
                (impact / avg_without * Decimal::from(100)).trunc_with_scale(2)
            } else {
                Decimal::ZERO
            };

            println!("  {} → Impact: ${:.2} ({:.1}%)", pair_name, impact, impact_pct);
        }
    }

    // Compare R:R performance
    println!("\n📊 PERFORMANCE BY R:R TARGET (avg across all TF pairs):");
    for (rr_name, _) in &rr_targets {
        let rr_results: Vec<&Stats> = all_results.iter()
            .filter(|s| s.pair_name.contains(rr_name) && !s.pair_name.contains("no fees"))
            .collect();

        if !rr_results.is_empty() {
            let avg_balance: Decimal = rr_results.iter().map(|s| s.final_balance).sum::<Decimal>() / Decimal::from(rr_results.len());
            let avg_trades: usize = rr_results.iter().map(|s| s.trades).sum::<usize>() / rr_results.len();
            let avg_wr: Decimal = rr_results.iter().map(|s| s.win_rate).sum::<Decimal>() / Decimal::from(rr_results.len());

            println!("  {} → Avg Balance: ${:.2} | Avg Trades: {} | Avg WR: {:.1}%",
                rr_name, avg_balance, avg_trades, avg_wr
            );
        }
    }

    println!("\n\n╔════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                      KEY INSIGHTS                                                      ║");
    println!("╚════════════════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    println!("📖 Multi-Timeframe Approach:");
    println!("   • HTF (4h/1h) determines bias and structure");
    println!("   • LTF (15m/5m) provides precise entry timing");
    println!("   • Alignment between timeframes reduces false signals");
    println!("   • Lower trade frequency compared to single-TF strategies\n");

    println!("🎯 Best Timeframe Combinations:");
    println!("   • 4h/15m: Balanced - good for swing style");
    println!("   • 1h/5m: More frequent - better for active trading");
    println!("   • 4h/5m: Very precise entries on major setups");
    println!("   • 1h/15m: Medium frequency with clean structure\n");

    println!("⚠️  Important Considerations:");
    println!("   • Lower trade count = lower fee impact");
    println!("   • Higher R:R requires more patience but better reward");
    println!("   • Multi-timeframe alignment is key to strategy success");
    println!("   • Performance varies based on market conditions\n");

    println!("✅ Test complete!\n");
}
