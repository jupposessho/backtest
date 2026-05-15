extern crate rust_decimal;

use chrono::NaiveTime;
use rayon::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::sync::Arc;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{
        backtest_result::BacktestResult, candle_stick::CandleStick, trade::Trade,
        trade_result::TradeResult,
    },
    strategies::mc::{
        EntryMode, ExecutionConfig, FvgConfig, LevelFilters, MarketEntryMode, Mc, McConfig, McMode,
        SignalPattern, TimeWindow, TrailingStopConfig, TrailingStopMode, TrendFilter,
    },
};

#[derive(Clone)]
struct SweepCase {
    label: String,
    timeframe: String,
    mode: McMode,
    entry_mode: EntryMode,
    rr_target: Decimal,
    trailing: TrailingStopMode,
    slippage_ticks_per_side: i32,
    commission_rate_per_side: Decimal,
}

fn mode_name(mode: &McMode) -> &'static str {
    match mode {
        McMode::Auto => "auto",
        McMode::ReversalDaily => "reversal_daily",
        McMode::ContinuationEma200 => "continuation_ema200",
        McMode::ContinuationStructure => "continuation_structure",
    }
}

fn entry_name(mode: &EntryMode) -> &'static str {
    match mode {
        EntryMode::Close => "close",
        EntryMode::PrevOpen => "prev_open",
        EntryMode::PairMidpoint => "pair_midpoint",
        EntryMode::PairExtreme => "pair_extreme",
    }
}

fn trailing_name(mode: &TrailingStopMode) -> &'static str {
    match mode {
        TrailingStopMode::None => "none",
        TrailingStopMode::StepHalfR => "step_half_r",
        TrailingStopMode::BreakEven1R => "be_1r",
        TrailingStopMode::Trail05RAt15R => "trail_05r_at_15r",
        TrailingStopMode::Trail1RAt2R => "trail_1r_at_2r",
        TrailingStopMode::Progressive => "progressive",
    }
}

#[derive(Clone)]
struct SweepRow {
    case: SweepCase,
    trades: usize,
    win_rate: Decimal,
    max_drawdown_pct: Decimal,
    gross_balance: Decimal,
    net_balance: Decimal,
    gross_profit_factor: Decimal,
    net_profit_factor: Decimal,
    total_costs: Decimal,
}

fn load_binance_5m() -> Arc<Vec<CandleStick>> {
    Arc::new(CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_5m.json"
    )))
}

fn load_binance_15m() -> Arc<Vec<CandleStick>> {
    Arc::new(CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_15m.json"
    )))
}

fn build_config(case: &SweepCase) -> McConfig {
    let (levels_enabled, trend_filter) = match case.mode {
        McMode::ReversalDaily => (true, TrendFilter::None),
        McMode::ContinuationEma200 => (
            false,
            TrendFilter::Ema {
                fast: 50,
                slow: 200,
            },
        ),
        _ => (true, TrendFilter::None),
    };

    McConfig {
        mode: case.mode.clone(),
        pattern: SignalPattern::Mc,
        entry_mode: case.entry_mode.clone(),
        rr_target: case.rr_target,
        trade_window: Some(TimeWindow {
            start: NaiveTime::from_hms_opt(5, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        }),
        prev_open_fill_window_candles: 3,
        trailing_stop: TrailingStopConfig {
            mode: case.trailing.clone(),
        },
        level_filters: LevelFilters {
            enabled: levels_enabled,
            sweep_window_candles: 5,
        },
        trend_filter,
        fvg_filter: FvgConfig {
            enabled: false,
            ..FvgConfig::default()
        },
        daily_open_time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        execution: ExecutionConfig {
            market_entry: MarketEntryMode::NextBarOpen,
            commission_rate_per_side: case.commission_rate_per_side,
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: case.slippage_ticks_per_side,
            tick_size: Decimal::from_f32(0.01).unwrap(),
        },
        ..McConfig::default()
    }
}

fn compute_equity_metrics(
    trades: &[Trade],
    start_capital: Decimal,
    include_costs: bool,
) -> (Decimal, Decimal, Decimal, Decimal) {
    let r = Decimal::from_f32(0.01).unwrap();
    let mut capital = start_capital;
    let mut peak = start_capital;
    let mut max_dd = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;

    for t in trades {
        let mut change = capital * r * t.gross_r().trunc_with_scale(4);
        if include_costs {
            change -= t.total_costs();
        }

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

fn run_case(data: Arc<Vec<CandleStick>>, case: &SweepCase) -> SweepRow {
    let config = build_config(case);
    let model = Mc {
        data: data.as_ref().clone(),
        config,
    };
    let result: BacktestResult = execute(model);

    let trades = result.trades;
    let total = trades.len();
    let winners = trades
        .iter()
        .filter(|t| t.result == TradeResult::Winner)
        .count();
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_i32(winners as i32).unwrap() / Decimal::from_i32(total as i32).unwrap()
            * Decimal::from(100))
        .trunc_with_scale(2)
    };

    let (gross_balance, _gross_dd, gross_profit, gross_loss) =
        compute_equity_metrics(&trades, Decimal::from(1000), false);
    let (net_balance, max_drawdown_pct, net_profit, net_loss) =
        compute_equity_metrics(&trades, Decimal::from(1000), true);
    let gross_profit_factor = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).trunc_with_scale(2)
    } else {
        Decimal::ZERO
    };
    let net_profit_factor = if net_loss > Decimal::ZERO {
        (net_profit / net_loss).trunc_with_scale(2)
    } else {
        Decimal::ZERO
    };
    let total_costs = trades
        .iter()
        .map(|t| t.total_costs())
        .sum::<Decimal>()
        .trunc_with_scale(2);

    SweepRow {
        case: case.clone(),
        trades: total,
        win_rate,
        max_drawdown_pct,
        gross_balance,
        net_balance,
        gross_profit_factor,
        net_profit_factor,
        total_costs,
    }
}

