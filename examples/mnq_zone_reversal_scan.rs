use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy)]
struct ZoneCfg {
    name: &'static str,
    low_mult: f64,
    high_mult: f64,
}

#[derive(Default, Clone, Copy)]
struct ZoneStats {
    top_touches: usize,
    top_reversals_to_opposite: usize,
    top_reversals_to_opposite_by_12: usize,
    bottom_touches: usize,
    bottom_reversals_to_opposite: usize,
    bottom_reversals_to_opposite_by_12: usize,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn main() {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");

    let zones = [
        ZoneCfg {
            name: "0.33-0.66R",
            low_mult: 0.33,
            high_mult: 0.66,
        },
        ZoneCfg {
            name: "1.33-1.66R",
            low_mult: 1.33,
            high_mult: 1.66,
        },
    ];
    let mut stats = [ZoneStats::default(), ZoneStats::default()];

    let mut i = 0usize;
    let mut valid_days = 0usize;

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
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut post_start = None;
        let mut has_range = false;

        for (k, c) in day.iter().enumerate() {
            let h = New_York
                .timestamp_opt(c.open_time, 0)
                .single()
                .expect("timestamp")
                .hour();
            if (6..9).contains(&h) {
                has_range = true;
                rh = rh.max(d2f(c.high.0));
                rl = rl.min(d2f(c.low.0));
            }
            if h >= 9 && post_start.is_none() {
                post_start = Some(k);
            }
        }

        let p0 = if let Some(v) = post_start { v } else { i = j; continue };
        if !has_range || rh <= rl {
            i = j;
            continue;
        }

        valid_days += 1;
        let range = rh - rl;

        for (zidx, z) in zones.iter().enumerate() {
            let top_low = rh + z.low_mult * range;
            let top_high = rh + z.high_mult * range;
            let bot_high = rl - z.low_mult * range;
            let bot_low = rl - z.high_mult * range;

            let mut top_touch_idx = None;
            let mut bot_touch_idx = None;

            for (k, c) in day.iter().enumerate().skip(p0) {
                let hi = d2f(c.high.0);
                let lo = d2f(c.low.0);

                if top_touch_idx.is_none() && hi >= top_low && lo <= top_high {
                    top_touch_idx = Some(k);
                }
                if bot_touch_idx.is_none() && lo <= bot_high && hi >= bot_low {
                    bot_touch_idx = Some(k);
                }
                if top_touch_idx.is_some() && bot_touch_idx.is_some() {
                    break;
                }
            }

            if let Some(ti) = top_touch_idx {
                stats[zidx].top_touches += 1;
                let mut reversed = false;
                let mut reversed_by_12 = false;
                for c in day.iter().skip(ti) {
                    let h = New_York
                        .timestamp_opt(c.open_time, 0)
                        .single()
                        .expect("timestamp")
                        .hour();
                    if d2f(c.low.0) <= rl {
                        reversed = true;
                        if h < 12 {
                            reversed_by_12 = true;
                        }
                        break;
                    }
                }
                if reversed {
                    stats[zidx].top_reversals_to_opposite += 1;
                }
                if reversed_by_12 {
                    stats[zidx].top_reversals_to_opposite_by_12 += 1;
                }
            }

            if let Some(bi) = bot_touch_idx {
                stats[zidx].bottom_touches += 1;
                let mut reversed = false;
                let mut reversed_by_12 = false;
                for c in day.iter().skip(bi) {
                    let h = New_York
                        .timestamp_opt(c.open_time, 0)
                        .single()
                        .expect("timestamp")
                        .hour();
                    if d2f(c.high.0) >= rh {
                        reversed = true;
                        if h < 12 {
                            reversed_by_12 = true;
                        }
                        break;
                    }
                }
                if reversed {
                    stats[zidx].bottom_reversals_to_opposite += 1;
                }
                if reversed_by_12 {
                    stats[zidx].bottom_reversals_to_opposite_by_12 += 1;
                }
            }
        }

        i = j;
    }

    println!("MNQ zone reversal scan from 6-9 NY range");
    println!("Valid days: {}", valid_days);
    println!("Reversal criterion: after touching zone, reaches opposite side of 6-9 range by EOD");
    for (z, s) in zones.iter().zip(stats.iter()) {
        let top_rate = if s.top_touches > 0 {
            s.top_reversals_to_opposite as f64 / s.top_touches as f64 * 100.0
        } else {
            0.0
        };
        let top_rate_12 = if s.top_touches > 0 {
            s.top_reversals_to_opposite_by_12 as f64 / s.top_touches as f64 * 100.0
        } else {
            0.0
        };
        let bot_rate = if s.bottom_touches > 0 {
            s.bottom_reversals_to_opposite as f64 / s.bottom_touches as f64 * 100.0
        } else {
            0.0
        };
        let bot_rate_12 = if s.bottom_touches > 0 {
            s.bottom_reversals_to_opposite_by_12 as f64 / s.bottom_touches as f64 * 100.0
        } else {
            0.0
        };
        let all_touches = s.top_touches + s.bottom_touches;
        let all_rev = s.top_reversals_to_opposite + s.bottom_reversals_to_opposite;
        let all_rate = if all_touches > 0 {
            all_rev as f64 / all_touches as f64 * 100.0
        } else {
            0.0
        };
        let all_rev_12 = s.top_reversals_to_opposite_by_12 + s.bottom_reversals_to_opposite_by_12;
        let all_rate_12 = if all_touches > 0 {
            all_rev_12 as f64 / all_touches as f64 * 100.0
        } else {
            0.0
        };
        println!("\nZone {}", z.name);
        println!(
            "- top first-touch reversals: EOD={} ({:.2}%), by 12:00={} ({:.2}%)",
            s.top_reversals_to_opposite,
            top_rate,
            s.top_reversals_to_opposite_by_12,
            top_rate_12
        );
        println!(
            "- bottom first-touch reversals: EOD={} ({:.2}%), by 12:00={} ({:.2}%)",
            s.bottom_reversals_to_opposite,
            bot_rate,
            s.bottom_reversals_to_opposite_by_12,
            bot_rate_12
        );
        println!(
            "- combined first touches={} | EOD reversals={} ({:.2}%) | by 12:00 reversals={} ({:.2}%)",
            all_touches, all_rev, all_rate, all_rev_12, all_rate_12
        );
    }
}
