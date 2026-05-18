extern crate rust_decimal;

use std::collections::HashMap;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::engine::types::ExecutionConfig;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::fee_config::FeeConfig;
use backtest::model::position_direction::PositionDirection;
use backtest::model::trade_result::TradeResult;
use backtest::model::trading_model::TradingModel;
use backtest::strategies::orb::{Orb, OrbConfig, OrbDuration, OrbEntryModel, OrbSlType};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

#[derive(Clone)]
struct RunRow {
    variant: String,
    base_variant: String,
    slip_ticks: i32,
    timeframe: String,
    trades: usize,
    win_rate: Decimal,
    pf_r: Decimal,
    pnl_pct: Decimal,
    points: Decimal,
    points_per_week: Decimal,
}

#[derive(Clone)]
struct RobustRow {
    base_variant: String,
    timeframe: String,
    trades_min: usize,
    pf_min: Decimal,
    win_rate_min: Decimal,
    points_per_week_min: Decimal,
    points_per_week_avg: Decimal,
    pnl_pct_min: Decimal,
    gap_to_40: Decimal,
}

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .unwrap_or_else(|e| panic!("failed loading {}: {}", path, e))
}

fn resample_minutes(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if data.is_empty() || minutes <= 1 {
        return data.to_vec();
    }
    let bucket_ms = minutes * 60 * 1000;
    let mut out: Vec<CandleStick> = Vec::new();
    let mut bucket_start: Option<i64> = None;
    let mut acc: Option<CandleStick> = None;

    for c in data {
        let b = c.open_time / bucket_ms;
        if bucket_start != Some(b) {
            if let Some(done) = acc.take() {
                out.push(done);
            }
            bucket_start = Some(b);
            acc = Some(*c);
            continue;
        }

        if let Some(mut a) = acc.take() {
            a.high.0 = a.high.0.max(c.high.0);
            a.low.0 = a.low.0.min(c.low.0);
            a.close = c.close;
            a.close_time = c.close_time;
            acc = Some(a);
        }
    }
    if let Some(done) = acc {
        out.push(done);
    }
    out
}

fn nq_to_mes_points(nq_points: Decimal) -> Decimal {
    let scaled = nq_points / Decimal::new(25, 1);
    let quarter = Decimal::new(25, 2);
    (scaled / quarter).round() * quarter
}

fn points_from_trade(t: &backtest::model::trade::Trade) -> Decimal {
    match t.direction {
        PositionDirection::Long => t.tp.0 - t.entry.0,
        PositionDirection::Short => t.entry.0 - t.tp.0,
    }
}

fn points_per_week(result: &BacktestResult, data: &[CandleStick]) -> Decimal {
    if result.trades.is_empty() || data.len() < 2 {
        return Decimal::ZERO;
    }
    let total_points: Decimal = result.trades.iter().map(points_from_trade).sum();
    let start = data.first().map(|c| c.open_time).unwrap_or(0);
    let end = data.last().map(|c| c.close_time).unwrap_or(start);
    if end <= start {
        return Decimal::ZERO;
    }
    let secs_per_week = Decimal::from(7_i64 * 24 * 60 * 60);
    let span_secs = Decimal::from(end - start);
    if span_secs <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    total_points / (span_secs / secs_per_week)
}

fn summarize(
    label: &str,
    base_variant: &str,
    slip_ticks: i32,
    tf: &str,
    result: BacktestResult,
    data: &[CandleStick],
) -> RunRow {
    let winners = result.result(TradeResult::Winner);
    let losers = result.result(TradeResult::Expense);
    let trades = result.number_of_trades();
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as u32) / Decimal::from(trades as u32) * Decimal::from(100)
    };
    let gross_loss_r = Decimal::from(losers as u32);
    let gross_profit_r = result.profit_in_r() + gross_loss_r;
    let pf_r = if gross_loss_r > Decimal::ZERO {
        gross_profit_r / gross_loss_r
    } else if gross_profit_r > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    };

    RunRow {
        variant: label.to_string(),
        base_variant: base_variant.to_string(),
        slip_ticks,
        timeframe: tf.to_string(),
        trades,
        win_rate,
        pf_r,
        pnl_pct: result.pnl(),
        points: result.profit_in_points(),
        points_per_week: points_per_week(&result, data),
    }
}

