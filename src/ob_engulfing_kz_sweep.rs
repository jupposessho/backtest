extern crate rust_decimal;

use std::collections::HashMap;
use std::sync::Arc;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade::Trade;
use backtest::model::trade_result::TradeResult;
use backtest::model::trading_model::TradingModel;
use backtest::strategies::mc::{
    EntryMode, ExecutionConfig, MaxSlMode, Mc, McConfig, McMode, SignalPattern,
    SignalQualityConfig, TakeProfitMode, TimeWindow, TrailingStopConfig, TrailingStopMode,
    TrendFilter,
};
use backtest::to_new_york_time;
use chrono::{NaiveTime, TimeZone, Timelike};
use chrono_tz::America::New_York;
use clap::Parser;
use rayon::prelude::*;
use rust_decimal::Decimal;

#[derive(Parser, Debug)]
#[command(name = "ob_engulfing_kz_sweep")]
struct Args {
    #[arg(long, default_value = "0,1,2,3")]
    ob_lookback_hours: String,
}

#[derive(Clone)]
struct Variant {
    label: String,
    robust_key: String,
    cfg: McConfig,
    ob_lookback_hours: i64,
    slippage: i32,
}

struct Row {
    label: String,
    robust_key: String,
    slippage: i32,
    trades: usize,
    win_rate: Decimal,
    profit_r: Decimal,
    pf_r: Decimal,
    pnl_pct: Decimal,
    net_points: Decimal,
    points_per_week: Decimal,
    net_usd_1mic: Decimal,
    net_usd_per_week: Decimal,
    trades_per_week: Decimal,
    max_dd_usd_1mic: Decimal,
}

fn d(v: i64) -> Decimal {
    Decimal::from(v)
}

fn parse_lookbacks(csv: &str) -> Vec<i64> {
    csv.split(',')
        .filter_map(|p| p.trim().parse::<i64>().ok())
        .filter(|h| *h >= 0 && *h <= 12)
        .collect()
}

fn load_mnq_1m() -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
        .unwrap_or_else(|e| panic!("failed loading assets/mnq_1m_cont.parquet: {e}"))
}

fn resample_from_1m(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if minutes <= 1 || data.is_empty() {
        return data.to_vec();
    }
    let bucket = minutes * 60;
    let mut out = Vec::new();
    let mut cur = data[0];
    let mut cur_bucket = cur.open_time / bucket;
    for c in data.iter().copied().skip(1) {
        let b = c.open_time / bucket;
        if b != cur_bucket {
            out.push(cur);
            cur = c;
            cur_bucket = b;
        } else {
            if c.high > cur.high {
                cur.high = c.high;
            }
            if c.low < cur.low {
                cur.low = c.low;
            }
            cur.close = c.close;
            cur.close_time = c.close_time;
        }
    }
    out.push(cur);
    out
}

fn in_entry_window(open_time: i64) -> bool {
    let ny = to_new_york_time(open_time);
    let hour = ny.hour();
    let minute = ny.minute();
    (hour >= 5) && (hour < 9 || (hour == 9 && minute == 0))
}

fn futures_net_usd(
    result: &BacktestResult,
    point_value_usd: Decimal,
    round_trip_fee_usd: Decimal,
) -> Decimal {
    let gross_points = result.profit_in_points();
    let gross_usd = gross_points * point_value_usd;
    let fees = Decimal::from(result.number_of_trades() as i64) * round_trip_fee_usd;
    gross_usd - fees
}

fn summarize(
    label: String,
    robust_key: String,
    slippage: i32,
    result: BacktestResult,
    weeks: Decimal,
    point_value_usd: Decimal,
    round_trip_fee_usd: Decimal,
) -> Row {
    let winners = result.result(TradeResult::Winner);
    let losers = result.result(TradeResult::Expense);
    let total = result.number_of_trades();
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as i64) / Decimal::from(total as i64) * d(100)
    };
    let gross_loss_r = Decimal::from(losers as i64);
    let gross_profit_r = result.profit_in_r() + gross_loss_r;
    let pf_r = if gross_loss_r > Decimal::ZERO {
        gross_profit_r / gross_loss_r
    } else if gross_profit_r > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    };
    let net_points = result.profit_in_points();
    let net_usd_1mic = futures_net_usd(&result, point_value_usd, round_trip_fee_usd);
    let (max_dd_usd_1mic, _) = max_drawdown_usd(&result, point_value_usd, round_trip_fee_usd);
    Row {
        label,
        robust_key,
        slippage,
        trades: total,
        win_rate,
        profit_r: result.profit_in_r(),
        pf_r,
        pnl_pct: result.pnl(),
        net_points,
        points_per_week: if weeks > Decimal::ZERO {
            net_points / weeks
        } else {
            Decimal::ZERO
        },
        net_usd_1mic,
        net_usd_per_week: if weeks > Decimal::ZERO {
            net_usd_1mic / weeks
        } else {
            Decimal::ZERO
        },
        trades_per_week: if weeks > Decimal::ZERO {
            Decimal::from(total as i64) / weeks
        } else {
            Decimal::ZERO
        },
        max_dd_usd_1mic,
    }
}

