use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy)]
struct Levels {
    p1: f64,
    p2_lo: f64,
    p2_hi: f64,
    p4: f64,
    p45: f64,
    n1: f64,
    n2_hi: f64,
    n2_lo: f64,
    n4: f64,
    n45: f64,
}

#[derive(Default, Clone, Copy)]
struct PairStat {
    starts: usize,
    hits: usize,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn main() {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");

    let mut valid_days = 0usize;

    // Requested transitions
    let mut p1_to_n1 = PairStat::default();
    let mut p1_to_n2 = PairStat::default();
    let mut p1_to_n4 = PairStat::default();
    let mut p2_to_n4 = PairStat::default();

    let mut n1_to_p1 = PairStat::default();
    let mut n1_to_p2 = PairStat::default();
    let mut n1_to_p4 = PairStat::default();
    let mut n2_to_p4 = PairStat::default();

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

        let day = &candles[i..j];
        let mut rh = f64::NEG_INFINITY;
        let mut rl = f64::INFINITY;
        let mut has_base = false;
        let mut start = None;

        for (k, c) in day.iter().enumerate() {
            let dt = New_York
                .timestamp_opt(c.open_time, 0)
                .single()
                .expect("ts");
            let h = dt.hour();
            let m = dt.minute();
            if h == 9 && m < 15 {
                has_base = true;
                rh = rh.max(d2f(c.high.0));
                rl = rl.min(d2f(c.low.0));
            }
            if (h > 9 || (h == 9 && m >= 15)) && start.is_none() {
                start = Some(k);
            }
        }

        if !has_base || rh <= rl || start.is_none() {
            i = j;
            continue;
        }
        valid_days += 1;

        let range = rh - rl;
        let lv = Levels {
            p1: rh + 1.0 * range,
            p2_lo: rh + 2.0 * range,
            p2_hi: rh + 2.5 * range,
            p4: rh + 4.0 * range,
            p45: rh + 4.5 * range,
            n1: rl - 1.0 * range,
            n2_hi: rl - 2.0 * range,
            n2_lo: rl - 2.5 * range,
            n4: rl - 4.0 * range,
            n45: rl - 4.5 * range,
        };

        let s = start.expect("start");

        let mut first_p1 = None;
        let mut first_n1 = None;
        let mut first_p2 = None;
        let mut first_n2 = None;

        for (k, c) in day.iter().enumerate().skip(s) {
            let hi = d2f(c.high.0);
            let lo = d2f(c.low.0);

            if first_p1.is_none() && hi >= lv.p1 {
                first_p1 = Some(k);
            }
            if first_n1.is_none() && lo <= lv.n1 {
                first_n1 = Some(k);
            }
            if first_p2.is_none() && hi >= lv.p2_lo && lo <= lv.p2_hi {
                first_p2 = Some(k);
            }
            if first_n2.is_none() && lo <= lv.n2_hi && hi >= lv.n2_lo {
                first_n2 = Some(k);
            }
        }

        if let Some(k) = first_p1 {
            p1_to_n1.starts += 1;
            p1_to_n2.starts += 1;
            p1_to_n4.starts += 1;
            let mut hit_n1 = false;
            let mut hit_n2 = false;
            let mut hit_n4 = false;
            for c in day.iter().skip(k) {
                let hi = d2f(c.high.0);
                let lo = d2f(c.low.0);
                if lo <= lv.n1 {
                    hit_n1 = true;
                }
                if lo <= lv.n2_hi && hi >= lv.n2_lo {
                    hit_n2 = true;
                }
                if lo <= lv.n4 && hi >= lv.n45 {
                    hit_n4 = true;
                }
            }
            if hit_n1 {
                p1_to_n1.hits += 1;
            }
            if hit_n2 {
                p1_to_n2.hits += 1;
            }
            if hit_n4 {
                p1_to_n4.hits += 1;
            }
        }

        if let Some(k) = first_p2 {
            p2_to_n4.starts += 1;
            let mut hit_n4 = false;
            for c in day.iter().skip(k) {
                let hi = d2f(c.high.0);
                let lo = d2f(c.low.0);
                if lo <= lv.n4 && hi >= lv.n45 {
                    hit_n4 = true;
                    break;
                }
            }
            if hit_n4 {
                p2_to_n4.hits += 1;
            }
        }

        if let Some(k) = first_n1 {
            n1_to_p1.starts += 1;
            n1_to_p2.starts += 1;
            n1_to_p4.starts += 1;
            let mut hit_p1 = false;
            let mut hit_p2 = false;
            let mut hit_p4 = false;
            for c in day.iter().skip(k) {
                let hi = d2f(c.high.0);
                let lo = d2f(c.low.0);
                if hi >= lv.p1 {
                    hit_p1 = true;
                }
                if hi >= lv.p2_lo && lo <= lv.p2_hi {
                    hit_p2 = true;
                }
                if hi >= lv.p4 && lo <= lv.p45 {
                    hit_p4 = true;
                }
            }
            if hit_p1 {
                n1_to_p1.hits += 1;
            }
            if hit_p2 {
                n1_to_p2.hits += 1;
            }
            if hit_p4 {
                n1_to_p4.hits += 1;
            }
        }

        if let Some(k) = first_n2 {
            n2_to_p4.starts += 1;
            let mut hit_p4 = false;
            for c in day.iter().skip(k) {
                let hi = d2f(c.high.0);
                let lo = d2f(c.low.0);
                if hi >= lv.p4 && lo <= lv.p45 {
                    hit_p4 = true;
                    break;
                }
            }
            if hit_p4 {
                n2_to_p4.hits += 1;
            }
        }

        i = j;
    }

    let print_pair = |label: &str, s: PairStat| {
        let rate = if s.starts > 0 {
            s.hits as f64 / s.starts as f64 * 100.0
        } else {
            0.0
        };
        println!("- {}: {}/{} = {:.2}%", label, s.hits, s.starts, rate);
    };

    println!("MNQ 9:00-9:15 transition stats");
    println!("Valid days: {}", valid_days);
    println!("Definition: after first source-touch, does price later hit target band by EOD");
    println!("\nUp -> Down transitions");
    print_pair("+1R -> -1R", p1_to_n1);
    print_pair("+1R -> -2.0/-2.5R", p1_to_n2);
    print_pair("+1R -> -4R", p1_to_n4);
    print_pair("+2.0/+2.5R -> -4.0/-4.5R", p2_to_n4);

    println!("\nDown -> Up transitions");
    print_pair("-1R -> +1R", n1_to_p1);
    print_pair("-1R -> +2.0/+2.5R", n1_to_p2);
    print_pair("-1R -> +4R", n1_to_p4);
    print_pair("-2.0/-2.5R -> +4.0/+4.5R", n2_to_p4);
}
