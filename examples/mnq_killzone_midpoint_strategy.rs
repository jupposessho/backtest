use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{Datelike, Days, NaiveDate, TimeZone, Timelike, Weekday};
use chrono_tz::America::New_York;
use csv::Writer;
use rayon::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

const ZONE_COUNT: usize = 5;

#[derive(Clone, Copy)]
struct ZoneDef {
    start_minute: u32,
    end_minute: u32,
    wraps_midnight: bool,
}

const ZONES: [ZoneDef; ZONE_COUNT] = [
    ZoneDef {
        start_minute: 20 * 60,
        end_minute: 0,
        wraps_midnight: true,
    },
    ZoneDef {
        start_minute: 2 * 60 + 20,
        end_minute: 5 * 60,
        wraps_midnight: false,
    },
    ZoneDef {
        start_minute: 9 * 60 + 30,
        end_minute: 11 * 60,
        wraps_midnight: false,
    },
    ZoneDef {
        start_minute: 12 * 60,
        end_minute: 13 * 60,
        wraps_midnight: false,
    },
    ZoneDef {
        start_minute: 13 * 60 + 30,
        end_minute: 16 * 60,
        wraps_midnight: false,
    },
];

const ASIA_IDX: usize = 0;
const LONDON_IDX: usize = 1;
const NYAM_IDX: usize = 2;
const LUNCH_IDX: usize = 3;
const NYPM_IDX: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Long,
    Short,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExitMode {
    TargetZoneEnd,
    SessionEnd,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SetupKind {
    AsiaToNyam,
    LondonToNyam,
    LunchToNypm,
    Combined,
}

#[derive(Clone, Copy)]
struct Bar {
    o: f64,
    hi: f64,
    lo: f64,
    c: f64,
    minute_of_day: u32,
}

#[derive(Clone, Copy)]
struct ZoneSummary {
    start_idx: usize,
    end_idx: usize,
    high: f64,
    low: f64,
    mid: f64,
    range: f64,
}

#[derive(Clone)]
struct DaySummary {
    session_day: NaiveDate,
    weekday: Weekday,
    bars: Vec<Bar>,
    closes: Vec<f64>,
    zones: [Option<ZoneSummary>; ZONE_COUNT],
}

#[derive(Clone, Copy)]
struct RelSpec {
    source_idx: usize,
    target_idx: usize,
}

#[derive(Clone, Copy)]
struct Cfg {
    max_reclaim_bars: usize,
    stop_cap_pct: f64,
    use_ema_gate: bool,
    exit_mode: ExitMode,
    min_reclaim_body_pct: f64,
    min_target_rr: f64,
    max_entry_dist_to_target_pct: f64,
    require_target_box_open_inside_source: bool,
}

#[derive(Clone)]
struct SweepRow {
    cfg: Cfg,
    label: String,
    setup: SetupKind,
    is_trades: usize,
    is_win_rate: f64,
    is_exp: f64,
    is_max_dd: f64,
    oos_trades: usize,
    oos_win_rate: f64,
    oos_exp: f64,
    oos_max_dd: f64,
}

#[derive(Clone, Copy)]
struct EvalStats {
    trades: usize,
    win_rate: f64,
    expectancy: f64,
    max_dd: f64,
}

#[derive(Clone, Copy)]
struct TradeCandidate {
    touch_idx: usize,
    entry_idx: usize,
    r: f64,
}

#[derive(Clone)]
struct TradeRecord {
    day: NaiveDate,
    weekday: Weekday,
    r: f64,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn in_zone(minute_of_day: u32, zone: ZoneDef) -> bool {
    if zone.wraps_midnight {
        minute_of_day >= zone.start_minute || minute_of_day < zone.end_minute
    } else {
        minute_of_day >= zone.start_minute && minute_of_day < zone.end_minute
    }
}

fn session_day_for(minute_of_day: u32, date: NaiveDate) -> NaiveDate {
    if minute_of_day >= 20 * 60 {
        date.checked_add_days(Days::new(1)).unwrap_or(date)
    } else {
        date
    }
}

fn ema(vals: &[f64], period: usize) -> Vec<f64> {
    if vals.is_empty() {
        return vec![];
    }
    let k = 2.0 / (period as f64 + 1.0);
    let mut out = Vec::with_capacity(vals.len());
    let mut e = vals[0];
    out.push(e);
    for v in vals.iter().skip(1) {
        e = *v * k + e * (1.0 - k);
        out.push(e);
    }
    out
}

fn summarize_zone(bars: &[Bar], zone: ZoneDef) -> Option<ZoneSummary> {
    let mut start_idx = None;
    let mut end_idx = 0usize;
    let mut high = f64::NEG_INFINITY;
    let mut low = f64::INFINITY;
    for (idx, bar) in bars.iter().enumerate() {
        if !in_zone(bar.minute_of_day, zone) {
            continue;
        }
        if start_idx.is_none() {
            start_idx = Some(idx);
        }
        end_idx = idx;
        high = high.max(bar.hi);
        low = low.min(bar.lo);
    }
    start_idx.and_then(|s| {
        if high > low {
            Some(ZoneSummary {
                start_idx: s,
                end_idx,
                high,
                low,
                mid: (high + low) * 0.5,
                range: high - low,
            })
        } else {
            None
        }
    })
}

fn load_days() -> Arc<Vec<DaySummary>> {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");
    let mut grouped: BTreeMap<NaiveDate, Vec<Bar>> = BTreeMap::new();
    for candle in candles {
        let dt = New_York
            .timestamp_opt(candle.open_time, 0)
            .single()
            .expect("valid timestamp");
        let md = dt.hour() * 60 + dt.minute();
        let sd = session_day_for(md, dt.date_naive());
        grouped.entry(sd).or_default().push(Bar {
            o: d2f(candle.open.0),
            hi: d2f(candle.high.0),
            lo: d2f(candle.low.0),
            c: d2f(candle.close.0),
            minute_of_day: md,
        });
    }

    let days: Vec<DaySummary> = grouped
        .into_iter()
        .filter_map(|(session_day, bars)| {
            let zones = ZONES.map(|z| summarize_zone(&bars, z));
            if zones.iter().all(Option::is_none) {
                None
            } else {
                Some(DaySummary {
                    session_day,
                    weekday: session_day.weekday(),
                    closes: bars.iter().map(|b| b.c).collect(),
                    bars,
                    zones,
                })
            }
        })
        .collect();
    Arc::new(days)
}

fn relation_specs(setup: SetupKind) -> &'static [RelSpec] {
    const ASIA_NYAM: [RelSpec; 1] = [RelSpec {
        source_idx: ASIA_IDX,
        target_idx: NYAM_IDX,
    }];
    const LONDON_NYAM: [RelSpec; 1] = [RelSpec {
        source_idx: LONDON_IDX,
        target_idx: NYAM_IDX,
    }];
    const LUNCH_NYPM: [RelSpec; 1] = [RelSpec {
        source_idx: LUNCH_IDX,
        target_idx: NYPM_IDX,
    }];
    const COMBINED: [RelSpec; 3] = [
        RelSpec {
            source_idx: ASIA_IDX,
            target_idx: NYAM_IDX,
        },
        RelSpec {
            source_idx: LONDON_IDX,
            target_idx: NYAM_IDX,
        },
        RelSpec {
            source_idx: LUNCH_IDX,
            target_idx: NYPM_IDX,
        },
    ];
    match setup {
        SetupKind::AsiaToNyam => &ASIA_NYAM,
        SetupKind::LondonToNyam => &LONDON_NYAM,
        SetupKind::LunchToNypm => &LUNCH_NYPM,
        SetupKind::Combined => &COMBINED,
    }
}

fn setup_name(s: SetupKind) -> &'static str {
    match s {
        SetupKind::AsiaToNyam => "ASIA->NYAM midpoint",
        SetupKind::LondonToNyam => "LONDON->NYAM midpoint",
        SetupKind::LunchToNypm => "LUNCH->NYPM midpoint",
        SetupKind::Combined => "COMBINED midpoint",
    }
}

