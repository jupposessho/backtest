use backtest::to_new_york_time;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct SessionDef {
    name: &'static str,
    start: (u32, u32),
    end: (u32, u32),
}

#[derive(Clone, Copy)]
struct LevelRef {
    session_name: &'static str,
    high: Decimal,
    low: Decimal,
    end_ts: i64,
}

#[derive(Clone, Copy)]
enum Dir {
    Long,
    Short,
}

#[derive(Clone, Copy)]
struct Setup {
    dir: Dir,
    entry_i: usize,
    stop: Decimal,
    target: Decimal,
}

#[derive(Clone, Copy)]
struct Trade {
    exit_i: usize,
    gross_r: Decimal,
    net_r: Decimal,
}

#[derive(Clone, Copy)]
struct Stats {
    trades: usize,
    wins: usize,
    gross_r: Decimal,
    net_r: Decimal,
    gross_profit_r: Decimal,
    gross_loss_r: Decimal,
}

#[derive(Default, Clone, Copy)]
struct Funnel {
    raw_sweeps: usize,
    raw_touches: usize,
    close_back_inside: usize,
    volume_ok: usize,
    entry_session_ok: usize,
    prior_level_ok: usize,
    setup_built: usize,
    target_ok: usize,
    risk_ok: usize,
    trades_closed: usize,
    debug_rows: usize,
}

#[derive(Debug, Deserialize)]
struct CsvBar {
    datetime: String,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
}

#[derive(Clone, Copy)]
struct Bar {
    open_time: i64,
    close_time: i64,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
}

const TICK_SIZE: Decimal = Decimal::from_parts(25, 0, 0, false, 2);
const STOP_BUFFER_TICKS: i64 = 1;
const MIN_SWEEP_TICKS: i64 = 2;
const VOL_SMA_LEN: usize = 20;
const VOL_MULT: Decimal = Decimal::from_parts(15, 0, 0, false, 1); // 1.5
const DEBUG_TOUCH_PRINTS: bool = false;
const MIN_TARGET_TICKS: i64 = 20;
const COOLDOWN_BARS_1M: usize = 30;
const COOLDOWN_BARS_3M: usize = 10;
const MIN_SESSION_RANGE_TICKS: i64 = 40;
const MAX_SESSION_RANGE_TICKS: i64 = 1200;
const PREV_DAY_MIN_RANGE_TICKS: i64 = 120;
const PREV_DAY_MAX_RANGE_TICKS: i64 = 2200;
const COMMISSION_PER_SIDE_USD: Decimal = Decimal::from_parts(62, 0, 0, false, 2);
const RISK_PER_TRADE_USD: Decimal = Decimal::from_parts(50, 0, 0, false, 0);

const ASIA: SessionDef = SessionDef {
    name: "ASIA",
    start: (20, 0),
    end: (0, 0),
};
const LONDON: SessionDef = SessionDef {
    name: "LONDON",
    start: (2, 0),
    end: (5, 0),
};
const NY_AM: SessionDef = SessionDef {
    name: "NYAM",
    start: (9, 30),
    end: (11, 30),
};
const NY_PM: SessionDef = SessionDef {
    name: "NYPM",
    start: (13, 0),
    end: (15, 0),
};

fn load_mnq_1m_with_volume() -> Vec<Bar> {
    let mut rdr = csv::Reader::from_path("/Users/waff/develop/play/nq/mnq_1m_cont.csv")
        .expect("open MNQ CSV");
    let mut out = Vec::new();
    for row in rdr.deserialize::<CsvBar>() {
        let r = row.expect("csv row");
        let dt = DateTime::parse_from_rfc3339(&r.datetime)
            .expect("datetime parse")
            .with_timezone(&Utc);
        let open_time = dt.timestamp();
        out.push(Bar {
            open_time,
            close_time: open_time + 60,
            open: r.open.parse::<Decimal>().expect("open decimal"),
            high: r.high.parse::<Decimal>().expect("high decimal"),
            low: r.low.parse::<Decimal>().expect("low decimal"),
            close: r.close.parse::<Decimal>().expect("close decimal"),
            volume: r.volume.parse::<Decimal>().expect("volume decimal"),
        });
    }
    out
}

