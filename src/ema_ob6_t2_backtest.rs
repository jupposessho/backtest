use backtest::candle_stick_loader::CandleStickLoader;
use backtest::model::candle_stick::CandleStick;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Timelike};
use chrono_tz::America::New_York;
use clap::Parser;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum TpMode {
    Fixed,
    Rr,
}

#[derive(Parser, Debug, Clone)]
struct Args {
    #[arg(long, default_value = "/Users/waff/develop/play/nq/mnq_1m_cont.parquet")]
    parquet: String,
    #[arg(long, default_value = "2025-01-01")]
    start: String,
    #[arg(long, default_value_t = 6)]
    ob_lookback: usize,
    #[arg(long, default_value_t = 2)]
    limit_timeout: usize,
    #[arg(long, default_value_t = 10.0)]
    trail_activate_pts: f64,
    #[arg(long, default_value_t = 10.0)]
    trail_dist_pts: f64,
    #[arg(long, default_value_t = 0.05)]
    ob_displacement_pct: f64,
    #[arg(long, default_value = "10:30")]
    entry_start: String,
    #[arg(long, default_value = "15:25")]
    entry_end: String,
    #[arg(long, default_value = "")]
    skip_weekdays: String,
    #[arg(long, default_value = "")]
    skip_windows: String,
    #[arg(long, value_enum, default_value = "fixed")]
    tp_mode: TpMode,
    #[arg(long, default_value_t = 150.0)]
    tp_pts: f64,
    #[arg(long, default_value_t = 2.0)]
    rr: f64,
    #[arg(long, default_value_t = false)]
    sweep2: bool,
}

#[derive(Clone, Copy)]
struct SweepStats {
    trades: usize,
    wr: f64,
    net: f64,
    pf: f64,
    dd: f64,
}

#[derive(Clone, Copy)]
struct Bar {
    ts: DateTime<chrono_tz::Tz>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[derive(Clone, Copy)]
struct Setup {
    direction: i32,
    entry: f64,
    sl: f64,
    tp: f64,
    expiry_bars: usize,
}

#[derive(Clone, Copy)]
struct OpenTrade {
    direction: i32,
    entry: f64,
    sl: f64,
    tp: f64,
    current_stop: f64,
    opened_ts: DateTime<chrono_tz::Tz>,
    trail_best: f64,
    last_close: f64,
    risk_points: f64,
}

#[derive(Clone)]
struct Trade {
    pnl_usd: f64,
}

fn load_mnq_1m(parquet: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_parquet(parquet).expect("load mnq parquet")
}

fn validate_data(data: &[CandleStick], expected_spacing_sec: i64) {
    assert!(!data.is_empty(), "empty dataset");
    for (i, b) in data.iter().enumerate() {
        assert!(b.high >= b.low, "OHLC invalid at {i}");
        assert!(b.high >= b.open && b.high >= b.close, "OHLC invalid at {i}");
        assert!(b.low <= b.open && b.low <= b.close, "OHLC invalid at {i}");
        if i > 0 {
            let prev = data[i - 1];
            assert!(b.open_time > prev.open_time, "timestamp not monotonic at {i}");
            let delta = b.open_time - prev.open_time;
            assert!(delta % expected_spacing_sec == 0, "unexpected spacing at {i}: {delta}");
        }
    }
}

fn to_bar(c: CandleStick) -> Bar {
    Bar {
        ts: DateTime::from_timestamp(c.open_time, 0)
            .unwrap()
            .with_timezone(&New_York),
        open: c.open.0.to_f64().unwrap(),
        high: c.high.0.to_f64().unwrap(),
        low: c.low.0.to_f64().unwrap(),
        close: c.close.0.to_f64().unwrap(),
        volume: 0.0,
    }
}

fn floor_bucket(ts: DateTime<chrono_tz::Tz>, minutes: i64) -> i64 {
    let midnight = ts
        .timezone()
        .with_ymd_and_hms(ts.year(), ts.month(), ts.day(), 0, 0, 0)
        .single()
        .expect("midnight");
    let minute_of_day = i64::from(ts.hour()) * 60 + i64::from(ts.minute());
    let bucket_minute = minute_of_day - (minute_of_day % minutes);
    (midnight + chrono::Duration::minutes(bucket_minute)).timestamp()
}

fn resample(bars: &[Bar], minutes: i64) -> Vec<Bar> {
    if bars.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    let mut cur = floor_bucket(bars[0].ts, minutes);
    let mut o = bars[0].open;
    let mut h = bars[0].high;
    let mut l = bars[0].low;
    let mut c = bars[0].close;
    let mut v = bars[0].volume;
    for b in bars.iter().copied() {
        let bk = floor_bucket(b.ts, minutes);
        if bk != cur {
            out.push(Bar {
                ts: DateTime::from_timestamp(cur, 0)
                    .unwrap()
                    .with_timezone(&New_York),
                open: o,
                high: h,
                low: l,
                close: c,
                volume: v,
            });
            cur = bk;
            o = b.open;
            h = b.high;
            l = b.low;
            c = b.close;
            v = b.volume;
        } else {
            h = h.max(b.high);
            l = l.min(b.low);
            c = b.close;
            v += b.volume;
        }
    }
    out.push(Bar {
        ts: DateTime::from_timestamp(cur, 0)
            .unwrap()
            .with_timezone(&New_York),
        open: o,
        high: h,
        low: l,
        close: c,
        volume: v,
    });
    out
}

fn ema(values: &[f64], span: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; values.len()];
    if values.is_empty() {
        return out;
    }
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut prev = values[0];
    out[0] = prev;
    for i in 1..values.len() {
        prev = alpha * values[i] + (1.0 - alpha) * prev;
        out[i] = prev;
    }
    out
}

