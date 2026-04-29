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
        McMode, SignalPattern, TimeWindow, TrailingStopConfig, TrendFilter,
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

#[derive(Clone)]
struct Stats {
    label: &'static str,
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
    trailing_stop: TrailingStopConfig,
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
        trailing_stop,
        execution: ExecutionConfig {
            market_entry: MarketEntryMode::NextBarOpen,
            commission_rate_per_side: Decimal::from_f32(0.001).unwrap(),
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 1,
            tick_size: Decimal::from_f32(0.01).unwrap(),
        },
    }
}

fn run_case(label: &'static str, data: &Vec<CandleStick>, config: McConfig) -> Stats {
    let trading_model = Mc {
        data: data.clone(),
        config,
    };
    let result: BacktestResult = execute(trading_model);
    compute_stats(label, &result)
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

    use backtest::strategies::mc::TrailingStopMode;

    let trail_none = TrailingStopConfig { mode: TrailingStopMode::None };
    let trail_be1r = TrailingStopConfig { mode: TrailingStopMode::BreakEven1R };
    let trail_05r_at_15r = TrailingStopConfig { mode: TrailingStopMode::Trail05RAt15R };
    let trail_1r_at_2r = TrailingStopConfig { mode: TrailingStopMode::Trail1RAt2R };
    let trail_progressive = TrailingStopConfig { mode: TrailingStopMode::Progressive };

    // Test cases to run (we'll run these for each timeframe)
    let test_cases = vec![
        // Original baseline cases with no trailing
        (
            "rev_daily_rr2_close",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                true,
                TrendFilter::None,
                trail_none.clone(),
            ),
        ),
        (
            "rev_daily_rr2_prevopen",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::PrevOpen,
                rr_2,
                true,
                TrendFilter::None,
                trail_none.clone(),
            ),
        ),
        (
            "rev_daily_rr1.5_close",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_1_5,
                true,
                TrendFilter::None,
                trail_none.clone(),
            ),
        ),
        (
            "rev_daily_rr1.5_prevopen",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::PrevOpen,
                rr_1_5,
                true,
                TrendFilter::None,
                trail_none.clone(),
            ),
        ),
        (
            "cont_ema200_rr2_close",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_none.clone(),
            ),
        ),
        (
            "cont_ema200_rr2_prevopen",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::PrevOpen,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_none.clone(),
            ),
        ),
        (
            "cont_ema200_rr1.5_close",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_1_5,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_none.clone(),
            ),
        ),
        (
            "cont_ema200_rr1.5_prevopen",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::PrevOpen,
                rr_1_5,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_none.clone(),
            ),
        ),
        (
            "cont_struct_rr2_close",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_none.clone(),
            ),
        ),
        (
            "cont_struct_rr2_prevopen",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Mc,
                EntryMode::PrevOpen,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_none.clone(),
            ),
        ),
        (
            "rev_daily_engulf_rr2_close",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Engulfing,
                EntryMode::Close,
                rr_2,
                true,
                TrendFilter::None,
                trail_none.clone(),
            ),
        ),
        (
            "rev_daily_engulf_rr2_prevopen",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Engulfing,
                EntryMode::PrevOpen,
                rr_2,
                true,
                TrendFilter::None,
                trail_none.clone(),
            ),
        ),
        (
            "cont_ema200_engulf_rr2_close",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Engulfing,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_none.clone(),
            ),
        ),
        (
            "cont_ema200_engulf_rr2_prevopen",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Engulfing,
                EntryMode::PrevOpen,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_none.clone(),
            ),
        ),
        (
            "cont_struct_engulf_rr2_close",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Engulfing,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_none.clone(),
            ),
        ),
        (
            "cont_struct_engulf_rr2_prevopen",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Engulfing,
                EntryMode::PrevOpen,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_none.clone(),
            ),
        ),

        // === TRAILING STOP VARIANTS ===
        // Reversal Daily with trailing stops
        (
            "rev_daily_rr2_close_BE1R",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                true,
                TrendFilter::None,
                trail_be1r.clone(),
            ),
        ),
        (
            "rev_daily_rr2_close_T05R",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                true,
                TrendFilter::None,
                trail_05r_at_15r.clone(),
            ),
        ),
        (
            "rev_daily_rr2_close_T1R",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                true,
                TrendFilter::None,
                trail_1r_at_2r.clone(),
            ),
        ),
        (
            "rev_daily_rr2_close_PROG",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                true,
                TrendFilter::None,
                trail_progressive.clone(),
            ),
        ),

        // Continuation EMA with trailing stops
        (
            "cont_ema200_rr2_close_BE1R",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_be1r.clone(),
            ),
        ),
        (
            "cont_ema200_rr2_close_T05R",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_05r_at_15r.clone(),
            ),
        ),
        (
            "cont_ema200_rr2_close_T1R",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_1r_at_2r.clone(),
            ),
        ),
        (
            "cont_ema200_rr2_close_PROG",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_progressive.clone(),
            ),
        ),

        // Continuation Structure with trailing stops
        (
            "cont_struct_rr2_close_BE1R",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_be1r.clone(),
            ),
        ),
        (
            "cont_struct_rr2_close_T05R",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_05r_at_15r.clone(),
            ),
        ),
        (
            "cont_struct_rr2_close_T1R",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_1r_at_2r.clone(),
            ),
        ),
        (
            "cont_struct_rr2_close_PROG",
            config_with(
                McMode::ContinuationStructure,
                SignalPattern::Mc,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::MarketStructure,
                trail_progressive.clone(),
            ),
        ),

        // Engulfing with trailing stops
        (
            "rev_daily_engulf_rr2_close_PROG",
            config_with(
                McMode::ReversalDaily,
                SignalPattern::Engulfing,
                EntryMode::Close,
                rr_2,
                true,
                TrendFilter::None,
                trail_progressive.clone(),
            ),
        ),
        (
            "cont_ema200_engulf_rr2_close_PROG",
            config_with(
                McMode::ContinuationEma200,
                SignalPattern::Engulfing,
                EntryMode::Close,
                rr_2,
                false,
                TrendFilter::Ema { fast: 50, slow: 200 },
                trail_progressive.clone(),
            ),
        ),
    ];

    // Run all test cases for each timeframe
    for (tf_name, candlesticks) in timeframes {
        println!("\n");
        println!("╔═══════════════════════════════════════════════════════════════════════════════╗");
        println!("║  TIMEFRAME: {:^68} ║", tf_name);
        println!("╚═══════════════════════════════════════════════════════════════════════════════╝");

        run_all_cases(tf_name, &candlesticks, &test_cases);
    }

    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  ANALYSIS COMPLETE - All Timeframes Processed                                ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════════╝");
    println!("\n=== LEGEND ===");
    println!("BE1R  = Break Even at 1R");
    println!("T05R  = Trail to 0.5R at 1.5R");
    println!("T1R   = Trail to 1R at 2R");
    println!("PROG  = Progressive (BE at 1R, 0.5R at 1.5R, 1R at 2R, continues...)");
    println!("\n=== BALANCE CALCULATION ===");
    println!("Starting balance: $1000");
    println!("Risk per trade: 1% of current balance");
    println!("Balance shown: Final account value after all trades");
    println!("\nExample: Balance of $19,311.70 means you turned $1000 into $19,311.70");
}

