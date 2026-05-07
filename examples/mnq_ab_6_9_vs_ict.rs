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
struct EntrySignal {
    side: Side,
    entry_idx: usize,
    stop_anchor_hi: f64,
    stop_anchor_lo: f64,
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
        out.push(B1 { ts: bucket, h: dt.hour(), o: s[0].o, hi, lo, c: s[s.len() - 1].c });
        i = j;
    }
    out
}

fn find_mss_5m(v5: &[B1], side: Side, start: usize, min_disp: f64) -> Option<usize> {
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
                if v5[i].c < p && body / range >= min_disp {
                    return Some(i);
                }
            }
        } else {
            if v5[i].hi > v5[i - 1].hi && v5[i].hi > v5[i + 1].hi {
                pivot = Some(v5[i].hi);
            }
            if let Some(p) = pivot {
                if v5[i].c > p && body / range >= min_disp {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn collect_days() -> Vec<Day> {
    let data = CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet")).expect("load");
    let mut bars = Vec::new();
    for c in data {
        let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        bars.push(B1 { ts: c.open_time, h: dt.hour(), o: d2f(c.open.0), hi: d2f(c.high.0), lo: d2f(c.low.0), c: d2f(c.close.0) });
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bars.len() {
        let d = New_York.timestamp_opt(bars[i].ts, 0).single().expect("ts").date_naive();
        let mut j = i;
        while j < bars.len() && New_York.timestamp_opt(bars[j].ts, 0).single().expect("ts").date_naive() == d {
            j += 1;
        }
        let day_bars = bars[i..j].to_vec();
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has = false;
        for b in &day_bars {
            if (6..9).contains(&b.h) {
                has = true;
                rh = rh.max(b.hi);
                rl = rl.min(b.lo);
            }
        }
        if has && rh > rl {
            out.push(Day { bars: day_bars, range_hi: rh, range_lo: rl });
        }
        i = j;
    }
    out
}

fn signal_baseline(day: &Day) -> Option<EntrySignal> {
    let range = day.range_hi - day.range_lo;
    let mut first = None;
    for (k, b) in day.bars.iter().enumerate() {
        if b.h < 9 { continue; }
        if b.hi >= day.range_hi { first = Some((k, Side::Short)); break; }
        if b.lo <= day.range_lo { first = Some((k, Side::Long)); break; }
    }
    let (fidx, side) = first?;

    let mut break_extreme = if side == Side::Short { day.range_hi } else { day.range_lo };
    let mut reclaim = None;
    let mut overshoot: f64 = 0.0;
    for (off, b) in day.bars.iter().enumerate().skip(fidx) {
        if side == Side::Short {
            if b.hi > break_extreme { break_extreme = b.hi; }
            overshoot = overshoot.max((break_extreme - day.range_hi) / range * 100.0);
            if b.c <= day.range_hi { reclaim = Some(off); break; }
        } else {
            if b.lo < break_extreme { break_extreme = b.lo; }
            overshoot = overshoot.max((day.range_lo - break_extreme) / range * 100.0);
            if b.c >= day.range_lo { reclaim = Some(off); break; }
        }
    }
    let ridx = reclaim?;
    if ridx - fidx > 1 { return None; }
    if overshoot > 35.0 { return None; }
    if day.bars[ridx].h > 10 { return None; }
    let eidx = ridx + 1;
    if eidx >= day.bars.len() { return None; }

    Some(EntrySignal {
        side,
        entry_idx: eidx,
        stop_anchor_hi: if side == Side::Short { break_extreme } else { day.range_hi },
        stop_anchor_lo: if side == Side::Long { break_extreme } else { day.range_lo },
    })
}

fn signal_ict(day: &Day) -> Option<EntrySignal> {
    let range = day.range_hi - day.range_lo;
    let mut sweep = None;
    for (k, b) in day.bars.iter().enumerate() {
        if b.h < 9 { continue; }
        if b.hi >= day.range_hi { sweep = Some((k, Side::Short)); break; }
        if b.lo <= day.range_lo { sweep = Some((k, Side::Long)); break; }
    }
    let (sidx, side) = sweep?;

    let d5 = resample_5m(&day.bars);
    if d5.len() >= 3 {
        let n = d5.len();
        let trend_up = d5[n - 1].c > d5[n - 3].c;
        if (side == Side::Short && trend_up) || (side == Side::Long && !trend_up) {
            return None;
        }
    }
    let s5 = d5.iter().position(|b| b.ts >= day.bars[sidx].ts)?;
    let mss5 = find_mss_5m(&d5, side, s5, 0.55)?;
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
            if (upper - lower) / range * 100.0 > 35.0 { continue; }
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
                    if b.h > 10 { break; }
                    if b.hi >= ob_lo && b.lo <= ob_hi { entry_idx = Some(e + 1); break; }
                }
            }
        } else {
            let lower = day.bars[k - 2].hi;
            let upper = day.bars[k].lo;
            if lower >= upper { continue; }
            if (upper - lower) / range * 100.0 > 35.0 { continue; }
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
                    if b.h > 10 { break; }
                    if b.hi >= ob_lo && b.lo <= ob_hi { entry_idx = Some(e + 1); break; }
                }
            }
        }
        if entry_idx.is_some() { break; }
    }

    let eidx = entry_idx?;
    if eidx >= day.bars.len() { return None; }
    Some(EntrySignal { side, entry_idx: eidx, stop_anchor_hi: ob_hi, stop_anchor_lo: ob_lo })
}