fn find_ob_near_ema(opens: &[f64], closes: &[f64], direction: i32, displacement_pct: f64) -> Option<(usize, usize)> {
    let n = closes.len();
    if n < 2 {
        return None;
    }
    let min_disp = displacement_pct / 100.0;
    for i in (1..n).rev() {
        let body_pct = if closes[i] != 0.0 {
            (closes[i] - opens[i]).abs() / closes[i].abs()
        } else {
            0.0
        };
        if body_pct < min_disp {
            continue;
        }
        if direction == 1 {
            if closes[i] <= opens[i] {
                continue;
            }
            for j in (0..i).rev() {
                if closes[j] < opens[j] && closes[i] >= opens[j] {
                    return Some((j, i));
                }
            }
        } else {
            if closes[i] >= opens[i] {
                continue;
            }
            for j in (0..i).rev() {
                if closes[j] > opens[j] && closes[i] <= opens[j] {
                    return Some((j, i));
                }
            }
        }
    }
    None
}

fn simulate_collect(args: &Args, ltf: &[Bar], htf: &[Bar]) -> [(i32, SweepStats); 3] {
    let entry_start = NaiveTime::parse_from_str(&args.entry_start, "%H:%M").expect("entry-start");
    let entry_end = NaiveTime::parse_from_str(&args.entry_end, "%H:%M").expect("entry-end");
    let mut skip_weekdays = [false; 7];
    if !args.skip_weekdays.trim().is_empty() {
        for part in args.skip_weekdays.split(',') {
            let idx = match part.trim().to_ascii_lowercase().as_str() {
                "mon" | "monday" => Some(0usize),
                "tue" | "tuesday" => Some(1usize),
                "wed" | "wednesday" => Some(2usize),
                "thu" | "thursday" => Some(3usize),
                "fri" | "friday" => Some(4usize),
                "sat" | "saturday" => Some(5usize),
                "sun" | "sunday" => Some(6usize),
                _ => None,
            };
            if let Some(i) = idx {
                skip_weekdays[i] = true;
            }
        }
    }
    let mut skip_windows: Vec<(NaiveTime, NaiveTime)> = Vec::new();
    if !args.skip_windows.trim().is_empty() {
        for win in args.skip_windows.split(',') {
            let parts: Vec<&str> = win.trim().split('-').collect();
            if parts.len() == 2 {
                let s = NaiveTime::parse_from_str(parts[0].trim(), "%H:%M").expect("skip start");
                let e = NaiveTime::parse_from_str(parts[1].trim(), "%H:%M").expect("skip end");
                skip_windows.push((s, e));
            }
        }
    }

    let ltf_close: Vec<f64> = ltf.iter().map(|b| b.close).collect();
    let ltf_high: Vec<f64> = ltf.iter().map(|b| b.high).collect();
    let ltf_low: Vec<f64> = ltf.iter().map(|b| b.low).collect();
    let ltf_open: Vec<f64> = ltf.iter().map(|b| b.open).collect();
    let htf_close: Vec<f64> = htf.iter().map(|b| b.close).collect();
    let ltf_ema = ema(&ltf_close, 9);
    let htf_fast = ema(&htf_close, 13);
    let htf_slow = ema(&htf_close, 200);

    let mut htf_bias_by_ltf = vec![0i32; ltf.len()];
    let mut h = 0usize;
    for (i, b) in ltf.iter().enumerate() {
        while h + 1 < htf.len() && htf[h + 1].ts <= b.ts {
            h += 1;
        }
        let idx = h.saturating_sub(1);
        if idx < htf.len() {
            htf_bias_by_ltf[i] = if htf_fast[idx] > htf_slow[idx] {
                1
            } else if htf_fast[idx] < htf_slow[idx] {
                -1
            } else {
                0
            };
        }
    }

    let mut atr = vec![f64::NAN; ltf.len()];
    for i in 0..ltf.len() {
        let tr = if i == 0 {
            ltf_high[i] - ltf_low[i]
        } else {
            let a = ltf_high[i] - ltf_low[i];
            let b = (ltf_high[i] - ltf_close[i - 1]).abs();
            let c = (ltf_low[i] - ltf_close[i - 1]).abs();
            a.max(b).max(c)
        };
        atr[i] = if i == 0 { tr } else { (2.0 / 15.0) * tr + (13.0 / 15.0) * atr[i - 1] };
    }

    let mut out = [
        (
            1,
            SweepStats {
                trades: 0,
                wr: 0.0,
                net: 0.0,
                pf: 0.0,
                dd: 0.0,
            },
        ),
        (
            2,
            SweepStats {
                trades: 0,
                wr: 0.0,
                net: 0.0,
                pf: 0.0,
                dd: 0.0,
            },
        ),
        (
            3,
            SweepStats {
                trades: 0,
                wr: 0.0,
                net: 0.0,
                pf: 0.0,
                dd: 0.0,
            },
        ),
    ];
    for (ix, slip_ticks) in [1.0, 2.0, 3.0].into_iter().enumerate() {
        let point_value = 2.0;
        let commission_rt = 0.92;
        let slippage_pts_per_side = slip_ticks * 0.25;
        let mut trades = Vec::<Trade>::new();
        let mut pending: Option<(Setup, usize, usize)> = None;
        let mut open: Option<OpenTrade> = None;
        let mut blocked_ts: Option<DateTime<chrono_tz::Tz>> = None;
        let mut daily_count: HashMap<NaiveDate, usize> = HashMap::new();

        for i in 0..ltf.len() {
            let bar = ltf[i];
            let d = bar.ts.date_naive();
            let t = NaiveTime::from_hms_opt(bar.ts.hour(), bar.ts.minute(), 0).unwrap();
            let weekday_idx = bar.ts.weekday().num_days_from_monday() as usize;

            if let Some(mut ot) = open {
                if bar.ts != ot.opened_ts {
                    if t >= NaiveTime::from_hms_opt(15, 25, 0).unwrap() {
                        let pnl_pts = (bar.close - ot.entry) * ot.direction as f64;
                        trades.push(Trade { pnl_usd: (pnl_pts - slippage_pts_per_side * 2.0) * point_value - commission_rt });
                        *daily_count.entry(d).or_insert(0) += 1;
                        blocked_ts = Some(bar.ts);
                        open = None;
                    } else {
                        let profit_pts = (ot.last_close - ot.entry) * ot.direction as f64;
                        if profit_pts >= args.trail_activate_pts {
                            if ot.direction == 1 {
                                let candidate = ot.last_close - args.trail_dist_pts;
                                if candidate > ot.current_stop {
                                    ot.current_stop = candidate;
                                    ot.trail_best = ot.last_close.max(ot.trail_best);
                                }
                            } else {
                                let candidate = ot.last_close + args.trail_dist_pts;
                                if candidate < ot.current_stop {
                                    ot.current_stop = candidate;
                                    ot.trail_best = ot.last_close.min(ot.trail_best);
                                }
                            }
                        }
                        let stop_hit = if ot.direction == 1 { bar.low <= ot.current_stop } else { bar.high >= ot.current_stop };
                        let tp_hit = if ot.direction == 1 { bar.high >= ot.tp } else { bar.low <= ot.tp };
                        if stop_hit {
                            let stop_fill = if ot.direction == 1 && bar.open < ot.current_stop {
                                bar.open
                            } else if ot.direction == -1 && bar.open > ot.current_stop {
                                bar.open
                            } else {
                                ot.current_stop
                            };
                            let pnl_pts = (stop_fill - ot.entry) * ot.direction as f64;
                            trades.push(Trade { pnl_usd: (pnl_pts - slippage_pts_per_side * 2.0) * point_value - commission_rt });
                            *daily_count.entry(d).or_insert(0) += 1;
                            blocked_ts = Some(bar.ts);
                            open = None;
                        } else if tp_hit {
                            let pnl_pts = (ot.tp - ot.entry) * ot.direction as f64;
                            trades.push(Trade { pnl_usd: (pnl_pts - slippage_pts_per_side * 2.0) * point_value - commission_rt });
                            *daily_count.entry(d).or_insert(0) += 1;
                            blocked_ts = Some(bar.ts);
                            open = None;
                        } else {
                            ot.last_close = bar.close;
                            open = Some(ot);
                        }
                    }
                } else {
                    open = Some(ot);
                }
            }
            if open.is_some() {
                continue;
            }

            if let Some((setup, _created_i, waited)) = pending {
                if t >= NaiveTime::from_hms_opt(15, 25, 0).unwrap() {
                    pending = None;
                } else {
                    let invalidated = if setup.direction == 1 { bar.low <= setup.sl } else { bar.high >= setup.sl };
                    let touched = if setup.direction == 1 { bar.low <= setup.entry } else { bar.high >= setup.entry };
                    let traded_through = if setup.direction == 1 {
                        bar.low <= setup.entry - 0.25
                    } else {
                        bar.high >= setup.entry + 0.25
                    };
                    if invalidated && !touched {
                        pending = None;
                    } else if traded_through {
                        let ot = OpenTrade {
                            direction: setup.direction,
                            entry: setup.entry,
                            sl: setup.sl,
                            tp: setup.tp,
                            current_stop: setup.sl,
                            opened_ts: bar.ts,
                            trail_best: setup.entry,
                            last_close: bar.close,
                            risk_points: (setup.entry - setup.sl).abs(),
                        };
                        let stop_hit_on_fill = if ot.direction == 1 { bar.low <= ot.sl } else { bar.high >= ot.sl };
                        if invalidated || stop_hit_on_fill {
                            let stop_fill = if ot.direction == 1 && bar.open < ot.sl {
                                bar.open
                            } else if ot.direction == -1 && bar.open > ot.sl {
                                bar.open
                            } else {
                                ot.sl
                            };
                            let pnl_pts = (stop_fill - ot.entry) * ot.direction as f64;
                            trades.push(Trade { pnl_usd: (pnl_pts - slippage_pts_per_side * 2.0) * point_value - commission_rt });
                            *daily_count.entry(d).or_insert(0) += 1;
                            blocked_ts = Some(bar.ts);
                        } else {
                            open = Some(ot);
                        }
                        pending = None;
                    } else if waited + 1 >= setup.expiry_bars {
                        pending = None;
                    } else {
                        pending = Some((setup, i, waited + 1));
                    }
                }
            }

            if open.is_some() || pending.is_some() || blocked_ts == Some(bar.ts) || *daily_count.get(&d).unwrap_or(&0) >= 4 {
                continue;
            }
            if t < NaiveTime::from_hms_opt(9, 30, 0).unwrap() || t >= NaiveTime::from_hms_opt(15, 25, 0).unwrap() || t < entry_start || t >= entry_end || skip_weekdays[weekday_idx] {
                continue;
            }
            if skip_windows.iter().any(|(s, e)| t >= *s && t < *e) {
                continue;
            }
            if i == 0 || !ltf_ema[i].is_finite() || !atr[i].is_finite() {
                continue;
            }
            let bias = htf_bias_by_ltf[i];
            if bias == 0 {
                continue;
            }
            let ema_zone_hi = ltf_ema[i] + 0.5 * atr[i];
            let ema_zone_lo = ltf_ema[i] - 0.5 * atr[i];
            let direction = if bias == 1 && bar.low <= ema_zone_hi && bar.close >= ema_zone_lo {
                1
            } else if bias == -1 && bar.high >= ema_zone_lo && bar.close <= ema_zone_hi {
                -1
            } else {
                0
            };
            if direction == 0 {
                continue;
            }

            let lb = i.saturating_sub(args.ob_lookback);
            if let Some((ob_idx_rel, disp_idx_rel)) = find_ob_near_ema(&ltf_open[lb..i], &ltf_close[lb..i], direction, args.ob_displacement_pct) {
                let ob_pos = lb + ob_idx_rel;
                let disp_pos = lb + disp_idx_rel;
                let limit_price = (ltf_high[ob_pos] + ltf_low[ob_pos]) / 2.0;
                let limit_sl = if direction == 1 { ltf_low[ob_pos].min(ltf_low[disp_pos]) } else { ltf_high[ob_pos].max(ltf_high[disp_pos]) };
                let limit_risk = if direction == 1 { limit_price - limit_sl } else { limit_sl - limit_price };
                let limit_is_better = if direction == 1 { limit_price < bar.close } else { limit_price > bar.close };
                if limit_risk > 0.0 && limit_is_better {
                    let tp = match args.tp_mode {
                        TpMode::Fixed => limit_price + direction as f64 * args.tp_pts,
                        TpMode::Rr => limit_price + direction as f64 * (args.rr * limit_risk),
                    };
                    pending = Some((Setup { direction, entry: limit_price, sl: limit_sl, tp, expiry_bars: args.limit_timeout }, i, 0));
                }
            }
        }

        let n = trades.len() as f64;
        let winners = trades.iter().filter(|t| t.pnl_usd > 0.0).count() as f64;
        let gross_profit: f64 = trades.iter().filter(|t| t.pnl_usd > 0.0).map(|t| t.pnl_usd).sum();
        let gross_loss_abs: f64 = trades.iter().filter(|t| t.pnl_usd < 0.0).map(|t| -t.pnl_usd).sum();
        let net: f64 = trades.iter().map(|t| t.pnl_usd).sum();
        let pf = if gross_loss_abs > 0.0 { gross_profit / gross_loss_abs } else { 0.0 };
        let mut eq = 0.0;
        let mut running_max: Option<f64> = None;
        let mut max_dd = 0.0;
        for t in &trades {
            eq += t.pnl_usd;
            let rm = match running_max {
                Some(v) => {
                    let nv = v.max(eq);
                    running_max = Some(nv);
                    nv
                }
                None => {
                    running_max = Some(eq);
                    eq
                }
            };
            let dd = eq - rm;
            if dd < max_dd {
                max_dd = dd;
            }
        }
        out[ix] = (
            slip_ticks as i32,
            SweepStats {
                trades: trades.len(),
                wr: if n > 0.0 { winners * 100.0 / n } else { 0.0 },
                net,
                pf,
                dd: max_dd,
            },
        );
    }
    out
}

