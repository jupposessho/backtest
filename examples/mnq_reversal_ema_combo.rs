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
    range_hi: f64,
    range_lo: f64,
}

#[derive(Clone, Copy)]
struct Signal {
    side: Side,
    entry_idx: usize,
    ob_hi: f64,
    ob_lo: f64,
}

#[derive(Clone, Copy)]
enum EmaGate {
    None,
    Trend5m,
    Entry1m,
    Both,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }
    let k = 2.0 / (period as f64 + 1.0);
    let mut out = Vec::with_capacity(values.len());
    let mut e = values[0];
    out.push(e);
    for v in values.iter().skip(1) {
        e = *v * k + e * (1.0 - k);
        out.push(e);
    }
    out
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
        out.push(B1 { ts: bucket, h: dt.hour(), o: s[0].o, hi, lo, c: s[s.len() - 1].c });
        i = j;
    }
    out
}

fn collect_days() -> Vec<Day> {
    let data = CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet")).expect("load");
    let mut all = Vec::new();
    for c in data {
        let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        all.push(B1 { ts: c.open_time, h: dt.hour(), o: d2f(c.open.0), hi: d2f(c.high.0), lo: d2f(c.low.0), c: d2f(c.close.0) });
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let d = New_York.timestamp_opt(all[i].ts, 0).single().expect("ts").date_naive();
        let mut j = i;
        while j < all.len() && New_York.timestamp_opt(all[j].ts, 0).single().expect("ts").date_naive() == d {
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
            out.push(Day { bars, range_hi: rh, range_lo: rl });
        }
        i = j;
    }
    out
}

fn find_mss_5m(v5: &[B1], side: Side, start: usize, min_disp: f64) -> Option<usize> {
    if v5.len() < 4 { return None; }
    let mut pivot = None;
    for i in (start + 1)..(v5.len() - 1) {
        let body = (v5[i].c - v5[i].o).abs();
        let range = (v5[i].hi - v5[i].lo).max(0.0001);
        if side == Side::Short {
            if v5[i].lo < v5[i - 1].lo && v5[i].lo < v5[i + 1].lo { pivot = Some(v5[i].lo); }
            if let Some(p) = pivot {
                if v5[i].c < p && body / range >= min_disp { return Some(i); }
            }
        } else {
            if v5[i].hi > v5[i - 1].hi && v5[i].hi > v5[i + 1].hi { pivot = Some(v5[i].hi); }
            if let Some(p) = pivot {
                if v5[i].c > p && body / range >= min_disp { return Some(i); }
            }
        }
    }
    None
}