fn execute(day: &Day, s: EntrySignal) -> Option<(f64, bool)> {
    let tick = 0.25;
    let slip = tick;
    let comm_rt_pts = 0.5;
    let range = day.range_hi - day.range_lo;
    let entry = if s.side == Side::Short { day.bars[s.entry_idx].o - slip } else { day.bars[s.entry_idx].o + slip };
    let stop_struct = if s.side == Side::Short { s.stop_anchor_hi + tick + slip } else { s.stop_anchor_lo - tick - slip };
    let stop_cap = if s.side == Side::Short { entry + 0.20 * range } else { entry - 0.20 * range };
    let stop = if s.side == Side::Short { stop_struct.min(stop_cap) } else { stop_struct.max(stop_cap) };
    let risk = (entry - stop).abs();
    if risk < tick { return None; }

    let tp1 = if s.side == Side::Short { entry - risk } else { entry + risk };
    let tp2 = if s.side == Side::Short { day.range_lo + slip } else { day.range_hi - slip };
    let cost_r = comm_rt_pts / risk;

    let mut got_tp1 = false;
    for b in day.bars.iter().skip(s.entry_idx) {
        if b.h >= 12 { return Some((if got_tp1 { 0.4 - cost_r } else { -0.1 - cost_r }, false)); }
        let stop_hit = if s.side == Side::Short { b.hi >= stop } else { b.lo <= stop };
        if stop_hit { return Some((if got_tp1 { 0.5 - cost_r } else { -1.0 - cost_r }, false)); }
        if !got_tp1 {
            let hit = if s.side == Side::Short { b.lo <= tp1 } else { b.hi >= tp1 };
            if hit { got_tp1 = true; }
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

fn eval(days: &[Day], signal_fn: fn(&Day) -> Option<EntrySignal>) -> (usize, f64, f64) {
    let mut trades = 0usize;
    let mut wins = 0usize;
    let mut sum = 0.0;
    for d in days {
        if let Some(sig) = signal_fn(d) {
            if let Some((r, win)) = execute(d, sig) {
                trades += 1;
                if win { wins += 1; }
                sum += r;
            }
        }
    }
    let wr = if trades > 0 { wins as f64 / trades as f64 * 100.0 } else { 0.0 };
    let exp = if trades > 0 { sum / trades as f64 } else { 0.0 };
    (trades, wr, exp)
}

fn main() {
    let days = collect_days();
    let split = days.len() * 70 / 100;
    let train = &days[..split];
    let test = &days[split..];

    let (bt, bwr, bexp) = eval(train, signal_baseline);
    let (it, iwr, iexp) = eval(train, signal_ict);
    let (bt2, bwr2, bexp2) = eval(test, signal_baseline);
    let (it2, iwr2, iexp2) = eval(test, signal_ict);

    println!("A/B with identical execution (70/30 chronological split)");
    println!("Execution: slip=1 tick, comm_rt=0.5 pts, stop_cap=20% range, TP1 50%@1R, TP2 50%@opp range, flat>=12:00");
    println!("IN-SAMPLE 70%:");
    println!("- baseline(6-9 reclaim): trades={} win_rate={:.2}% exp={:.3}R", bt, bwr, bexp);
    println!("- ict(MSS5+iFVG1+OB1): trades={} win_rate={:.2}% exp={:.3}R", it, iwr, iexp);
    println!("OUT-OF-SAMPLE 30%:");
    println!("- baseline(6-9 reclaim): trades={} win_rate={:.2}% exp={:.3}R", bt2, bwr2, bexp2);
    println!("- ict(MSS5+iFVG1+OB1): trades={} win_rate={:.2}% exp={:.3}R", it2, iwr2, iexp2);
}