fn max_drawdown_usd(
    result: &BacktestResult,
    point_value_usd: Decimal,
    round_trip_fee_usd: Decimal,
) -> (Decimal, Decimal) {
    let mut equity = Decimal::ZERO;
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;

    for t in &result.trades {
        let pnl = t.points().0 * point_value_usd - round_trip_fee_usd;
        equity += pnl;
        if equity > peak {
            peak = equity;
        }
        let dd = peak - equity;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    let dd_pct = if peak > Decimal::ZERO {
        (max_dd / peak) * Decimal::from(100)
    } else {
        Decimal::ZERO
    };
    (max_dd, dd_pct)
}

fn filtered_result(result: BacktestResult) -> BacktestResult {
    let filtered: Vec<Trade> = result
        .trades
        .into_iter()
        .filter(|t| in_entry_window(t.open_time))
        .collect();
    BacktestResult {
        trades: filtered,
        capital: Decimal::from(1000),
    }
}

fn tw(start_h: u32, end_h: u32) -> TimeWindow {
    TimeWindow {
        start: NaiveTime::from_hms_opt(start_h, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(end_h, 0, 0).unwrap(),
    }
}

fn build_variants(base: &McConfig, lookbacks: &[i64]) -> Vec<Variant> {
    let entry_modes = [
        (EntryMode::Close, "close"),
        (EntryMode::PrevOpen, "prev_open"),
        (EntryMode::PairMidpoint, "pair_mid"),
        (EntryMode::PairExtreme, "pair_extreme"),
    ];
    let rr_targets = [Decimal::new(15, 1), Decimal::from(2), Decimal::new(25, 1)];
    let rr_labels = ["1.5", "2.0", "2.5"];
    let tp_points = [
        Decimal::from(25),
        Decimal::from(30),
        Decimal::from(40),
        Decimal::from(50),
        Decimal::from(75),
        Decimal::from(100),
        Decimal::from(125),
        Decimal::from(150),
        Decimal::from(175),
        Decimal::from(200),
    ];
    let trailing_modes = [
        (TrailingStopMode::None, "trail_none"),
        (TrailingStopMode::BreakEven1R, "trail_be1r"),
        (TrailingStopMode::Progressive, "trail_prog"),
    ];
    let sl_caps = [None, Some(Decimal::from(25)), Some(Decimal::from(30))];
    let sl_mode_opts = [
        (MaxSlMode::KeepEntryMoveStop, "kems"),
        (MaxSlMode::KeepStopMoveEntry, "ksme"),
    ];
    let slippages = [1, 2, 3];

    let mut out = Vec::new();
    for lookback in lookbacks {
        let start_hour = if *lookback >= 5 {
            0
        } else {
            5 - (*lookback as u32)
        };
        for (entry_mode, entry_label) in &entry_modes {
            for (rr_idx, rr_target) in rr_targets.iter().enumerate() {
                for (trail_mode, trail_label) in &trailing_modes {
                    for sl_cap in sl_caps {
                        for (sl_mode, sl_mode_label) in &sl_mode_opts {
                            if sl_cap.is_none() && matches!(sl_mode, MaxSlMode::KeepStopMoveEntry) {
                                continue;
                            }
                            for slippage in slippages {
                                let label = format!(
                                    "lb{}h {} rr{} {} sl{:?} {} slip{}",
                                    lookback,
                                    entry_label,
                                    rr_labels[rr_idx],
                                    trail_label,
                                    sl_cap,
                                    sl_mode_label,
                                    slippage
                                );
                                out.push(Variant {
                                    label,
                                    robust_key: format!(
                                        "lb{}h {} rr{} {} sl{:?} {}",
                                        lookback,
                                        entry_label,
                                        rr_labels[rr_idx],
                                        trail_label,
                                        sl_cap,
                                        sl_mode_label
                                    ),
                                    ob_lookback_hours: *lookback,
                                    slippage,
                                    cfg: McConfig {
                                        trade_window: Some(tw(start_hour, 9)),
                                        entry_mode: entry_mode.clone(),
                                        rr_target: *rr_target,
                                        trailing_stop: TrailingStopConfig {
                                            mode: trail_mode.clone(),
                                        },
                                        max_sl_points: sl_cap,
                                        max_sl_mode: sl_mode.clone(),
                                        execution: ExecutionConfig {
                                            slippage_ticks_per_side: slippage,
                                            ..base.execution.clone()
                                        },
                                        ..base.clone()
                                    },
                                });
                            }
                        }
                    }
                }
            }

            for tp in tp_points {
                for (trail_mode, trail_label) in &trailing_modes {
                    for sl_cap in sl_caps {
                        for (sl_mode, sl_mode_label) in &sl_mode_opts {
                            if sl_cap.is_none() && matches!(sl_mode, MaxSlMode::KeepStopMoveEntry) {
                                continue;
                            }
                            for slippage in slippages {
                                let label = format!(
                                    "lb{}h {} tp{}pts {} sl{:?} {} slip{}",
                                    lookback,
                                    entry_label,
                                    tp,
                                    trail_label,
                                    sl_cap,
                                    sl_mode_label,
                                    slippage
                                );
                                out.push(Variant {
                                    label,
                                    robust_key: format!(
                                        "lb{}h {} tp{}pts {} sl{:?} {}",
                                        lookback,
                                        entry_label,
                                        tp,
                                        trail_label,
                                        sl_cap,
                                        sl_mode_label
                                    ),
                                    ob_lookback_hours: *lookback,
                                    slippage,
                                    cfg: McConfig {
                                        trade_window: Some(tw(start_hour, 9)),
                                        entry_mode: entry_mode.clone(),
                                        take_profit_mode: TakeProfitMode::FixedPoints(tp),
                                        rr_target: Decimal::from(2),
                                        trailing_stop: TrailingStopConfig {
                                            mode: trail_mode.clone(),
                                        },
                                        max_sl_points: sl_cap,
                                        max_sl_mode: sl_mode.clone(),
                                        execution: ExecutionConfig {
                                            slippage_ticks_per_side: slippage,
                                            ..base.execution.clone()
                                        },
                                        ..base.clone()
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn build_focused_variants(base: &McConfig, lookbacks: &[i64]) -> Vec<Variant> {
    let rr_targets = [Decimal::from(2), Decimal::new(22, 1), Decimal::new(25, 1)];
    let rr_labels = ["2.0", "2.2", "2.5"];
    let trailing_modes = [
        (TrailingStopMode::None, "trail_none"),
        (TrailingStopMode::Progressive, "trail_prog"),
    ];
    let quality = [
        (
            SignalQualityConfig {
                min_body_to_range: Decimal::ZERO,
                min_range_to_prev_range: Decimal::ZERO,
                min_range_to_avg_range: Decimal::ZERO,
                avg_range_lookback: 20,
            },
            "q0",
        ),
        (
            SignalQualityConfig {
                min_body_to_range: Decimal::new(35, 2),
                min_range_to_prev_range: Decimal::new(11, 1),
                min_range_to_avg_range: Decimal::ZERO,
                avg_range_lookback: 20,
            },
            "q1",
        ),
        (
            SignalQualityConfig {
                min_body_to_range: Decimal::new(40, 2),
                min_range_to_prev_range: Decimal::new(11, 1),
                min_range_to_avg_range: Decimal::new(105, 2),
                avg_range_lookback: 20,
            },
            "q2",
        ),
    ];

    let mut out = Vec::new();
    for lookback in lookbacks {
        let start_hour = if *lookback >= 5 {
            0
        } else {
            5 - (*lookback as u32)
        };
        for (rr_idx, rr_target) in rr_targets.iter().enumerate() {
            for (trail_mode, trail_label) in &trailing_modes {
                for (sq, q_label) in &quality {
                    for slippage in [1, 2, 3] {
                        for (sl_cap, sl_label) in [
                            (Some(Decimal::from(25)), "sl25"),
                            (Some(Decimal::new(275, 1)), "sl27.5"),
                            (Some(Decimal::from(30)), "sl30"),
                        ] {
                            for (sl_mode, sl_mode_label) in [
                                (MaxSlMode::KeepEntryMoveStop, "kems"),
                                (MaxSlMode::KeepStopMoveEntry, "ksme"),
                            ] {
                                let label = format!(
                                    "FOCUSED lb{}h close rr{} {} {} {} {} slip{}",
                                    lookback,
                                    rr_labels[rr_idx],
                                    trail_label,
                                    q_label,
                                    sl_label,
                                    sl_mode_label,
                                    slippage
                                );
                                out.push(Variant {
                                    label,
                                    robust_key: format!(
                                        "FOCUSED lb{}h close rr{} {} {} {} {}",
                                        lookback,
                                        rr_labels[rr_idx],
                                        trail_label,
                                        q_label,
                                        sl_label,
                                        sl_mode_label
                                    ),
                                    ob_lookback_hours: *lookback,
                                    slippage,
                                    cfg: McConfig {
                                        trade_window: Some(tw(start_hour, 9)),
                                        entry_mode: EntryMode::Close,
                                        rr_target: *rr_target,
                                        trailing_stop: TrailingStopConfig {
                                            mode: trail_mode.clone(),
                                        },
                                        signal_quality: sq.clone(),
                                        max_sl_points: sl_cap,
                                        max_sl_mode: sl_mode,
                                        execution: ExecutionConfig {
                                            slippage_ticks_per_side: slippage,
                                            ..base.execution.clone()
                                        },
                                        ..base.clone()
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn main() {
    let args = Args::parse();
    let lookbacks = parse_lookbacks(&args.ob_lookback_hours);
    if lookbacks.is_empty() {
        panic!("no valid lookback hours, provide --ob-lookback-hours like 0,1,2,3");
    }

    let all_data_15m = resample_from_1m(&load_mnq_1m(), 15);
    let start_ts = New_York
        .with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
        .single()
        .unwrap()
        .timestamp();
    let data_15m_vec: Vec<CandleStick> = all_data_15m
        .into_iter()
        .filter(|c| c.open_time >= start_ts)
        .collect();
    let data_15m = Arc::new(data_15m_vec);
    let first_ts = data_15m.first().map(|c| c.open_time).unwrap_or(start_ts);
    let last_ts = data_15m
        .last()
        .map(|c| c.close_time)
        .unwrap_or(start_ts + 1);
    let span_secs = (last_ts - first_ts).max(1);
    let weeks = Decimal::from(span_secs) / Decimal::from(7 * 24 * 60 * 60);
    println!("MNQ 15m OB(engulfing) killzone sweep");
    println!("bars: {}", data_15m.len());
    println!("date filter: >= 2025-01-01 NY");
    println!("span weeks: {}", weeks.round_dp(2));
    println!("entry window: 05:00-09:00 NY");
    println!(
        "OB formation window starts at 05:00 - lookback_hours ({:?})",
        lookbacks
    );

    let base = McConfig {
        mode: McMode::ReversalDaily,
        pattern: SignalPattern::Engulfing,
        trend_filter: TrendFilter::None,
        trade_window: Some(tw(5, 9)),
        prev_open_fill_window_candles: 8,
        execution: ExecutionConfig {
            market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
            commission_rate_per_side: Decimal::ZERO,
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 1,
            tick_size: Decimal::new(25, 2),
        },
        ..McConfig::default()
    };

    let mnq_micro_point_value = Decimal::from(2);
    let round_trip_fee_1mic = Decimal::new(124, 2);

    let variants = build_variants(&base, &lookbacks);
    println!("variants: {}", variants.len());

    let mut rows: Vec<Row> = variants
        .par_iter()
        .map(|v| {
            let result = Mc {
                data: data_15m.as_ref().clone(),
                config: v.cfg.clone(),
            }
            .execute();
            let filtered = filtered_result(result);
            summarize(
                format!(
                    "{} [entry 05-09 only, lb={}h]",
                    v.label, v.ob_lookback_hours
                ),
                v.robust_key.clone(),
                v.slippage,
                filtered,
                weeks,
                mnq_micro_point_value,
                round_trip_fee_1mic,
            )
        })
        .collect();

    rows.sort_by(|a, b| {
        b.points_per_week
            .cmp(&a.points_per_week)
            .then(b.net_usd_per_week.cmp(&a.net_usd_per_week))
            .then(b.trades.cmp(&a.trades))
    });

    let mut csv =
        String::from("rank,label,slippage,trades,win_rate_pct,profit_r,pf_r,points_per_week,net_usd_per_week,net_usd_1mic,max_dd_usd_1mic\n");
    for (idx, r) in rows.iter().enumerate() {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            idx + 1,
            r.label.replace(',', " "),
            r.slippage,
            r.trades,
            r.win_rate.round_dp(4),
            r.profit_r.round_dp(6),
            r.pf_r.round_dp(6),
            r.points_per_week.round_dp(4),
            r.net_usd_per_week.round_dp(4),
            r.net_usd_1mic.round_dp(2),
            r.max_dd_usd_1mic.round_dp(2)
        ));
    }

    let mut md = String::new();
    md.push_str("# OB Engulfing Killzone Sweep\n\n");
    md.push_str("- Symbol: MNQ\n");
    md.push_str("- Timeframe: 15m\n");
    md.push_str("- Entry window filter: 05:00-09:00 NY\n");
    md.push_str(&format!("- OB lookback hours: {:?}\n", lookbacks));
    md.push_str("- Costs: fixed fee $1.24 round-trip per 1 micro contract\n");
    md.push_str("- Slippage sweep: 1/2/3 ticks per side\n\n");
    md.push_str("## Top 20 by Points/Week\n\n");
    md.push_str("| Rank | Variant | Slip | Trades | Win% | PF R | Points/Wk | Net USD/Wk (1 micro) | Net USD (1 micro) | Max DD USD |\n");
    md.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for (idx, r) in rows.iter().take(20).enumerate() {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            idx + 1,
            r.label,
            r.slippage,
            r.trades,
            r.win_rate.round_dp(2),
            r.pf_r.round_dp(2),
            r.points_per_week.round_dp(2),
            r.net_usd_per_week.round_dp(2),
            r.net_usd_1mic.round_dp(2),
            r.max_dd_usd_1mic.round_dp(2)
        ));
    }

    std::fs::create_dir_all("reports/strategy_overviews")
        .unwrap_or_else(|e| panic!("failed creating reports dir: {e}"));
    std::fs::write("reports/strategy_overviews/OB_ENGULFING_KZ_SWEEP.csv", csv)
        .unwrap_or_else(|e| panic!("failed writing csv report: {e}"));
    std::fs::write("reports/strategy_overviews/OB_ENGULFING_KZ_SWEEP.md", md)
        .unwrap_or_else(|e| panic!("failed writing markdown report: {e}"));

    println!("\nTop 20 variants by points_per_week:");
    for r in rows.iter().take(20) {
        println!(
            "{} | slip={} trades={} win%={} pf_r={} points/wk={} net_usd/wk={} net_usd_1mic={} max_dd_usd={}",
            r.label,
            r.slippage,
            r.trades,
            r.win_rate.round_dp(2),
            r.pf_r.round_dp(2),
            r.points_per_week.round_dp(2),
            r.net_usd_per_week.round_dp(2),
            r.net_usd_1mic.round_dp(2),
            r.max_dd_usd_1mic.round_dp(2)
        );
    }

    println!("\nWrote reports:");
    println!("- reports/strategy_overviews/OB_ENGULFING_KZ_SWEEP.md");
    println!("- reports/strategy_overviews/OB_ENGULFING_KZ_SWEEP.csv");

    println!("\n--- Focused second pass around gross winners ---");
    let focused = build_focused_variants(&base, &lookbacks);
    println!("focused variants: {}", focused.len());
    let mut focused_rows: Vec<Row> = focused
        .par_iter()
        .map(|v| {
            let result = Mc {
                data: data_15m.as_ref().clone(),
                config: v.cfg.clone(),
            }
            .execute();
            let filtered = filtered_result(result);
            summarize(
                format!(
                    "{} [entry 05-09 only, lb={}h]",
                    v.label, v.ob_lookback_hours
                ),
                v.robust_key.clone(),
                v.slippage,
                filtered,
                weeks,
                mnq_micro_point_value,
                round_trip_fee_1mic,
            )
        })
        .collect();
    focused_rows.sort_by(|a, b| {
        b.points_per_week
            .cmp(&a.points_per_week)
            .then(b.profit_r.cmp(&a.profit_r))
            .then(b.pf_r.cmp(&a.pf_r))
    });

    println!("Top 15 focused variants by points_per_week:");
    for r in focused_rows.iter().take(15) {
        println!(
            "{} | slip={} trades={} win%={} pf_r={} points/wk={} net_usd/wk={} net_usd_1mic={} max_dd_usd={}",
            r.label,
            r.slippage,
            r.trades,
            r.win_rate.round_dp(2),
            r.pf_r.round_dp(2),
            r.points_per_week.round_dp(2),
            r.net_usd_per_week.round_dp(2),
            r.net_usd_1mic.round_dp(2),
            r.max_dd_usd_1mic.round_dp(2)
        );
    }

    let mut focused_md = String::new();
    focused_md.push_str("# OB Engulfing Killzone Focused Pass\n\n");
    focused_md.push_str("Focused around: close entry, RR 2.0-2.5, SL caps 25/27.5/30, slip 1-2, quality filters q0-q2.\n\n");
    focused_md.push_str("| Rank | Variant | Slip | Trades | Win% | PF R | Points/Wk | Net USD/Wk | Net USD | Max DD USD |\n");
    focused_md.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for (idx, r) in focused_rows.iter().enumerate() {
        focused_md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            idx + 1,
            r.label,
            r.slippage,
            r.trades,
            r.win_rate.round_dp(2),
            r.pf_r.round_dp(2),
            r.points_per_week.round_dp(2),
            r.net_usd_per_week.round_dp(2),
            r.net_usd_1mic.round_dp(2),
            r.max_dd_usd_1mic.round_dp(2)
        ));
    }

    let mut by_key: HashMap<String, HashMap<i32, Row>> = HashMap::new();
    for r in rows.iter().chain(focused_rows.iter()) {
        by_key.entry(r.robust_key.clone()).or_default().insert(
            r.slippage,
            Row {
                label: r.label.clone(),
                robust_key: r.robust_key.clone(),
                slippage: r.slippage,
                trades: r.trades,
                win_rate: r.win_rate,
                profit_r: r.profit_r,
                pf_r: r.pf_r,
                pnl_pct: r.pnl_pct,
                net_points: r.net_points,
                points_per_week: r.points_per_week,
                net_usd_1mic: r.net_usd_1mic,
                net_usd_per_week: r.net_usd_per_week,
                trades_per_week: r.trades_per_week,
                max_dd_usd_1mic: r.max_dd_usd_1mic,
            },
        );
    }

    let mut robust_passes: Vec<(String, Decimal, Decimal)> = Vec::new();
    for (key, map) in by_key {
        if let (Some(s1), Some(s2), Some(s3)) = (map.get(&1), map.get(&2), map.get(&3)) {
            let min_points_week = s1
                .points_per_week
                .min(s2.points_per_week)
                .min(s3.points_per_week);
            let min_net_usd_week = s1
                .net_usd_per_week
                .min(s2.net_usd_per_week)
                .min(s3.net_usd_per_week);
            if min_points_week >= Decimal::from(100) {
                robust_passes.push((key, min_points_week, min_net_usd_week));
            }
        }
    }
    robust_passes.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));

    println!("\n--- Robustness gate (slip 1/2/3) : points/week >= 100 ---");
    if robust_passes.is_empty() {
        println!("No variant passed the 100 points/week robustness gate.");
    } else {
        for (i, (k, p, u)) in robust_passes.iter().take(20).enumerate() {
            println!(
                "{}. {} | min_points/week={} min_net_usd/week={}",
                i + 1,
                k,
                p.round_dp(2),
                u.round_dp(2)
            );
        }
    }
    std::fs::write(
        "reports/strategy_overviews/OB_ENGULFING_KZ_SWEEP_FOCUSED.md",
        focused_md,
    )
    .unwrap_or_else(|e| panic!("failed writing focused markdown report: {e}"));
    println!("- reports/strategy_overviews/OB_ENGULFING_KZ_SWEEP_FOCUSED.md");
}
