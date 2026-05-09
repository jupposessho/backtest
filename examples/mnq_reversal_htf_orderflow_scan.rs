use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;
use rayon::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    HighSweep,
    LowSweep,
}

#[derive(Clone)]
struct MiniCandle {
    ts: i64,
    hour: u32,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[derive(Clone)]
struct DayData {
    range_high: f64,
    range_low: f64,
    sweep_side: Side,
    sweep_ts: i64,
    bars_1m: Vec<MiniCandle>,
}

#[derive(Clone, Copy)]
enum Pattern {
    CisdBodyFlip,
    CisdStrictWickBreak,
    CisdLastSeriesCloseBreak,
    Ifvg,
    Ob,
}

#[derive(Clone, Copy)]
enum BiasMode {
    None,
    Ema200,
    Structure,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn resample(bars: &[MiniCandle], minutes: i64) -> Vec<MiniCandle> {
    if bars.is_empty() {
        return vec![];
    }
    let bucket = minutes * 60;
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bars.len() {
        let start = (bars[i].ts / bucket) * bucket;
        let mut j = i + 1;
        while j < bars.len() && (bars[j].ts / bucket) * bucket == start {
            j += 1;
        }
        let slice = &bars[i..j];
        let open = slice[0].open;
        let close = slice[slice.len() - 1].close;
        let mut high = f64::NEG_INFINITY;
        let mut low = f64::INFINITY;
        for b in slice {
            high = high.max(b.high);
            low = low.min(b.low);
        }
        let dt = New_York.timestamp_opt(start, 0).single().expect("ts");
        out.push(MiniCandle {
            ts: start,
            hour: dt.hour(),
            open,
            high,
            low,
            close,
        });
        i = j;
    }
    out
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

fn collect_days(candles: &[CandleStick]) -> Vec<DayData> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < candles.len() {
        let d = New_York
            .timestamp_opt(candles[i].open_time, 0)
            .single()
            .expect("ts")
            .date_naive();
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

        let mut bars = Vec::new();
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has_range = false;
        for c in &candles[i..j] {
            let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
            let b = MiniCandle {
                ts: c.open_time,
                hour: dt.hour(),
                open: d2f(c.open.0),
                high: d2f(c.high.0),
                low: d2f(c.low.0),
                close: d2f(c.close.0),
            };
            if (6..9).contains(&b.hour) {
                has_range = true;
                rh = rh.max(b.high);
                rl = rl.min(b.low);
            }
            bars.push(b);
        }

        if !has_range || rh <= rl {
            i = j;
            continue;
        }

        let mut sweep = None;
        let mut invalid_breakout_close = false;
        for b in &bars {
            if b.hour < 9 {
                continue;
            }
            if b.high >= rh {
                if b.close <= rh {
                    sweep = Some((Side::HighSweep, b.ts));
                    break;
                }
                invalid_breakout_close = true;
                break;
            }
            if b.low <= rl {
                if b.close >= rl {
                    sweep = Some((Side::LowSweep, b.ts));
                    break;
                }
                invalid_breakout_close = true;
                break;
            }
        }

        if invalid_breakout_close {
            i = j;
            continue;
        }
        if let Some((side, ts)) = sweep {
            out.push(DayData {
                range_high: rh,
                range_low: rl,
                sweep_side: side,
                sweep_ts: ts,
                bars_1m: bars,
            });
        }
        i = j;
    }
    out
}

fn find_cisd_body_flip_idx(bars: &[MiniCandle], side: Side, start_idx: usize) -> Option<usize> {
    for i in (start_idx + 1)..bars.len() {
        if side == Side::HighSweep {
            if bars[i - 1].close > bars[i - 1].open
                && bars[i].close < bars[i].open
                && bars[i].close < bars[i - 1].close
            {
                return Some(i);
            }
        } else if bars[i - 1].close < bars[i - 1].open
            && bars[i].close > bars[i].open
            && bars[i].close > bars[i - 1].close
        {
            return Some(i);
        }
    }
    None
}

fn find_cisd_strict_wick_break_idx(
    bars: &[MiniCandle],
    side: Side,
    start_idx: usize,
) -> Option<usize> {
    for i in (start_idx + 1)..bars.len() {
        if side == Side::HighSweep {
            if bars[i].close < bars[i - 1].low {
                return Some(i);
            }
        } else if bars[i].close > bars[i - 1].high {
            return Some(i);
        }
    }
    None
}

fn find_cisd_last_series_close_break_idx(
    bars: &[MiniCandle],
    side: Side,
    start_idx: usize,
) -> Option<usize> {
    if start_idx + 3 >= bars.len() {
        return None;
    }
    let mut ref_close = bars[start_idx].close;
    for i in (start_idx + 1)..bars.len() {
        if side == Side::HighSweep {
            if bars[i - 1].close > bars[i - 1].open {
                ref_close = bars[i - 1].close;
            }
            if bars[i].close < ref_close {
                return Some(i);
            }
        } else {
            if bars[i - 1].close < bars[i - 1].open {
                ref_close = bars[i - 1].close;
            }
            if bars[i].close > ref_close {
                return Some(i);
            }
        }
    }
    None
}

fn find_ifvg_idx(bars: &[MiniCandle], side: Side, start_idx: usize) -> Option<usize> {
    for i in (start_idx + 2)..bars.len() {
        if side == Side::HighSweep {
            let upper = bars[i - 2].low;
            let lower = bars[i].high;
            if upper > lower {
                for (k, b) in bars.iter().enumerate().skip(i + 1) {
                    if b.high >= lower + 0.5 * (upper - lower) {
                        return Some(k);
                    }
                }
            }
        } else {
            let lower = bars[i - 2].high;
            let upper = bars[i].low;
            if lower < upper {
                for (k, b) in bars.iter().enumerate().skip(i + 1) {
                    if b.low <= upper - 0.5 * (upper - lower) {
                        return Some(k);
                    }
                }
            }
        }
    }
    None
}

fn find_ob_idx(bars: &[MiniCandle], side: Side, start_idx: usize) -> Option<usize> {
    if start_idx == 0 {
        return None;
    }
    for i in (1..=start_idx).rev() {
        if side == Side::HighSweep {
            if bars[i].close > bars[i].open && bars[start_idx].close < bars[i].low {
                return Some(i);
            }
        } else if bars[i].close < bars[i].open && bars[start_idx].close > bars[i].high {
            return Some(i);
        }
    }
    None
}

fn pattern_entry_idx(
    bars: &[MiniCandle],
    side: Side,
    p: Pattern,
    sweep_idx: usize,
) -> Option<usize> {
    match p {
        Pattern::CisdBodyFlip => find_cisd_body_flip_idx(bars, side, sweep_idx),
        Pattern::CisdStrictWickBreak => find_cisd_strict_wick_break_idx(bars, side, sweep_idx),
        Pattern::CisdLastSeriesCloseBreak => {
            find_cisd_last_series_close_break_idx(bars, side, sweep_idx)
        }
        Pattern::Ifvg => find_ifvg_idx(bars, side, sweep_idx),
        Pattern::Ob => find_ob_idx(bars, side, sweep_idx),
    }
}

fn htf_bias(htf: &[MiniCandle], sweep_ts: i64, mode: BiasMode) -> Option<bool> {
    if matches!(mode, BiasMode::None) {
        return Some(true);
    }
    let idx = htf.iter().position(|b| b.ts >= sweep_ts)?;
    match mode {
        BiasMode::None => Some(true),
        BiasMode::Ema200 => {
            if idx < 210 {
                return None;
            }
            let closes: Vec<f64> = htf.iter().map(|b| b.close).collect();
            let e = ema(&closes, 200);
            if idx == 0 {
                return None;
            }
            let bull = htf[idx].close > e[idx] && e[idx] > e[idx - 1];
            let bear = htf[idx].close < e[idx] && e[idx] < e[idx - 1];
            if bull {
                Some(true)
            } else if bear {
                Some(false)
            } else {
                None
            }
        }
        BiasMode::Structure => {
            if idx < 10 {
                return None;
            }
            let upto = &htf[..=idx];
            let mut swing_highs: Vec<f64> = Vec::new();
            let mut swing_lows: Vec<f64> = Vec::new();
            for i in 1..upto.len().saturating_sub(1) {
                if upto[i].high > upto[i - 1].high && upto[i].high > upto[i + 1].high {
                    swing_highs.push(upto[i].high);
                }
                if upto[i].low < upto[i - 1].low && upto[i].low < upto[i + 1].low {
                    swing_lows.push(upto[i].low);
                }
            }
            if swing_highs.len() < 2 || swing_lows.len() < 2 {
                return None;
            }
            let hh = swing_highs[swing_highs.len() - 1] > swing_highs[swing_highs.len() - 2];
            let hl = swing_lows[swing_lows.len() - 1] > swing_lows[swing_lows.len() - 2];
            let lh = swing_highs[swing_highs.len() - 1] < swing_highs[swing_highs.len() - 2];
            let ll = swing_lows[swing_lows.len() - 1] < swing_lows[swing_lows.len() - 2];
            if hh && hl {
                Some(true)
            } else if lh && ll {
                Some(false)
            } else {
                None
            }
        }
    }
}

fn simulate(
    day: &DayData,
    tf: i64,
    p: Pattern,
    rr: f64,
    tstop_hour: u32,
    slip_ticks: i32,
    comm_rt_pts: f64,
    bias_mode: BiasMode,
    htf_minutes: i64,
) -> Option<f64> {
    let tick = 0.25;
    let slip = slip_ticks as f64 * tick;
    let bars = if tf == 1 {
        day.bars_1m.clone()
    } else {
        resample(&day.bars_1m, tf)
    };
    if bars.len() < 10 {
        return None;
    }

    let bias_ok = if matches!(bias_mode, BiasMode::None) {
        true
    } else {
        let htf = resample(&day.bars_1m, htf_minutes);
        let bias = htf_bias(&htf, day.sweep_ts, bias_mode)?;
        match day.sweep_side {
            Side::LowSweep => bias,
            Side::HighSweep => !bias,
        }
    };
    if !bias_ok {
        return None;
    }

    let sweep_idx = bars.iter().position(|b| b.ts >= day.sweep_ts)?;
    let signal_idx = pattern_entry_idx(&bars, day.sweep_side, p, sweep_idx)?;
    let entry_idx = signal_idx + 1;
    if entry_idx >= bars.len() {
        return None;
    }

    let entry_raw = bars[entry_idx].open;
    let entry = if day.sweep_side == Side::HighSweep {
        entry_raw - slip
    } else {
        entry_raw + slip
    };
    let stop = if day.sweep_side == Side::HighSweep {
        day.range_high + tick + slip
    } else {
        day.range_low - tick - slip
    };
    let risk = (entry - stop).abs();
    if risk < tick {
        return None;
    }
    let target = if day.sweep_side == Side::HighSweep {
        entry - rr * risk
    } else {
        entry + rr * risk
    };
    let cost_r = comm_rt_pts / risk;

    for b in &bars[entry_idx..] {
        if b.hour >= tstop_hour {
            return Some(-0.1 - cost_r);
        }
        let stop_hit = if day.sweep_side == Side::HighSweep {
            b.high >= stop
        } else {
            b.low <= stop
        };
        let tp_hit = if day.sweep_side == Side::HighSweep {
            b.low <= target
        } else {
            b.high >= target
        };
        if stop_hit && tp_hit {
            return Some(-1.0 - cost_r);
        }
        if stop_hit {
            return Some(-1.0 - cost_r);
        }
        if tp_hit {
            return Some(rr - cost_r);
        }
    }
    Some(-0.1 - cost_r)
}

fn pattern_name(p: Pattern) -> &'static str {
    match p {
        Pattern::CisdBodyFlip => "cisd_bodyflip",
        Pattern::CisdStrictWickBreak => "cisd_strictwick",
        Pattern::CisdLastSeriesCloseBreak => "cisd_lastclose",
        Pattern::Ifvg => "ifvg",
        Pattern::Ob => "ob",
    }
}

fn bias_name(b: BiasMode) -> &'static str {
    match b {
        BiasMode::None => "baseline",
        BiasMode::Ema200 => "htf_ema200",
        BiasMode::Structure => "htf_structure",
    }
}

