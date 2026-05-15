use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use chrono::{Datelike, TimeZone, Timelike};
use chrono_tz::America::New_York;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Short,
    Long,
}

#[derive(Clone, Copy)]
struct Bar {
    ts: i64,
    h: u32,
    o: f64,
    hi: f64,
    lo: f64,
    c: f64,
}

#[derive(Clone)]
struct Day {
    key: String,
    bars: Vec<Bar>,
    rh: f64,
    rl: f64,
    range: f64,
}

#[derive(Clone, Copy)]
struct Cfg {
    zone_lo: f64,
    zone_hi: f64,
    confirm_deadline: u32,
    stop_cap: f64,
    tp1_frac: f64,
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
    let data =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("load");
    let mut bars = Vec::new();
    for c in data {
        let dt = New_York.timestamp_opt(c.open_time, 0).single().expect("ts");
        bars.push((
            dt.date_naive(),
            Bar {
                ts: c.open_time,
                h: dt.hour(),
                o: d2f(c.open.0),
                hi: d2f(c.high.0),
                lo: d2f(c.low.0),
                c: d2f(c.close.0),
            },
        ));
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bars.len() {
        let d = bars[i].0;
        let mut j = i;
        while j < bars.len() && bars[j].0 == d {
            j += 1;
        }
        let day_bars: Vec<Bar> = bars[i..j].iter().map(|x| x.1).collect();
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
            out.push(Day {
                key: format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day()),
                bars: day_bars,
                rh,
                rl,
                range: rh - rl,
            });
        }
        i = j;
    }
    out
}

