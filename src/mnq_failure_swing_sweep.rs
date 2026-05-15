extern crate rust_decimal;

use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::execute;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade_result::TradeResult;
use backtest::strategies::ttrades_fractal_mtf::{
    CisdVariant, EntryVariant, FractalMTFConfig, KillzoneMode, ReversalConfirmMode,
    TTradesFractalMTF,
};
use chrono::TimeZone;
use chrono_tz::America::New_York;
use clap::{Arg, ArgAction, Command};
use rayon::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

#[derive(Clone)]
struct Dataset {
    name: &'static str,
    ltf: Arc<Vec<CandleStick>>,
    htf: Arc<Vec<CandleStick>>,
    tick_size: Decimal,
}

#[derive(Clone)]
struct Variant {
    label: String,
    robust_key: String,
    dataset: Dataset,
    cfg: FractalMTFConfig,
    slippage: i32,
}

#[derive(Clone)]
struct Row {
    label: String,
    robust_key: String,
    timeframe: &'static str,
    slippage: i32,
    trades: usize,
    win_rate: Decimal,
    pf_r: Decimal,
    net_points: Decimal,
    points_per_week: Decimal,
    net_usd: Decimal,
    net_usd_per_week: Decimal,
    max_dd_usd: Decimal,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortMode {
    Points,
    Trades,
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

fn cutoff_2025_ny() -> i64 {
    New_York
        .with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
        .single()
        .expect("valid NY cutoff")
        .timestamp()
}

fn trim_from_ts(mut data: Vec<CandleStick>, from_ts: i64) -> Vec<CandleStick> {
    data.retain(|c| c.open_time >= from_ts);
    data
}

fn align_htf_to_ltf(mut htf: Vec<CandleStick>, ltf: &[CandleStick]) -> Vec<CandleStick> {
    if let Some(last) = ltf.last().map(|c| c.open_time) {
        htf.retain(|c| c.open_time <= last);
    }
    htf
}

fn entry_name(v: EntryVariant) -> &'static str {
    match v {
        EntryVariant::Close => "close",
        EntryVariant::ObLevel => "ob_level",
        EntryVariant::ObMidpoint => "ob_mid",
    }
}

fn confirm_name(v: ReversalConfirmMode) -> &'static str {
    match v {
        ReversalConfirmMode::CisdOnly => "cisd_only",
        ReversalConfirmMode::IfvgOnly => "ifvg_only",
        ReversalConfirmMode::CisdAndIfvg => "cisd_and_ifvg",
        ReversalConfirmMode::CisdOrIfvg => "cisd_or_ifvg",
    }
}

fn cisd_name(v: CisdVariant) -> &'static str {
    match v {
        CisdVariant::BodyFlip => "body_flip",
        CisdVariant::StrictWickBreak => "strict_wick_break",
        CisdVariant::LastSeriesCloseBreak => "last_series_close_break",
        CisdVariant::FailureSwing => "failure_swing",
        CisdVariant::KillzoneReclaim => "killzone_reclaim",
        CisdVariant::ContinuationBreak => "continuation_break",
    }
}

fn killzone_name(v: KillzoneMode) -> &'static str {
    match v {
        KillzoneMode::Off => "all_day",
        KillzoneMode::NyOnly => "ny_only",
        KillzoneMode::LondonNy => "london_ny",
        KillzoneMode::NamedSessions => "named_kz",
    }
}

fn weeks_in_sample(data: &[CandleStick]) -> Decimal {
    if data.len() < 2 {
        return Decimal::ZERO;
    }
    let secs = data.last().unwrap().open_time - data.first().unwrap().open_time;
    if secs <= 0 {
        Decimal::ZERO
    } else {
        Decimal::from_i64(secs).unwrap() / Decimal::from(604_800)
    }
}

fn profit_factor_r(result: &BacktestResult) -> Decimal {
    let winners = result.result(TradeResult::Winner);
    let losers = result.result(TradeResult::Expense);
    let gross_loss_r = Decimal::from(losers as i64);
    let gross_profit_r = result.profit_in_r() + gross_loss_r;
    if gross_loss_r > Decimal::ZERO {
        (gross_profit_r / gross_loss_r).round_dp(2)
    } else if gross_profit_r > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    }
}

