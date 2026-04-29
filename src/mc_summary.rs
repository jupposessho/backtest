extern crate rust_decimal;

use chrono::NaiveTime;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, trade_result::TradeResult},
    strategies::mc::{
        EntryMode, ExecutionConfig, FvgConfig, LevelFilters, MarketEntryMode, Mc, McConfig,
        McMode, SignalPattern, TimeWindow, TrailingStopConfig, TrailingStopMode, TrendFilter,
    },
};

fn load_binance_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_5m.json"
    ))
}

fn load_binance_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_15m.json"
    ))
}

fn load_binance_30m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_30m.json"
    ))
}

fn load_binance_1h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_1h.json"
    ))
}

fn load_binance_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_4h.json"
    ))
}

fn load_binance_12h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_12h.json"
    ))
}

#[derive(Clone, Debug)]
struct Stats {
    label: String,
    timeframe: String,
    trades: usize,
    winners: usize,
    losers: usize,
    break_evens: usize,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    profit_factor: Decimal,
    final_balance: Decimal,
    total_costs: Decimal,
}

fn compute_stats(label: String, timeframe: String, result: &BacktestResult) -> Stats {
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
        (Decimal::from_i32(winners as i32).unwrap()
            / Decimal::from_i32(total as i32).unwrap()
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
    let total_costs = trades.iter().map(|t| t.total_costs()).sum::<Decimal>().trunc_with_scale(2);

    Stats {
        label,
        timeframe,
        trades: total,
        winners,
        losers,
        break_evens,
        win_rate,
        max_drawdown_pct,
        profit_factor,
        final_balance,
        total_costs,
    }
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

    let final_balance = capital.trunc_with_scale(2);

    (final_balance, max_dd, gross_profit, gross_loss)
}

fn config_with(
    mode: McMode,
    pattern: SignalPattern,
    entry_mode: EntryMode,
    rr_target: Decimal,
    levels_enabled: bool,
    trend_filter: TrendFilter,
) -> McConfig {
    McConfig {
        mode,
        pattern,
        entry_mode,
        rr_target,
        trade_window: Some(TimeWindow::default()),
        prev_open_fill_window_candles: 3,
        level_filters: LevelFilters {
            enabled: levels_enabled,
            ..LevelFilters::default()
        },
        trend_filter,
        fvg_filter: FvgConfig {
            enabled: false,
            ..FvgConfig::default()
        },
        daily_open_time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        trailing_stop: TrailingStopConfig {
            mode: TrailingStopMode::None,
        },
        execution: ExecutionConfig {
            market_entry: MarketEntryMode::NextBarOpen,
            commission_rate_per_side: Decimal::from_f32(0.001).unwrap(),
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 1,
            tick_size: Decimal::from_f32(0.01).unwrap(),
        },
    }
}

fn run_case(label: String, timeframe: String, data: &Vec<CandleStick>, config: McConfig) -> Stats {
    let trading_model = Mc {
        data: data.clone(),
        config,
    };
    let result: BacktestResult = execute(trading_model);
    compute_stats(label, timeframe, &result)
}

fn main() {
    let timeframes = vec![
        ("5m", load_binance_5m()),
        ("15m", load_binance_15m()),
        ("30m", load_binance_30m()),
        ("1h", load_binance_1h()),
        ("4h", load_binance_4h()),
        ("12h", load_binance_12h()),
    ];

    let rr_1_5 = Decimal::from_f32(1.5).unwrap();
    let rr_2 = Decimal::from(2);

    // Define top baseline strategies only (no trailing variants)
    let strategies = vec![
        ("rev_daily_rr2_close", McMode::ReversalDaily, SignalPattern::Mc, EntryMode::Close, rr_2, true, TrendFilter::None),
        ("rev_daily_rr2_prevopen", McMode::ReversalDaily, SignalPattern::Mc, EntryMode::PrevOpen, rr_2, true, TrendFilter::None),
        ("cont_ema200_rr2_prevopen", McMode::ContinuationEma200, SignalPattern::Mc, EntryMode::PrevOpen, rr_2, false, TrendFilter::Ema { fast: 50, slow: 200 }),
        ("cont_ema200_rr1.5_prevopen", McMode::ContinuationEma200, SignalPattern::Mc, EntryMode::PrevOpen, rr_1_5, false, TrendFilter::Ema { fast: 50, slow: 200 }),
        ("cont_struct_rr2_prevopen", McMode::ContinuationStructure, SignalPattern::Mc, EntryMode::PrevOpen, rr_2, false, TrendFilter::MarketStructure),
        ("rev_daily_engulf_rr2_prevopen", McMode::ReversalDaily, SignalPattern::Engulfing, EntryMode::PrevOpen, rr_2, true, TrendFilter::None),
        ("cont_ema200_engulf_rr2_close", McMode::ContinuationEma200, SignalPattern::Engulfing, EntryMode::Close, rr_2, false, TrendFilter::Ema { fast: 50, slow: 200 }),
        ("cont_ema200_engulf_rr2_prevopen", McMode::ContinuationEma200, SignalPattern::Engulfing, EntryMode::PrevOpen, rr_2, false, TrendFilter::Ema { fast: 50, slow: 200 }),
        ("cont_struct_engulf_rr2_prevopen", McMode::ContinuationStructure, SignalPattern::Engulfing, EntryMode::PrevOpen, rr_2, false, TrendFilter::MarketStructure),
    ];

    let mut all_results: Vec<Stats> = Vec::new();

    println!("\n╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║              MULTI-TIMEFRAME ANALYSIS - TOP STRATEGIES ONLY                  ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝\n");

    // Run analysis for each timeframe
    for (tf_name, candlesticks) in &timeframes {
        println!("╔═══════════════════════════════════════════════════════════════════════════════╗");
        println!("║  TIMEFRAME: {:^68} ║", tf_name);
        println!("╚═══════════════════════════════════════════════════════════════════════════════╝\n");

        println!("{:<35} {:>7} {:>10} {:>8} {:>12} {:>13} {:>12} {:>12}",
            "strategy", "trades", "win_rate", "max_dd%", "profit_factor", "balance", "gain", "costs"
        );
        println!("{}", "-".repeat(100));

        for (label, mode, pattern, entry, rr, levels, trend) in &strategies {
            let config = config_with(mode.clone(), pattern.clone(), entry.clone(), *rr, *levels, trend.clone());
            let stats = run_case(label.to_string(), tf_name.to_string(), candlesticks, config);

            let gain_x = (stats.final_balance / Decimal::from(1000)).trunc_with_scale(2);

            println!("{:<35} {:>7} {:>10} {:>8} {:>13} {:>12.2} {:>11}x {:>12.2}",
                stats.label,
                stats.trades,
                stats.win_rate,
                stats.max_drawdown_pct,
                stats.profit_factor,
                stats.final_balance,
                gain_x,
                stats.total_costs
            );

            all_results.push(stats);
        }
        println!("\n");
    }

    // Summary: Best strategy per timeframe
    println!("\n╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    BEST STRATEGY PER TIMEFRAME                               ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝\n");

    println!("{:<10} {:<35} {:>12} {:>8} {:>7} {:>10}",
        "timeframe", "strategy", "balance", "gain", "trades", "win_rate"
    );
    println!("{}", "=".repeat(95));

    for (tf_name, _) in &timeframes {
        let mut best: Option<&Stats> = None;
        for stats in &all_results {
            if stats.timeframe == *tf_name {
                if best.is_none() || stats.final_balance > best.unwrap().final_balance {
                    best = Some(stats);
                }
            }
        }

        if let Some(best_stats) = best {
            let gain_x = (best_stats.final_balance / Decimal::from(1000)).trunc_with_scale(2);
            println!("{:<10} {:<35} {:>12.2} {:>7}x {:>7} {:>10}",
                best_stats.timeframe,
                best_stats.label,
                best_stats.final_balance,
                gain_x,
                best_stats.trades,
                best_stats.win_rate
            );
        }
    }

    // Overall best strategy across all timeframes
    println!("\n╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    TOP 10 ACROSS ALL TIMEFRAMES                              ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝\n");

    let mut sorted_results = all_results.clone();
    sorted_results.sort_by(|a, b| b.final_balance.cmp(&a.final_balance));

    println!("{:<6} {:<10} {:<35} {:>12} {:>8} {:>7} {:>10}",
        "rank", "timeframe", "strategy", "balance", "gain", "trades", "win_rate"
    );
    println!("{}", "=".repeat(105));

    for (i, stats) in sorted_results.iter().take(10).enumerate() {
        let gain_x = (stats.final_balance / Decimal::from(1000)).trunc_with_scale(2);
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        println!("{}{:<4} {:<10} {:<35} {:>12.2} {:>7}x {:>7} {:>10}",
            medal,
            i + 1,
            stats.timeframe,
            stats.label,
            stats.final_balance,
            gain_x,
            stats.trades,
            stats.win_rate
        );
    }

    println!("\n╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                              INSIGHTS                                         ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝\n");

    // Find best timeframe
    let mut tf_totals: std::collections::HashMap<String, (Decimal, usize)> = std::collections::HashMap::new();
    for stats in &all_results {
        let entry = tf_totals.entry(stats.timeframe.clone()).or_insert((Decimal::ZERO, 0));
        entry.0 += stats.final_balance;
        entry.1 += 1;
    }

    println!("Average Balance by Timeframe:");
    let mut tf_avgs: Vec<_> = tf_totals.iter()
        .map(|(tf, (total, count))| (tf.clone(), total / Decimal::from(*count as i32)))
        .collect();
    tf_avgs.sort_by(|a, b| b.1.cmp(&a.1));

    for (tf, avg) in tf_avgs {
        println!("  {:<10} Average: ${:>10.2}", tf, avg);
    }

    println!("\n=== NOTES ===");
    println!("Starting balance: $1000");
    println!("Risk per trade: 1% of current balance");
    println!("All results are BASELINE (no trailing stops)");
    println!("Only top 9 strategies tested per timeframe for clarity");
    println!("\nResults show historical backtest performance only.");
    println!("Past performance does not guarantee future results.");
}