fn robust_rows(rows: &[RunRow]) -> Vec<RobustRow> {
    let mut grouped: HashMap<(String, String), Vec<&RunRow>> = HashMap::new();
    for r in rows {
        grouped
            .entry((r.base_variant.clone(), r.timeframe.clone()))
            .or_default()
            .push(r);
    }

    let mut out = Vec::new();
    for ((variant, tf), group) in grouped {
        if group.len() < 3 {
            continue;
        }
        let points_min = group
            .iter()
            .map(|r| r.points_per_week)
            .min()
            .unwrap_or(Decimal::ZERO);
        let points_sum: Decimal = group.iter().map(|r| r.points_per_week).sum();
        let points_avg = points_sum / Decimal::from(group.len() as u32);
        let trades_min = group.iter().map(|r| r.trades).min().unwrap_or(0);
        let pf_min = group.iter().map(|r| r.pf_r).min().unwrap_or(Decimal::ZERO);
        let win_rate_min = group.iter().map(|r| r.win_rate).min().unwrap_or(Decimal::ZERO);
        let pnl_pct_min = group.iter().map(|r| r.pnl_pct).min().unwrap_or(Decimal::ZERO);
        if trades_min == 0 {
            continue;
        }
        out.push(RobustRow {
            base_variant: variant,
            timeframe: tf,
            trades_min,
            pf_min,
            win_rate_min,
            points_per_week_min: points_min,
            points_per_week_avg: points_avg,
            pnl_pct_min,
            gap_to_40: Decimal::from(40) - points_min,
        });
    }
    out
}