fn run_all_cases(tf_name: &str, candlesticks: &Vec<CandleStick>, test_cases: &Vec<(&'static str, McConfig)>) {
    println!("\n=== BASELINE (No Trailing Stops) ===");
    println!(
        "{:<28} {:>7} {:>10} {:>8} {:>8} {:>8} {:>12} {:>13} {:>12} {:>12}",
        "case", "trades", "win_rate", "wins", "losses", "b/e", "max_dd%", "profit_factor", "balance", "costs"
    );

    for (i, (label, cfg)) in test_cases.iter().enumerate() {
        // Print section headers
        if i == 16 {
            println!("\n=== TRAILING STOP VARIANTS ===");
            println!(
                "{:<28} {:>7} {:>10} {:>8} {:>8} {:>8} {:>12} {:>13} {:>12} {:>12}",
                "case", "trades", "win_rate", "wins", "losses", "b/e", "max_dd%", "profit_factor", "balance", "costs"
            );
        } else if i == 20 {
            println!("\n--- Continuation EMA200 Trailing Variants ---");
        } else if i == 24 {
            println!("\n--- Continuation Structure Trailing Variants ---");
        } else if i == 28 {
            println!("\n--- Engulfing Trailing Variants ---");
        }

        let stats = run_case(label, &candlesticks, cfg.clone());
        println!(
            "{:<28} {:>7} {:>10} {:>8} {:>8} {:>8} {:>12} {:>13} {:>12.2} {:>12.2}",
            stats.label,
            stats.trades,
            stats.win_rate,
            stats.winners,
            stats.losers,
            stats.break_evens,
            stats.max_drawdown_pct,
            stats.profit_factor,
            stats.final_balance,
            stats.total_costs
        );
    }
}
