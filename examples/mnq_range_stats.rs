use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy)]
struct DayRow {
    first_side: FirstSide,
    deviation: f64,
    range_size: f64,
}

#[derive(Clone, Copy)]
enum FirstSide {
    TopFirst,
    BottomFirst,
    BothSameBar,
}

fn percentile(mut values: Vec<f64>, p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let idx = ((values.len() - 1) as f64 * p).round() as usize;
    values[idx]
}

fn main() {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");

    if candles.is_empty() {
        println!("No candles loaded.");
        return;
    }

    let mut i = 0usize;
    let mut valid_days = 0usize;
    let mut both_rows: Vec<DayRow> = Vec::new();

    while i < candles.len() {
        let dt = New_York
            .timestamp_opt(candles[i].open_time, 0)
            .single()
            .expect("valid timestamp");
        let current_date = dt.date_naive();

        let mut j = i;
        while j < candles.len() {
            let dj = New_York
                .timestamp_opt(candles[j].open_time, 0)
                .single()
                .expect("valid timestamp")
                .date_naive();
            if dj != current_date {
                break;
            }
            j += 1;
        }

        let day = &candles[i..j];

        let mut range_high = f64::NEG_INFINITY;
        let mut range_low = f64::INFINITY;
        let mut range_exists = false;
        let mut post_indices: Vec<usize> = Vec::new();

        for (k, c) in day.iter().enumerate() {
            let t = New_York
                .timestamp_opt(c.open_time, 0)
                .single()
                .expect("valid timestamp");
            let hour = t.hour();

            if (6..9).contains(&hour) {
                range_exists = true;
                let h = c.high.0.to_string().parse::<f64>().unwrap_or(0.0);
                let l = c.low.0.to_string().parse::<f64>().unwrap_or(0.0);
                if h > range_high {
                    range_high = h;
                }
                if l < range_low {
                    range_low = l;
                }
            }

            if hour >= 9 {
                post_indices.push(k);
            }
        }

        if range_exists && !post_indices.is_empty() {
            valid_days += 1;

            let mut first_touch: Option<(FirstSide, usize)> = None;
            for idx in &post_indices {
                let c = day[*idx];
                let h = c.high.0.to_string().parse::<f64>().unwrap_or(0.0);
                let l = c.low.0.to_string().parse::<f64>().unwrap_or(0.0);
                let hit_top = h >= range_high;
                let hit_bottom = l <= range_low;
                if hit_top && hit_bottom {
                    first_touch = Some((FirstSide::BothSameBar, *idx));
                    break;
                }
                if hit_top {
                    first_touch = Some((FirstSide::TopFirst, *idx));
                    break;
                }
                if hit_bottom {
                    first_touch = Some((FirstSide::BottomFirst, *idx));
                    break;
                }
            }

            if let Some((side, idx)) = first_touch {
                match side {
                    FirstSide::BothSameBar => both_rows.push(DayRow {
                        first_side: FirstSide::BothSameBar,
                        deviation: 0.0,
                        range_size: range_high - range_low,
                    }),
                    FirstSide::TopFirst => {
                        let mut max_dev = 0.0f64;
                        let mut reached_other = false;
                        for c in &day[idx..] {
                            let h = c.high.0.to_string().parse::<f64>().unwrap_or(0.0);
                            let l = c.low.0.to_string().parse::<f64>().unwrap_or(0.0);
                            if h > range_high {
                                max_dev = max_dev.max(h - range_high);
                            }
                            if l <= range_low {
                                reached_other = true;
                                break;
                            }
                        }
                        if reached_other {
                            both_rows.push(DayRow {
                                first_side: FirstSide::TopFirst,
                                deviation: max_dev,
                                range_size: range_high - range_low,
                            });
                        }
                    }
                    FirstSide::BottomFirst => {
                        let mut max_dev = 0.0f64;
                        let mut reached_other = false;
                        for c in &day[idx..] {
                            let h = c.high.0.to_string().parse::<f64>().unwrap_or(0.0);
                            let l = c.low.0.to_string().parse::<f64>().unwrap_or(0.0);
                            if l < range_low {
                                max_dev = max_dev.max(range_low - l);
                            }
                            if h >= range_high {
                                reached_other = true;
                                break;
                            }
                        }
                        if reached_other {
                            both_rows.push(DayRow {
                                first_side: FirstSide::BottomFirst,
                                deviation: max_dev,
                                range_size: range_high - range_low,
                            });
                        }
                    }
                }
            }
        }

        i = j;
    }

    let both_count = both_rows.len();
    let rate = if valid_days > 0 {
        both_count as f64 / valid_days as f64 * 100.0
    } else {
        0.0
    };

    let top_first = both_rows
        .iter()
        .filter(|r| matches!(r.first_side, FirstSide::TopFirst))
        .count();
    let bottom_first = both_rows
        .iter()
        .filter(|r| matches!(r.first_side, FirstSide::BottomFirst))
        .count();
    let both_same_bar = both_rows
        .iter()
        .filter(|r| matches!(r.first_side, FirstSide::BothSameBar))
        .count();

    let deviations: Vec<f64> = both_rows.iter().map(|r| r.deviation).collect();
    let dev_pct_range: Vec<f64> = both_rows
        .iter()
        .map(|r| {
            if r.range_size > 0.0 {
                r.deviation / r.range_size * 100.0
            } else {
                0.0
            }
        })
        .collect();

    let mean_dev = if deviations.is_empty() {
        0.0
    } else {
        deviations.iter().sum::<f64>() / deviations.len() as f64
    };
    let mean_pct = if dev_pct_range.is_empty() {
        0.0
    } else {
        dev_pct_range.iter().sum::<f64>() / dev_pct_range.len() as f64
    };

    println!("FILE=assets/mnq_1m_cont.parquet");
    println!("DAYS_WITH_6_9_AND_POST_DATA={}", valid_days);
    println!("DAYS_TOUCH_BOTH_AFTER_9={}", both_count);
    println!("RATE={:.2}%", rate);
    println!(
        "FIRST_TOUCH_BREAKDOWN top_first={} bottom_first={} both_same_bar={}",
        top_first, bottom_first, both_same_bar
    );
    println!(
        "DEV_POINTS mean={:.2} median={:.2} p90={:.2} max={:.2}",
        mean_dev,
        percentile(deviations.clone(), 0.5),
        percentile(deviations.clone(), 0.9),
        deviations.iter().cloned().fold(0.0, f64::max)
    );
    println!(
        "DEV_%RANGE mean={:.2}% median={:.2}% p90={:.2}% max={:.2}%",
        mean_pct,
        percentile(dev_pct_range.clone(), 0.5),
        percentile(dev_pct_range.clone(), 0.9),
        dev_pct_range.iter().cloned().fold(0.0, f64::max)
    );
}