fn write_report(path: &str, rows: &[RunRow], robust: &[RobustRow]) {
    let mut md = String::new();
    md.push_str("# MES ORB Variant Port (Structural Switch)\n\n");
    md.push_str("Reality settings: next-bar-open entries, conservative intrabar handling, MES tick slippage stress (1/2/3 ticks per side), and non-zero fees.\n\n");
    md.push_str("NQ->ES fixed-point scaling rule applied where relevant: `mes_points = round_to_0.25(nq_points / 2.5)`.\n\n");

    md.push_str("## Top MES Runs By Points/Week\n\n");
    md.push_str("| variant | tf | slip | trades | win_rate_% | points | points/week | pf_r | pnl_% |\n");
    md.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|\n");
    let mut top_runs: Vec<&RunRow> = rows.iter().collect();
    top_runs.sort_by(|a, b| b.points_per_week.cmp(&a.points_per_week));
    for r in top_runs.into_iter().take(15) {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.variant,
            r.timeframe,
            r.slip_ticks,
            r.trades,
            r.win_rate.round_dp(2),
            r.points.round_dp(2),
            r.points_per_week.round_dp(2),
            r.pf_r.round_dp(2),
            r.pnl_pct.round_dp(2)
        ));
    }

    md.push_str("\n## Robust Leaderboard (Min Across Slip 1/2/3)\n\n");
    md.push_str("| base_variant | tf | min_trades | min_win_rate_% | min_pf_r | min_points/week | avg_points/week | gap_to_40 | min_pnl_% |\n");
    md.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|\n");
    let mut robust_sorted = robust.to_vec();
    robust_sorted.sort_by(|a, b| b.points_per_week_min.cmp(&a.points_per_week_min));
    for r in &robust_sorted {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.base_variant,
            r.timeframe,
            r.trades_min,
            r.win_rate_min.round_dp(2),
            r.pf_min.round_dp(2),
            r.points_per_week_min.round_dp(2),
            r.points_per_week_avg.round_dp(2),
            r.gap_to_40.round_dp(2),
            r.pnl_pct_min.round_dp(2)
        ));
    }

    md.push_str("\n## Gap-to-40 Leaderboard\n\n");
    md.push_str("Target = 40 points/week using robust min across slip 1/2/3.\n\n");
    md.push_str("| rank | base_variant | tf | robust_min_points/week | gap_to_40 |\n");
    md.push_str("|---:|---|---|---:|---:|\n");
    let mut gap_rank = robust.to_vec();
    gap_rank.sort_by(|a, b| a.gap_to_40.cmp(&b.gap_to_40));
    for (i, r) in gap_rank.into_iter().take(10).enumerate() {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            i + 1,
            r.base_variant,
            r.timeframe,
            r.points_per_week_min.round_dp(2),
            r.gap_to_40.round_dp(2)
        ));
    }

    let best = robust_sorted.first();
    md.push_str("\n## Realism Verdict\n\n");
    md.push_str("- Fees: enabled (`FeeConfig::binance_standard`)\n");
    md.push_str("- Slippage: stress-tested at 1/2/3 MES ticks per side\n");
    md.push_str("- Entry timing: next-bar-open\n");
    md.push_str("- Intrabar handling: conservative (stop-first)\n");
    if let Some(b) = best {
        let verdict = if b.points_per_week_min >= Decimal::from(40) {
            "FULLY_TESTED"
        } else if b.points_per_week_min > Decimal::ZERO {
            "PARTIALLY_TESTED"
        } else {
            "NOT_RECOMMENDED"
        };
        md.push_str(&format!(
            "- Verdict: **{}** (best robust min points/week = {})\n",
            verdict,
            b.points_per_week_min.round_dp(2)
        ));
    }

    std::fs::write(path, md).unwrap_or_else(|e| panic!("failed writing report {}: {}", path, e));
}

