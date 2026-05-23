use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{Datelike, TimeZone, Timelike};
use chrono_tz::America::New_York;
use std::collections::BTreeMap;

fn as_f64(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn percentile(mut v: Vec<f64>, p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    let idx = ((v.len() - 1) as f64 * p).round() as usize;
    v[idx]
}

fn trade_long_be(
    day: &[CandleStick],
    entry_idx: usize,
    stop: f64,
    target: f64,
    be: f64,
    slip_ticks: f64,
    comm_rt_points: f64,
) -> (f64, usize) {
    let tick = 0.25;
    let slip = slip_ticks * tick;
    let entry = as_f64(day[entry_idx].open.0) + slip;
    let mut armed = false;
    let mut i = entry_idx;
    while i < day.len() {
        let t = New_York
            .timestamp_opt(day[i].open_time, 0)
            .single()
            .expect("ts");
        if t.hour() > 11 || (t.hour() == 11 && t.minute() > 0) {
            return (-comm_rt_points, i);
        }
        let h = as_f64(day[i].high.0);
        let l = as_f64(day[i].low.0);
        let s = if armed { entry } else { entry - stop };
        let tp = entry + target;
        if l <= s {
            return (
                if armed {
                    -comm_rt_points
                } else {
                    -stop - comm_rt_points
                },
                i,
            );
        }
        if h >= tp {
            return (target - comm_rt_points, i);
        }
        if !armed && h >= entry + be {
            armed = true;
        }
        i += 1;
    }
    (-comm_rt_points, day.len().saturating_sub(1))
}

fn trade_short_be(
    day: &[CandleStick],
    entry_idx: usize,
    stop: f64,
    target: f64,
    be: f64,
    slip_ticks: f64,
    comm_rt_points: f64,
) -> (f64, usize) {
    let tick = 0.25;
    let slip = slip_ticks * tick;
    let entry = as_f64(day[entry_idx].open.0) - slip;
    let mut armed = false;
    let mut i = entry_idx;
    while i < day.len() {
        let t = New_York
            .timestamp_opt(day[i].open_time, 0)
            .single()
            .expect("ts");
        if t.hour() > 11 || (t.hour() == 11 && t.minute() > 0) {
            return (-comm_rt_points, i);
        }
        let h = as_f64(day[i].high.0);
        let l = as_f64(day[i].low.0);
        let s = if armed { entry } else { entry + stop };
        let tp = entry - target;
        if h >= s {
            return (
                if armed {
                    -comm_rt_points
                } else {
                    -stop - comm_rt_points
                },
                i,
            );
        }
        if l <= tp {
            return (target - comm_rt_points, i);
        }
        if !armed && l <= entry - be {
            armed = true;
        }
        i += 1;
    }
    (-comm_rt_points, day.len().saturating_sub(1))
}

fn same_day_gate(day: &[CandleStick]) -> bool {
    let mut hi69 = f64::NEG_INFINITY;
    let mut lo69 = f64::INFINITY;
    let mut ok69 = false;
    for c in day {
        let t = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        if (6..9).contains(&t.hour()) {
            ok69 = true;
            hi69 = hi69.max(as_f64(c.high.0));
            lo69 = lo69.min(as_f64(c.low.0));
        }
    }
    if !ok69 || hi69 <= lo69 {
        return false;
    }
    let range = hi69 - lo69;
    let max_overshoot = 0.35 * range;

    let mut break_idx: Option<(usize, bool)> = None;
    for (i, c) in day.iter().enumerate() {
        let t = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        let in_window = (t.hour() > 9 || (t.hour() == 9 && t.minute() >= 0))
            && (t.hour() < 9 || (t.hour() == 9 && t.minute() <= 35));
        if !in_window || t.hour() > 9 {
            continue;
        }
        let h = as_f64(c.high.0);
        let l = as_f64(c.low.0);
        if h > hi69 {
            break_idx = Some((i, true));
            break;
        }
        if l < lo69 {
            break_idx = Some((i, false));
            break;
        }
    }
    let Some((i0, top_break)) = break_idx else {
        return false;
    };

    let i1 = (i0 + 2).min(day.len().saturating_sub(1));
    let mut over: f64 = 0.0;
    for c in day.iter().take(i1 + 1).skip(i0) {
        let h = as_f64(c.high.0);
        let l = as_f64(c.low.0);
        if top_break {
            over = over.max((h - hi69).max(0.0));
        } else {
            over = over.max((lo69 - l).max(0.0));
        }
    }
    if over > max_overshoot {
        return false;
    }

    for c in day.iter().take(i1 + 1).skip(i0) {
        let cl = as_f64(c.close.0);
        if top_break && cl <= hi69 {
            return true;
        }
        if !top_break && cl >= lo69 {
            return true;
        }
    }
    false
}

fn main() {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("load mnq");

    let rev_stop = 15.0;
    let rev_target = 30.0;
    let rev_be = 30.0;
    let rev_max_trades = 6usize;
    let rev_cooldown = 3usize;
    let cont_params = [(20.0, 30.0), (20.0, 40.0), (25.0, 40.0), (30.0, 45.0)];
    let slips = [1.0, 2.0, 3.0];
    let comm_rt_points = 0.62;

    println!(
        "fusion weekly scan: 9:35 core + SAME-DAY non-lookahead gate (6-9 break reclaim), 2025+"
    );
    println!("cont_stop,cont_target,slip,weeks,qualified_days,avg_pts_w,med_pts_w,pct_ge_80,pct_ge_100,worst_week,max_week");

    for (cont_stop, cont_target) in cont_params {
        for slip in slips {
            let mut weekly: BTreeMap<(i32, u32), f64> = BTreeMap::new();
            let mut qualified_days = 0usize;

            let mut i = 0usize;
            while i < candles.len() {
                let dt = New_York
                    .timestamp_opt(candles[i].open_time, 0)
                    .single()
                    .expect("ts");
                let d = dt.date_naive();
                let mut j = i;
                while j < candles.len() {
                    let dj = New_York
                        .timestamp_opt(candles[j].open_time, 0)
                        .single()
                        .expect("ts")
                        .date_naive();
                    if dj != d {
                        break;
                    }
                    j += 1;
                }
                if dt.year() < 2025 {
                    i = j;
                    continue;
                }

                let day = &candles[i..j];
                if !same_day_gate(day) {
                    i = j;
                    continue;
                }

                let mut od_hi = f64::NEG_INFINITY;
                let mut od_lo = f64::INFINITY;
                let mut od_ok = false;
                let mut r_hi = f64::NEG_INFINITY;
                let mut r_lo = f64::INFINITY;
                let mut r_ok = false;

                for c in day {
                    let t = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
                    if t.hour() == 9 && (30..=39).contains(&t.minute()) {
                        od_ok = true;
                        od_hi = od_hi.max(as_f64(c.high.0));
                        od_lo = od_lo.min(as_f64(c.low.0));
                    }
                    if t.hour() == 9 && (35..=55).contains(&t.minute()) {
                        r_ok = true;
                        r_hi = r_hi.max(as_f64(c.high.0));
                        r_lo = r_lo.min(as_f64(c.low.0));
                    }
                }
                if !od_ok || !r_ok {
                    i = j;
                    continue;
                }
                let od = od_hi - od_lo;
                let rr = r_hi - r_lo;
                if !(od >= 50.0 && rr >= 30.0 && rr <= 70.0) {
                    i = j;
                    continue;
                }

                qualified_days += 1;
                let wk = dt.iso_week();
                let key = (wk.year(), wk.week());

                let mut idx = 0usize;
                let mut trades = 0usize;
                while idx < day.len() && trades < rev_max_trades {
                    let t = New_York
                        .timestamp_opt(day[idx].open_time, 0)
                        .single()
                        .expect("ts");
                    let in_window = (t.hour() > 9 || (t.hour() == 9 && t.minute() >= 35))
                        && (t.hour() < 10 || (t.hour() == 10 && t.minute() <= 30));
                    if !in_window {
                        idx += 1;
                        continue;
                    }
                    let (pts, eidx) =
                        trade_long_be(day, idx, rev_stop, rev_target, rev_be, slip, comm_rt_points);
                    *weekly.entry(key).or_insert(0.0) += pts;
                    trades += 1;
                    idx = eidx.saturating_add(1 + rev_cooldown);
                }

                let mut sig_idx: Option<(usize, bool)> = None;
                for (k, c) in day.iter().enumerate() {
                    let t = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
                    if t.hour() < 10 || (t.hour() == 10 && t.minute() <= 0) {
                        continue;
                    }
                    if t.hour() > 10 || (t.hour() == 10 && t.minute() > 40) {
                        break;
                    }
                    let h = as_f64(c.high.0);
                    let l = as_f64(c.low.0);
                    let cl = as_f64(c.close.0);
                    if h > r_hi && cl > r_hi {
                        sig_idx = Some((k, true));
                        break;
                    }
                    if l < r_lo && cl < r_lo {
                        sig_idx = Some((k, false));
                        break;
                    }
                }
                if let Some((k, is_long)) = sig_idx {
                    let entry_idx = (k + 1).min(day.len().saturating_sub(1));
                    let (pts, _) = if is_long {
                        trade_long_be(
                            day,
                            entry_idx,
                            cont_stop,
                            cont_target,
                            30.0,
                            slip,
                            comm_rt_points,
                        )
                    } else {
                        trade_short_be(
                            day,
                            entry_idx,
                            cont_stop,
                            cont_target,
                            30.0,
                            slip,
                            comm_rt_points,
                        )
                    };
                    *weekly.entry(key).or_insert(0.0) += pts;
                }

                i = j;
            }

            let vals: Vec<f64> = weekly.values().copied().collect();
            let weeks = vals.len();
            let avg = if weeks > 0 {
                vals.iter().sum::<f64>() / weeks as f64
            } else {
                0.0
            };
            let med = percentile(vals.clone(), 0.5);
            let pct80 = if weeks > 0 {
                vals.iter().filter(|x| **x >= 80.0).count() as f64 * 100.0 / weeks as f64
            } else {
                0.0
            };
            let pct100 = if weeks > 0 {
                vals.iter().filter(|x| **x >= 100.0).count() as f64 * 100.0 / weeks as f64
            } else {
                0.0
            };
            let worst = vals.iter().copied().fold(f64::INFINITY, f64::min).min(0.0);
            let maxw = vals
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .max(0.0);

            println!(
                "{:.1},{:.1},{:.0},{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
                cont_stop,
                cont_target,
                slip,
                weeks,
                qualified_days,
                avg,
                med,
                pct80,
                pct100,
                worst,
                maxw
            );
        }
    }
}
