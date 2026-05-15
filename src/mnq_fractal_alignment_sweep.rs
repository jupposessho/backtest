extern crate rust_decimal;

use clap::{Arg, ArgAction, Command};
use rayon::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::sync::Arc;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick},
    strategies::fractal_alignment::{
        resample_from_1m, BagMode, EntryVariant, FractalAlignmentConfig, FractalAlignmentPlaybook,
        SessionFilter,
    },
};
use chrono::NaiveTime;

#[derive(Clone)]
struct SweepCase {
    fast_ema: usize,
    slow_ema: usize,
    anchor_ema: usize,
    rr_target: Decimal,
    max_trigger_bars: usize,
    max_hold_bars: usize,
    session: SessionFilter,
    slippage_ticks_per_side: i32,
    entry_variant: EntryVariant,
    bag_mode: BagMode,
    min_bag_gap_ticks: i32,
    inversion_min_body_ticks: i32,
    max_bars_after_bag_confirm: usize,
    anchor_range_min_mult: Decimal,
    stop_buffer_ticks: i32,
    require_anchor_expansion: bool,
    inversion_close_pct: Decimal,
}

#[derive(Clone)]
struct Row {
    label: String,
    trades: usize,
    win_rate: Decimal,
    gross_points: Decimal,
    net_points: Decimal,
    gross_usd: Decimal,
    net_usd: Decimal,
    profit_factor: Decimal,
    max_drawdown_usd: Decimal,
}

fn load_mnq_1m() -> Vec<CandleStick> {
    CandleStickLoader::load_parquet("assets/mnq_1m_cont.parquet").expect("load mnq parquet")
}

fn validate_data(data: &[CandleStick], spacing: i64) {
    assert!(!data.is_empty(), "empty dataset");
    for i in 1..data.len() {
        let prev = data[i - 1];
        let cur = data[i];
        assert!(
            cur.open_time > prev.open_time,
            "timestamp order broken at {i}"
        );
        assert!(cur.high >= cur.low, "high/low broken at {i}");
        assert!(
            cur.high >= cur.open && cur.high >= cur.close,
            "ohlc broken at {i}"
        );
        assert!(
            cur.low <= cur.open && cur.low <= cur.close,
            "ohlc broken at {i}"
        );
        let delta = cur.open_time - prev.open_time;
        assert!(delta % spacing == 0, "unexpected spacing at {i}: {delta}");
    }
}

fn summarize(label: String, result: BacktestResult) -> Row {
    let point_value = Decimal::from(2);
    let trades = result.trades.len();
    let winners = result
        .trades
        .iter()
        .filter(|trade| trade.points().0 > Decimal::ZERO)
        .count();
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_usize(winners).unwrap() / Decimal::from_usize(trades).unwrap()
            * Decimal::from(100))
        .round_dp(2)
    };

    let gross_points = result
        .trades
        .iter()
        .map(|trade| trade.points().0)
        .sum::<Decimal>()
        .round_dp(2);
    let total_cost_points = result
        .trades
        .iter()
        .map(|trade| trade.total_costs())
        .sum::<Decimal>()
        .round_dp(2);
    let net_points = (gross_points - total_cost_points).round_dp(2);
    let gross_usd = (gross_points * point_value).round_dp(2);
    let net_usd = (net_points * point_value).round_dp(2);

    let mut equity = Decimal::ZERO;
    let mut peak = Decimal::ZERO;
    let mut max_drawdown = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;
    for trade in &result.trades {
        let trade_usd = (trade.points().0 - trade.total_costs()) * point_value;
        if trade_usd > Decimal::ZERO {
            gross_profit += trade_usd;
        } else if trade_usd < Decimal::ZERO {
            gross_loss += -trade_usd;
        }
        equity += trade_usd;
        if equity > peak {
            peak = equity;
        }
        let drawdown = peak - equity;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    let profit_factor = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).round_dp(2)
    } else if gross_profit > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    };

    Row {
        label,
        trades,
        win_rate,
        gross_points,
        net_points,
        gross_usd,
        net_usd,
        profit_factor,
        max_drawdown_usd: max_drawdown.round_dp(2),
    }
}