fn main() {
    let mes_1m = load_parquet("assets/mes_1m_cont.parquet");
    let datasets: Vec<(&str, Vec<CandleStick>)> = vec![
        ("1m", mes_1m.clone()),
        ("5m", resample_minutes(&mes_1m, 5)),
        ("15m", resample_minutes(&mes_1m, 15)),
        ("1h", resample_minutes(&mes_1m, 60)),
    ];

    let mut variants: Vec<(String, OrbConfig)> = vec![
        (
            "orb15_opp_rr2".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes15,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        (
            "orb30_opp_rr2".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        (
            "orb30_opp_rr3".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::from(3),
                ..OrbConfig::default()
            },
        ),
        (
            "orb30_rangepct25_rr2".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::RangePct(Decimal::from(25)),
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        (
            "nq_multiorb".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes15,
                sl_type: OrbSlType::RangePct(Decimal::from(50)),
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        (
            "nq_finalboss".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes15,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::ONE,
                ..OrbConfig::default()
            },
        ),
        (
            "nq_orb30m15m".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::RangePct(Decimal::from(50)),
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
    ];

    let durations = [OrbDuration::Minutes15, OrbDuration::Minutes30];
    let duration_labels = ["15", "30"];
    let rr_vals = [
        Decimal::new(15, 1),
        Decimal::from(2),
        Decimal::new(25, 1),
        Decimal::from(3),
    ];
    let sl_types = [
        OrbSlType::OppositeRange,
        OrbSlType::RangePct(Decimal::from(25)),
        OrbSlType::RangePct(Decimal::from(50)),
    ];
    let sl_labels = ["opp", "rp25", "rp50"];
    let active_windows = [120usize, 240usize, 360usize];
    let hold_limits: [Option<usize>; 3] = [None, Some(24), Some(48)];
    let hold_labels = ["holdnone", "hold24", "hold48"];
    let retest_modes = [false, true];
    let retest_bars = [6usize, 12usize];

    for (di, d) in durations.iter().enumerate() {
        for rr in rr_vals {
            for (si, sl) in sl_types.iter().enumerate() {
                for aw in active_windows {
                    for (hi, hold) in hold_limits.iter().enumerate() {
                        for rm in retest_modes {
                            if !rm {
                                let name = format!(
                                    "grid_or{}_rr{}_{}_aw{}_{}",
                                    duration_labels[di], rr, sl_labels[si], aw, hold_labels[hi]
                                )
                                .replace('.', "p");
                                variants.push((
                                    name,
                                    OrbConfig {
                                        duration: *d,
                                        sl_type: sl.clone(),
                                        rr_target: rr,
                                        active_window_minutes: aw,
                                        max_hold_bars: *hold,
                                        retest_mode: false,
                                        ..OrbConfig::default()
                                    },
                                ));
                            } else {
                                for rb in retest_bars {
                                    let name = format!(
                                        "grid_or{}_rr{}_{}_aw{}_{}_retest{}",
                                        duration_labels[di],
                                        rr,
                                        sl_labels[si],
                                        aw,
                                        hold_labels[hi],
                                        rb
                                    )
                                    .replace('.', "p");
                                    variants.push((
                                        name,
                                        OrbConfig {
                                            duration: *d,
                                            sl_type: sl.clone(),
                                            rr_target: rr,
                                            active_window_minutes: aw,
                                            max_hold_bars: *hold,
                                            retest_mode: true,
                                            retest_max_bars: rb,
                                            ..OrbConfig::default()
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mes_tick = Decimal::new(25, 2);
    let _example_scaled = nq_to_mes_points(Decimal::from_f64(2.5).unwrap_or(Decimal::from(25)));

    let mut rows: Vec<RunRow> = Vec::new();
    for (tf, data) in datasets {
        for (name, cfg) in &variants {
            for slip_ticks in [1_i32, 2_i32, 3_i32] {
                let mut realistic_cfg = cfg.clone();
                realistic_cfg.entry_model = OrbEntryModel::NextBarOpen;
                realistic_cfg.conservative_intrabar = true;
                realistic_cfg.fee_config = FeeConfig::binance_standard();
                realistic_cfg.execution = ExecutionConfig {
                    slippage_ticks_per_side: slip_ticks,
                    tick_size: mes_tick,
                    ..ExecutionConfig::default()
                };
                let label = format!("mes_real_s{}_{}", slip_ticks, name);
                let realistic = Orb {
                    data: data.clone(),
                    config: realistic_cfg,
                }
                .execute();
                rows.push(summarize(&label, name, slip_ticks, tf, realistic, &data));
            }
        }
    }

    let robust = robust_rows(&rows);
    write_report(
        "reports/strategy_overviews/MES_ORB_VARIANTS_STRUCTURAL_SWITCH.md",
        &rows,
        &robust,
    );

    let mut robust_sorted = robust.clone();
    robust_sorted.sort_by(|a, b| b.points_per_week_min.cmp(&a.points_per_week_min));
    println!("Wrote reports/strategy_overviews/MES_ORB_VARIANTS_STRUCTURAL_SWITCH.md");
    if let Some(best) = robust_sorted.first() {
        println!(
            "Best robust MES: {} {} min_pts_wk={} gap_to_40={}",
            best.base_variant,
            best.timeframe,
            best.points_per_week_min.round_dp(2),
            best.gap_to_40.round_dp(2)
        );
    }
    let maybe_40 = robust_sorted
        .iter()
        .filter(|r| r.points_per_week_min >= Decimal::from(40))
        .count();
    println!("Robust variants >= 40 points/week: {}", maybe_40);

    let scale_check = nq_to_mes_points(Decimal::from_f64(12.5).unwrap_or(Decimal::ZERO));
    println!("NQ->MES scale check: 12.5 -> {} points", scale_check);
}