fn print_stats(stats: &[(i32, SweepStats); 3]) {
    for (slip, s) in stats {
        println!(
            "slip={} trades={} win_rate={:.1}% net=${:.2} pf={:.2} max_dd=${:.2}",
            slip, s.trades, s.wr, s.net, s.pf, s.dd
        );
    }
}

fn main() {
    let args = Args::parse();
    let start_date = NaiveDate::parse_from_str(&args.start, "%Y-%m-%d").expect("start date");
    let start_ts = DateTime::from_timestamp(
        start_date.and_hms_opt(0, 0, 0).expect("midnight").and_utc().timestamp(),
        0,
    )
    .unwrap()
    .with_timezone(&New_York);

    let one_min_raw = load_mnq_1m(&args.parquet);
    validate_data(&one_min_raw, 60);
    let one_min: Vec<Bar> = one_min_raw
        .into_iter()
        .filter(|c| DateTime::from_timestamp(c.open_time, 0).unwrap().with_timezone(&New_York) >= start_ts)
        .map(to_bar)
        .collect();
    let ltf = resample(&one_min, 15);
    let htf = resample(&one_min, 60);
    if !args.sweep2 {
        let stats = simulate_collect(&args, &ltf, &htf);
        print_stats(&stats);
        return;
    }

    let skip_days_set = [
        "fri", "mon,fri", "tue,fri", "wed,fri", "thu,fri", "mon", "tue", "wed", "thu", "",
    ];
    let skip_windows_set = [
        "",
        "10:30-11:00",
        "11:00-11:30",
        "11:30-12:15",
        "12:00-12:45",
        "13:00-13:30",
        "13:30-14:15",
        "14:00-14:30",
        "14:30-15:25",
    ];
    let tp_fixed = [120.0, 130.0, 140.0, 150.0, 160.0, 170.0, 180.0, 200.0, 220.0, 250.0];
    let tp_rr = [2.0, 2.5, 3.0, 3.5, 4.0, 5.0, 6.0];

    let mut best: Vec<(f64, String, String, String, f64, [(i32, SweepStats); 3])> = Vec::new();

    for skip_days in skip_days_set {
        for skip_win in skip_windows_set {
            for tp in tp_fixed {
                let mut cfg = args.clone();
                cfg.skip_weekdays = skip_days.to_string();
                cfg.skip_windows = skip_win.to_string();
                cfg.tp_mode = TpMode::Fixed;
                cfg.tp_pts = tp;
                let stats = simulate_collect(&cfg, &ltf, &htf);
                let s1 = stats[0].1;
                let s3 = stats[2].1;
                if s1.trades < 180 {
                    continue;
                }
                let cal1 = if s1.dd != 0.0 {
                    (s1.net / s1.dd).abs()
                } else {
                    0.0
                };
                let score = s3.net + 0.4 * s1.net + 400.0 * s3.pf + 100.0 * cal1 - 0.15 * s3.dd.abs();
                best.push((
                    score,
                    skip_days.to_string(),
                    skip_win.to_string(),
                    "fixed".to_string(),
                    tp,
                    stats,
                ));
            }
            for rr in tp_rr {
                let mut cfg = args.clone();
                cfg.skip_weekdays = skip_days.to_string();
                cfg.skip_windows = skip_win.to_string();
                cfg.tp_mode = TpMode::Rr;
                cfg.rr = rr;
                let stats = simulate_collect(&cfg, &ltf, &htf);
                let s1 = stats[0].1;
                let s3 = stats[2].1;
                if s1.trades < 180 {
                    continue;
                }
                let cal1 = if s1.dd != 0.0 {
                    (s1.net / s1.dd).abs()
                } else {
                    0.0
                };
                let score = s3.net + 0.4 * s1.net + 400.0 * s3.pf + 100.0 * cal1 - 0.15 * s3.dd.abs();
                best.push((
                    score,
                    skip_days.to_string(),
                    skip_win.to_string(),
                    "rr".to_string(),
                    rr,
                    stats,
                ));
            }
        }
    }

    best.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    println!("sweep2 tested={}", best.len());
    for (rank, row) in best.into_iter().take(20).enumerate() {
        let (_, skip_days, skip_win, mode, tpv, stats) = row;
        let s1 = stats[0].1;
        let s2 = stats[1].1;
        let s3 = stats[2].1;
        println!(
            "#{rank} skip_days='{skip_days}' skip_win='{skip_win}' mode={mode} val={tpv:.2} | s1(trades={},wr={:.1},net={:.2},pf={:.2},dd={:.2}) s2(net={:.2},pf={:.2},dd={:.2}) s3(net={:.2},pf={:.2},dd={:.2})",
            s1.trades,
            s1.wr,
            s1.net,
            s1.pf,
            s1.dd,
            s2.net,
            s2.pf,
            s2.dd,
            s3.net,
            s3.pf,
            s3.dd
        );
    }
}
