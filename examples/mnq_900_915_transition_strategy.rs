use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Long,
    Short,
}

#[derive(Clone, Copy)]
struct Bar {
    h: u32,
    m: u32,
    o: f64,
    hi: f64,
    lo: f64,
    c: f64,
}

#[derive(Clone)]
struct Day {
    bars: Vec<Bar>,
    rh: f64,
    rl: f64,
    r: f64,
}

#[derive(Clone, Copy)]
struct Cfg {
    target_mult: f64,
    stop_mult: f64,
    confirm_bars: usize,
    latest_touch_hour: u32,
    use_ema_gate: bool,
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

fn load_days() -> Vec<Day> {
    let data: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("load parquet");

    let mut all: Vec<(chrono::NaiveDate, Bar)> = Vec::new();
    for c in data {
        let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        all.push((
            dt.date_naive(),
            Bar {
                h: dt.hour(),
                m: dt.minute(),
                o: d2f(c.open.0),
                hi: d2f(c.high.0),
                lo: d2f(c.low.0),
                c: d2f(c.close.0),
            },
        ));
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    while i < all.len() {
        let d = all[i].0;
        let mut j = i;
        while j < all.len() && all[j].0 == d {
            j += 1;
        }
        let bars: Vec<Bar> = all[i..j].iter().map(|x| x.1).collect();
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has = false;
        for b in &bars {
            if b.h == 9 && b.m < 15 {
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
                r: rh - rl,
            });
        }
        i = j;
    }
    out
}

fn run_day(day: &Day, cfg: Cfg) -> Option<f64> {
    let n1 = day.rl - 1.0 * day.r;
    let p1 = day.rh + 1.0 * day.r;
    let p2 = day.rh + cfg.target_mult * day.r;
    let n2 = day.rl - cfg.target_mult * day.r;

    let closes: Vec<f64> = day.bars.iter().map(|b| b.c).collect();
    let ema50 = ema(&closes, 50);

    // first touch of +/-1 after 9:15
    let mut touch: Option<(usize, Side)> = None;
    for (i, b) in day.bars.iter().enumerate() {
        if b.h < 9 || (b.h == 9 && b.m < 15) {
            continue;
        }
        if b.h > cfg.latest_touch_hour {
            break;
        }
        if b.lo <= n1 {
            touch = Some((i, Side::Long));
            break;
        }
        if b.hi >= p1 {
            touch = Some((i, Side::Short));
            break;
        }
    }
    let (tidx, side) = touch?;

    // confirmation: reclaim beyond +/-1 level within N bars
    let mut cidx = None;
    let end = (tidx + cfg.confirm_bars).min(day.bars.len().saturating_sub(1));
    for i in tidx..=end {
        let ok = if side == Side::Long {
            day.bars[i].c >= n1
        } else {
            day.bars[i].c <= p1
        };
        if ok {
            cidx = Some(i);
            break;
        }
    }
    let cidx = cidx?;
    let eidx = cidx + 1;
    if eidx >= day.bars.len() {
        return None;
    }

    if cfg.use_ema_gate {
        let ema_ok = if side == Side::Long {
            day.bars[cidx].c >= ema50[cidx]
        } else {
            day.bars[cidx].c <= ema50[cidx]
        };
        if !ema_ok {
            return None;
        }
    }

    // execution
    let tick = 0.25;
    let slip = tick;
    let comm_rt = 0.5;
    let entry = if side == Side::Long {
        day.bars[eidx].o + slip
    } else {
        day.bars[eidx].o - slip
    };
    let stop = if side == Side::Long {
        day.rl - cfg.stop_mult * day.r - slip
    } else {
        day.rh + cfg.stop_mult * day.r + slip
    };
    let target = if side == Side::Long { p2 - slip } else { n2 + slip };

    let risk = (entry - stop).abs();
    if risk < tick {
        return None;
    }
    let rr = (target - entry).abs() / risk;
    let cost_r = comm_rt / risk;

    for b in day.bars.iter().skip(eidx) {
        if b.h >= 12 {
            return Some(-0.1 - cost_r);
        }
        let stop_hit = if side == Side::Long {
            b.lo <= stop
        } else {
            b.hi >= stop
        };
        let tp_hit = if side == Side::Long {
            b.hi >= target
        } else {
            b.lo <= target
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

fn eval(days: &[Day], cfg: Cfg) -> (usize, f64, f64) {
    let mut n = 0usize;
    let mut w = 0usize;
    let mut sum = 0.0;
    for d in days {
        if let Some(r) = run_day(d, cfg) {
            n += 1;
            if r > 0.0 {
                w += 1;
            }
            sum += r;
        }
    }
    let wr = if n > 0 { w as f64 / n as f64 * 100.0 } else { 0.0 };
    let exp = if n > 0 { sum / n as f64 } else { 0.0 };
    (n, wr, exp)
}

fn main() {
    let days = load_days();
    let split = days.len() * 70 / 100;
    let train = &days[..split];
    let test = &days[split..];

    let targets = [1.0_f64, 2.25_f64]; // -1->+1 and -1->+2-2.5 (mid)
    let stops = [0.5_f64, 1.0_f64, 1.5_f64];
    let confirms = [1_usize, 2, 3, 5];
    let latest = [10_u32, 11_u32];
    let ema_gate = [false, true];

    let mut rows: Vec<(String, usize, f64, f64, usize, f64, f64)> = Vec::new();

    for t in targets {
        for s in stops {
            for c in confirms {
                for l in latest {
                    for e in ema_gate {
                        let cfg = Cfg {
                            target_mult: t,
                            stop_mult: s,
                            confirm_bars: c,
                            latest_touch_hour: l,
                            use_ema_gate: e,
                        };
                        let (n1, wr1, ex1) = eval(train, cfg);
                        let (n2, wr2, ex2) = eval(test, cfg);
                        if n2 < 25 {
                            continue;
                        }
                        rows.push((
                            format!(
                                "target=opp{} stop={}R confirm<= {} bars latest_touch<= {}:59 ema_gate={}",
                                if (t - 1.0).abs() < f64::EPSILON {
                                    "1R"
                                } else {
                                    "2.25R"
                                },
                                s,
                                c,
                                l,
                                e
                            ),
                            n1,
                            wr1,
                            ex1,
                            n2,
                            wr2,
                            ex2,
                        ));
                    }
                }
            }
        }
    }

    rows.sort_by(|a, b| b.6.total_cmp(&a.6).then(b.4.cmp(&a.4)).then(b.5.total_cmp(&a.5)));

    println!("MNQ 9:00-9:15 transition strategy sweep");
    println!("Patterns covered: -1 -> +1 / +2.25 and +1 -> -1 / -2.25 (mirrored)");
    for (i, r) in rows.iter().take(15).enumerate() {
        println!(
            "{}. {} | IS: n={} wr={:.2}% exp={:.3}R | OOS: n={} wr={:.2}% exp={:.3}R",
            i + 1,
            r.0,
            r.1,
            r.2,
            r.3,
            r.4,
            r.5,
            r.6
        );
    }
}