fn trade_for_relation(day: &DaySummary, rel: RelSpec, cfg: Cfg) -> Option<TradeCandidate> {
    let source = day.zones[rel.source_idx]?;
    let target = day.zones[rel.target_idx]?;

    if cfg.require_target_box_open_inside_source {
        let ob = day.bars[target.start_idx];
        if ob.o > source.high || ob.o < source.low {
            return None;
        }
    }

    let ema50 = if cfg.use_ema_gate {
        Some(ema(&day.closes, 50))
    } else {
        None
    };

    let mut touch: Option<(usize, Side)> = None;
    for idx in target.start_idx..=target.end_idx {
        let b = day.bars[idx];
        if b.o > source.mid && b.lo <= source.mid {
            touch = Some((idx, Side::Long));
            break;
        }
        if b.o < source.mid && b.hi >= source.mid {
            touch = Some((idx, Side::Short));
            break;
        }
    }
    let (touch_idx, side) = touch?;

    let confirm_deadline = (touch_idx + cfg.max_reclaim_bars).min(target.end_idx);
    let confirm_idx = (touch_idx..=confirm_deadline).find(|&idx| {
        if side == Side::Long {
            day.bars[idx].c > source.mid
        } else {
            day.bars[idx].c < source.mid
        }
    })?;

    let rb = day.bars[confirm_idx];
    let rrng = (rb.hi - rb.lo).max(0.0001);
    let rbod = (rb.c - rb.o).abs();
    if rbod / rrng * 100.0 < cfg.min_reclaim_body_pct {
        return None;
    }

    if cfg.use_ema_gate {
        let e = ema50.as_ref().expect("ema");
        let ok = if side == Side::Long {
            day.bars[confirm_idx].c >= e[confirm_idx]
        } else {
            day.bars[confirm_idx].c <= e[confirm_idx]
        };
        if !ok {
            return None;
        }
    }

    let entry_idx = confirm_idx + 1;
    if entry_idx >= day.bars.len() {
        return None;
    }

    for idx in target.start_idx..=confirm_idx {
        if side == Side::Long && day.bars[idx].hi >= source.high {
            return None;
        }
        if side == Side::Short && day.bars[idx].lo <= source.low {
            return None;
        }
    }

    let tick = 0.25;
    let slip = tick;
    let comm_rt = 0.5;
    let entry = if side == Side::Long {
        day.bars[entry_idx].o + slip
    } else {
        day.bars[entry_idx].o - slip
    };
    let struct_stop = if side == Side::Long {
        (touch_idx..=confirm_idx)
            .map(|i| day.bars[i].lo)
            .fold(f64::INFINITY, f64::min)
            - tick
            - slip
    } else {
        (touch_idx..=confirm_idx)
            .map(|i| day.bars[i].hi)
            .fold(f64::NEG_INFINITY, f64::max)
            + tick
            + slip
    };
    let cap_stop = if side == Side::Long {
        entry - cfg.stop_cap_pct * source.range
    } else {
        entry + cfg.stop_cap_pct * source.range
    };
    let stop = if side == Side::Long {
        struct_stop.max(cap_stop)
    } else {
        struct_stop.min(cap_stop)
    };
    let target_px = if side == Side::Long {
        source.high - slip
    } else {
        source.low + slip
    };

    let risk = (entry - stop).abs();
    if risk < tick {
        return None;
    }
    let rr = (target_px - entry).abs() / risk;
    if rr < cfg.min_target_rr {
        return None;
    }
    let dist_pct = (target_px - entry).abs() / source.range * 100.0;
    if dist_pct > cfg.max_entry_dist_to_target_pct {
        return None;
    }
    let cost_r = comm_rt / risk;

    let exit_idx = match cfg.exit_mode {
        ExitMode::TargetZoneEnd => target.end_idx,
        ExitMode::SessionEnd => day.bars.len().saturating_sub(1),
    };
    for i in entry_idx..=exit_idx {
        let b = day.bars[i];
        let stop_hit = if side == Side::Long {
            b.lo <= stop
        } else {
            b.hi >= stop
        };
        let tp_hit = if side == Side::Long {
            b.hi >= target_px
        } else {
            b.lo <= target_px
        };
        if stop_hit && tp_hit {
            return Some(TradeCandidate {
                touch_idx,
                entry_idx,
                r: -1.0 - cost_r,
            });
        }
        if stop_hit {
            return Some(TradeCandidate {
                touch_idx,
                entry_idx,
                r: -1.0 - cost_r,
            });
        }
        if tp_hit {
            return Some(TradeCandidate {
                touch_idx,
                entry_idx,
                r: rr - cost_r,
            });
        }
    }

    let eb = day.bars[exit_idx];
    let exit_px = if side == Side::Long {
        eb.c - slip
    } else {
        eb.c + slip
    };
    let gross_r = if side == Side::Long {
        (exit_px - entry) / risk
    } else {
        (entry - exit_px) / risk
    };
    Some(TradeCandidate {
        touch_idx,
        entry_idx,
        r: gross_r - cost_r,
    })
}