fn resample(bars: &[Bar], minutes: i64) -> Vec<Bar> {
    if minutes <= 1 || bars.is_empty() {
        return bars.to_vec();
    }
    let bucket = minutes * 60;
    let mut out = Vec::new();
    let mut cur_bucket = bars[0].open_time - (bars[0].open_time % bucket);
    let mut o = bars[0].open;
    let mut h = bars[0].high;
    let mut l = bars[0].low;
    let mut c = bars[0].close;
    let mut v = bars[0].volume;

    for bar in bars.iter().copied() {
        let b = bar.open_time - (bar.open_time % bucket);
        if b != cur_bucket {
            out.push(Bar {
                open_time: cur_bucket,
                close_time: cur_bucket + bucket,
                open: o,
                high: h,
                low: l,
                close: c,
                volume: v,
            });
            cur_bucket = b;
            o = bar.open;
            h = bar.high;
            l = bar.low;
            c = bar.close;
            v = bar.volume;
        } else {
            if bar.high > h {
                h = bar.high;
            }
            if bar.low < l {
                l = bar.low;
            }
            c = bar.close;
            v += bar.volume;
        }
    }

    out.push(Bar {
        open_time: cur_bucket,
        close_time: cur_bucket + bucket,
        open: o,
        high: h,
        low: l,
        close: c,
        volume: v,
    });
    out
}

fn is_in_window(ts: i64, s: SessionDef) -> bool {
    let t = to_new_york_time(ts).time();
    let cur = (t.hour(), t.minute());
    if s.start <= s.end {
        cur >= s.start && cur < s.end
    } else {
        cur >= s.start || cur < s.end
    }
}

fn is_same_ny_day(a: i64, b: i64) -> bool {
    let da = to_new_york_time(a);
    let db = to_new_york_time(b);
    da.year() == db.year() && da.month() == db.month() && da.day() == db.day()
}

fn validate_data(data: &[Bar], expected_spacing_sec: i64) {
    assert!(!data.is_empty(), "empty dataset");
    for i in 0..data.len() {
        let b = data[i];
        assert!(b.high >= b.low, "OHLC invalid at {i}");
        assert!(b.high >= b.open && b.high >= b.close, "OHLC invalid at {i}");
        assert!(b.low <= b.open && b.low <= b.close, "OHLC invalid at {i}");
        if i > 0 {
            let prev = data[i - 1];
            assert!(b.open_time > prev.open_time, "timestamp not monotonic at {i}");
            let delta = b.open_time - prev.open_time;
            assert!(
                delta % expected_spacing_sec == 0,
                "unexpected spacing at {i}: {delta}"
            );
        }
    }
}

fn build_day_levels(data: &[Bar]) -> BTreeMap<String, Vec<LevelRef>> {
    let sessions = [ASIA, LONDON, NY_AM, NY_PM];
    let mut per_day_and_session: BTreeMap<String, BTreeMap<&'static str, LevelRef>> = BTreeMap::new();

    for s in sessions {
        let mut i = 0usize;
        while i < data.len() {
            while i < data.len() && !is_in_window(data[i].open_time, s) {
                i += 1;
            }
            if i >= data.len() {
                break;
            }

            let start_i = i;
            let mut hi = data[i].high;
            let mut lo = data[i].low;
            while i < data.len() && is_in_window(data[i].open_time, s) {
                hi = hi.max(data[i].high);
                lo = lo.min(data[i].low);
                i += 1;
            }

            let end_ts = data[i - 1].close_time;
            let anchor_ts = data[start_i].open_time;
            let d = session_trade_day(anchor_ts, s);
            let key = format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
            per_day_and_session.entry(key).or_default().insert(
                s.name,
                LevelRef {
                    session_name: s.name,
                    high: hi,
                    low: lo,
                    end_ts,
                },
            );
        }
    }

    let mut out: BTreeMap<String, Vec<LevelRef>> = BTreeMap::new();
    for (day_key, cur_map) in &per_day_and_session {
        let mut all = Vec::new();
        if let Some((_, prev_map)) = per_day_and_session.range(..day_key.clone()).next_back() {
            for s in sessions {
                if let Some(lvl) = prev_map.get(s.name) {
                    all.push(*lvl);
                }
            }
        }
        for s in sessions {
            if let Some(lvl) = cur_map.get(s.name) {
                all.push(*lvl);
            }
        }
        out.insert(day_key.clone(), all);
    }

    out
}