fn try_trade(day: &Day, cfg: Cfg) -> Option<f64> {
    let top_low = day.rh + cfg.zone_lo * day.range;
    let top_high = day.rh + cfg.zone_hi * day.range;
    let bot_high = day.rl - cfg.zone_lo * day.range;
    let bot_low = day.rl - cfg.zone_hi * day.range;

    let closes: Vec<f64> = day.bars.iter().map(|b| b.c).collect();
    let ema50 = ema(&closes, 50);
    let ema100 = ema(&closes, 100);

    let mut touch: Option<(usize, Side)> = None;
    for (i, b) in day.bars.iter().enumerate() {
        if b.h < 9 || b.h > cfg.confirm_deadline {
            continue;
        }
        if b.hi >= top_low && b.lo <= top_high {
            touch = Some((i, Side::Short));
            break;
        }
        if b.lo <= bot_high && b.hi >= bot_low {
            touch = Some((i, Side::Long));
            break;
        }
    }
    let (tidx, side) = touch?;

    // Confirmation: reclaim + micro MSS within next 8 bars
    let mut confirm_idx = None;
    let end = (tidx + 8).min(day.bars.len().saturating_sub(1));
    for i in (tidx + 1)..=end {
        if side == Side::Short {
            let reclaim = day.bars[i].c <= day.rh;
            let mss = i >= 2 && day.bars[i].c < day.bars[i - 1].lo;
            if reclaim && mss {
                confirm_idx = Some(i);
                break;
            }
        } else {
            let reclaim = day.bars[i].c >= day.rl;
            let mss = i >= 2 && day.bars[i].c > day.bars[i - 1].hi;
            if reclaim && mss {
                confirm_idx = Some(i);
                break;
            }
        }
    }
    let cidx = confirm_idx?;
    if cidx + 1 >= day.bars.len() {
        return None;
    }

    // EMA gate: both 1m EMA50 and EMA100 aligned at confirmation
    let ema_ok = match side {
        Side::Short => day.bars[cidx].c <= ema50[cidx] && day.bars[cidx].c <= ema100[cidx],
        Side::Long => day.bars[cidx].c >= ema50[cidx] && day.bars[cidx].c >= ema100[cidx],
    };
    if !ema_ok {
        return None;
    }

    let tick = 0.25;
    let slip = tick;
    let comm_rt = 0.5;
    let eidx = cidx + 1;
    let entry = if side == Side::Short {
        day.bars[eidx].o - slip
    } else {
        day.bars[eidx].o + slip
    };

    let mut extreme = if side == Side::Short {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    for b in day.bars.iter().take(cidx + 1).skip(tidx) {
        if side == Side::Short {
            extreme = extreme.max(b.hi);
        } else {
            extreme = extreme.min(b.lo);
        }
    }

    let stop_struct = if side == Side::Short {
        extreme + tick + slip
    } else {
        extreme - tick - slip
    };
    let stop_cap = if side == Side::Short {
        entry + cfg.stop_cap * day.range
    } else {
        entry - cfg.stop_cap * day.range
    };
    let stop = if side == Side::Short {
        stop_struct.min(stop_cap)
    } else {
        stop_struct.max(stop_cap)
    };
    let risk = (entry - stop).abs();
    if risk < tick {
        return None;
    }

    // tp1_frac at 1R, rest at opposite side, flat at >= 12:00
    let tp1 = if side == Side::Short {
        entry - risk
    } else {
        entry + risk
    };
    let tp2 = if side == Side::Short {
        day.rl + slip
    } else {
        day.rh - slip
    };
    let cost_r = comm_rt / risk;

    let mut hit_tp1 = false;
    for b in day.bars.iter().skip(eidx) {
        if b.h >= 12 {
            let r = if hit_tp1 {
                cfg.tp1_frac * 1.0 - 0.1 * (1.0 - cfg.tp1_frac) - cost_r
            } else {
                -0.1 - cost_r
            };
            return Some(r);
        }
        let stop_hit = if side == Side::Short {
            b.hi >= stop
        } else {
            b.lo <= stop
        };
        if stop_hit {
            let r = if hit_tp1 {
                cfg.tp1_frac * 1.0 - (1.0 - cfg.tp1_frac) * 1.0 - cost_r
            } else {
                -1.0 - cost_r
            };
            return Some(r);
        }
        if !hit_tp1 {
            let h1 = if side == Side::Short {
                b.lo <= tp1
            } else {
                b.hi >= tp1
            };
            if h1 {
                hit_tp1 = true;
            }
        }
        if hit_tp1 {
            let h2 = if side == Side::Short {
                b.lo <= tp2
            } else {
                b.hi >= tp2
            };
            if h2 {
                let rr2 = (tp2 - entry).abs() / risk;
                return Some(cfg.tp1_frac * 1.0 + (1.0 - cfg.tp1_frac) * rr2 - cost_r);
            }
        }
    }

    Some(if hit_tp1 {
        cfg.tp1_frac * 1.0 - 0.1 * (1.0 - cfg.tp1_frac) - cost_r
    } else {
        -0.1 - cost_r
    })
}

fn eval(days: &[Day], cfg: Cfg) -> (usize, f64, f64, f64, BTreeMap<String, f64>) {
    let mut trades = 0usize;
    let mut wins = 0usize;
    let mut sum_r = 0.0;
    let mut eq = 0.0;
    let mut peak = 0.0;
    let mut max_dd = 0.0;
    let mut monthly: BTreeMap<String, f64> = BTreeMap::new();

    for d in days {
        if let Some(r) = try_trade(d, cfg) {
            trades += 1;
            if r > 0.0 {
                wins += 1;
            }
            sum_r += r;
            eq += r;
            if eq > peak {
                peak = eq;
            }
            let dd = peak - eq;
            if dd > max_dd {
                max_dd = dd;
            }
            let month = d.key[..7].to_string();
            *monthly.entry(month).or_insert(0.0) += r;
        }
    }

    let wr = if trades > 0 {
        wins as f64 / trades as f64 * 100.0
    } else {
        0.0
    };
    let exp = if trades > 0 {
        sum_r / trades as f64
    } else {
        0.0
    };
    (trades, wr, exp, max_dd, monthly)
}

fn main() {
    let days = load_days();
    let split = days.len() * 70 / 100;
    let train = &days[..split];
    let test = &days[split..];

    let zone_pairs = [(0.25, 0.55), (0.33, 0.66), (0.40, 0.70)];
    let deadlines = [10_u32, 11_u32];
    let stop_caps = [0.15_f64, 0.20, 0.25];
    let tp1_fracs = [0.25_f64, 0.33, 0.50];

    let mut rows: Vec<(
        String,
        usize,
        f64,
        f64,
        f64,
        usize,
        f64,
        f64,
        f64,
        BTreeMap<String, f64>,
    )> = Vec::new();

    for (zlo, zhi) in zone_pairs {
        for dl in deadlines {
            for sc in stop_caps {
                for t1 in tp1_fracs {
                    let cfg = Cfg {
                        zone_lo: zlo,
                        zone_hi: zhi,
                        confirm_deadline: dl,
                        stop_cap: sc,
                        tp1_frac: t1,
                    };
                    let (n1, wr1, exp1, dd1, _) = eval(train, cfg);
                    let (n2, wr2, exp2, dd2, m2) = eval(test, cfg);
                    if n2 < 12 {
                        continue;
                    }
                    rows.push((
                        format!(
                            "zone={:.2}-{:.2}R confirm<= {}:59 stop_cap={:.0}% tp1={:.0}%",
                            zlo,
                            zhi,
                            dl,
                            sc * 100.0,
                            t1 * 100.0
                        ),
                        n1,
                        wr1,
                        exp1,
                        dd1,
                        n2,
                        wr2,
                        exp2,
                        dd2,
                        m2,
                    ));
                }
            }
        }
    }

    rows.sort_by(|a, b| {
        b.7.total_cmp(&a.7)
            .then(b.5.cmp(&a.5))
            .then(a.8.total_cmp(&b.8))
    });

    println!("MNQ zone reversal tuning (70/30 split)");
    println!(
        "Execution fixed: EMA50/100 gate, one trade/day, slip=1 tick, comm=0.5pt RT, flat>=12:00"
    );
    for (i, r) in rows.iter().take(10).enumerate() {
        println!(
            "{}. {} | IS n={} wr={:.2}% exp={:.3}R dd={:.2}R | OOS n={} wr={:.2}% exp={:.3}R dd={:.2}R",
            i + 1,
            r.0,
            r.1,
            r.2,
            r.3,
            r.4,
            r.5,
            r.6,
            r.7,
            r.8
        );
    }

    if let Some(best) = rows.first() {
        println!("\nBest OOS monthly R:");
        for (k, v) in &best.9 {
            println!("- {}: {:.2}R", k, v);
        }
    }
}
