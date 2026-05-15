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
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
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
    let n1 = day.rl - day.r;
    let p1 = day.rh + day.r;
    let p2 = day.rh + cfg.target_mult * day.r;
    let n2 = day.rl - cfg.target_mult * day.r;

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
    let target = if side == Side::Long {
        p2 - slip
    } else {
        n2 + slip
    };
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
    let wr = if n > 0 {
        w as f64 / n as f64 * 100.0
    } else {
        0.0
    };
    let exp = if n > 0 { sum / n as f64 } else { 0.0 };
    (n, wr, exp)
}

fn main() {
    let days = load_days();
    let configs = [
        Cfg {
            target_mult: 1.0,
            stop_mult: 1.0,
            confirm_bars: 1,
            latest_touch_hour: 10,
        },
        Cfg {
            target_mult: 1.0,
            stop_mult: 1.5,
            confirm_bars: 1,
            latest_touch_hour: 10,
        },
        Cfg {
            target_mult: 1.0,
            stop_mult: 1.5,
            confirm_bars: 2,
            latest_touch_hour: 10,
        },
        Cfg {
            target_mult: 2.25,
            stop_mult: 1.0,
            confirm_bars: 1,
            latest_touch_hour: 10,
        },
        Cfg {
            target_mult: 2.25,
            stop_mult: 1.5,
            confirm_bars: 1,
            latest_touch_hour: 10,
        },
    ];

    let folds = 6usize;
    let win = days.len() / (folds + 1);

    println!("MNQ 9:00-9:15 transition robustness (rolling walk-forward)");
    println!("Rule: require each fold IS>=0 and OOS>=0");

    for (idx, cfg) in configs.iter().enumerate() {
        let mut pass = 0usize;
        let mut total_oos = 0.0;
        let mut total_oos_n = 0usize;
        for f in 0..folds {
            let tr_start = f * win;
            let tr_end = tr_start + win;
            let te_start = tr_end;
            let te_end = (te_start + win).min(days.len());
            if te_end <= te_start || tr_end > days.len() {
                continue;
            }
            let train = &days[tr_start..tr_end];
            let test = &days[te_start..te_end];
            let (n1, _, e1) = eval(train, *cfg);
            let (n2, _, e2) = eval(test, *cfg);
            if n1 >= 40 && n2 >= 20 {
                if e1 >= 0.0 && e2 >= 0.0 {
                    pass += 1;
                }
                total_oos += e2 * n2 as f64;
                total_oos_n += n2;
            }
        }
        let avg_oos = if total_oos_n > 0 {
            total_oos / total_oos_n as f64
        } else {
            0.0
        };
        println!(
            "{}. target={}R stop={}R confirm<={} latest<= {}:59 | pass_folds={}/{} oos_weighted_exp={:.3}R",
            idx+1, cfg.target_mult, cfg.stop_mult, cfg.confirm_bars, cfg.latest_touch_hour, pass, folds, avg_oos
        );
    }
}