fn build_prev_day_regime_flags(data: &[Bar]) -> BTreeMap<String, bool> {
    let mut day_hi_lo: BTreeMap<String, (Decimal, Decimal)> = BTreeMap::new();
    for b in data {
        let d = to_new_york_time(b.open_time);
        let key = format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
        day_hi_lo
            .entry(key)
            .and_modify(|v| {
                v.0 = v.0.max(b.high);
                v.1 = v.1.min(b.low);
            })
            .or_insert((b.high, b.low));
    }

    let mut out = BTreeMap::new();
    for day in day_hi_lo.keys() {
        let Some((_, (hi, lo))) = day_hi_lo.range(..day.clone()).next_back() else {
            out.insert(day.clone(), false);
            continue;
        };
        let ticks = ((*hi - *lo) / TICK_SIZE).round_dp(0);
        let ok = ticks >= Decimal::from(PREV_DAY_MIN_RANGE_TICKS)
            && ticks <= Decimal::from(PREV_DAY_MAX_RANGE_TICKS);
        out.insert(day.clone(), ok);
    }
    out
}

fn session_trade_day(anchor_ts: i64, s: SessionDef) -> chrono::DateTime<chrono_tz::Tz> {
    let mut d = to_new_york_time(anchor_ts);
    if s.name == "ASIA" {
        d += Duration::days(1);
    }
    d
}

fn is_entry_session(ts: i64) -> bool {
    is_in_window(ts, LONDON) || is_in_window(ts, NY_AM) || is_in_window(ts, NY_PM)
}

fn entry_session_name(ts: i64) -> Option<&'static str> {
    if is_in_window(ts, LONDON) {
        Some(LONDON.name)
    } else if is_in_window(ts, NY_AM) {
        Some(NY_AM.name)
    } else if is_in_window(ts, NY_PM) {
        Some(NY_PM.name)
    } else {
        None
    }
}

fn level_allowed_for_entry(entry_session: &str, level_session: &str) -> bool {
    match entry_session {
        "LONDON" => level_session == "ASIA",
        "NYAM" | "NYPM" => level_session == "LONDON" || level_session == "ASIA",
        _ => false,
    }
}

fn volume_ok(i: usize, data: &[Bar], vol_mult: Decimal) -> bool {
    if i < VOL_SMA_LEN {
        return false;
    }
    let mut sum = Decimal::ZERO;
    for bar in data.iter().take(i).skip(i - VOL_SMA_LEN) {
        sum += bar.volume;
    }
    let avg = sum / Decimal::from(VOL_SMA_LEN as i64);
    data[i].volume >= avg * vol_mult
}