fn session_name(session: SessionFilter) -> &'static str {
    match session {
        SessionFilter::All => "all",
        SessionFilter::NyOpen => "ny_open",
        SessionFilter::NyAm => "ny_am",
    }
}

fn entry_name(entry: EntryVariant) -> &'static str {
    match entry {
        EntryVariant::BreakoutOnly => "breakout",
        EntryVariant::RmbRetestOnly => "rmb_retest",
        EntryVariant::BreakoutOrRmbRetest => "breakout_or_rmb",
    }
}

fn bag_mode_name(mode: BagMode) -> &'static str {
    match mode {
        BagMode::RealOnly => "real_bag_only",
        BagMode::AllowSyntheticFallback => "bag_fallback",
    }
}

fn run_case(
    data_1m: Arc<Vec<CandleStick>>,
    data_3m: Arc<Vec<CandleStick>>,
    data_27m: Arc<Vec<CandleStick>>,
    case: &SweepCase,
) -> Row {
    let mut config = FractalAlignmentConfig::default();
    config.fast_ema_period = case.fast_ema;
    config.slow_ema_period = case.slow_ema;
    config.anchor_ema_period = case.anchor_ema;
    config.rr_target = case.rr_target;
    config.max_trigger_bars = case.max_trigger_bars;
    config.max_hold_bars = case.max_hold_bars;
    config.session = case.session;
    config.slippage_ticks_per_side = case.slippage_ticks_per_side;
    config.entry_variant = case.entry_variant;
    config.bag_mode = case.bag_mode;
    config.min_bag_gap_ticks = case.min_bag_gap_ticks;
    config.inversion_min_body_ticks = case.inversion_min_body_ticks;
    config.max_bars_after_bag_confirm = case.max_bars_after_bag_confirm;
    config.anchor_range_min_mult = case.anchor_range_min_mult;
    config.stop_buffer_ticks = case.stop_buffer_ticks;
    config.require_anchor_expansion = case.require_anchor_expansion;
    config.inversion_close_pct = case.inversion_close_pct;
    match case.session {
        SessionFilter::All => {
            config.session_start = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            config.session_end = NaiveTime::from_hms_opt(23, 59, 0).unwrap();
        }
        SessionFilter::NyOpen => {
            config.session_start = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
            config.session_end = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
        }
        SessionFilter::NyAm => {
            config.session_start = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
            config.session_end = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        }
    }

    let model = FractalAlignmentPlaybook {
        data_1m,
        data_3m,
        data_27m,
        config,
    };
    let result = execute(model);

    summarize(
        format!(
            "fast={} slow={} anchor={} rr={} trigger={} hold={} session={} slip={} entry={} bag={} bag_gap_ticks={} inv_body_ticks={} inv_wait={} anchor_mult={} stop_buf={} anchor_exp={} inv_close_pct={}",
            case.fast_ema,
            case.slow_ema,
            case.anchor_ema,
            case.rr_target,
            case.max_trigger_bars,
            case.max_hold_bars,
            session_name(case.session),
            case.slippage_ticks_per_side,
            entry_name(case.entry_variant),
            bag_mode_name(case.bag_mode),
            case.min_bag_gap_ticks,
            case.inversion_min_body_ticks,
            case.max_bars_after_bag_confirm,
            case.anchor_range_min_mult,
            case.stop_buffer_ticks,
            case.require_anchor_expansion,
            case.inversion_close_pct,
        ),
        result,
    )
}