fn ict_signal(day: &Day, min_disp: f64, max_ifvg_pct: f64, entry_deadline: u32) -> Option<Signal> {
    let range = day.range_hi - day.range_lo;
    let mut sweep = None;
    for (k, b) in day.bars.iter().enumerate() {
        if b.h < 9 { continue; }
        if b.hi >= day.range_hi { sweep = Some((k, Side::Short)); break; }
        if b.lo <= day.range_lo { sweep = Some((k, Side::Long)); break; }
    }
    let (sidx, side) = sweep?;
    let d5 = resample_5m(&day.bars);
    let s5 = d5.iter().position(|b| b.ts >= day.bars[sidx].ts)?;
    let mss5 = find_mss_5m(&d5, side, s5, min_disp)?;
    let mss_ts = d5[mss5].ts;
    let start1 = day.bars.iter().position(|b| b.ts >= mss_ts)?;

    let mut entry_idx = None;
    let mut ob_hi = 0.0;
    let mut ob_lo = 0.0;

    for k in (start1 + 2)..day.bars.len() {
        if side == Side::Short {
            let upper = day.bars[k - 2].lo;
            let lower = day.bars[k].hi;
            if upper <= lower { continue; }
            if (upper - lower) / range * 100.0 > max_ifvg_pct { continue; }
            let mid = lower + (upper - lower) * 0.5;
            let mut mit = None;
            for (m, b) in day.bars.iter().enumerate().skip(k + 1) {
                if b.hi >= mid { mit = Some(m); break; }
            }
            if let Some(m) = mit {
                let mut ob = None;
                for t in (start1..=k).rev() {
                    if day.bars[t].c > day.bars[t].o { ob = Some(t); break; }
                }
                let o = ob?;
                ob_hi = day.bars[o].hi;
                ob_lo = day.bars[o].lo;
                let mut invalid = false;
                for b in day.bars.iter().skip(o + 1).take(m.saturating_sub(o + 1)) {
                    if b.c > ob_hi { invalid = true; break; }
                }
                if invalid { continue; }
                for (e, b) in day.bars.iter().enumerate().skip(m) {
                    if b.h > entry_deadline { break; }
                    if b.hi >= ob_lo && b.lo <= ob_hi { entry_idx = Some(e + 1); break; }
                }
            }
        } else {
            let lower = day.bars[k - 2].hi;
            let upper = day.bars[k].lo;
            if lower >= upper { continue; }
            if (upper - lower) / range * 100.0 > max_ifvg_pct { continue; }
            let mid = upper - (upper - lower) * 0.5;
            let mut mit = None;
            for (m, b) in day.bars.iter().enumerate().skip(k + 1) {
                if b.lo <= mid { mit = Some(m); break; }
            }
            if let Some(m) = mit {
                let mut ob = None;
                for t in (start1..=k).rev() {
                    if day.bars[t].c < day.bars[t].o { ob = Some(t); break; }
                }
                let o = ob?;
                ob_hi = day.bars[o].hi;
                ob_lo = day.bars[o].lo;
                let mut invalid = false;
                for b in day.bars.iter().skip(o + 1).take(m.saturating_sub(o + 1)) {
                    if b.c < ob_lo { invalid = true; break; }
                }
                if invalid { continue; }
                for (e, b) in day.bars.iter().enumerate().skip(m) {
                    if b.h > entry_deadline { break; }
                    if b.hi >= ob_lo && b.lo <= ob_hi { entry_idx = Some(e + 1); break; }
                }
            }
        }
        if entry_idx.is_some() { break; }
    }

    let eidx = entry_idx?;
    if eidx >= day.bars.len() { return None; }
    Some(Signal { side, entry_idx: eidx, ob_hi, ob_lo })
}

fn ema_gate_pass(day: &Day, s: Signal, gate: EmaGate, ema_p: usize) -> bool {
    if matches!(gate, EmaGate::None) { return true; }
    let closes1: Vec<f64> = day.bars.iter().map(|b| b.c).collect();
    let ema1 = ema(&closes1, ema_p);
    let ok_entry = match s.side {
        Side::Short => day.bars[s.entry_idx].c <= ema1[s.entry_idx],
        Side::Long => day.bars[s.entry_idx].c >= ema1[s.entry_idx],
    };

    if matches!(gate, EmaGate::Entry1m) { return ok_entry; }

    let d5 = resample_5m(&day.bars);
    let closes5: Vec<f64> = d5.iter().map(|b| b.c).collect();
    let ema5 = ema(&closes5, ema_p);
    let t = day.bars[s.entry_idx].ts;
    let idx5 = d5.iter().position(|b| b.ts >= t).unwrap_or(d5.len().saturating_sub(1));
    let ok_trend = match s.side {
        Side::Short => d5[idx5].c <= ema5[idx5],
        Side::Long => d5[idx5].c >= ema5[idx5],
    };

    match gate {
        EmaGate::Trend5m => ok_trend,
        EmaGate::Both => ok_trend && ok_entry,
        EmaGate::None | EmaGate::Entry1m => true,
    }
}