fn main() {
    let candles =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("load parquet");
    let days = Arc::new(collect_days(&candles));

    let tfs = [15_i64, 5, 3, 1];
    let patterns = [
        Pattern::CisdBodyFlip,
        Pattern::CisdStrictWickBreak,
        Pattern::CisdLastSeriesCloseBreak,
        Pattern::Ifvg,
        Pattern::Ob,
    ];
    let rrs = [1.0_f64, 1.5, 2.0];
    let t_stops = [11_u32, 12_u32];
    let slips = [0_i32, 1_i32];
    let comms = [0.25_f64, 0.5_f64];
    let bias_modes = [BiasMode::None, BiasMode::Ema200, BiasMode::Structure];
    let htf_minutes = 60_i64;

    let mut jobs = Vec::new();
    for tf in tfs {
        for p in patterns {
            for rr in rrs {
                for ts in t_stops {
                    for slip in slips {
                        for comm in comms {
                            for bias in bias_modes {
                                jobs.push((tf, p, rr, ts, slip, comm, bias));
                            }
                        }
                    }
                }
            }
        }
    }

    let mut rows: Vec<(String, usize, f64, f64)> = jobs
        .par_iter()
        .filter_map(|(tf, p, rr, ts, slip, comm, bias)| {
            let mut vals = Vec::new();
            let mut wins = 0usize;
            for d in days.iter() {
                if let Some(r) = simulate(d, *tf, *p, *rr, *ts, *slip, *comm, *bias, htf_minutes) {
                    if r > 0.0 {
                        wins += 1;
                    }
                    vals.push(r);
                }
            }
            if vals.len() < 30 {
                return None;
            }
            let exp = vals.iter().sum::<f64>() / vals.len() as f64;
            let wr = wins as f64 / vals.len() as f64 * 100.0;
            Some((
                format!(
                    "tf={}m pattern={} bias={} rr={:.1} tstop={} slip={} comm={:.2}",
                    tf,
                    pattern_name(*p),
                    bias_name(*bias),
                    rr,
                    ts,
                    slip,
                    comm
                ),
                vals.len(),
                wr,
                exp,
            ))
        })
        .collect();

    rows.sort_by(|a, b| b.3.total_cmp(&a.3).then(b.2.total_cmp(&a.2)));

    println!("MNQ reversal scan with wick-only sweep + HTF orderflow gating");
    println!("Days: {}", days.len());
    println!("HTF bias timeframe: {}m", htf_minutes);
    println!("\nBest by pattern and bias mode:");
    for p in patterns {
        println!("- pattern={}", pattern_name(p));
        for bias in bias_modes {
            if let Some(r) = rows.iter().find(|r| {
                r.0.contains(&format!("pattern={} ", pattern_name(p)))
                    && r.0.contains(&format!("bias={} ", bias_name(bias)))
            }) {
                println!("  {} | trades={} wr={:.2}% exp={:.3}R", r.0, r.1, r.2, r.3);
            }
        }
    }

    println!("\nTop results:");
    for r in rows.iter().take(30) {
        println!(
            "- {} | trades={} win_rate={:.2}% exp={:.3}R",
            r.0, r.1, r.2, r.3
        );
    }
}
