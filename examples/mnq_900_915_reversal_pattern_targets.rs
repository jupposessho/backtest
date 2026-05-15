use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    HighSweep,
    LowSweep,
}

#[derive(Clone, Copy)]
struct Band {
    name: &'static str,
    lo: f64,
    hi: f64,
}

#[derive(Clone, Copy)]
struct MiniCandle {
    ts: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[derive(Clone)]
struct DayData {
    range_high: f64,
    range_low: f64,
    range: f64,
    bars_1m: Vec<MiniCandle>,
}

#[derive(Default, Clone, Copy)]
struct Stats {
    touches: usize,
    pattern_found: usize,
    target_hit: usize,
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
        out.push(MiniCandle {
            ts: start,
            open,
            high,
            low,
            close,
        });
        i = j;
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
                open: d2f(c.open.0),
                high: d2f(c.high.0),
                low: d2f(c.low.0),
                close: d2f(c.close.0),
            };
            if dt.hour() == 9 && dt.minute() < 15 {
                has_range = true;
                rh = rh.max(b.high);
                rl = rl.min(b.low);
            }
            bars.push(b);
        }

        if has_range && rh > rl {
            out.push(DayData {
                range_high: rh,
                range_low: rl,
                range: rh - rl,
                bars_1m: bars,
            });
        }
        i = j;
    }
    out
}

fn find_ifvg_idx(bars: &[MiniCandle], side: Side, start_idx: usize) -> Option<usize> {
    if start_idx + 2 >= bars.len() {
        return None;
    }
    for i in (start_idx + 2)..bars.len() {
        if side == Side::HighSweep {
            if bars[i].high < bars[i - 2].low {
                return Some(i);
            }
        } else if bars[i].low > bars[i - 2].high {
            return Some(i);
        }
    }
    None
}

fn find_ob_idx(bars: &[MiniCandle], side: Side, start_idx: usize) -> Option<usize> {
    if start_idx + 2 >= bars.len() {
        return None;
    }
    for i in (start_idx + 1)..bars.len() {
        if side == Side::HighSweep {
            let prev_bull = bars[i - 1].close > bars[i - 1].open;
            let curr_bear = bars[i].close < bars[i].open;
            if prev_bull && curr_bear && bars[i].close < bars[i - 1].low {
                return Some(i);
            }
        } else {
            let prev_bear = bars[i - 1].close < bars[i - 1].open;
            let curr_bull = bars[i].close > bars[i].open;
            if prev_bear && curr_bull && bars[i].close > bars[i - 1].high {
                return Some(i);
            }
        }
    }
    None
}

fn first_touch_idx(day: &DayData, ext: Band) -> Option<(usize, Side)> {
    for (i, b) in day.bars_1m.iter().enumerate() {
        let dt = New_York.timestamp_opt(b.ts, 0).single().expect("ts");
        if dt.hour() < 9 || (dt.hour() == 9 && dt.minute() < 15) {
            continue;
        }
        let top_lo = day.range_high + ext.lo * day.range;
        let top_hi = day.range_high + ext.hi * day.range;
        if b.high >= top_lo && b.low <= top_hi {
            return Some((i, Side::HighSweep));
        }
        let bot_hi = day.range_low - ext.lo * day.range;
        let bot_lo = day.range_low - ext.hi * day.range;
        if b.low <= bot_hi && b.high >= bot_lo {
            return Some((i, Side::LowSweep));
        }
    }
    None
}

fn main() {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");
    let days = collect_days(&candles);

    let extensions = [
        Band {
            name: "2.0-2.5R",
            lo: 2.0,
            hi: 2.5,
        },
        Band {
            name: "4.0-4.5R",
            lo: 4.0,
            hi: 4.5,
        },
        Band {
            name: "6.0-6.5R",
            lo: 6.0,
            hi: 6.5,
        },
    ];
    let targets = extensions;

    for tf in [1_i64, 3_i64] {
        println!("\n===== 9:00-9:15 Reversal Pattern Scan ({tf}m) =====");
        for ext in extensions {
            for pattern_name in ["ifvg", "ob"] {
                for target in targets {
                    let mut stats = Stats::default();

                    for day in &days {
                        let Some((touch_1m_idx, side)) = first_touch_idx(day, ext) else {
                            continue;
                        };
                        stats.touches += 1;

                        let touch_ts = day.bars_1m[touch_1m_idx].ts;
                        let bars_tf = if tf == 1 {
                            day.bars_1m.clone()
                        } else {
                            resample(&day.bars_1m, tf)
                        };
                        let start_tf_idx = bars_tf
                            .iter()
                            .position(|b| b.ts >= touch_ts)
                            .unwrap_or(bars_tf.len().saturating_sub(1));

                        let pat_idx = if pattern_name == "ifvg" {
                            find_ifvg_idx(&bars_tf, side, start_tf_idx)
                        } else {
                            find_ob_idx(&bars_tf, side, start_tf_idx)
                        };
                        let Some(pidx) = pat_idx else {
                            continue;
                        };
                        stats.pattern_found += 1;

                        let hit = if side == Side::HighSweep {
                            let t_hi = day.range_low - target.lo * day.range;
                            let t_lo = day.range_low - target.hi * day.range;
                            bars_tf
                                .iter()
                                .skip(pidx)
                                .any(|b| b.low <= t_hi && b.high >= t_lo)
                        } else {
                            let t_lo = day.range_high + target.lo * day.range;
                            let t_hi = day.range_high + target.hi * day.range;
                            bars_tf
                                .iter()
                                .skip(pidx)
                                .any(|b| b.high >= t_lo && b.low <= t_hi)
                        };
                        if hit {
                            stats.target_hit += 1;
                        }
                    }

                    let pattern_rate = if stats.touches > 0 {
                        stats.pattern_found as f64 / stats.touches as f64 * 100.0
                    } else {
                        0.0
                    };
                    let hit_rate = if stats.pattern_found > 0 {
                        stats.target_hit as f64 / stats.pattern_found as f64 * 100.0
                    } else {
                        0.0
                    };

                    println!(
                        "ext={} | pattern={} | target=-{} | touches={} pattern_found={} ({:.2}%) target_hits={} ({:.2}% of patterns)",
                        ext.name,
                        pattern_name,
                        target.name,
                        stats.touches,
                        stats.pattern_found,
                        pattern_rate,
                        stats.target_hit,
                        hit_rate
                    );
                }
            }
        }
    }
}