fn make_setup(
    i: usize,
    data: &[Bar],
    levels: &[LevelRef],
    min_sweep_ticks: i64,
    vol_mult: Decimal,
    f: &mut Funnel,
) -> Option<Setup> {
    let b = data[i];
    let vol_ok = volume_ok(i, data, vol_mult);
    if vol_ok {
        f.volume_ok += 1;
    }
    let ts = b.open_time;
    let Some(ent_session) = entry_session_name(ts) else {
        return None;
    };
    let min_sweep = TICK_SIZE * Decimal::from(min_sweep_ticks);
    let touch_eps = Decimal::ZERO;

    for lvl in levels {
        if !level_allowed_for_entry(ent_session, lvl.session_name) {
            continue;
        }
        let session_range_ticks = ((lvl.high - lvl.low) / TICK_SIZE).round_dp(0);
        if session_range_ticks < Decimal::from(MIN_SESSION_RANGE_TICKS)
            || session_range_ticks > Decimal::from(MAX_SESSION_RANGE_TICKS)
        {
            continue;
        }
        let mature = lvl.end_ts < ts;
        if mature {
            f.prior_level_ok += 1;
        }
        if !mature {
            continue;
        }

        let swept_high = b.high >= lvl.high + min_sweep;
        let swept_low = b.low <= lvl.low - min_sweep;
        let touched_high = b.high >= lvl.high + touch_eps;
        let touched_low = b.low <= lvl.low - touch_eps;
        if touched_high || touched_low {
            f.raw_touches += 1;
        }
        if swept_high || swept_low {
            f.raw_sweeps += 1;
        }

        if DEBUG_TOUCH_PRINTS && f.debug_rows < 40 && (touched_high || touched_low) {
            let t = to_new_york_time(ts);
            println!(
                "debug_touch      {} {:02}:{:02} entry_sess={} lvl_sess={} bh={} bl={} lh={} ll={} dh_ticks={} dl_ticks={}",
                t.date_naive(),
                t.hour(),
                t.minute(),
                ent_session,
                lvl.session_name,
                b.high,
                b.low,
                lvl.high,
                lvl.low,
                ((b.high - lvl.high) / TICK_SIZE).round_dp(2),
                ((lvl.low - b.low) / TICK_SIZE).round_dp(2),
            );
            f.debug_rows += 1;
        }

        if swept_high && b.close < lvl.high {
            f.close_back_inside += 1;
            if !vol_ok {
                continue;
            }
            
            let stop = b.high + TICK_SIZE * Decimal::from(STOP_BUFFER_TICKS);
            f.setup_built += 1;
            return Some(Setup {
                dir: Dir::Short,
                entry_i: i + 1,
                stop,
                target: lvl.low,
            });
        }

        if swept_low && b.close > lvl.low {
            f.close_back_inside += 1;
            if !vol_ok {
                continue;
            }

            let stop = b.low - TICK_SIZE * Decimal::from(STOP_BUFFER_TICKS);
            f.setup_built += 1;
            return Some(Setup {
                dir: Dir::Long,
                entry_i: i + 1,
                stop,
                target: lvl.high,
            });
        }
    }

    None
}

