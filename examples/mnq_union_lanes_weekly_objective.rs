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
    cutoff_h: u32,
    cutoff_m: u32,
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
        if t.hour() > cutoff_h || (t.hour() == cutoff_h && t.minute() > cutoff_m) {
            return (-comm_rt_points, i);
        }
        let h = as_f64(day[i].high.0);
        let l = as_f64(day[i].low.0);
        let s = if armed { entry } else { entry - stop };
        let tp = entry + target;
        if l <= s {
            return (if armed { -comm_rt_points } else { -stop - comm_rt_points }, i);
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
    cutoff_h: u32,
    cutoff_m: u32,
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
        if t.hour() > cutoff_h || (t.hour() == cutoff_h && t.minute() > cutoff_m) {
            return (-comm_rt_points, i);
        }
        let h = as_f64(day[i].high.0);
        let l = as_f64(day[i].low.0);
        let s = if armed { entry } else { entry + stop };
        let tp = entry - target;
        if h >= s {
            return (if armed { -comm_rt_points } else { -stop - comm_rt_points }, i);
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

fn lane_b_lunch_nypm(day: &[CandleStick], slip_ticks: f64, comm_rt_points: f64) -> f64 {
    let mut lunch_hi = f64::NEG_INFINITY;
    let mut lunch_lo = f64::INFINITY;
    let mut lunch_ok = false;
    let mut ny_start = None;
    let mut ny_end = None;
    for (i, c) in day.iter().enumerate() {
        let t = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        if t.hour() == 12 {
            lunch_ok = true;
            lunch_hi = lunch_hi.max(as_f64(c.high.0));
            lunch_lo = lunch_lo.min(as_f64(c.low.0));
        }
        if t.hour() > 13 || (t.hour() == 13 && t.minute() >= 30) {
            if ny_start.is_none() {
                ny_start = Some(i);
            }
            ny_end = Some(i);
        }
    }
    let (Some(ns), Some(ne)) = (ny_start, ny_end) else {
        return 0.0;
    };
    if !lunch_ok || lunch_hi <= lunch_lo {
        return 0.0;
    }

    let mid = (lunch_hi + lunch_lo) * 0.5;
    let range = lunch_hi - lunch_lo;
    let mut touch: Option<(usize, bool)> = None;
    for (i, c) in day.iter().enumerate().take(ne + 1).skip(ns) {
        let o = as_f64(c.open.0);
        let h = as_f64(c.high.0);
        let l = as_f64(c.low.0);
        if o > mid && l <= mid {
            touch = Some((i, true));
            break;
        }
        if o < mid && h >= mid {
            touch = Some((i, false));
            break;
        }
    }
    let Some((ti, is_long)) = touch else {
        return 0.0;
    };

    let ci = (ti + 1).min(ne);
    let cc = as_f64(day[ci].close.0);
    if is_long && cc <= mid {
        return 0.0;
    }
    if !is_long && cc >= mid {
        return 0.0;
    }
    let rb_o = as_f64(day[ci].open.0);
    let rb_h = as_f64(day[ci].high.0);
    let rb_l = as_f64(day[ci].low.0);
    let body_pct = ((cc - rb_o).abs() / (rb_h - rb_l).max(0.0001)) * 100.0;
    if body_pct < 40.0 {
        return 0.0;
    }

    let eidx = (ci + 1).min(day.len().saturating_sub(1));
    let entry = as_f64(day[eidx].open.0);
    let stop = 0.20 * range;
    let target = if is_long {
        (lunch_hi - entry).max(0.0)
    } else {
        (entry - lunch_lo).max(0.0)
    };
    if target <= 0.0 {
        return 0.0;
    }
    let rr = target / stop.max(0.0001);
    if rr < 0.30 {
        return 0.0;
    }

    if is_long {
        let (pts, _) = trade_long_be(day, eidx, stop, target, 9999.0, slip_ticks, comm_rt_points, 16, 0);
        pts
    } else {
        let (pts, _) = trade_short_be(day, eidx, stop, target, 9999.0, slip_ticks, comm_rt_points, 16, 0);
        pts
    }
}

#[derive(Clone, Copy)]
struct Row {
    cont_stop: f64,
    cont_target: f64,
    slip: f64,
    use_lane_b: bool,
    weeks: usize,
    avg: f64,
    med: f64,
    pct80: f64,
    pct100: f64,
    worst: f64,
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

    let mut rows: Vec<Row> = Vec::new();

    for use_lane_b in [false, true] {
        for (cont_stop, cont_target) in cont_params {
            for slip in slips {
                let mut weekly: BTreeMap<(i32, u32), f64> = BTreeMap::new();
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
                        let (pts, eidx) = trade_long_be(
                            day,
                            idx,
                            rev_stop,
                            rev_target,
                            rev_be,
                            slip,
                            comm_rt_points,
                            11,
                            0,
                        );
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
                                11,
                                0,
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
                                11,
                                0,
                            )
                        };
                        *weekly.entry(key).or_insert(0.0) += pts;
                    }

                    if use_lane_b {
                        let pts_b = lane_b_lunch_nypm(day, slip, comm_rt_points);
                        *weekly.entry(key).or_insert(0.0) += pts_b;
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
                rows.push(Row {
                    cont_stop,
                    cont_target,
                    slip,
                    use_lane_b,
                    weeks,
                    avg,
                    med,
                    pct80,
                    pct100,
                    worst,
                });
            }
        }
    }

    rows.sort_by(|a, b| {
        b.pct80
            .total_cmp(&a.pct80)
            .then_with(|| b.pct100.total_cmp(&a.pct100))
            .then_with(|| b.worst.total_cmp(&a.worst))
            .then_with(|| b.avg.total_cmp(&a.avg))
    });

    println!("weekly-objective rank (primary pct>=80, secondary pct>=100, tertiary worst_week)");
    println!(
        "rank,use_lane_b,cont_stop,cont_target,slip,weeks,avg_pts_w,med_pts_w,pct_ge_80,pct_ge_100,worst_week"
    );
    for (k, r) in rows.iter().take(20).enumerate() {
        println!(
            "{},{},{:.1},{:.1},{:.0},{},{:.2},{:.2},{:.2},{:.2},{:.2}",
            k + 1,
            r.use_lane_b,
            r.cont_stop,
            r.cont_target,
            r.slip,
            r.weeks,
            r.avg,
            r.med,
            r.pct80,
            r.pct100,
            r.worst
        );
    }
}
