use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Short,
    Long,
}

#[derive(Clone, Copy)]
struct B1 {
    ts: i64,
    h: u32,
    o: f64,
    hi: f64,
    lo: f64,
    c: f64,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn resample_5m(v: &[B1]) -> Vec<B1> {
    if v.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < v.len() {
        let bucket = (v[i].ts / 300) * 300;
        let mut j = i + 1;
        while j < v.len() && (v[j].ts / 300) * 300 == bucket {
            j += 1;
        }
        let s = &v[i..j];
        let mut hi = f64::NEG_INFINITY;
        let mut lo = f64::INFINITY;
        for b in s {
            hi = hi.max(b.hi);
            lo = lo.min(b.lo);
        }
        let dt = New_York.timestamp_opt(bucket, 0).single().expect("ts");
        out.push(B1 {
            ts: bucket,
            h: dt.hour(),
            o: s[0].o,
            hi,
            lo,
            c: s[s.len() - 1].c,
        });
        i = j;
    }
    out
}

fn find_mss_5m(v5: &[B1], side: Side, start: usize) -> Option<usize> {
    if v5.len() < 4 {
        return None;
    }
    let mut pivot = None;
    for i in (start + 1)..(v5.len() - 1) {
        let body = (v5[i].c - v5[i].o).abs();
        let range = (v5[i].hi - v5[i].lo).max(0.0001);
        if side == Side::Short {
            if v5[i].lo < v5[i - 1].lo && v5[i].lo < v5[i + 1].lo {
                pivot = Some(v5[i].lo);
            }
            if let Some(p) = pivot {
                if v5[i].c < p && body / range > 0.45 {
                    return Some(i);
                }
            }
        } else {
            if v5[i].hi > v5[i - 1].hi && v5[i].hi > v5[i + 1].hi {
                pivot = Some(v5[i].hi);
            }
            if let Some(p) = pivot {
                if v5[i].c > p && body / range > 0.45 {
                    return Some(i);
                }
            }
        }
    }
    None
}

#[derive(Clone, Copy)]
struct Cfg {
    min_disp_body: f64,
    max_ifvg_pct: f64,
    stop_cap_pct: f64,
    entry_deadline_hour: u32,
    time_stop_hour: u32,
    use_bias_filter: bool,
}

fn run_cfg(days: &[Vec<B1>], cfg: Cfg) -> (usize, usize, f64) {
    let mut trades = 0usize;
    let mut wins = 0usize;
    let mut sum_r = 0.0;
    let tick = 0.25;
    let slip = 1.0 * tick;
    let comm_rt_pts = 0.5;

    for day in days {
        if day.is_empty() {
            continue;
        }
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has = false;
        for b in day {
            if (6..9).contains(&b.h) {
                has = true;
                rh = rh.max(b.hi);
                rl = rl.min(b.lo);
            }
        }
        if !has || rh <= rl {
            continue;
        }
        let range = rh - rl;

        let mut sweep_idx = None;
        let mut side = Side::Short;
        for (k, b) in day.iter().enumerate() {
            if b.h < 9 {
                continue;
            }
            if b.hi >= rh {
                sweep_idx = Some(k);
                side = Side::Short;
                break;
            }
            if b.lo <= rl {
                sweep_idx = Some(k);
                side = Side::Long;
                break;
            }
        }
        let sidx = if let Some(v) = sweep_idx { v } else { continue };

        if cfg.use_bias_filter {
            let d5bias = resample_5m(day);
            if d5bias.len() >= 3 {
                let n = d5bias.len();
                let trend_up = d5bias[n - 1].c > d5bias[n - 3].c;
                if (side == Side::Short && trend_up) || (side == Side::Long && !trend_up) {
                    continue;
                }
            }
        }

        let d5 = resample_5m(day);
        let sweep_ts = day[sidx].ts;
        let s5 = if let Some(pos) = d5.iter().position(|b| b.ts >= sweep_ts) {
            pos
        } else {
            continue;
        };
        let mss5 = if let Some(v) = find_mss_5m(&d5, side, s5) {
            let body = (d5[v].c - d5[v].o).abs();
            let rg = (d5[v].hi - d5[v].lo).max(0.0001);
            if body / rg < cfg.min_disp_body {
                continue;
            }
            v
        } else {
            continue;
        };
        let mss_ts = d5[mss5].ts;
        let start1 = if let Some(v) = day.iter().position(|b| b.ts >= mss_ts) {
            v
        } else {
            continue;
        };

        let mut entry_idx = None;
        let mut ob_hi = 0.0;
        let mut ob_lo = 0.0;

        for k in (start1 + 2)..day.len() {
            if side == Side::Short {
                let upper = day[k - 2].lo;
                let lower = day[k].hi;
                if upper > lower {
                    if (upper - lower) / range * 100.0 > cfg.max_ifvg_pct {
                        continue;
                    }
                    let mid = lower + (upper - lower) * 0.5;
                    let mut mit = None;
                    for (m, b) in day.iter().enumerate().skip(k + 1) {
                        if b.hi >= mid {
                            mit = Some(m);
                            break;
                        }
                    }
                    if let Some(m) = mit {
                        let mut ob = None;
                        for t in (start1..=k).rev() {
                            if day[t].c > day[t].o {
                                ob = Some(t);
                                break;
                            }
                        }
                        if let Some(o) = ob {
                            ob_hi = day[o].hi;
                            ob_lo = day[o].lo;
                            let mut invalid = false;
                            for b in day.iter().skip(o + 1).take(m.saturating_sub(o + 1)) {
                                if b.c > ob_hi {
                                    invalid = true;
                                    break;
                                }
                            }
                            if invalid {
                                continue;
                            }
                            for (e, b) in day.iter().enumerate().skip(m) {
                                if b.h > cfg.entry_deadline_hour {
                                    break;
                                }
                                if b.hi >= ob_lo && b.lo <= ob_hi {
                                    entry_idx = Some(e + 1);
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                let lower = day[k - 2].hi;
                let upper = day[k].lo;
                if lower < upper {
                    if (upper - lower) / range * 100.0 > cfg.max_ifvg_pct {
                        continue;
                    }
                    let mid = upper - (upper - lower) * 0.5;
                    let mut mit = None;
                    for (m, b) in day.iter().enumerate().skip(k + 1) {
                        if b.lo <= mid {
                            mit = Some(m);
                            break;
                        }
                    }
                    if let Some(m) = mit {
                        let mut ob = None;
                        for t in (start1..=k).rev() {
                            if day[t].c < day[t].o {
                                ob = Some(t);
                                break;
                            }
                        }
                        if let Some(o) = ob {
                            ob_hi = day[o].hi;
                            ob_lo = day[o].lo;
                            let mut invalid = false;
                            for b in day.iter().skip(o + 1).take(m.saturating_sub(o + 1)) {
                                if b.c < ob_lo {
                                    invalid = true;
                                    break;
                                }
                            }
                            if invalid {
                                continue;
                            }
                            for (e, b) in day.iter().enumerate().skip(m) {
                                if b.h > cfg.entry_deadline_hour {
                                    break;
                                }
                                if b.hi >= ob_lo && b.lo <= ob_hi {
                                    entry_idx = Some(e + 1);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if entry_idx.is_some() {
                break;
            }
        }

        let eidx = if let Some(v) = entry_idx { v } else { continue };
        if eidx >= day.len() {
            continue;
        }
        let entry = if side == Side::Short {
            day[eidx].o - slip
        } else {
            day[eidx].o + slip
        };
        let stop_struct = if side == Side::Short {
            ob_hi + tick + slip
        } else {
            ob_lo - tick - slip
        };
        let stop_capped = if side == Side::Short {
            entry + range * cfg.stop_cap_pct
        } else {
            entry - range * cfg.stop_cap_pct
        };
        let stop = if side == Side::Short {
            stop_struct.min(stop_capped)
        } else {
            stop_struct.max(stop_capped)
        };
        let risk = (entry - stop).abs();
        if risk < tick {
            continue;
        }
        let tp1 = if side == Side::Short {
            entry - risk
        } else {
            entry + risk
        };
        let tp2 = if side == Side::Short {
            rl + slip
        } else {
            rh - slip
        };
        let cost_r = comm_rt_pts / risk;

        let mut got_tp1 = false;
        let mut done = false;
        for b in day.iter().skip(eidx) {
            if b.h >= cfg.time_stop_hour {
                sum_r += if got_tp1 { 0.4 - cost_r } else { -0.1 - cost_r };
                done = true;
                break;
            }
            let stop_hit = if side == Side::Short {
                b.hi >= stop
            } else {
                b.lo <= stop
            };
            if stop_hit {
                sum_r += if got_tp1 { 0.5 - cost_r } else { -1.0 - cost_r };
                done = true;
                break;
            }
            if !got_tp1 {
                let h1 = if side == Side::Short {
                    b.lo <= tp1
                } else {
                    b.hi >= tp1
                };
                if h1 {
                    got_tp1 = true;
                }
            }
            if got_tp1 {
                let h2 = if side == Side::Short {
                    b.lo <= tp2
                } else {
                    b.hi >= tp2
                };
                if h2 {
                    let rr2 = (tp2 - entry).abs() / risk;
                    sum_r += 0.5 + 0.5 * rr2 - cost_r;
                    wins += 1;
                    done = true;
                    break;
                }
            }
        }
        if !done {
            sum_r += if got_tp1 { 0.4 - cost_r } else { -0.1 - cost_r };
        }
        trades += 1;
    }

    (
        trades,
        wins,
        if trades > 0 {
            sum_r / trades as f64
        } else {
            0.0
        },
    )
}

fn main() {
    let data =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("load");
    let mut bars = Vec::new();
    for c in data {
        let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        bars.push(B1 {
            ts: c.open_time,
            h: dt.hour(),
            o: d2f(c.open.0),
            hi: d2f(c.high.0),
            lo: d2f(c.low.0),
            c: d2f(c.close.0),
        });
    }

    let mut days: Vec<Vec<B1>> = Vec::new();
    let mut i = 0usize;
    while i < bars.len() {
        let d = New_York
            .timestamp_opt(bars[i].ts, 0)
            .single()
            .expect("ts")
            .date_naive();
        let mut j = i;
        while j < bars.len()
            && New_York
                .timestamp_opt(bars[j].ts, 0)
                .single()
                .expect("ts")
                .date_naive()
                == d
        {
            j += 1;
        }
        days.push(bars[i..j].to_vec());
        i = j;
    }

    let disp = [0.45_f64, 0.55];
    let ifvg = [15.0_f64, 25.0, 35.0];
    let cap = [0.20_f64, 0.30, 0.40];
    let entry_deadline = [10_u32, 11];
    let tstop = [11_u32, 12];
    let bias = [false, true];

    let mut rows: Vec<(String, usize, f64, f64)> = Vec::new();
    for d in disp {
        for f in ifvg {
            for c in cap {
                for ed in entry_deadline {
                    for ts in tstop {
                        for b in bias {
                            let cfg = Cfg {
                                min_disp_body: d,
                                max_ifvg_pct: f,
                                stop_cap_pct: c,
                                entry_deadline_hour: ed,
                                time_stop_hour: ts,
                                use_bias_filter: b,
                            };
                            let (tr, w, exp) = run_cfg(&days, cfg);
                            if tr < 60 {
                                continue;
                            }
                            let wr = w as f64 / tr as f64 * 100.0;
                            rows.push((
                                format!(
                                    "disp>={:.2} ifvg<={:.0}% cap={:.0}% entry<= {}:59 tstop={} bias={}",
                                    d,
                                    f,
                                    c * 100.0,
                                    ed,
                                    ts,
                                    b
                                ),
                                tr,
                                wr,
                                exp,
                            ));
                        }
                    }
                }
            }
        }
    }

    rows.sort_by(|a, b| b.3.total_cmp(&a.3).then(b.2.total_cmp(&a.2)));
    println!("MNQ focused sweep: MSS(5m)+iFVG mitigation(1m)+OB retest");
    for r in rows.iter().take(20) {
        println!(
            "- {} | trades={} win_rate={:.2}% exp={:.3}R",
            r.0, r.1, r.2, r.3
        );
    }
}