fn max_drawdown_usd(
    result: &BacktestResult,
    point_value: Decimal,
    round_trip_fee: Decimal,
) -> Decimal {
    let mut equity = Decimal::ZERO;
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    for trade in &result.trades {
        let pnl = (trade.points().0 - trade.total_costs()) * point_value - round_trip_fee;
        equity += pnl;
        if equity > peak {
            peak = equity;
        }
        let dd = peak - equity;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    max_dd.round_dp(2)
}

fn summarize(
    row: &Variant,
    result: BacktestResult,
    weeks: Decimal,
    point_value: Decimal,
    round_trip_fee: Decimal,
) -> Row {
    let trades = result.number_of_trades();
    let wins = result.result(TradeResult::Winner);
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from(wins as i64) / Decimal::from(trades as i64) * Decimal::from(100)).round_dp(2)
    };
    let gross_points = result.profit_in_points();
    let slippage_points = result
        .trades
        .iter()
        .map(|t| t.total_costs())
        .sum::<Decimal>();
    let net_points = (gross_points - slippage_points).round_dp(2);
    let fees = Decimal::from(trades as i64) * round_trip_fee;
    let net_usd = (net_points * point_value - fees).round_dp(2);
    Row {
        label: row.label.clone(),
        robust_key: row.robust_key.clone(),
        timeframe: row.dataset.name,
        slippage: row.slippage,
        trades,
        win_rate,
        pf_r: profit_factor_r(&result),
        net_points,
        points_per_week: if weeks > Decimal::ZERO {
            (net_points / weeks).round_dp(2)
        } else {
            Decimal::ZERO
        },
        net_usd,
        net_usd_per_week: if weeks > Decimal::ZERO {
            (net_usd / weeks).round_dp(2)
        } else {
            Decimal::ZERO
        },
        max_dd_usd: max_drawdown_usd(&result, point_value, round_trip_fee),
    }
}

fn run_model(dataset: &Dataset, cfg: FractalMTFConfig) -> BacktestResult {
    execute(TTradesFractalMTF {
        ltf_data: Arc::clone(&dataset.ltf),
        htf_data: Arc::clone(&dataset.htf),
        config: cfg,
    })
}