fn simulate_trade(
    data: &[Bar],
    setup: Setup,
    slippage_ticks_per_side: i64,
    min_target_ticks: i64,
    f: &mut Funnel,
) -> Option<Trade> {
    if setup.entry_i >= data.len() {
        return None;
    }
    let entry_bar = data[setup.entry_i];
    let slip = TICK_SIZE * Decimal::from(slippage_ticks_per_side);
    let raw_entry = entry_bar.open;
    let entry = match setup.dir {
        Dir::Long => raw_entry + slip,
        Dir::Short => raw_entry - slip,
    };

    let risk = match setup.dir {
        Dir::Long => entry - setup.stop,
        Dir::Short => setup.stop - entry,
    };
    if risk <= Decimal::ZERO {
        return None;
    }
    f.risk_ok += 1;

    let target_ok = match setup.dir {
        Dir::Long => setup.target > entry,
        Dir::Short => setup.target < entry,
    };
    if !target_ok {
        return None;
    }

    let target_dist_ticks = match setup.dir {
        Dir::Long => (setup.target - entry) / TICK_SIZE,
        Dir::Short => (entry - setup.target) / TICK_SIZE,
    };
    if target_dist_ticks < Decimal::from(min_target_ticks) {
        return None;
    }
    f.target_ok += 1;

    for i in setup.entry_i..data.len() {
        if i > setup.entry_i && !is_same_ny_day(data[setup.entry_i].open_time, data[i].open_time) {
            let mut eod_exit = data[i - 1].close;
            eod_exit = match setup.dir {
                Dir::Long => eod_exit - slip,
                Dir::Short => eod_exit + slip,
            };

            let gross_r = match setup.dir {
                Dir::Long => (eod_exit - entry) / risk,
                Dir::Short => (entry - eod_exit) / risk,
            };
            let cost_r = ((COMMISSION_PER_SIDE_USD * Decimal::from(2)) / RISK_PER_TRADE_USD)
                + ((slip / risk) * Decimal::from(2));
            return Some(Trade {
                exit_i: i - 1,
                gross_r,
                net_r: gross_r - cost_r,
            });
        }

        let bar = data[i];
        let hit_tp = match setup.dir {
            Dir::Long => bar.high >= setup.target,
            Dir::Short => bar.low <= setup.target,
        };
        let hit_sl = match setup.dir {
            Dir::Long => bar.low <= setup.stop,
            Dir::Short => bar.high >= setup.stop,
        };

        if hit_sl || hit_tp {
            let exit_raw = if hit_sl { setup.stop } else { setup.target };
            let exit = match setup.dir {
                Dir::Long => exit_raw - slip,
                Dir::Short => exit_raw + slip,
            };
            let gross_r = match setup.dir {
                Dir::Long => (exit - entry) / risk,
                Dir::Short => (entry - exit) / risk,
            };
            let cost_r = ((COMMISSION_PER_SIDE_USD * Decimal::from(2)) / RISK_PER_TRADE_USD)
                + ((slip / risk) * Decimal::from(2));
            return Some(Trade {
                exit_i: i,
                gross_r,
                net_r: gross_r - cost_r,
            });
        }
    }
    None
}

fn run(
    data: &[Bar],
    slippage_ticks_per_side: i64,
    min_sweep_ticks: i64,
    vol_mult: Decimal,
    min_target_ticks: i64,
    cooldown_bars: usize,
    f: &mut Funnel,
) -> Vec<Trade> {
    let levels_by_day = build_day_levels(data);
    let regime_by_day = build_prev_day_regime_flags(data);
    let mut trades = Vec::new();
    let mut i = VOL_SMA_LEN;
    let mut last_trade_exit_i: Option<usize> = None;
    let mut traded_london_long = false;
    let mut traded_london_short = false;
    let mut traded_nyam_long = false;
    let mut traded_nyam_short = false;
    let mut traded_nypm_long = false;
    let mut traded_nypm_short = false;
    let mut current_day_key = String::new();

    while i + 1 < data.len() {
        let dt = to_new_york_time(data[i].open_time);
        let day_key = format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day());
        if day_key != current_day_key {
            current_day_key = day_key.clone();
            traded_london_long = false;
            traded_london_short = false;
            traded_nyam_long = false;
            traded_nyam_short = false;
            traded_nypm_long = false;
            traded_nypm_short = false;
        }

        if !is_entry_session(data[i].open_time) {
            i += 1;
            continue;
        }
        f.entry_session_ok += 1;

        if let Some(last_exit) = last_trade_exit_i {
            if i <= last_exit.saturating_add(cooldown_bars) {
                i += 1;
                continue;
            }
        }

        let levels = match levels_by_day.get(&day_key) {
            Some(v) => v,
            None => {
                i += 1;
                continue;
            }
        };
        if !regime_by_day.get(&day_key).copied().unwrap_or(false) {
            i += 1;
            continue;
        }

        if let Some(setup) = make_setup(i, data, levels, min_sweep_ticks, vol_mult, f) {
            if let Some(tr) = simulate_trade(
                data,
                setup,
                slippage_ticks_per_side,
                min_target_ticks,
                f,
            ) {
                let session = entry_session_name(data[i].open_time).unwrap_or("NONE");
                let allowed = match (session, setup.dir) {
                    ("LONDON", Dir::Long) => !traded_london_long,
                    ("LONDON", Dir::Short) => !traded_london_short,
                    ("NYAM", Dir::Long) => !traded_nyam_long,
                    ("NYAM", Dir::Short) => !traded_nyam_short,
                    ("NYPM", Dir::Long) => !traded_nypm_long,
                    ("NYPM", Dir::Short) => !traded_nypm_short,
                    _ => false,
                };
                if !allowed {
                    i += 1;
                    continue;
                }

                match (session, setup.dir) {
                    ("LONDON", Dir::Long) => traded_london_long = true,
                    ("LONDON", Dir::Short) => traded_london_short = true,
                    ("NYAM", Dir::Long) => traded_nyam_long = true,
                    ("NYAM", Dir::Short) => traded_nyam_short = true,
                    ("NYPM", Dir::Long) => traded_nypm_long = true,
                    ("NYPM", Dir::Short) => traded_nypm_short = true,
                    _ => {}
                }

                i = tr.exit_i.saturating_add(1);
                last_trade_exit_i = Some(tr.exit_i);
                trades.push(tr);
                f.trades_closed += 1;
                continue;
            }
        }
        i += 1;
    }

    trades
}