fn main() {
    let datasets: Vec<(String, Arc<Vec<CandleStick>>)> = vec![
        ("5m".to_string(), load_binance_5m()),
        ("15m".to_string(), load_binance_15m()),
    ];

    let rr_targets = [
        Decimal::ONE,
        Decimal::from_f32(1.5).unwrap(),
        Decimal::from(2),
    ];
    let entry_modes = [EntryMode::Close, EntryMode::PrevOpen];
    let trailing_modes = [TrailingStopMode::None, TrailingStopMode::BreakEven1R];
    let modes = [McMode::ReversalDaily, McMode::ContinuationEma200];
    let slippage_ticks = [0, 1, 2, 3];
    let commission = Decimal::from_f32(0.001).unwrap();

    let mut cases: Vec<SweepCase> = Vec::new();
    for (tf, _) in &datasets {
        for mode in &modes {
            for entry_mode in &entry_modes {
                for rr in &rr_targets {
                    for trailing in &trailing_modes {
                        for slip in &slippage_ticks {
                            let label = format!(
                                "mode={}|entry={}|rr={}|trail={}|slip={}",
                                mode_name(mode),
                                entry_name(entry_mode),
                                rr,
                                trailing_name(trailing),
                                slip
                            );
                            cases.push(SweepCase {
                                label,
                                timeframe: tf.clone(),
                                mode: mode.clone(),
                                entry_mode: entry_mode.clone(),
                                rr_target: *rr,
                                trailing: trailing.clone(),
                                slippage_ticks_per_side: *slip,
                                commission_rate_per_side: commission,
                            });
                        }
                    }
                }
            }
        }
    }

    let worker_cap = std::cmp::min(
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        8,
    );
    rayon::ThreadPoolBuilder::new()
        .num_threads(worker_cap)
        .build_global()
        .ok();

    let rows: Vec<SweepRow> = cases
        .par_iter()
        .map(|case| {
            let data = datasets
                .iter()
                .find(|(name, _)| name == &case.timeframe)
                .map(|(_, d)| Arc::clone(d))
                .expect("missing dataset");
            run_case(data, case)
        })
        .collect();

    println!(
        "label,timeframe,trades,win_rate,max_dd_pct,gross_balance,net_balance,gross_pf,net_pf,total_costs"
    );
    for r in rows {
        println!(
            "{},{},{},{},{},{},{},{},{},{}",
            r.case.label,
            r.case.timeframe,
            r.trades,
            r.win_rate,
            r.max_drawdown_pct,
            r.gross_balance,
            r.net_balance,
            r.gross_profit_factor,
            r.net_profit_factor,
            r.total_costs
        );
    }
}