fn main() {
    let matches = Command::new("mnq_fractal_alignment_sweep")
        .arg(Arg::new("fast").long("fast").action(ArgAction::SetTrue))
        .arg(
            Arg::new("strict-bag-only")
                .long("strict-bag-only")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("max-bars")
                .long("max-bars")
                .value_parser(clap::value_parser!(usize))
                .required(false),
        )
        .arg(
            Arg::new("min-trades")
                .long("min-trades")
                .value_parser(clap::value_parser!(usize))
                .required(false)
                .default_value("0"),
        )
        .get_matches();

    let fast_mode = matches.get_flag("fast");
    let strict_bag_only = matches.get_flag("strict-bag-only");
    let max_bars = matches.get_one::<usize>("max-bars").copied();
    let min_trades = *matches.get_one::<usize>("min-trades").unwrap_or(&0usize);

    let worker_cap = std::cmp::min(
        std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(4),
        8,
    );
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(worker_cap)
        .build_global();

    let mut one_min = load_mnq_1m();
    if let Some(limit) = max_bars {
        if one_min.len() > limit {
            one_min.truncate(limit);
        }
    }
    validate_data(&one_min, 60);

    let one_min = Arc::new(one_min);
    let three_min = Arc::new(resample_from_1m(&one_min, 3));
    let twenty_seven_min = Arc::new(resample_from_1m(&one_min, 27));

    let mut fast_emas = if fast_mode { vec![9, 13] } else { vec![9, 13] };
    let mut slow_emas = if fast_mode {
        vec![21, 34]
    } else {
        vec![21, 34]
    };
    let mut anchor_emas = if fast_mode { vec![9] } else { vec![9, 21] };
    let mut rr_targets = if fast_mode {
        vec![
            Decimal::from_f32(2.0).unwrap(),
            Decimal::from_f32(2.5).unwrap(),
        ]
    } else {
        vec![
            Decimal::from_f32(1.5).unwrap(),
            Decimal::from_f32(2.0).unwrap(),
            Decimal::from_f32(2.5).unwrap(),
        ]
    };
    let mut trigger_windows = if fast_mode { vec![8, 12] } else { vec![8, 12] };
    let mut hold_windows = if fast_mode {
        vec![30, 45]
    } else {
        vec![30, 45]
    };
    let mut sessions = if fast_mode {
        vec![SessionFilter::NyOpen, SessionFilter::NyAm]
    } else {
        vec![SessionFilter::NyOpen, SessionFilter::NyAm]
    };
    let mut slippages = if fast_mode { vec![1, 2] } else { vec![1, 2, 3] };
    let mut entry_variants = if fast_mode {
        vec![
            EntryVariant::BreakoutOnly,
            EntryVariant::RmbRetestOnly,
            EntryVariant::BreakoutOrRmbRetest,
        ]
    } else {
        vec![
            EntryVariant::BreakoutOnly,
            EntryVariant::RmbRetestOnly,
            EntryVariant::BreakoutOrRmbRetest,
        ]
    };
    let mut bag_modes = if strict_bag_only {
        vec![BagMode::RealOnly]
    } else if fast_mode {
        vec![BagMode::RealOnly, BagMode::AllowSyntheticFallback]
    } else {
        vec![BagMode::RealOnly, BagMode::AllowSyntheticFallback]
    };
    let mut bag_gap_ticks = if fast_mode {
        vec![1, 2, 3]
    } else {
        vec![1, 2, 3]
    };
    let mut inversion_body_ticks = if strict_bag_only { vec![1, 2] } else { vec![1] };
    let mut inversion_wait = if strict_bag_only {
        vec![3, 6, 9, 12]
    } else {
        vec![6]
    };
    let mut anchor_mults = if strict_bag_only {
        vec![
            Decimal::from_f32(1.0).unwrap(),
            Decimal::from_f32(1.1).unwrap(),
        ]
    } else {
        vec![Decimal::from_f32(1.1).unwrap()]
    };
    let mut stop_buffers = if strict_bag_only {
        vec![1, 2, 3]
    } else {
        vec![1]
    };
    let mut anchor_expansion_modes = if strict_bag_only {
        vec![true, false]
    } else {
        vec![true]
    };
    let mut inversion_close_pcts = if strict_bag_only {
        vec![
            Decimal::from_f32(0.55).unwrap(),
            Decimal::from_f32(0.65).unwrap(),
            Decimal::from_f32(0.75).unwrap(),
        ]
    } else {
        vec![Decimal::from_f32(0.70).unwrap()]
    };

    if fast_mode && strict_bag_only && min_trades > 0 {
        fast_emas = vec![9, 13];
        slow_emas = vec![21, 34];
        anchor_emas = vec![9];
        rr_targets = vec![
            Decimal::from_f32(2.0).unwrap(),
            Decimal::from_f32(2.5).unwrap(),
        ];
        trigger_windows = vec![8];
        hold_windows = vec![45];
        sessions = vec![SessionFilter::NyOpen, SessionFilter::NyAm];
        slippages = vec![1, 2];
        entry_variants = vec![EntryVariant::BreakoutOnly, EntryVariant::RmbRetestOnly];
        bag_modes = vec![BagMode::RealOnly];
        bag_gap_ticks = vec![2, 3];
        inversion_body_ticks = vec![1];
        inversion_wait = vec![6, 9, 12];
        anchor_mults = vec![Decimal::from_f32(1.0).unwrap()];
        stop_buffers = vec![1, 2];
        anchor_expansion_modes = vec![false];
        inversion_close_pcts = vec![
            Decimal::from_f32(0.55).unwrap(),
            Decimal::from_f32(0.65).unwrap(),
        ];
    }

    let mut cases = Vec::new();
    for fast in &fast_emas {
        for slow in &slow_emas {
            if slow <= fast {
                continue;
            }
            for anchor in &anchor_emas {
                for rr in &rr_targets {
                    for trigger in &trigger_windows {
                        for hold in &hold_windows {
                            for session in &sessions {
                                for slip in &slippages {
                                    for entry_variant in &entry_variants {
                                        for bag_mode in &bag_modes {
                                            for bag_gap in &bag_gap_ticks {
                                                for inv_body in &inversion_body_ticks {
                                                    for inv_wait in &inversion_wait {
                                                        for anchor_mult in &anchor_mults {
                                                            for stop_buf in &stop_buffers {
                                                                for anchor_expansion in
                                                                    &anchor_expansion_modes
                                                                {
                                                                    for inv_close in
                                                                        &inversion_close_pcts
                                                                    {
                                                                        cases.push(SweepCase {
                                                                            fast_ema: *fast,
                                                                            slow_ema: *slow,
                                                                            anchor_ema: *anchor,
                                                                            rr_target: *rr,
                                                                            max_trigger_bars: *trigger,
                                                                            max_hold_bars: *hold,
                                                                            session: *session,
                                                                            slippage_ticks_per_side: *slip,
                                                                            entry_variant: *entry_variant,
                                                                            bag_mode: *bag_mode,
                                                                            min_bag_gap_ticks: *bag_gap,
                                                                            inversion_min_body_ticks: *inv_body,
                                                                            max_bars_after_bag_confirm: *inv_wait,
                                                                            anchor_range_min_mult: *anchor_mult,
                                                                            stop_buffer_ticks: *stop_buf,
                                                                            require_anchor_expansion: *anchor_expansion,
                                                                            inversion_close_pct: *inv_close,
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut rows: Vec<Row> = cases
        .par_iter()
        .map(|case| {
            run_case(
                Arc::clone(&one_min),
                Arc::clone(&three_min),
                Arc::clone(&twenty_seven_min),
                case,
            )
        })
        .collect();

    rows.sort_by(|a, b| b.net_usd.cmp(&a.net_usd));
    if min_trades > 0 {
        rows.retain(|row| row.trades >= min_trades);
    }

    println!("MNQ fractal alignment sweep");
    println!("bars_1m   : {}", one_min.len());
    println!("bars_3m   : {}", three_min.len());
    println!("bars_27m  : {}", twenty_seven_min.len());
    println!("cases     : {}", rows.len());
    println!("workers   : {}", worker_cap);
    println!("min_trades: {}", min_trades);
    println!("realism   : next-bar-open entries, fixed MNQ commission, 1/2/3 tick slippage, gap-through stop handling");
    println!("\nTop results:\n");

    if rows.is_empty() {
        println!("no rows matched current filters");
    } else {
        for row in rows.iter().take(10) {
            println!("{}", row.label);
            println!("  trades={} win_rate%={} gross_pts={} net_pts={} gross_usd={} net_usd={} pf={} max_dd_usd={}",
                row.trades,
                row.win_rate,
                row.gross_points,
                row.net_points,
                row.gross_usd,
                row.net_usd,
                row.profit_factor,
                row.max_drawdown_usd,
            );
        }
    }
}
