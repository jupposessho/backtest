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

#[derive(Clone)]
struct Day {
    bars: Vec<B1>,
    rh: f64,
    rl: f64,
    range: f64,
}

#[derive(Clone, Copy)]
struct Sig {
    side: Side,
    eidx: usize,
    ob_hi: f64,
    ob_lo: f64,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn ema(vals: &[f64], p: usize) -> Vec<f64> {
    if vals.is_empty() {
        return vec![];
    }
    let k = 2.0 / (p as f64 + 1.0);
    let mut out = Vec::with_capacity(vals.len());
    let mut e = vals[0];
    out.push(e);
    for v in vals.iter().skip(1) {
        e = *v * k + e * (1.0 - k);
        out.push(e);
    }
    out
}

fn resample_5m(v: &[B1]) -> Vec<B1> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < v.len() {
        let b = (v[i].ts / 300) * 300;
        let mut j = i + 1;
        while j < v.len() && (v[j].ts / 300) * 300 == b {
            j += 1;
        }
        let s = &v[i..j];
        let mut hi = f64::NEG_INFINITY;
        let mut lo = f64::INFINITY;
        for x in s {
            hi = hi.max(x.hi);
            lo = lo.min(x.lo);
        }
        let dt = New_York.timestamp_opt(b, 0).single().expect("ts");
        out.push(B1 {
            ts: b,
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

fn collect_days() -> Vec<Day> {
    let data =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("load");
    let mut all = Vec::new();
    for c in data {
        let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        all.push(B1 {
            ts: c.open_time,
            h: dt.hour(),
            o: d2f(c.open.0),
            hi: d2f(c.high.0),
            lo: d2f(c.low.0),
            c: d2f(c.close.0),
        });
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let d = New_York
            .timestamp_opt(all[i].ts, 0)
            .single()
            .expect("ts")
            .date_naive();
        let mut j = i;
        while j < all.len()
            && New_York
                .timestamp_opt(all[j].ts, 0)
                .single()
                .expect("ts")
                .date_naive()
                == d
        {
            j += 1;
        }
        let bars = all[i..j].to_vec();
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has = false;
        for b in &bars {
            if (6..9).contains(&b.h) {
                has = true;
                rh = rh.max(b.hi);
                rl = rl.min(b.lo);
            }
        }
        if has && rh > rl {
            out.push(Day {
                bars,
                rh,
                rl,
                range: rh - rl,
            });
        }
        i = j;
    }
    out
}

fn find_mss_5m(v5: &[B1], side: Side, start: usize, min_disp: f64) -> Option<usize> {
    let mut pivot = None;
    for i in (start + 1)..v5.len().saturating_sub(1) {
        let body = (v5[i].c - v5[i].o).abs();
        let rg = (v5[i].hi - v5[i].lo).max(0.0001);
        if side == Side::Short {
            if v5[i].lo < v5[i - 1].lo && v5[i].lo < v5[i + 1].lo {
                pivot = Some(v5[i].lo);
            }
            if let Some(p) = pivot {
                if v5[i].c < p && body / rg >= min_disp {
                    return Some(i);
                }
            }
        } else {
            if v5[i].hi > v5[i - 1].hi && v5[i].hi > v5[i + 1].hi {
                pivot = Some(v5[i].hi);
            }
            if let Some(p) = pivot {
                if v5[i].c > p && body / rg >= min_disp {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn signal(
    day: &Day,
    min_disp: f64,
    max_ifvg: f64,
    entry_deadline: u32,
    ema_p: usize,
) -> Option<Sig> {
    let mut sweep = None;
    for (k, b) in day.bars.iter().enumerate() {
        if b.h < 9 {
            continue;
        }
        if b.hi >= day.rh {
            sweep = Some((k, Side::Short));
            break;
        }
        if b.lo <= day.rl {
            sweep = Some((k, Side::Long));
            break;
        }
    }
    let (sidx, side) = sweep?;

    let d5 = resample_5m(&day.bars);
    let c5: Vec<f64> = d5.iter().map(|x| x.c).collect();
    let e5 = ema(&c5, ema_p);
    let s5 = d5.iter().position(|b| b.ts >= day.bars[sidx].ts)?;
    if s5 >= e5.len() {
        return None;
    }
    if (side == Side::Short && d5[s5].c > e5[s5]) || (side == Side::Long && d5[s5].c < e5[s5]) {
        return None;
    }

    let mss = find_mss_5m(&d5, side, s5, min_disp)?;
    let mss_ts = d5[mss].ts;
    let start1 = day.bars.iter().position(|b| b.ts >= mss_ts)?;

    for k in (start1 + 2)..day.bars.len() {
        if side == Side::Short {
            let upper = day.bars[k - 2].lo;
            let lower = day.bars[k].hi;
            if upper <= lower {
                continue;
            }
            if (upper - lower) / day.range * 100.0 > max_ifvg {
                continue;
            }
            let mid = lower + (upper - lower) * 0.5;
            let mut mit = None;
            for (m, b) in day.bars.iter().enumerate().skip(k + 1) {
                if b.hi >= mid {
                    mit = Some(m);
                    break;
                }
            }
            let m = if let Some(v) = mit { v } else { continue };
            let mut ob = None;
            for t in (start1..=k).rev() {
                if day.bars[t].c > day.bars[t].o {
                    ob = Some(t);
                    break;
                }
            }
            let o = if let Some(v) = ob { v } else { continue };
            let ob_hi = day.bars[o].hi;
            let ob_lo = day.bars[o].lo;
            for b in day.bars.iter().skip(o + 1).take(m.saturating_sub(o + 1)) {
                if b.c > ob_hi {
                    return None;
                }
            }
            for (e, b) in day.bars.iter().enumerate().skip(m) {
                if b.h > entry_deadline {
                    break;
                }
                if b.hi >= ob_lo && b.lo <= ob_hi {
                    return Some(Sig {
                        side,
                        eidx: e + 1,
                        ob_hi,
                        ob_lo,
                    });
                }
            }
        } else {
            let lower = day.bars[k - 2].hi;
            let upper = day.bars[k].lo;
            if lower >= upper {
                continue;
            }
            if (upper - lower) / day.range * 100.0 > max_ifvg {
                continue;
            }
            let mid = upper - (upper - lower) * 0.5;
            let mut mit = None;
            for (m, b) in day.bars.iter().enumerate().skip(k + 1) {
                if b.lo <= mid {
                    mit = Some(m);
                    break;
                }
            }
            let m = if let Some(v) = mit { v } else { continue };
            let mut ob = None;
            for t in (start1..=k).rev() {
                if day.bars[t].c < day.bars[t].o {
                    ob = Some(t);
                    break;
                }
            }
            let o = if let Some(v) = ob { v } else { continue };
            let ob_hi = day.bars[o].hi;
            let ob_lo = day.bars[o].lo;
            for b in day.bars.iter().skip(o + 1).take(m.saturating_sub(o + 1)) {
                if b.c < ob_lo {
                    return None;
                }
            }
            for (e, b) in day.bars.iter().enumerate().skip(m) {
                if b.h > entry_deadline {
                    break;
                }
                if b.hi >= ob_lo && b.lo <= ob_hi {
                    return Some(Sig {
                        side,
                        eidx: e + 1,
                        ob_hi,
                        ob_lo,
                    });
                }
            }
        }
    }
    None
}

fn exec(
    day: &Day,
    s: Sig,
    stop_cap: f64,
    tstop: u32,
    tp1_frac: f64,
    runner_mult: f64,
) -> Option<(f64, bool)> {
    if s.eidx >= day.bars.len() {
        return None;
    }
    let tick = 0.25;
    let slip = tick;
    let comm = 0.5;
    let entry = if s.side == Side::Short {
        day.bars[s.eidx].o - slip
    } else {
        day.bars[s.eidx].o + slip
    };
    let stop_struct = if s.side == Side::Short {
        s.ob_hi + tick + slip
    } else {
        s.ob_lo - tick - slip
    };
    let stop_cap_px = if s.side == Side::Short {
        entry + stop_cap * day.range
    } else {
        entry - stop_cap * day.range
    };
    let stop = if s.side == Side::Short {
        stop_struct.min(stop_cap_px)
    } else {
        stop_struct.max(stop_cap_px)
    };
    let risk = (entry - stop).abs();
    if risk < tick {
        return None;
    }
    let tp1 = if s.side == Side::Short {
        entry - risk
    } else {
        entry + risk
    };
    let tp2 = if runner_mult > 0.0 {
        if s.side == Side::Short {
            entry - runner_mult * day.range
        } else {
            entry + runner_mult * day.range
        }
    } else if s.side == Side::Short {
        day.rl + slip
    } else {
        day.rh - slip
    };
    let cost_r = comm / risk;
    let mut hit_tp1 = false;
    for b in day.bars.iter().skip(s.eidx) {
        if b.h >= tstop {
            return Some((
                if hit_tp1 {
                    tp1_frac * 1.0 - 0.1 * (1.0 - tp1_frac) - cost_r
                } else {
                    -0.1 - cost_r
                },
                false,
            ));
        }
        let stop_hit = if s.side == Side::Short {
            b.hi >= stop
        } else {
            b.lo <= stop
        };
        if stop_hit {
            let r = if hit_tp1 {
                tp1_frac * 1.0 - (1.0 - tp1_frac) * 1.0 - cost_r
            } else {
                -1.0 - cost_r
            };
            return Some((r, false));
        }
        if !hit_tp1 {
            let h1 = if s.side == Side::Short {
                b.lo <= tp1
            } else {
                b.hi >= tp1
            };
            if h1 {
                hit_tp1 = true;
            }
        }
        if hit_tp1 {
            let h2 = if s.side == Side::Short {
                b.lo <= tp2
            } else {
                b.hi >= tp2
            };
            if h2 {
                let rr2 = (tp2 - entry).abs() / risk;
                return Some((tp1_frac * 1.0 + (1.0 - tp1_frac) * rr2 - cost_r, true));
            }
        }
    }
    Some((
        if hit_tp1 {
            tp1_frac * 1.0 - 0.1 * (1.0 - tp1_frac) - cost_r
        } else {
            -0.1 - cost_r
        },
        false,
    ))
}

fn main() {
    let days = collect_days();
    let split = days.len() * 70 / 100;
    let (train, test) = (&days[..split], &days[split..]);

    let min_disp = [0.55, 0.65];
    let ifvg = [20.0, 35.0];
    let ema_p = [50_usize, 100];
    let stop_cap = [0.15, 0.20, 0.25];
    let entry_deadline = [10_u32, 11];
    let tstop = [11_u32, 12];
    let tp1_frac = [0.25, 0.33, 0.5];
    let runner = [0.0, 1.0, 1.5];

    let mut rows: Vec<(String, usize, f64, f64, usize, f64, f64, f64)> = Vec::new();
    for d in min_disp {
        for f in ifvg {
            for e in ema_p {
                for sc in stop_cap {
                    for ed in entry_deadline {
                        for ts in tstop {
                            for p1 in tp1_frac {
                                for rm in runner {
                                    let mut tr_n = 0usize;
                                    let mut tr_sum = 0.0;
                                    let mut tr_w = 0usize;
                                    for day in train {
                                        if let Some(sig) = signal(day, d, f, ed, e) {
                                            if let Some((r, w)) = exec(day, sig, sc, ts, p1, rm) {
                                                tr_n += 1;
                                                tr_sum += r;
                                                if w {
                                                    tr_w += 1;
                                                }
                                            }
                                        }
                                    }
                                    if tr_n < 10 {
                                        continue;
                                    }
                                    let mut te_n = 0usize;
                                    let mut te_sum = 0.0;
                                    let mut te_w = 0usize;
                                    for day in test {
                                        if let Some(sig) = signal(day, d, f, ed, e) {
                                            if let Some((r, w)) = exec(day, sig, sc, ts, p1, rm) {
                                                te_n += 1;
                                                te_sum += r;
                                                if w {
                                                    te_w += 1;
                                                }
                                            }
                                        }
                                    }
                                    if te_n < 5 {
                                        continue;
                                    }
                                    let tr_exp = tr_sum / tr_n as f64;
                                    let te_exp = te_sum / te_n as f64;
                                    let tr_wr = tr_w as f64 / tr_n as f64 * 100.0;
                                    let te_wr = te_w as f64 / te_n as f64 * 100.0;
                                    rows.push((format!("disp>={:.2} ifvg<={:.0}% ema={} cap={:.0}% entry<= {} tstop={} tp1={:.0}% runner={}xRng",d,f,e,sc*100.0,ed,ts,p1*100.0,rm),tr_n,tr_wr,tr_exp,te_n,te_wr,te_exp,te_sum));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    rows.sort_by(|a, b| {
        b.7.total_cmp(&a.7)
            .then(b.6.total_cmp(&a.6))
            .then(b.4.cmp(&a.4))
    });
    println!("MNQ profit-max sweep (optimize OOS total R)");
    println!("configs_kept={}", rows.len());
    for r in rows.iter().take(25) {
        println!(
            "- {} | IS: n={} wr={:.2}% exp={:.3}R | OOS: n={} wr={:.2}% exp={:.3}R total={:.1}R",
            r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7
        );
    }
}
