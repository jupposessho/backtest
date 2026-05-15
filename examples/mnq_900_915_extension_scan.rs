use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy)]
struct Band {
    name: &'static str,
    lo: f64,
    hi: f64,
}

#[derive(Default, Clone, Copy)]
struct Stats {
    top_touches: usize,
    top_reversal_to_base_low: usize,
    bot_touches: usize,
    bot_reversal_to_base_high: usize,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn main() {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");

    let bands = [
        Band {
            name: "1.0R",
            lo: 1.0,
            hi: 1.0,
        },
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
    ];
    let mut out = [Stats::default(), Stats::default(), Stats::default()];

    let mut i = 0usize;
    let mut valid_days = 0usize;

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

        let day = &candles[i..j];
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has_base = false;
        let mut after_start = None;

        for (k, c) in day.iter().enumerate() {
            let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
            let h = dt.hour();
            let m = dt.minute();
            if h == 9 && m < 15 {
                has_base = true;
                rh = rh.max(d2f(c.high.0));
                rl = rl.min(d2f(c.low.0));
            }
            if (h > 9 || (h == 9 && m >= 15)) && after_start.is_none() {
                after_start = Some(k);
            }
        }

        if !has_base || rh <= rl || after_start.is_none() {
            i = j;
            continue;
        }

        let start = after_start.expect("after start");
        let range = rh - rl;
        valid_days += 1;

        for (bidx, b) in bands.iter().enumerate() {
            let top_lo = rh + b.lo * range;
            let top_hi = rh + b.hi * range;
            let bot_hi = rl - b.lo * range;
            let bot_lo = rl - b.hi * range;

            let mut top_touch = None;
            let mut bot_touch = None;

            for (k, c) in day.iter().enumerate().skip(start) {
                let hi = d2f(c.high.0);
                let lo = d2f(c.low.0);
                if top_touch.is_none() {
                    let hit = if (b.lo - b.hi).abs() < f64::EPSILON {
                        hi >= top_lo
                    } else {
                        hi >= top_lo && lo <= top_hi
                    };
                    if hit {
                        top_touch = Some(k);
                    }
                }
                if bot_touch.is_none() {
                    let hit = if (b.lo - b.hi).abs() < f64::EPSILON {
                        lo <= bot_hi
                    } else {
                        lo <= bot_hi && hi >= bot_lo
                    };
                    if hit {
                        bot_touch = Some(k);
                    }
                }
                if top_touch.is_some() && bot_touch.is_some() {
                    break;
                }
            }

            if let Some(t) = top_touch {
                out[bidx].top_touches += 1;
                let mut rev = false;
                for c in day.iter().skip(t) {
                    if d2f(c.low.0) <= rl {
                        rev = true;
                        break;
                    }
                }
                if rev {
                    out[bidx].top_reversal_to_base_low += 1;
                }
            }
            if let Some(t) = bot_touch {
                out[bidx].bot_touches += 1;
                let mut rev = false;
                for c in day.iter().skip(t) {
                    if d2f(c.high.0) >= rh {
                        rev = true;
                        break;
                    }
                }
                if rev {
                    out[bidx].bot_reversal_to_base_high += 1;
                }
            }
        }

        i = j;
    }

    println!("MNQ 9:00-9:15 range extension scan");
    println!("Valid days: {}", valid_days);
    println!("Reversal criterion: after extension touch, returns to opposite side of 9:00-9:15 range by EOD");

    for (b, s) in bands.iter().zip(out.iter()) {
        let tr = if s.top_touches > 0 {
            s.top_reversal_to_base_low as f64 / s.top_touches as f64 * 100.0
        } else {
            0.0
        };
        let br = if s.bot_touches > 0 {
            s.bot_reversal_to_base_high as f64 / s.bot_touches as f64 * 100.0
        } else {
            0.0
        };
        let t = s.top_touches + s.bot_touches;
        let r = s.top_reversal_to_base_low + s.bot_reversal_to_base_high;
        let cr = if t > 0 {
            r as f64 / t as f64 * 100.0
        } else {
            0.0
        };
        println!("\nBand {}", b.name);
        println!(
            "- top touches={} reversals={} rate={:.2}%",
            s.top_touches, s.top_reversal_to_base_low, tr
        );
        println!(
            "- bottom touches={} reversals={} rate={:.2}%",
            s.bot_touches, s.bot_reversal_to_base_high, br
        );
        println!("- combined touches={} reversals={} rate={:.2}%", t, r, cr);
    }
}