fn summarize(trades: &[Trade]) -> Stats {
    let mut s = Stats {
        trades: trades.len(),
        wins: 0,
        gross_r: Decimal::ZERO,
        net_r: Decimal::ZERO,
        gross_profit_r: Decimal::ZERO,
        gross_loss_r: Decimal::ZERO,
    };
    for t in trades {
        s.gross_r += t.gross_r;
        s.net_r += t.net_r;
        if t.gross_r > Decimal::ZERO {
            s.wins += 1;
            s.gross_profit_r += t.gross_r;
        } else {
            s.gross_loss_r += t.gross_r.abs();
        }
    }
    s
}

fn print_row(label: &str, st: Stats) {
    let wr = if st.trades > 0 {
        Decimal::from(st.wins as i64) / Decimal::from(st.trades as i64) * Decimal::from(100)
    } else {
        Decimal::ZERO
    };
    let pf = if st.gross_loss_r > Decimal::ZERO {
        st.gross_profit_r / st.gross_loss_r
    } else {
        Decimal::ZERO
    };
    let gross_usd = st.gross_r * RISK_PER_TRADE_USD;
    let net_usd = st.net_r * RISK_PER_TRADE_USD;
    println!(
        "{:<14} trades={:<5} win%={:<6} PF={:<5} grossR={:<8} netR={:<8} gross$={:<10} net$={:<10}",
        label,
        st.trades,
        format!("{:.1}", wr),
        format!("{:.2}", pf),
        format!("{:.2}", st.gross_r),
        format!("{:.2}", st.net_r),
        format!("{:.2}", gross_usd),
        format!("{:.2}", net_usd)
    );
}

fn split_half(data: &[Bar]) -> (Vec<Bar>, Vec<Bar>) {
    let mid = data.len() / 2;
    (data[..mid].to_vec(), data[mid..].to_vec())
}