fn execute(day: &Day, s: Signal, stop_cap_pct: f64, tstop: u32) -> Option<(f64, bool)> {
    let tick = 0.25;
    let slip = tick;
    let comm_rt_pts = 0.5;
    let range = day.range_hi - day.range_lo;
    let entry = if s.side == Side::Short { day.bars[s.entry_idx].o - slip } else { day.bars[s.entry_idx].o + slip };
    let stop_struct = if s.side == Side::Short { s.ob_hi + tick + slip } else { s.ob_lo - tick - slip };
    let stop_cap = if s.side == Side::Short { entry + stop_cap_pct * range } else { entry - stop_cap_pct * range };
    let stop = if s.side == Side::Short { stop_struct.min(stop_cap) } else { stop_struct.max(stop_cap) };
    let risk = (entry - stop).abs();
    if risk < tick { return None; }

    let tp1 = if s.side == Side::Short { entry - risk } else { entry + risk };
    let tp2 = if s.side == Side::Short { day.range_lo + slip } else { day.range_hi - slip };
    let cost_r = comm_rt_pts / risk;
    let mut got_tp1 = false;

    for b in day.bars.iter().skip(s.entry_idx) {
        if b.h >= tstop { return Some((if got_tp1 { 0.4 - cost_r } else { -0.1 - cost_r }, false)); }
        let stop_hit = if s.side == Side::Short { b.hi >= stop } else { b.lo <= stop };
        if stop_hit { return Some((if got_tp1 { 0.5 - cost_r } else { -1.0 - cost_r }, false)); }
        if !got_tp1 {
            let hit1 = if s.side == Side::Short { b.lo <= tp1 } else { b.hi >= tp1 };
            if hit1 { got_tp1 = true; }
        }
        if got_tp1 {
            let hit2 = if s.side == Side::Short { b.lo <= tp2 } else { b.hi >= tp2 };
            if hit2 {
                let rr2 = (tp2 - entry).abs() / risk;
                return Some((0.5 + 0.5 * rr2 - cost_r, true));
            }
        }
    }
    Some((if got_tp1 { 0.4 - cost_r } else { -0.1 - cost_r }, false))
}

fn main() {
    let days = collect_days();
    let split = days.len() * 70 / 100;
    let train = &days[..split];
    let test = &days[split..];

    let gates = [EmaGate::None, EmaGate::Trend5m, EmaGate::Entry1m, EmaGate::Both];
    let ema_ps = [20_usize, 50, 100];
    let stop_caps = [0.2_f64, 0.25, 0.3];
    let tstop = [11_u32, 12_u32];

    let mut rows: Vec<(String, usize, f64, f64, usize, f64, f64)> = Vec::new();

    for g in gates {
        for ep in ema_ps {
            for sc in stop_caps {
                for ts in tstop {
                    let mut tr_n = 0usize;
                    let mut tr_w = 0usize;
                    let mut tr_sum = 0.0;
                    for d in train {
                        if let Some(sig) = ict_signal(d, 0.55, 35.0, 10) {
                            if ema_gate_pass(d, sig, g, ep) {
                                if let Some((r, w)) = execute(d, sig, sc, ts) {
                                    tr_n += 1;
                                    tr_sum += r;
                                    if w {
                                        tr_w += 1;
                                    }
                                }
                            }
                        }
                    }
                    if tr_n < 80 { continue; }
                    let tr_wr = tr_w as f64 / tr_n as f64 * 100.0;
                    let tr_exp = tr_sum / tr_n as f64;

                    let mut te_n = 0usize;
                    let mut te_w = 0usize;
                    let mut te_sum = 0.0;
                    for d in test {
                        if let Some(sig) = ict_signal(d, 0.55, 35.0, 10) {
                            if ema_gate_pass(d, sig, g, ep) {
                                if let Some((r, w)) = execute(d, sig, sc, ts) {
                                    te_n += 1;
                                    te_sum += r;
                                    if w {
                                        te_w += 1;
                                    }
                                }
                            }
                        }
                    }
                    if te_n < 25 { continue; }
                    let te_wr = te_w as f64 / te_n as f64 * 100.0;
                    let te_exp = te_sum / te_n as f64;
                    let gname = match g {
                        EmaGate::None => "none",
                        EmaGate::Trend5m => "trend5m",
                        EmaGate::Entry1m => "entry1m",
                        EmaGate::Both => "both",
                    };
                    rows.push((
                        format!("gate={} ema={} stop_cap={:.0}% tstop={}", gname, ep, sc * 100.0, ts),
                        tr_n,
                        tr_wr,
                        tr_exp,
                        te_n,
                        te_wr,
                        te_exp,
                    ));
                }
            }
        }
    }

    rows.sort_by(|a, b| b.6.total_cmp(&a.6).then(b.3.total_cmp(&a.3)).then(b.4.cmp(&a.4)));
    println!("MNQ reversal + EMA combo (ICT core: MSS5+iFVG1+OB1)");
    println!("Top configs by OOS expectancy (70/30 split):");
    for r in rows.iter().take(20) {
        println!(
            "- {} | IS: trades={} wr={:.2}% exp={:.3}R | OOS: trades={} wr={:.2}% exp={:.3}R",
            r.0, r.1, r.2, r.3, r.4, r.5, r.6
        );
    }
}