fn build_variants(datasets: &[Dataset], focused: bool, sort_mode: SortMode) -> Vec<Variant> {
    let throughput_mode = focused;
    let cisd_variants = if focused {
        vec![CisdVariant::ContinuationBreak]
    } else {
        vec![CisdVariant::FailureSwing]
    };
    let entries = if focused && sort_mode == SortMode::Trades {
        vec![EntryVariant::Close]
    } else if focused {
        vec![EntryVariant::ObMidpoint]
    } else {
        vec![EntryVariant::Close, EntryVariant::ObMidpoint]
    };
    let confirms = if focused && sort_mode == SortMode::Trades {
        vec![
            ReversalConfirmMode::CisdOnly,
            ReversalConfirmMode::CisdOrIfvg,
        ]
    } else if focused {
        vec![ReversalConfirmMode::CisdOnly, ReversalConfirmMode::CisdOrIfvg]
    } else {
        vec![
            ReversalConfirmMode::CisdOnly,
            ReversalConfirmMode::CisdAndIfvg,
            ReversalConfirmMode::CisdOrIfvg,
        ]
    };
    let rrs = if focused && sort_mode == SortMode::Trades {
        vec![Decimal::new(6, 1), Decimal::new(8, 1), Decimal::from(1)]
    } else if focused {
        vec![Decimal::from(1)]
    } else {
        vec![Decimal::new(12, 1), Decimal::new(15, 1), Decimal::from(2)]
    };
    let killzones = if focused && sort_mode == SortMode::Trades {
        vec![
            KillzoneMode::Off,
            KillzoneMode::LondonNy,
            KillzoneMode::NyOnly,
        ]
    } else if focused {
        vec![KillzoneMode::NamedSessions]
    } else {
        vec![KillzoneMode::NyOnly, KillzoneMode::LondonNy]
    };
    let poi_pads = if focused && sort_mode == SortMode::Trades {
        vec![0, 8, 16]
    } else if focused {
        vec![8]
    } else {
        vec![0, 5]
    };
    let ob_tols = if focused && sort_mode == SortMode::Trades {
        vec![0, 8, 16]
    } else if focused {
        vec![12]
    } else {
        vec![0, 5]
    };
    let lookbacks = if focused && sort_mode == SortMode::Trades {
        vec![8, 24]
    } else if focused {
        vec![8]
    } else {
        vec![12, 24, 36]
    };
    let close_only_modes = if focused {
        vec![false]
    } else {
        vec![false, true]
    };
    let stop_buffers = if focused { vec![1, 2] } else { vec![0, 1, 3] };
    let retest_tolerances = if focused && sort_mode == SortMode::Trades {
        vec![25, 50]
    } else if focused {
        vec![10]
    } else {
        vec![25]
    };
    let reclaim_ratios = if focused && sort_mode == SortMode::Trades {
        vec![4000, 5000]
    } else if focused {
        vec![5000]
    } else {
        vec![5000]
    };
    let htf_bias_modes = if focused {
        vec![false]
    } else {
        vec![false]
    };
    let htf_fvg_modes = if focused {
        vec![true]
    } else {
        vec![false]
    };
    let kz_level_hit_modes = if focused { vec![true] } else { vec![false] };
    let kz_level_lookbacks = if focused {
        vec![12usize]
    } else {
        vec![12usize]
    };
    let slips = [1, 2, 3];
    let weekday_profiles: Vec<(&'static str, u8)> = if focused && sort_mode == SortMode::Trades {
        vec![("mon_fri", 0b0001_1111), ("mon_thu", 0b0000_1111)]
    } else if focused {
        vec![("tue_thu", 0b0000_1110)]
    } else {
        vec![("mon_fri", 0b0001_1111)]
    };

    let mut out = Vec::new();
    for dataset in datasets {
        if throughput_mode && dataset.name != "1m/15m" {
            continue;
        }
        for cisd_variant in &cisd_variants {
            for entry in &entries {
                for confirm in &confirms {
                    for rr in &rrs {
                    for killzone in &killzones {
                        for (weekday_name, weekday_mask) in &weekday_profiles {
                            for poi_pad in &poi_pads {
                                for ob_tol in &ob_tols {
                                    for lookback in &lookbacks {
                                        for close_only in &close_only_modes {
                                            for stop_buffer in &stop_buffers {
                                                for retest_tol in &retest_tolerances {
                                                    for reclaim_ratio in &reclaim_ratios {
                                                        for htf_bias_strict in &htf_bias_modes {
                                                            for require_htf_fvg in &htf_fvg_modes {
                                                                for require_kz_hit in
                                                                    &kz_level_hit_modes
                                                                {
                                                                    for kz_hit_lookback in
                                                                        &kz_level_lookbacks
                                                                    {
                                                                        let robust_key = format!(
                                                                            "tf={};trigger={};entry={};confirm={};rr={};kz={};weekdays={};poi={};ob_tol={};fs_lb={};close_only={};stop_bps={};retest_bps={};reclaim_bps={};htf_strict={};htf_fvg={};kz_hit={};kz_lb={}",
                                                                            dataset.name,
                                                                            cisd_name(*cisd_variant),
                                                                            entry_name(*entry),
                                                                            confirm_name(*confirm),
                                                                            rr.round_dp(2),
                                                                            killzone_name(*killzone),
                                                                            weekday_name,
                                                                            poi_pad,
                                                                            ob_tol,
                                                                            lookback,
                                                                            close_only,
                                                                            stop_buffer,
                                                                            retest_tol,
                                                                            reclaim_ratio,
                                                                            htf_bias_strict,
                                                                            require_htf_fvg,
                                                                            require_kz_hit,
                                                                            kz_hit_lookback,
                                                                        );

                                                                        for slip in slips {
                                                                            let mut cfg =
                                                                                FractalMTFConfig::default();
                                                                            cfg.tick_size =
                                                                                dataset.tick_size;
                                                                            cfg.fee_config = backtest::model::fee_config::FeeConfig::zero();
                                                                            cfg.entry_variant =
                                                                                *entry;
                                                                            cfg.cisd_variant = *cisd_variant;
                                                                            cfg.reversal_confirm_mode =
                                                                                *confirm;
                                                                            cfg.weekday_mask =
                                                                                *weekday_mask;
                                                                            cfg.killzone_mode =
                                                                                *killzone;
                                                                            cfg.rr_target = *rr;
                                                                            cfg.poi_padding_bps =
                                                                                *poi_pad;
                                                                            cfg.ob_sweep_tolerance_bps =
                                                                                *ob_tol;
                                                                            cfg.failure_swing_lookback_bars =
                                                                                *lookback;
                                                                            cfg.failure_swing_breakout_close_only =
                                                                                *close_only;
                                                                            cfg.failure_swing_retest_tolerance_bps = *retest_tol;
                                                                            cfg.failure_swing_min_reclaim_ratio_bps = *reclaim_ratio;
                                                                            cfg.stop_buffer_bps =
                                                                                *stop_buffer;
                                                                            cfg.htf_bias_strict =
                                                                                *htf_bias_strict;
                                                                            cfg.require_htf_fvg =
                                                                                *require_htf_fvg;
                                                                            cfg.require_killzone_level_hit =
                                                                                *require_kz_hit;
                                                                            cfg.killzone_level_hit_lookback_bars =
                                                                                *kz_hit_lookback;
                                                                            cfg.slippage_ticks_per_side =
                                                                                slip;
                                                                            cfg.log_progress =
                                                                                false;

                                                                            out.push(Variant {
                                                                                label: format!(
                                                                                    "{};slip={}",
                                                                                    robust_key,
                                                                                    slip
                                                                                ),
                                                                                robust_key:
                                                                                    robust_key
                                                                                        .clone(),
                                                                                dataset: dataset
                                                                                    .clone(),
                                                                                cfg,
                                                                                slippage: slip,
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
        }
    }
    out
}

fn write_report(path: &str, title: &str, rows: &[Row], robust: &[(String, Decimal, Decimal)]) {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", title));
    md.push_str("- Symbol: MNQ\n");
    md.push_str("- Date filter: >= 2025-01-01 NY\n");
    md.push_str(
        "- Strategy family: TTrades Fractal MTF with `failure_swing` reversal confirmation\n",
    );
    md.push_str("- Costs: fixed fee $1.24 round-trip per 1 micro contract\n");
    md.push_str("- Slippage stress: 1 / 2 / 3 ticks per side\n\n");

    md.push_str("## Top Rows By Points/Week\n\n");
    md.push_str("| rank | timeframe | slippage | points/week | net_usd/week | trades | win_rate | pf_r | max_dd_usd | variant |\n");
    md.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for (i, row) in rows.iter().take(25).enumerate() {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            row.timeframe,
            row.slippage,
            row.points_per_week,
            row.net_usd_per_week,
            row.trades,
            row.win_rate,
            row.pf_r,
            row.max_dd_usd,
            row.label,
        ));
    }

    md.push_str("\n## Robustness Ranking\n\n");
    md.push_str("| rank | min_points/week slip123 | min_net_usd/week slip123 | variant |\n");
    md.push_str("|---:|---:|---:|---|\n");
    for (i, (key, min_points, min_usd)) in robust.iter().take(20).enumerate() {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            i + 1,
            min_points,
            min_usd,
            key,
        ));
    }

    fs::write(path, md).expect("write report");
}

fn main() {
    let matches = Command::new("mnq_failure_swing_sweep")
        .arg(
            Arg::new("focused")
                .long("focused")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("sort")
                .long("sort")
                .value_parser(["points", "trades"])
                .default_value("points"),
        )
        .get_matches();
    let focused = matches.get_flag("focused");
    let sort_mode = match matches.get_one::<String>("sort").map(String::as_str) {
        Some("trades") => SortMode::Trades,
        _ => SortMode::Points,
    };

    let workers = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);
    rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build_global()
        .ok();

    let from_ts = cutoff_2025_ny();
    let mnq_1m_full = load_mnq_1m();
    let mnq_1m = trim_from_ts(mnq_1m_full, from_ts);
    let mnq_5m = resample_from_1m(&mnq_1m, 5);
    let mnq_15m = resample_from_1m(&mnq_1m, 15);
    let mnq_1h = resample_from_1m(&mnq_1m, 60);
    let mnq_4h = resample_from_1m(&mnq_1m, 240);

    let datasets = vec![
        Dataset {
            name: "1m/15m",
            ltf: Arc::new(mnq_1m.clone()),
            htf: Arc::new(align_htf_to_ltf(mnq_15m.clone(), &mnq_1m)),
            tick_size: Decimal::new(25, 2),
        },
        Dataset {
            name: "5m/1h",
            ltf: Arc::new(mnq_5m.clone()),
            htf: Arc::new(align_htf_to_ltf(mnq_1h.clone(), &mnq_5m)),
            tick_size: Decimal::new(25, 2),
        },
        Dataset {
            name: "15m/4h",
            ltf: Arc::new(mnq_15m.clone()),
            htf: Arc::new(align_htf_to_ltf(mnq_4h.clone(), &mnq_15m)),
            tick_size: Decimal::new(25, 2),
        },
    ];

    let point_value = Decimal::from(2);
    let round_trip_fee = Decimal::new(124, 2);
    let variants = build_variants(&datasets, focused, sort_mode);

    println!("MNQ failure swing sweep");
    println!("date filter: >= 2025-01-01 NY");
    println!("datasets: 1m/15m, 5m/1h, 15m/4h");
    println!("variants: {}", variants.len());
    println!("mode: {}", if focused { "focused" } else { "broad" });

    let rows: Vec<Row> = variants
        .par_iter()
        .filter_map(|variant| {
            let mut naive_cfg = variant.cfg.clone();
            naive_cfg.slippage_ticks_per_side = 0;
            let naive_result = run_model(&variant.dataset, naive_cfg);
            if naive_result.profit_in_points() <= Decimal::ZERO
                || profit_factor_r(&naive_result) < Decimal::ONE
            {
                return None;
            }

            let result = run_model(&variant.dataset, variant.cfg.clone());
            let weeks = weeks_in_sample(variant.dataset.ltf.as_slice());
            Some(summarize(
                variant,
                result,
                weeks,
                point_value,
                round_trip_fee,
            ))
        })
        .collect();

    let mut sorted_rows = rows.clone();
    sorted_rows.sort_by(|a, b| match sort_mode {
        SortMode::Points => b
            .points_per_week
            .cmp(&a.points_per_week)
            .then_with(|| b.net_usd_per_week.cmp(&a.net_usd_per_week))
            .then_with(|| b.pf_r.cmp(&a.pf_r)),
        SortMode::Trades => b
            .trades
            .cmp(&a.trades)
            .then_with(|| b.win_rate.cmp(&a.win_rate))
            .then_with(|| b.points_per_week.cmp(&a.points_per_week)),
    });

    println!(
        "Top 20 rows by {}:",
        if sort_mode == SortMode::Trades {
            "trades"
        } else {
            "points/week"
        }
    );
    for row in sorted_rows.iter().take(20) {
        println!(
            "{} | trades={} win%={} pf_r={} points/week={} net_usd/week={} max_dd_usd={}",
            row.label,
            row.trades,
            row.win_rate,
            row.pf_r,
            row.points_per_week,
            row.net_usd_per_week,
            row.max_dd_usd,
        );
    }

    let mut robust_map: HashMap<String, (Decimal, Decimal, usize)> = HashMap::new();
    for row in &rows {
        let entry = robust_map.entry(row.robust_key.clone()).or_insert((
            row.points_per_week,
            row.net_usd_per_week,
            0,
        ));
        if row.points_per_week < entry.0 {
            entry.0 = row.points_per_week;
        }
        if row.net_usd_per_week < entry.1 {
            entry.1 = row.net_usd_per_week;
        }
        entry.2 += 1;
    }
    let mut robust: Vec<(String, Decimal, Decimal)> = robust_map
        .into_iter()
        .filter(|(_, (_, _, count))| *count == 3)
        .map(|(key, (min_points, min_usd, _))| (key, min_points.round_dp(2), min_usd.round_dp(2)))
        .collect();
    robust.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)));

    println!("\nRobustness gate (slip 1/2/3 min points/week >= 100):");
    let passed: Vec<_> = robust
        .iter()
        .filter(|(_, pts, _)| *pts >= Decimal::from(100))
        .collect();
    if passed.is_empty() {
        println!("No candidate passed.");
    } else {
        for (i, (key, pts, usd)) in passed.iter().take(10).enumerate() {
            println!(
                "{}. {} | min_points/week={} min_net_usd/week={}",
                i + 1,
                key,
                pts,
                usd
            );
        }
    }

    let report_path = if focused {
        "reports/strategy_overviews/MNQ_FAILURE_SWING_2025_FOCUSED.md"
    } else {
        "reports/strategy_overviews/MNQ_FAILURE_SWING_2025_SWEEP.md"
    };
    fs::create_dir_all("reports/strategy_overviews").expect("create reports dir");
    write_report(
        report_path,
        if focused {
            "MNQ Failure Swing 2025 Focused Sweep"
        } else {
            "MNQ Failure Swing 2025 Sweep"
        },
        &sorted_rows,
        &robust,
    );
    println!("Wrote {}", report_path);
}
