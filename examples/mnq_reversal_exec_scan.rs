use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy)]
enum Side {
    Top,
    Bottom,
}

#[derive(Clone)]
struct DaySample {
    range_high: f64,
    range_low: f64,
    range_size: f64,
    break_side: Side,
    break_extreme: f64,
    reclaim_idx: Option<usize>,
    first_break_idx: usize,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    hours: Vec<u32>,
}

#[derive(Clone)]
struct ResultRow {
    name: String,
    trades: usize,
    win_rate: f64,
    exp_r: f64,
}

#[derive(Clone, Copy)]
enum EntryMode {
    NextOpen,
    ReclaimBoundaryLimit,
    MidPullback,
}

#[derive(Clone, Copy)]
enum StopMode {
    BreakExtremePlusTick,
    CappedStructure35Range,
}

#[derive(Clone, Copy)]
enum ExitMode {
    OppRange,
    Fixed2R,
    Partial1RRunnerOpp,
}

#[derive(Clone, Copy)]
struct ExecCfg {
    entry: EntryMode,
    stop: StopMode,
    exit: ExitMode,
    time_stop_hour: u32,
    slip_ticks: i32,
    comm_points_rt: f64,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn percentile_rank(sorted: &[f64], value: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let mut lo = 0usize;
    let mut hi = sorted.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if sorted[mid] <= value {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo as f64 / sorted.len() as f64
}

fn collect_samples(candles: &[CandleStick]) -> Vec<DaySample> {
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < candles.len() {
        let d = New_York
            .timestamp_opt(candles[i].open_time, 0)
            .single()
            .expect("timestamp")
            .date_naive();
        let mut j = i;
        while j < candles.len() {
            let dj = New_York
                .timestamp_opt(candles[j].open_time, 0)
                .single()
                .expect("timestamp")
                .date_naive();
            if dj != d {
                break;
            }
            j += 1;
        }

        let day = &candles[i..j];
        let opens: Vec<f64> = day.iter().map(|c| d2f(c.open.0)).collect();
        let highs: Vec<f64> = day.iter().map(|c| d2f(c.high.0)).collect();
        let lows: Vec<f64> = day.iter().map(|c| d2f(c.low.0)).collect();
        let hours: Vec<u32> = day
            .iter()
            .map(|c| {
                New_York
                    .timestamp_opt(c.open_time, 0)
                    .single()
                    .expect("timestamp")
                    .hour()
            })
            .collect();

        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has_range = false;
        for k in 0..day.len() {
            if (6..9).contains(&hours[k]) {
                has_range = true;
                rh = rh.max(highs[k]);
                rl = rl.min(lows[k]);
            }
        }
        if !has_range || rh <= rl {
            i = j;
            continue;
        }

        let mut first_break_idx = None;
        let mut break_side = None;
        for k in 0..day.len() {
            if hours[k] < 9 {
                continue;
            }
            let hit_top = highs[k] >= rh;
            let hit_bottom = lows[k] <= rl;
            if hit_top {
                first_break_idx = Some(k);
                break_side = Some(Side::Top);
                break;
            }
            if hit_bottom {
                first_break_idx = Some(k);
                break_side = Some(Side::Bottom);
                break;
            }
        }
        if first_break_idx.is_none() {
            i = j;
            continue;
        }
        let bidx = first_break_idx.expect("break idx");
        let side = break_side.expect("break side");

        let mut break_extreme = match side {
            Side::Top => rh,
            Side::Bottom => rl,
        };
        let mut reclaim_idx = None;
        for k in bidx..day.len() {
            match side {
                Side::Top => {
                    break_extreme = break_extreme.max(highs[k]);
                    if day[k].close.0 <= rust_decimal::Decimal::from_f64_retain(rh).unwrap() {
                        reclaim_idx = Some(k);
                        break;
                    }
                }
                Side::Bottom => {
                    break_extreme = break_extreme.min(lows[k]);
                    if day[k].close.0 >= rust_decimal::Decimal::from_f64_retain(rl).unwrap() {
                        reclaim_idx = Some(k);
                        break;
                    }
                }
            }
        }

        out.push(DaySample {
            range_high: rh,
            range_low: rl,
            range_size: rh - rl,
            break_side: side,
            break_extreme,
            reclaim_idx,
            first_break_idx: bidx,
            opens,
            highs,
            lows,
            hours,
        });

        i = j;
    }
    out
}

fn simulate(sample: &DaySample, cfg: ExecCfg) -> Option<(f64, bool)> {
    let tick = 0.25;
    let slip = cfg.slip_ticks as f64 * tick;
    let r_idx = sample.reclaim_idx?;
    let probe_idx = r_idx + 1;
    if probe_idx >= sample.opens.len() {
        return None;
    }

    let mut entry = sample.opens[probe_idx];
    match cfg.entry {
        EntryMode::ReclaimBoundaryLimit => {
            entry = match sample.break_side {
                Side::Top => sample.range_high,
                Side::Bottom => sample.range_low,
            };
        }
        EntryMode::MidPullback => {
            entry = match sample.break_side {
                Side::Top => (sample.range_high + sample.break_extreme) * 0.5,
                Side::Bottom => (sample.range_low + sample.break_extreme) * 0.5,
            };
        }
        EntryMode::NextOpen => {}
    }

    entry = match sample.break_side {
        Side::Top => entry - slip,
        Side::Bottom => entry + slip,
    };

    let entry_idx = if matches!(cfg.entry, EntryMode::NextOpen) {
        probe_idx
    } else {
        let mut filled = None;
        for k in probe_idx..sample.opens.len() {
            if sample.hours[k] >= cfg.time_stop_hour {
                break;
            }
            let hit = match sample.break_side {
                Side::Top => sample.lows[k] <= entry,
                Side::Bottom => sample.highs[k] >= entry,
            };
            if hit {
                filled = Some(k);
                break;
            }
        }
        filled?
    };

    let stop = match cfg.stop {
        StopMode::BreakExtremePlusTick => match sample.break_side {
            Side::Top => sample.break_extreme + tick + slip,
            Side::Bottom => sample.break_extreme - tick - slip,
        },
        StopMode::CappedStructure35Range => {
            let cap = sample.range_size * 0.35;
            match sample.break_side {
                Side::Top => {
                    let s_struct = sample.break_extreme + tick + slip;
                    let s_cap = entry + cap;
                    s_struct.min(s_cap)
                }
                Side::Bottom => {
                    let s_struct = sample.break_extreme - tick - slip;
                    let s_cap = entry - cap;
                    s_struct.max(s_cap)
                }
            }
        }
    };

    let risk = (entry - stop).abs();
    if risk < tick {
        return None;
    }
    let target_opp = match sample.break_side {
        Side::Top => sample.range_low + slip,
        Side::Bottom => sample.range_high - slip,
    };
    let target_1r = match sample.break_side {
        Side::Top => entry - risk,
        Side::Bottom => entry + risk,
    };
    let target_2r = match sample.break_side {
        Side::Top => entry - 2.0 * risk,
        Side::Bottom => entry + 2.0 * risk,
    };
    let cost_r = cfg.comm_points_rt / risk;

    let mut hit_1r = false;
    for k in entry_idx..sample.highs.len() {
        if sample.hours[k] >= cfg.time_stop_hour {
            return Some((-0.1 - cost_r, false));
        }
        let h = sample.highs[k];
        let l = sample.lows[k];

        let stop_hit = match sample.break_side {
            Side::Top => h >= stop,
            Side::Bottom => l <= stop,
        };

        if stop_hit {
            if matches!(cfg.exit, ExitMode::Partial1RRunnerOpp) && hit_1r {
                return Some((0.5 - 0.5 - cost_r, false));
            }
            return Some((-1.0 - cost_r, false));
        }

        match cfg.exit {
            ExitMode::OppRange => {
                let tp_hit = match sample.break_side {
                    Side::Top => l <= target_opp,
                    Side::Bottom => h >= target_opp,
                };
                if tp_hit {
                    let rr = (target_opp - entry).abs() / risk;
                    return Some((rr - cost_r, true));
                }
            }
            ExitMode::Fixed2R => {
                let tp_hit = match sample.break_side {
                    Side::Top => l <= target_2r,
                    Side::Bottom => h >= target_2r,
                };
                if tp_hit {
                    return Some((2.0 - cost_r, true));
                }
            }
            ExitMode::Partial1RRunnerOpp => {
                if !hit_1r {
                    let hit = match sample.break_side {
                        Side::Top => l <= target_1r,
                        Side::Bottom => h >= target_1r,
                    };
                    if hit {
                        hit_1r = true;
                    }
                }
                if hit_1r {
                    let hit_runner = match sample.break_side {
                        Side::Top => l <= target_opp,
                        Side::Bottom => h >= target_opp,
                    };
                    if hit_runner {
                        let rr_runner = (target_opp - entry).abs() / risk;
                        return Some((0.5 * 1.0 + 0.5 * rr_runner - cost_r, true));
                    }
                }
            }
        }
    }

    Some((-0.1 - cost_r, false))
}

fn main() {
    let candles =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("load parquet");
    let samples = collect_samples(&candles);
    let mut ranges: Vec<f64> = samples.iter().map(|s| s.range_size).collect();
    ranges.sort_by(f64::total_cmp);

    let reclaim_caps = [1usize, 2, 3];
    let ov_caps = [25.0, 35.0, 50.0];
    let range_caps = [0.3, 0.4, 0.5];
    let entries = [
        EntryMode::NextOpen,
        EntryMode::ReclaimBoundaryLimit,
        EntryMode::MidPullback,
    ];
    let stops = [
        StopMode::BreakExtremePlusTick,
        StopMode::CappedStructure35Range,
    ];
    let exits = [
        ExitMode::OppRange,
        ExitMode::Fixed2R,
        ExitMode::Partial1RRunnerOpp,
    ];
    let t_stops = [11_u32, 12_u32];
    let slips = [0_i32, 1_i32];
    let comms = [0.25_f64, 0.5_f64];

    let mut rows: Vec<ResultRow> = Vec::new();

    for reclaim_cap in reclaim_caps {
        for ov_cap in ov_caps {
            for range_cap in range_caps {
                for entry in entries {
                    for stop in stops {
                        for exit in exits {
                            for t_stop in t_stops {
                                for slip in slips {
                                    for comm in comms {
                                        let cfg = ExecCfg {
                                            entry,
                                            stop,
                                            exit,
                                            time_stop_hour: t_stop,
                                            slip_ticks: slip,
                                            comm_points_rt: comm,
                                        };

                                        let mut vals = Vec::new();
                                        let mut wins = 0usize;
                                        for s in &samples {
                                            let rp = percentile_rank(&ranges, s.range_size);
                                            if rp > range_cap {
                                                continue;
                                            }
                                            if s.reclaim_idx.is_none() {
                                                continue;
                                            }
                                            let rb =
                                                s.reclaim_idx.expect("reclaim") - s.first_break_idx;
                                            if rb > reclaim_cap {
                                                continue;
                                            }
                                            let ov = match s.break_side {
                                                Side::Top => {
                                                    (s.break_extreme - s.range_high) / s.range_size
                                                        * 100.0
                                                }
                                                Side::Bottom => {
                                                    (s.range_low - s.break_extreme) / s.range_size
                                                        * 100.0
                                                }
                                            };
                                            if ov > ov_cap {
                                                continue;
                                            }

                                            if let Some((r, win)) = simulate(s, cfg) {
                                                vals.push(r);
                                                if win {
                                                    wins += 1;
                                                }
                                            }
                                        }

                                        if vals.len() < 60 {
                                            continue;
                                        }
                                        let exp = vals.iter().sum::<f64>() / vals.len() as f64;
                                        let wr = wins as f64 / vals.len() as f64 * 100.0;
                                        let entry_s = match entry {
                                            EntryMode::NextOpen => "next_open",
                                            EntryMode::ReclaimBoundaryLimit => "reclaim_limit",
                                            EntryMode::MidPullback => "mid_pullback",
                                        };
                                        let stop_s = match stop {
                                            StopMode::BreakExtremePlusTick => "extreme+tick",
                                            StopMode::CappedStructure35Range => "cap35%",
                                        };
                                        let exit_s = match exit {
                                            ExitMode::OppRange => "opp_range",
                                            ExitMode::Fixed2R => "fixed_2r",
                                            ExitMode::Partial1RRunnerOpp => "partial_1r_runner_opp",
                                        };
                                        rows.push(ResultRow {
                                            name: format!(
                                                "reclaim<={reclaim_cap} ov<={ov_cap:.0}% range<={:.0}% entry={entry_s} stop={stop_s} exit={exit_s} tstop={t_stop} slip={} comm={:.2}",
                                                range_cap * 100.0,
                                                slip,
                                                comm
                                            ),
                                            trades: vals.len(),
                                            win_rate: wr,
                                            exp_r: exp,
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

    rows.sort_by(|a, b| {
        b.exp_r
            .total_cmp(&a.exp_r)
            .then(b.win_rate.total_cmp(&a.win_rate))
    });

    println!("MNQ reversal execution scan (6-9 NY range)");
    println!("Samples: {}", samples.len());
    println!("Top configs:");
    for r in rows.iter().take(20) {
        println!(
            "- {} | trades={} win_rate={:.2}% exp={:.3}R",
            r.name, r.trades, r.win_rate, r.exp_r
        );
    }
}