fn trades_for_day(day: &DaySummary, setup: SetupKind, cfg: Cfg) -> Vec<f64> {
    let rels = relation_specs(setup);
    if setup == SetupKind::Combined {
        let mut cands: Vec<TradeCandidate> = rels
            .iter()
            .filter_map(|rel| trade_for_relation(day, *rel, cfg))
            .collect();
        cands.sort_by_key(|c| (c.touch_idx, c.entry_idx));
        cands.first().map(|t| vec![t.r]).unwrap_or_default()
    } else {
        rels.iter()
            .filter_map(|rel| trade_for_relation(day, *rel, cfg).map(|t| t.r))
            .collect()
    }
}

fn eval(days: &[DaySummary], setup: SetupKind, cfg: Cfg) -> EvalStats {
    let mut trades = 0usize;
    let mut wins = 0usize;
    let mut sum_r = 0.0;
    let mut eq = 0.0;
    let mut peak = 0.0;
    let mut max_dd = 0.0;
    for day in days {
        for r in trades_for_day(day, setup, cfg) {
            trades += 1;
            if r > 0.0 {
                wins += 1;
            }
            sum_r += r;
            eq += r;
            if eq > peak {
                peak = eq;
            }
            let dd = peak - eq;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    EvalStats {
        trades,
        win_rate: if trades > 0 {
            wins as f64 * 100.0 / trades as f64
        } else {
            0.0
        },
        expectancy: if trades > 0 {
            sum_r / trades as f64
        } else {
            0.0
        },
        max_dd,
    }
}

fn run_setup_days(days: &[DaySummary], setup: SetupKind, cfg: Cfg) -> Vec<TradeRecord> {
    let mut out = Vec::new();
    for day in days {
        for r in trades_for_day(day, setup, cfg) {
            out.push(TradeRecord {
                day: day.session_day,
                weekday: day.weekday,
                r,
            });
        }
    }
    out
}

fn cfg_label(cfg: Cfg) -> String {
    format!(
        "reclaim<={} stop_cap={:.0}% ema={} exit={} body>={:.0}% rr>={:.2} dist<={:.0}% open_in={}",
        cfg.max_reclaim_bars,
        cfg.stop_cap_pct * 100.0,
        cfg.use_ema_gate,
        if cfg.exit_mode == ExitMode::SessionEnd {
            "eod"
        } else {
            "target_end"
        },
        cfg.min_reclaim_body_pct,
        cfg.min_target_rr,
        cfg.max_entry_dist_to_target_pct,
        cfg.require_target_box_open_inside_source,
    )
}

fn wr_of(rs: &[f64]) -> f64 {
    if rs.is_empty() {
        0.0
    } else {
        rs.iter().filter(|r| **r > 0.0).count() as f64 * 100.0 / rs.len() as f64
    }
}

fn exp_of(rs: &[f64]) -> f64 {
    if rs.is_empty() {
        0.0
    } else {
        rs.iter().sum::<f64>() / rs.len() as f64
    }
}

fn dd_of(rs: &[f64]) -> f64 {
    let mut eq = 0.0;
    let mut peak = 0.0;
    let mut max_dd = 0.0;
    for r in rs {
        eq += *r;
        if eq > peak {
            peak = eq;
        }
        let dd = peak - eq;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    max_dd
}

fn main() {
    let worker_cap = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);
    rayon::ThreadPoolBuilder::new()
        .num_threads(worker_cap)
        .build_global()
        .ok();

    let days = load_days();
    let split = days.len() * 70 / 100;
    let train = Arc::new(days[..split].to_vec());
    let test = Arc::new(days[split..].to_vec());

    let setups = [
        SetupKind::AsiaToNyam,
        SetupKind::LondonToNyam,
        SetupKind::LunchToNypm,
        SetupKind::Combined,
    ];
    let reclaim_bars = [1usize, 2, 3, 5];
    let stop_caps = [0.20_f64, 0.30, 0.40, 0.50];
    let ema_gate = [false, true];
    let exit_modes = [ExitMode::TargetZoneEnd, ExitMode::SessionEnd];
    let body_pcts = [20.0_f64, 30.0, 40.0];
    let min_rrs = [0.30_f64, 0.50, 0.70];
    let max_target_dist_pcts = [90.0_f64, 75.0, 60.0];
    let require_open_inside = [false, true];

    let mut jobs = Vec::new();
    for setup in setups {
        for max_reclaim_bars in reclaim_bars {
            for stop_cap_pct in stop_caps {
                for use_ema_gate in ema_gate {
                    for exit_mode in exit_modes {
                        for min_reclaim_body_pct in body_pcts {
                            for min_target_rr in min_rrs {
                                for max_entry_dist_to_target_pct in max_target_dist_pcts {
                                    for require_target_box_open_inside_source in require_open_inside
                                    {
                                        jobs.push((
                                            setup,
                                            Cfg {
                                                max_reclaim_bars,
                                                stop_cap_pct,
                                                use_ema_gate,
                                                exit_mode,
                                                min_reclaim_body_pct,
                                                min_target_rr,
                                                max_entry_dist_to_target_pct,
                                                require_target_box_open_inside_source,
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
    }

    let mut rows: Vec<SweepRow> = jobs
        .par_iter()
        .filter_map(|(setup, cfg)| {
            let is_stats = eval(train.as_slice(), *setup, *cfg);
            let oos_stats = eval(test.as_slice(), *setup, *cfg);
            if oos_stats.trades < 25 {
                return None;
            }
            Some(SweepRow {
                cfg: *cfg,
                label: cfg_label(*cfg),
                setup: *setup,
                is_trades: is_stats.trades,
                is_win_rate: is_stats.win_rate,
                is_exp: is_stats.expectancy,
                is_max_dd: is_stats.max_dd,
                oos_trades: oos_stats.trades,
                oos_win_rate: oos_stats.win_rate,
                oos_exp: oos_stats.expectancy,
                oos_max_dd: oos_stats.max_dd,
            })
        })
        .collect();

    rows.sort_by(|a, b| {
        b.oos_exp
            .total_cmp(&a.oos_exp)
            .then(b.oos_trades.cmp(&a.oos_trades))
            .then(b.oos_win_rate.total_cmp(&a.oos_win_rate))
    });

    let csv_path = "reports/strategy_overviews/mnq_killzone_midpoint_strategy_sweep.csv";
    let mut w = Writer::from_path(csv_path).expect("create csv");
    w.write_record([
        "setup",
        "cfg",
        "is_trades",
        "is_win_rate",
        "is_exp_r",
        "is_max_dd_r",
        "oos_trades",
        "oos_win_rate",
        "oos_exp_r",
        "oos_max_dd_r",
    ])
    .expect("header");
    for r in &rows {
        w.write_record([
            setup_name(r.setup),
            r.label.as_str(),
            &r.is_trades.to_string(),
            &format!("{:.4}", r.is_win_rate),
            &format!("{:.6}", r.is_exp),
            &format!("{:.6}", r.is_max_dd),
            &r.oos_trades.to_string(),
            &format!("{:.4}", r.oos_win_rate),
            &format!("{:.6}", r.oos_exp),
            &format!("{:.6}", r.oos_max_dd),
        ])
        .expect("row");
    }
    w.flush().expect("flush");

    println!("MNQ focused killzone midpoint strategy study (v2)");
    println!("Dataset: assets/mnq_1m_cont.parquet");
    println!(
        "Session days: {} (`{}` -> `{}`)",
        days.len(),
        days.first()
            .map(|d| d.session_day.to_string())
            .unwrap_or_default(),
        days.last()
            .map(|d| d.session_day.to_string())
            .unwrap_or_default()
    );
    println!("Split: {} train / {} test", train.len(), test.len());
    println!("Signal: midpoint touch -> reclaim -> source extreme target.");
    println!("Added filters: reclaim body%, min target RR, max target distance, target-box open-inside-source.");
    println!(
        "Runtime: load once, Arc + rayon sweep, workers cap={}",
        worker_cap
    );
    println!("CSV: {}", csv_path);

    for setup in setups {
        println!("\n## {}", setup_name(setup));
        let top: Vec<&SweepRow> = rows.iter().filter(|r| r.setup == setup).take(8).collect();
        for (i, r) in top.iter().enumerate() {
            println!(
                "{}. {} | IS: n={} wr={:.2}% exp={:.3}R maxDD={:.2}R | OOS: n={} wr={:.2}% exp={:.3}R maxDD={:.2}R",
                i + 1,
                r.label,
                r.is_trades,
                r.is_win_rate,
                r.is_exp,
                r.is_max_dd,
                r.oos_trades,
                r.oos_win_rate,
                r.oos_exp,
                r.oos_max_dd,
            );
        }
    }

    let best_lunch = rows
        .iter()
        .find(|r| r.setup == SetupKind::LunchToNypm)
        .expect("best lunch");
    let lunch_oos = run_setup_days(test.as_slice(), SetupKind::LunchToNypm, best_lunch.cfg);
    println!("\n## Robustness: LUNCH->NYPM (best OOS cfg)");
    println!("- {}", best_lunch.label);

    let mut ymap: BTreeMap<i32, Vec<f64>> = BTreeMap::new();
    let mut wdmap: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for t in lunch_oos {
        ymap.entry(t.day.year()).or_default().push(t.r);
        wdmap
            .entry(format!("{:?}", t.weekday))
            .or_default()
            .push(t.r);
    }
    println!("- OOS yearly");
    for (y, rs) in ymap {
        println!(
            "  - {}: n={} wr={:.2}% exp={:.3}R maxDD={:.2}R",
            y,
            rs.len(),
            wr_of(&rs),
            exp_of(&rs),
            dd_of(&rs),
        );
    }
    println!("- OOS weekday");
    for (wd, rs) in wdmap {
        println!(
            "  - {}: n={} wr={:.2}% exp={:.3}R maxDD={:.2}R",
            wd,
            rs.len(),
            wr_of(&rs),
            exp_of(&rs),
            dd_of(&rs),
        );
    }
}