fn run_suite(
    label: &str,
    data: &[Bar],
    min_sweep_ticks: i64,
    vol_mult: Decimal,
    min_target_ticks: i64,
) {
    println!("\n=== {label} ===");
    for slip in [1i64, 2, 3] {
        let mut f = Funnel::default();
        let trades = run(
            data,
            slip,
            min_sweep_ticks,
            vol_mult,
            min_target_ticks,
            if label.contains("1m") { COOLDOWN_BARS_1M } else { COOLDOWN_BARS_3M },
            &mut f,
        );
        print_row(&format!("slip{}", slip), summarize(&trades));
        if slip == 1 {
            println!(
                "funnel          session_bars={} prior_levels={} raw_touches={} raw_sweeps={} close_back={} vol_ok_bars={} setups={} target_ok={} risk_ok={} trades={}",
                f.entry_session_ok,
                f.prior_level_ok,
                f.raw_touches,
                f.raw_sweeps,
                f.close_back_inside,
                f.volume_ok,
                f.setup_built,
                f.target_ok,
                f.risk_ok,
                f.trades_closed
            );
        }
    }

    let (a, b) = split_half(data);
    let mut f_a = Funnel::default();
    let mut f_b = Funnel::default();
    let a_st = summarize(&run(
        &a,
        1,
        min_sweep_ticks,
        vol_mult,
        min_target_ticks,
        if label.contains("1m") { COOLDOWN_BARS_1M } else { COOLDOWN_BARS_3M },
        &mut f_a,
    ));
    let b_st = summarize(&run(
        &b,
        1,
        min_sweep_ticks,
        vol_mult,
        min_target_ticks,
        if label.contains("1m") { COOLDOWN_BARS_1M } else { COOLDOWN_BARS_3M },
        &mut f_b,
    ));
    println!(
        "split_check      first_half_net$={:.2} second_half_net$={:.2}",
        (a_st.net_r * RISK_PER_TRADE_USD).round_dp(2),
        (b_st.net_r * RISK_PER_TRADE_USD).round_dp(2)
    );
}

fn main() {
    let one_min = load_mnq_1m_with_volume();
    validate_data(&one_min, 60);
    let three_min = resample(&one_min, 3);
    validate_data(&three_min, 180);

    run_suite(
        "MNQ 1m turtle-soup vol",
        &one_min,
        MIN_SWEEP_TICKS,
        VOL_MULT,
        MIN_TARGET_TICKS,
    );
    run_suite(
        "MNQ 3m turtle-soup vol",
        &three_min,
        MIN_SWEEP_TICKS,
        VOL_MULT,
        MIN_TARGET_TICKS,
    );

    println!("\n=== 3m parameter sweep (slip1/slip2) ===");
    let sweep_ticks = [1i64, 2, 3];
    let sweep_vol = [
        Decimal::from_parts(12, 0, 0, false, 1),
        Decimal::from_parts(13, 0, 0, false, 1),
        Decimal::from_parts(15, 0, 0, false, 1),
    ];
    let sweep_target = [20i64, 30, 40];
    for t in sweep_ticks {
        for vm in sweep_vol {
            for mt in sweep_target {
                let mut f1 = Funnel::default();
                let mut f2 = Funnel::default();
                let st1 = summarize(&run(&three_min, 1, t, vm, mt, COOLDOWN_BARS_3M, &mut f1));
                let st2 = summarize(&run(&three_min, 2, t, vm, mt, COOLDOWN_BARS_3M, &mut f2));
                println!(
                    "grid            sweep_ticks={} vol_mult={:.1} min_target_ticks={} | slip1 net$={:.2} trades={} PF={:.2} | slip2 net$={:.2} trades={} PF={:.2}",
                    t,
                    vm,
                    mt,
                    (st1.net_r * RISK_PER_TRADE_USD).round_dp(2),
                    st1.trades,
                    if st1.gross_loss_r > Decimal::ZERO {
                        st1.gross_profit_r / st1.gross_loss_r
                    } else {
                        Decimal::ZERO
                    },
                    (st2.net_r * RISK_PER_TRADE_USD).round_dp(2),
                    st2.trades,
                    if st2.gross_loss_r > Decimal::ZERO {
                        st2.gross_profit_r / st2.gross_loss_r
                    } else {
                        Decimal::ZERO
                    }
                );
            }
        }
    }

    println!("\nRealism Validation");
    println!(
        "- Fees model: ${:.2}/side commission, reported in netR and net$",
        COMMISSION_PER_SIDE_USD
    );
    println!("- Slippage scenarios: 1/2/3 ticks per side");
    println!("- Entry model: next-bar-open after turtle-soup reversal bar");
    println!("- Gap-stop handling: stop/target evaluated intrabar; if both touched SL wins");
    println!("- Gross vs net: both shown per scenario");
    println!("- Sensitivity conclusion: inspect slip2/slip3 net$ vs slip1 before promotion");

    let _ = TICK_SIZE.to_f64();
}
