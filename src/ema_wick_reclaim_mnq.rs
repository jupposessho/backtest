use backtest::candle_stick_loader::CandleStickLoader;
use backtest::model::candle_stick::CandleStick;
use backtest::model::position_direction::PositionDirection;
use backtest::to_new_york_time;
use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use rayon::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct EmaPoint {
    ts: i64,
    value: Decimal,
}

#[derive(Clone, Copy)]
struct Trade {
    close_time: i64,
    pnl_usd: Decimal,
}

#[derive(Clone, Copy)]
enum SessionFilter {
    All,
    London,
    NyAm,
    NyOpen,
    NyLate,
}

#[derive(Clone, Copy)]
enum StopMode {
    Wick,
    Atr,
    Hybrid,
}

#[derive(Clone, Copy)]
enum EntryMode {
    Immediate,
    ObMidRetest,
}

#[derive(Clone, Copy)]
struct RunCfg {
    rr: Decimal,
    max_hold_bars: usize,
    min_wick_ticks: Decimal,
    min_stop_ticks: Decimal,
    atr_floor_mult: Decimal,
    atr_period: usize,
    max_cost_r: Decimal,
    tick_size: Decimal,
    fee_rt: Decimal,
    slippage_rt: Decimal,
    cost_filter_slippage_rt: Decimal,
    session: SessionFilter,
    stop_mode: StopMode,
    entry_mode: EntryMode,
    ob_wait_bars: usize,
    use_regime_filter: bool,
    regime_min_atr_ticks: Decimal,
    regime_max_atr_ticks: Decimal,
    require_micro_confirm: bool,
    use_dynamic_rr: bool,
    rr_low_vol: Decimal,
    rr_mid_vol: Decimal,
    rr_high_vol: Decimal,
    // 1) No-trade zone around EMA
    use_ema_distance_filter: bool,
    min_close_ema_dist_ticks: Decimal,
    // 2) Candle quality filter
    use_candle_quality_filter: bool,
    min_body_pct: Decimal,
    min_range_ticks: Decimal,
    // 3) Trend/structure alignment
    use_trend_structure_filter: bool,
    structure_lookback: usize,
    // 5) Loss-streak breaker
    use_loss_streak_breaker: bool,
    max_losses_per_day: usize,
}

#[derive(Clone)]
struct Row {
    label: String,
    trades: usize,
    win_rate: Decimal,
    net_usd: Decimal,
    avg_usd: Decimal,
}

fn load_mnq_1m() -> Vec<CandleStick> {
    CandleStickLoader::load_parquet("assets/mnq_1m_cont.parquet").expect("load mnq parquet")
}

fn resample(candles: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if candles.is_empty() {
        return Vec::new();
    }
    let bucket = minutes * 60;
    let mut out = Vec::new();

    let mut cur = candles[0].open_time - (candles[0].open_time % bucket);
    let mut o = candles[0].open;
    let mut h = candles[0].high;
    let mut l = candles[0].low;
    let mut c = candles[0].close;

    for x in candles.iter().copied() {
        let b = x.open_time - (x.open_time % bucket);
        if b != cur {
            out.push(CandleStick {
                open_time: cur,
                close_time: cur + bucket,
                open: o,
                high: h,
                low: l,
                close: c,
            });
            cur = b;
            o = x.open;
            h = x.high;
            l = x.low;
            c = x.close;
        } else {
            if x.high > h {
                h = x.high;
            }
            if x.low < l {
                l = x.low;
            }
            c = x.close;
        }
    }

    out.push(CandleStick {
        open_time: cur,
        close_time: cur + bucket,
        open: o,
        high: h,
        low: l,
        close: c,
    });

    out
}

fn ema_series(candles: &[CandleStick], period: usize) -> Vec<EmaPoint> {
    let mut out = Vec::new();
    if candles.len() < period {
        return out;
    }
    let k = Decimal::from_i64(2).unwrap() / Decimal::from_usize(period + 1).unwrap();
    let mut seed = Decimal::ZERO;
    for c in candles.iter().take(period) {
        seed += c.close.0;
    }
    let mut ema = seed / Decimal::from_usize(period).unwrap();
    out.push(EmaPoint {
        ts: candles[period - 1].close_time,
        value: ema,
    });
    for c in candles.iter().skip(period) {
        ema = c.close.0 * k + ema * (Decimal::ONE - k);
        out.push(EmaPoint {
            ts: c.close_time,
            value: ema,
        });
    }
    out
}

fn cutoff(date: &str) -> i64 {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("date");
    Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).expect("midnight")).timestamp()
}

fn dataset_start_ts(tf: &[CandleStick]) -> i64 {
    tf.first().map(|c| c.open_time).unwrap_or(0)
}

fn in_session(ts: i64, s: SessionFilter) -> bool {
    let t = to_new_york_time(ts).time();
    let hm = (t.hour(), t.minute());
    match s {
        SessionFilter::All => true,
        SessionFilter::London => hm >= (3, 0) && hm <= (5, 0),
        SessionFilter::NyAm => hm >= (9, 30) && hm <= (11, 30),
        SessionFilter::NyOpen => hm >= (9, 30) && hm <= (10, 30),
        SessionFilter::NyLate => hm >= (10, 30) && hm <= (11, 30),
    }
}

fn atr(tf: &[CandleStick], i: usize, period: usize) -> Decimal {
    if i + 1 < period {
        return Decimal::ZERO;
    }
    let mut sum = Decimal::ZERO;
    let start = i + 1 - period;
    for k in start..=i {
        let cur = tf[k];
        let prev_close = if k > 0 { tf[k - 1].close.0 } else { cur.close.0 };
        let tr1 = cur.high.0 - cur.low.0;
        let tr2 = (cur.high.0 - prev_close).abs();
        let tr3 = (cur.low.0 - prev_close).abs();
        sum += tr1.max(tr2).max(tr3);
    }
    sum / Decimal::from_usize(period).unwrap()
}

fn run(
    tf: &[CandleStick],
    ema_5m: &[EmaPoint],
    from_ts: i64,
    cfg: RunCfg,
) -> Vec<Trade> {
    let mut out = Vec::new();
    let mut i = 1usize;
    let mut eidx = 0usize;
    let mut current_day: Option<chrono::NaiveDate> = None;
    let mut loss_streak_today = 0usize;

    let point_value = Decimal::from(2);

    while i < tf.len() {
        let c = tf[i];
        let d = to_new_york_time(c.open_time).date_naive();
        if current_day != Some(d) {
            current_day = Some(d);
            loss_streak_today = 0;
        }

        if cfg.use_loss_streak_breaker && loss_streak_today >= cfg.max_losses_per_day {
            i += 1;
            continue;
        }

        if c.open_time < from_ts {
            i += 1;
            continue;
        }
        if !in_session(c.open_time, cfg.session) {
            i += 1;
            continue;
        }
        while eidx + 1 < ema_5m.len() && ema_5m[eidx + 1].ts <= c.close_time {
            eidx += 1;
        }
        if eidx >= ema_5m.len() || ema_5m[eidx].ts > c.close_time {
            i += 1;
            continue;
        }
        let ema = ema_5m[eidx].value;
        // 1) no-trade zone around EMA
        if cfg.use_ema_distance_filter {
            let dist_ticks = (c.close.0 - ema).abs() / cfg.tick_size;
            if dist_ticks < cfg.min_close_ema_dist_ticks {
                i += 1;
                continue;
            }
        }

        // 2) candle quality
        if cfg.use_candle_quality_filter {
            let range = c.high.0 - c.low.0;
            if range <= Decimal::ZERO {
                i += 1;
                continue;
            }
            let body = (c.close.0 - c.open.0).abs();
            let body_pct = body / range * Decimal::from(100);
            let range_ticks = range / cfg.tick_size;
            if body_pct < cfg.min_body_pct || range_ticks < cfg.min_range_ticks {
                i += 1;
                continue;
            }
        }

        let long_signal = c.low.0 < ema && c.close.0 > ema;
        let short_signal = c.high.0 > ema && c.close.0 < ema;
        if !long_signal && !short_signal {
            i += 1;
            continue;
        }

        let direction = if long_signal {
            PositionDirection::Long
        } else {
            PositionDirection::Short
        };

        // 3) trend/structure alignment
        if cfg.use_trend_structure_filter {
            if eidx == 0 || i < cfg.structure_lookback.max(2) {
                i += 1;
                continue;
            }
            let ema_slope_up = ema_5m[eidx].value > ema_5m[eidx - 1].value;
            let ema_slope_down = ema_5m[eidx].value < ema_5m[eidx - 1].value;

            let mut hh = true;
            let mut hl = true;
            let mut lh = true;
            let mut ll = true;
            let start = i + 1 - cfg.structure_lookback;
            for k in (start + 1)..=i {
                if tf[k].high.0 <= tf[k - 1].high.0 {
                    hh = false;
                }
                if tf[k].low.0 <= tf[k - 1].low.0 {
                    hl = false;
                }
                if tf[k].high.0 >= tf[k - 1].high.0 {
                    lh = false;
                }
                if tf[k].low.0 >= tf[k - 1].low.0 {
                    ll = false;
                }
            }

            let trend_ok = match direction {
                PositionDirection::Long => ema_slope_up && hh && hl,
                PositionDirection::Short => ema_slope_down && lh && ll,
            };
            if !trend_ok {
                i += 1;
                continue;
            }
        }

        let wick_pen = if direction == PositionDirection::Long {
            ema - c.low.0
        } else {
            c.high.0 - ema
        };
        if wick_pen < cfg.min_wick_ticks * cfg.tick_size {
            i += 1;
            continue;
        }

        let entry = c.close.0;
        let wick_risk = if direction == PositionDirection::Long {
            entry - c.low.0
        } else {
            c.high.0 - entry
        };
        let mut risk = wick_risk;
        let atr_v = atr(tf, i, cfg.atr_period);
        if cfg.use_regime_filter {
            let atr_ticks = if cfg.tick_size > Decimal::ZERO {
                atr_v / cfg.tick_size
            } else {
                Decimal::ZERO
            };
            if atr_ticks < cfg.regime_min_atr_ticks || atr_ticks > cfg.regime_max_atr_ticks {
                i += 1;
                continue;
            }
        }
        let atr_floor = atr_v * cfg.atr_floor_mult;
        let min_stop = cfg.min_stop_ticks * cfg.tick_size;
        let atr_stop = if atr_floor > min_stop { atr_floor } else { min_stop };
        match cfg.stop_mode {
            StopMode::Wick => {
                if min_stop > risk {
                    risk = min_stop;
                }
            }
            StopMode::Atr => {
                risk = atr_stop;
            }
            StopMode::Hybrid => {
                if atr_stop > risk {
                    risk = atr_stop;
                }
            }
        }
        if risk <= Decimal::ZERO {
            i += 1;
            continue;
        }
        let cost_r = (cfg.fee_rt + cfg.cost_filter_slippage_rt) / (risk * point_value);
        if cost_r > cfg.max_cost_r {
            i += 1;
            continue;
        }
        let signal_open = c.open.0;
        let signal_close = c.close.0;
        let ob_top = if signal_open > signal_close { signal_open } else { signal_close };
        let ob_bottom = if signal_open < signal_close { signal_open } else { signal_close };
        let mut actual_entry = entry;
        let mut entry_idx = i;
        if let EntryMode::ObMidRetest = cfg.entry_mode {
            let mid = (ob_top + ob_bottom) / Decimal::from(2);
            let mut found = false;
            let max_j = (i + cfg.ob_wait_bars).min(tf.len().saturating_sub(1));
            let mut j2 = i + 1;
            while j2 <= max_j {
                let nx = tf[j2];
                let touch = if direction == PositionDirection::Long {
                    nx.low.0 <= mid
                } else {
                    nx.high.0 >= mid
                };
                let confirm = if direction == PositionDirection::Long {
                    nx.close.0 > nx.open.0
                } else {
                    nx.close.0 < nx.open.0
                };
                if touch && confirm {
                    let next_idx = j2 + 1;
                    if next_idx >= tf.len() {
                        break;
                    }
                    actual_entry = tf[next_idx].open.0;
                    entry_idx = next_idx;
                    found = true;
                    break;
                }
                j2 += 1;
            }
            if !found {
                i += 1;
                continue;
            }
        }

        let sl = if direction == PositionDirection::Long { actual_entry - risk } else { actual_entry + risk };
        let rr_eff = if cfg.use_dynamic_rr {
            let atr_ticks = if cfg.tick_size > Decimal::ZERO {
                atr_v / cfg.tick_size
            } else {
                Decimal::ZERO
            };
            if atr_ticks < Decimal::from(6) {
                cfg.rr_low_vol
            } else if atr_ticks < Decimal::from(14) {
                cfg.rr_mid_vol
            } else {
                cfg.rr_high_vol
            }
        } else {
            cfg.rr
        };
        let tp = if direction == PositionDirection::Long {
            actual_entry + risk * rr_eff
        } else {
            actual_entry - risk * rr_eff
        };

        let mut pnl_points = Decimal::ZERO;
        let mut j = entry_idx + 1;
        let mut exit_ts = tf[entry_idx].close_time;
        let mut held = 0usize;
        while j < tf.len() {
            let nx = tf[j];
            let hit_sl = if direction == PositionDirection::Long {
                nx.low.0 <= sl
            } else {
                nx.high.0 >= sl
            };
            let hit_tp = if direction == PositionDirection::Long {
                nx.high.0 >= tp
            } else {
                nx.low.0 <= tp
            };
            if hit_sl && hit_tp {
                pnl_points = -risk;
                exit_ts = nx.close_time;
                break;
            }
            if hit_sl {
                pnl_points = -risk;
                exit_ts = nx.close_time;
                break;
            }
            if hit_tp {
                pnl_points = risk * rr_eff;
                exit_ts = nx.close_time;
                break;
            }
            if cfg.require_micro_confirm && held == 0 {
                let bad_confirm = if direction == PositionDirection::Long {
                    nx.close.0 < nx.open.0
                } else {
                    nx.close.0 > nx.open.0
                };
                if bad_confirm {
                    pnl_points = (nx.close.0 - actual_entry)
                        * if direction == PositionDirection::Long {
                            Decimal::ONE
                        } else {
                            -Decimal::ONE
                        };
                    exit_ts = nx.close_time;
                    break;
                }
            }
            held += 1;
            if held >= cfg.max_hold_bars {
                pnl_points = (nx.close.0 - actual_entry)
                    * if direction == PositionDirection::Long {
                        Decimal::ONE
                    } else {
                        -Decimal::ONE
                    };
                exit_ts = nx.close_time;
                break;
            }
            j += 1;
        }

        let pnl_usd = pnl_points * point_value - cfg.fee_rt - cfg.slippage_rt;
        if cfg.use_loss_streak_breaker {
            if pnl_usd < Decimal::ZERO {
                loss_streak_today += 1;
            } else if pnl_usd > Decimal::ZERO {
                loss_streak_today = 0;
            }
        }
        out.push(Trade {
            close_time: exit_ts,
            pnl_usd,
        });
        i = if j > i { j } else { i + 1 };
    }

    out
}

fn print_stats(label: &str, trades: &[Trade]) {
    let n = trades.len();
    let wins = trades.iter().filter(|t| t.pnl_usd > Decimal::ZERO).count();
    let losses = trades.iter().filter(|t| t.pnl_usd < Decimal::ZERO).count();
    let net: Decimal = trades.iter().map(|t| t.pnl_usd).sum();
    let avg = if n > 0 {
        net / Decimal::from_usize(n).unwrap()
    } else {
        Decimal::ZERO
    };
    let wr = if n > 0 {
        Decimal::from_usize(wins).unwrap() * Decimal::from(100) / Decimal::from_usize(n).unwrap()
    } else {
        Decimal::ZERO
    };

    println!("\n{}", label);
    println!("trades: {}", n);
    println!("wins/losses: {}/{}", wins, losses);
    println!("win rate: {}%", wr.round_dp(2));
    println!("net profit (1 MNQ micro): ${}", net.round_dp(2));
    println!("avg per trade: ${}", avg.round_dp(2));
}

fn evaluate(label: String, trades: &[Trade]) -> Row {
    let n = trades.len();
    let wins = trades.iter().filter(|t| t.pnl_usd > Decimal::ZERO).count();
    let net: Decimal = trades.iter().map(|t| t.pnl_usd).sum();
    let avg = if n > 0 { net / Decimal::from_usize(n).unwrap() } else { Decimal::ZERO };
    let wr = if n > 0 {
        Decimal::from_usize(wins).unwrap() * Decimal::from(100) / Decimal::from_usize(n).unwrap()
    } else {
        Decimal::ZERO
    };
    Row { label, trades: n, win_rate: wr, net_usd: net, avg_usd: avg }
}

fn monthly_breakdown(label: &str, trades: &[Trade]) {
    let mut m: BTreeMap<String, Decimal> = BTreeMap::new();
    for t in trades {
        let d = to_new_york_time(t.close_time);
        let k = format!("{:04}-{:02}", d.year(), d.month());
        *m.entry(k).or_insert(Decimal::ZERO) += t.pnl_usd;
    }
    let mut pos = 0usize;
    let mut neg = 0usize;
    for v in m.values() {
        if *v > Decimal::ZERO {
            pos += 1;
        } else if *v < Decimal::ZERO {
            neg += 1;
        }
    }
    println!("\n{} monthly: +{} / -{} / total {}", label, pos, neg, m.len());
    for (k, v) in m {
        println!("{}: ${}", k, v.round_dp(2));
    }
}

fn slippage_stress(label: &str, tf: &[CandleStick], ema: &[EmaPoint], from_ts: i64, base: RunCfg) {
    println!("\n{} slippage stress", label);
    for slip in [Decimal::ONE, Decimal::new(15, 1), Decimal::from(2)] {
        let mut c = base;
        c.slippage_rt = slip;
        let t = run(tf, ema, from_ts, c);
        let net: Decimal = t.iter().map(|x| x.pnl_usd).sum();
        let n = t.len();
        let wins = t.iter().filter(|x| x.pnl_usd > Decimal::ZERO).count();
        let wr = if n > 0 {
            Decimal::from_usize(wins).unwrap() * Decimal::from(100) / Decimal::from_usize(n).unwrap()
        } else {
            Decimal::ZERO
        };
        println!("slip=${}: trades={}, win={}%, net=${}", slip, n, wr.round_dp(2), net.round_dp(2));
    }
}

fn main() {
    let one_min = load_mnq_1m();
    let three_min = resample(&one_min, 3);
    let five_min = resample(&one_min, 5);
    let ema_periods = [100usize, 150usize, 200usize, 250usize, 300usize];
    let mut ema_sets: Vec<(usize, Vec<EmaPoint>)> = Vec::new();
    for p in ema_periods {
        ema_sets.push((p, ema_series(&five_min, p)));
    }

    let from_ts = cutoff("2025-01-01");
    let base = RunCfg {
        rr: Decimal::from(4),
        max_hold_bars: 120,
        min_wick_ticks: Decimal::from(4),
        min_stop_ticks: Decimal::from(8),
        atr_floor_mult: Decimal::new(5, 1),
        atr_period: 14,
        max_cost_r: Decimal::new(15, 2),
        tick_size: Decimal::new(25, 2),
        fee_rt: Decimal::new(124, 2),
        slippage_rt: Decimal::ONE,
        cost_filter_slippage_rt: Decimal::ONE,
        session: SessionFilter::All,
        stop_mode: StopMode::Hybrid,
        entry_mode: EntryMode::Immediate,
        ob_wait_bars: 8,
        use_regime_filter: false,
        regime_min_atr_ticks: Decimal::from(4),
        regime_max_atr_ticks: Decimal::from(30),
        require_micro_confirm: false,
        use_dynamic_rr: false,
        rr_low_vol: Decimal::from(3),
        rr_mid_vol: Decimal::from(4),
        rr_high_vol: Decimal::from(5),
        use_ema_distance_filter: false,
        min_close_ema_dist_ticks: Decimal::from(2),
        use_candle_quality_filter: false,
        min_body_pct: Decimal::from(30),
        min_range_ticks: Decimal::from(6),
        use_trend_structure_filter: false,
        structure_lookback: 3,
        use_loss_streak_breaker: false,
        max_losses_per_day: 2,
    };

    let t1 = run(&one_min, &ema_sets[2].1, from_ts, base);
    let t3 = run(&three_min, &ema_sets[2].1, from_ts, base);

    println!("MNQ futures wick-reclaim backtest (1 micro contract)");
    println!("rule: wick through EMA200(5m), close back, RR=4, max_hold=120 bars");
    println!("costs: $1.24 round-trip fees + $1.00 round-trip slippage");
    println!("start date: 2025-01-01");

    print_stats("1m entries", &t1);
    print_stats("3m entries", &t3);

    let rrs = [Decimal::from(2), Decimal::from(3), Decimal::from(4), Decimal::from(5)];
    let min_wicks = [Decimal::from(2), Decimal::from(4), Decimal::from(6), Decimal::from(8)];
    let atr_mults = [Decimal::new(5, 1), Decimal::new(75, 2), Decimal::ONE];
    let cost_caps = [Decimal::new(10, 2), Decimal::new(15, 2), Decimal::new(20, 2)];
    let sessions = [
        SessionFilter::All,
        SessionFilter::London,
        SessionFilter::NyAm,
        SessionFilter::NyOpen,
        SessionFilter::NyLate,
    ];
    let stop_modes = [StopMode::Wick, StopMode::Atr, StopMode::Hybrid];
    let entry_modes = [EntryMode::Immediate, EntryMode::ObMidRetest];

    let mut rows_1m: Vec<Row> = Vec::new();
    let mut rows_3m: Vec<Row> = Vec::new();
    let mut rows_ema_1m: Vec<Row> = Vec::new();
    let mut rows_ema_3m: Vec<Row> = Vec::new();

    for (ep, ema) in &ema_sets {
        let cfg = RunCfg {
            rr: Decimal::from(3),
            min_wick_ticks: Decimal::from(8),
            atr_floor_mult: Decimal::ONE,
            max_cost_r: Decimal::new(10, 2),
            session: SessionFilter::London,
            ..base
        };
        let a = run(&one_min, ema, from_ts, cfg);
        rows_ema_1m.push(evaluate(format!("ema{} rr3 wick8 atr1 cap0.10 london", ep), &a));
    }
    for (ep, ema) in &ema_sets {
        let cfg = RunCfg {
            rr: Decimal::from(3),
            min_wick_ticks: Decimal::from(2),
            atr_floor_mult: Decimal::ONE,
            max_cost_r: Decimal::new(10, 2),
            session: SessionFilter::NyAm,
            ..base
        };
        let b = run(&three_min, ema, from_ts, cfg);
        rows_ema_3m.push(evaluate(format!("ema{} rr3 wick2 atr1 cap0.10 ny", ep), &b));
    }
    for rr in rrs {
        for mw in min_wicks {
            for am in atr_mults {
                for cc in cost_caps {
                    for s in sessions {
                        for sm in stop_modes {
                            for em in entry_modes {
                                let cfg = RunCfg {
                                    rr,
                                    min_wick_ticks: mw,
                                    atr_floor_mult: am,
                                    max_cost_r: cc,
                                    session: s,
                                    stop_mode: sm,
                                    entry_mode: em,
                                    ..base
                                };
                                let sname = match s {
                                    SessionFilter::All => "all",
                                    SessionFilter::London => "london",
                                    SessionFilter::NyAm => "ny",
                                    SessionFilter::NyOpen => "ny_open",
                                    SessionFilter::NyLate => "ny_late",
                                };
                                let smn = match sm {
                                    StopMode::Wick => "wick",
                                    StopMode::Atr => "atr",
                                    StopMode::Hybrid => "hybrid",
                                };
                                let emn = match em {
                                    EntryMode::Immediate => "imm",
                                    EntryMode::ObMidRetest => "obmid",
                                };
                                let label = format!("rr{} wick{} atr{} cap{} {} {} {}", rr, mw, am, cc, sname, smn, emn);
                                let a = run(&one_min, &ema_sets[2].1, from_ts, cfg);
                                let b = run(&three_min, &ema_sets[2].1, from_ts, cfg);
                                rows_1m.push(evaluate(label.clone(), &a));
                                rows_3m.push(evaluate(label, &b));
                            }
                        }
                    }
                }
            }
        }
    }

    rows_1m.sort_by(|x, y| y.net_usd.partial_cmp(&x.net_usd).unwrap());
    rows_3m.sort_by(|x, y| y.net_usd.partial_cmp(&x.net_usd).unwrap());
    println!("\nTop 10 configs - 1m");
    for r in rows_1m.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }
    println!("\nTop 10 configs - 3m");
    for r in rows_3m.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }

    rows_ema_1m.sort_by(|x, y| y.net_usd.partial_cmp(&x.net_usd).unwrap());
    rows_ema_3m.sort_by(|x, y| y.net_usd.partial_cmp(&x.net_usd).unwrap());
    println!("\nEMA period comparison - 1m (best known filter profile)");
    for r in &rows_ema_1m {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }
    println!("\nEMA period comparison - 3m (best known filter profile)");
    for r in &rows_ema_3m {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }

    println!("\nRobustness check: 3m NY rr3 wick2 atr1 cap0.10");
    let best_300 = RunCfg {
        rr: Decimal::from(3),
        min_wick_ticks: Decimal::from(2),
        atr_floor_mult: Decimal::ONE,
        max_cost_r: Decimal::new(10, 2),
        session: SessionFilter::NyAm,
        ..base
    };
    let mut best_200 = best_300;

    let t_ema300 = run(&three_min, &ema_sets.iter().find(|(p, _)| *p == 300).unwrap().1, from_ts, best_300);
    let t_ema200 = run(&three_min, &ema_sets.iter().find(|(p, _)| *p == 200).unwrap().1, from_ts, best_200);
    print_stats("3m EMA300 baseline", &t_ema300);
    print_stats("3m EMA200 baseline", &t_ema200);
    monthly_breakdown("3m EMA300 baseline", &t_ema300);
    monthly_breakdown("3m EMA200 baseline", &t_ema200);

    println!("\nSlippage stress (round-trip slippage): $1.00 / $1.50 / $2.00");
    for slip in [Decimal::ONE, Decimal::new(15, 1), Decimal::from(2)] {
        let mut c300 = best_300;
        c300.slippage_rt = slip;
        let mut c200 = best_200;
        c200.slippage_rt = slip;
        let a = run(&three_min, &ema_sets.iter().find(|(p, _)| *p == 300).unwrap().1, from_ts, c300);
        let b = run(&three_min, &ema_sets.iter().find(|(p, _)| *p == 200).unwrap().1, from_ts, c200);
        let na: Decimal = a.iter().map(|t| t.pnl_usd).sum();
        let nb: Decimal = b.iter().map(|t| t.pnl_usd).sum();
        println!("slip=${}: EMA300 net=${}, EMA200 net=${}", slip, na.round_dp(2), nb.round_dp(2));
    }

    println!("\n===== Robustness: New Top Configs =====");
    let top1_1m = RunCfg {
        rr: Decimal::from(5),
        min_wick_ticks: Decimal::from(8),
        atr_floor_mult: Decimal::new(5, 1),
        max_cost_r: Decimal::new(20, 2),
        session: SessionFilter::All,
        stop_mode: StopMode::Atr,
        entry_mode: EntryMode::ObMidRetest,
        ..base
    };
    let top1_3m = RunCfg {
        rr: Decimal::from(5),
        min_wick_ticks: Decimal::from(8),
        atr_floor_mult: Decimal::new(5, 1),
        max_cost_r: Decimal::new(20, 2),
        session: SessionFilter::NyAm,
        stop_mode: StopMode::Atr,
        entry_mode: EntryMode::ObMidRetest,
        ..base
    };
    let top_wr_3m = RunCfg {
        rr: Decimal::from(4),
        min_wick_ticks: Decimal::from(8),
        atr_floor_mult: Decimal::new(5, 1),
        max_cost_r: Decimal::new(20, 2),
        session: SessionFilter::NyAm,
        stop_mode: StopMode::Atr,
        entry_mode: EntryMode::ObMidRetest,
        ..base
    };

    let improved_1m = RunCfg {
        use_regime_filter: true,
        regime_min_atr_ticks: Decimal::from(2),
        regime_max_atr_ticks: Decimal::from(200),
        require_micro_confirm: true,
        use_dynamic_rr: true,
        rr_low_vol: Decimal::from(3),
        rr_mid_vol: Decimal::from(4),
        rr_high_vol: Decimal::from(5),
        ..top1_1m
    };
    let improved_3m = RunCfg {
        use_regime_filter: true,
        regime_min_atr_ticks: Decimal::from(2),
        regime_max_atr_ticks: Decimal::from(200),
        require_micro_confirm: true,
        use_dynamic_rr: true,
        rr_low_vol: Decimal::from(3),
        rr_mid_vol: Decimal::from(4),
        rr_high_vol: Decimal::from(5),
        ..top_wr_3m
    };

    let t_top1_1m = run(&one_min, &ema_sets[2].1, from_ts, top1_1m);
    let t_top1_3m = run(&three_min, &ema_sets[2].1, from_ts, top1_3m);
    let t_topwr_3m = run(&three_min, &ema_sets[2].1, from_ts, top_wr_3m);

    print_stats("TOP 1m net config", &t_top1_1m);
    print_stats("TOP 3m net config", &t_top1_3m);
    print_stats("TOP 3m win-rate config", &t_topwr_3m);

    monthly_breakdown("TOP 1m net config", &t_top1_1m);
    monthly_breakdown("TOP 3m net config", &t_top1_3m);
    monthly_breakdown("TOP 3m win-rate config", &t_topwr_3m);

    slippage_stress("TOP 1m net config", &one_min, &ema_sets[2].1, from_ts, top1_1m);
    slippage_stress("TOP 3m net config", &three_min, &ema_sets[2].1, from_ts, top1_3m);
    slippage_stress("TOP 3m win-rate config", &three_min, &ema_sets[2].1, from_ts, top_wr_3m);

    let full_from_ts = dataset_start_ts(&one_min);
    println!("\n===== Full Dataset Numbers (no 2025 cutoff) =====");
    let full_top1_1m = run(&one_min, &ema_sets[2].1, full_from_ts, top1_1m);
    let full_top1_3m = run(&three_min, &ema_sets[2].1, full_from_ts, top1_3m);
    let full_topwr_3m = run(&three_min, &ema_sets[2].1, full_from_ts, top_wr_3m);
    print_stats("FULL DATASET TOP 1m net config", &full_top1_1m);
    print_stats("FULL DATASET TOP 3m net config", &full_top1_3m);
    print_stats("FULL DATASET TOP 3m win-rate config", &full_topwr_3m);

    println!("\n===== Improvements Trial =====");
    let t_im1 = run(&one_min, &ema_sets[2].1, from_ts, improved_1m);
    let t_im3 = run(&three_min, &ema_sets[2].1, from_ts, improved_3m);
    print_stats("IMPROVED 1m (regime + microconfirm + dynRR)", &t_im1);
    print_stats("IMPROVED 3m (regime + microconfirm + dynRR)", &t_im3);
    monthly_breakdown("IMPROVED 1m", &t_im1);
    monthly_breakdown("IMPROVED 3m", &t_im3);
    slippage_stress("IMPROVED 1m", &one_min, &ema_sets[2].1, from_ts, improved_1m);
    slippage_stress("IMPROVED 3m", &three_min, &ema_sets[2].1, from_ts, improved_3m);

    println!("\n===== Focused Sweep: All 5 Filters Enabled =====");
    let rr_f = [Decimal::from(3), Decimal::from(4), Decimal::from(5)];
    let wick_f = [Decimal::from(4), Decimal::from(6), Decimal::from(8)];
    let atr_f = [Decimal::new(5, 1), Decimal::new(75, 2), Decimal::ONE];
    let ema_dist_f = [Decimal::from(1), Decimal::from(2), Decimal::from(3)];
    let body_f = [Decimal::from(25), Decimal::from(30), Decimal::from(35)];
    let range_f = [Decimal::from(5), Decimal::from(6), Decimal::from(8)];
    let struct_lb_f = [3usize, 4usize];
    let loss_cap_f = [2usize, 3usize];
    let sessions_f = [SessionFilter::All, SessionFilter::NyAm, SessionFilter::NyOpen];
    let stop_f = [StopMode::Atr, StopMode::Hybrid];
    let entry_f = [EntryMode::ObMidRetest];

    let mut cfgs: Vec<(String, RunCfg)> = Vec::new();
    for rr in rr_f {
        for mw in wick_f {
            for am in atr_f {
                for ed in ema_dist_f {
                    for bp in body_f {
                        for rt in range_f {
                            for lb in struct_lb_f {
                                for lc in loss_cap_f {
                                    for s in sessions_f {
                                        for sm in stop_f {
                                            for em in entry_f {
                                                let cfg = RunCfg {
                                                    rr,
                                                    min_wick_ticks: mw,
                                                    atr_floor_mult: am,
                                                    max_cost_r: Decimal::new(20, 2),
                                                    session: s,
                                                    stop_mode: sm,
                                                    entry_mode: em,
                                                    use_regime_filter: true,
                                                    regime_min_atr_ticks: Decimal::from(2),
                                                    regime_max_atr_ticks: Decimal::from(200),
                                                    require_micro_confirm: true,
                                                    use_dynamic_rr: true,
                                                    rr_low_vol: Decimal::from(3),
                                                    rr_mid_vol: Decimal::from(4),
                                                    rr_high_vol: Decimal::from(5),
                                                    use_ema_distance_filter: true,
                                                    min_close_ema_dist_ticks: ed,
                                                    use_candle_quality_filter: true,
                                                    min_body_pct: bp,
                                                    min_range_ticks: rt,
                                                    use_trend_structure_filter: true,
                                                    structure_lookback: lb,
                                                    use_loss_streak_breaker: true,
                                                    max_losses_per_day: lc,
                                                    ..base
                                                };
                                                let sname = match s {
                                                    SessionFilter::All => "all",
                                                    SessionFilter::NyAm => "ny",
                                                    SessionFilter::NyOpen => "ny_open",
                                                    SessionFilter::London => "london",
                                                    SessionFilter::NyLate => "ny_late",
                                                };
                                                let smn = match sm { StopMode::Atr => "atr", StopMode::Hybrid => "hyb", StopMode::Wick => "wick" };
                                                let label = format!(
                                                    "rr{} wick{} atr{} emad{} body{} rng{} lb{} lc{} {} {}",
                                                    rr, mw, am, ed, bp, rt, lb, lc, sname, smn
                                                );
                                                cfgs.push((label, cfg));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let one_min_arc = Arc::new(one_min.clone());
    let three_min_arc = Arc::new(three_min.clone());
    let ema200_5m_arc = Arc::new(ema_sets[2].1.clone());

    let rows: Vec<(Row, Row)> = cfgs
        .par_iter()
        .map(|(label, cfg)| {
            let a = run(one_min_arc.as_slice(), ema200_5m_arc.as_slice(), from_ts, *cfg);
            let b = run(three_min_arc.as_slice(), ema200_5m_arc.as_slice(), from_ts, *cfg);
            (evaluate(label.clone(), &a), evaluate(label.clone(), &b))
        })
        .collect();

    let mut f1: Vec<Row> = rows.iter().map(|(a, _)| a.clone()).collect();
    let mut f3: Vec<Row> = rows.iter().map(|(_, b)| b.clone()).collect();
    f1.sort_by(|x, y| y.net_usd.partial_cmp(&x.net_usd).unwrap());
    f3.sort_by(|x, y| y.net_usd.partial_cmp(&x.net_usd).unwrap());
    println!("Top 10 FILTERED - 1m");
    for r in f1.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }
    println!("Top 10 FILTERED - 3m");
    for r in f3.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }

    println!("\n===== Targeted Improvement Search Around Winners =====");
    let rr_t = [Decimal::from(4), Decimal::from(5), Decimal::from(6)];
    let wick_t = [Decimal::from(6), Decimal::from(8), Decimal::from(10)];
    let atr_t = [Decimal::new(4, 1), Decimal::new(5, 1), Decimal::new(6, 1)];
    let cap_t = [Decimal::new(15, 2), Decimal::new(20, 2), Decimal::new(25, 2)];
    let sessions_1m = [SessionFilter::All, SessionFilter::NyAm];
    let sessions_3m = [SessionFilter::NyAm, SessionFilter::NyOpen, SessionFilter::All];
    let ob_wait_t = [6usize, 8usize, 10usize];
    let max_hold_t = [90usize, 120usize, 150usize];
    let ema_period_t = [200usize, 250usize, 300usize];

    let mut targeted_cfg_1m: Vec<(String, RunCfg)> = Vec::new();
    let mut targeted_cfg_3m: Vec<(String, RunCfg, usize)> = Vec::new();
    for rr in rr_t {
        for wk in wick_t {
            for am in atr_t {
                for cp in cap_t {
                    for ow in ob_wait_t {
                        for mh in max_hold_t {
                            for s in sessions_1m {
                                let cfg = RunCfg {
                                    rr,
                                    min_wick_ticks: wk,
                                    atr_floor_mult: am,
                                    max_cost_r: cp,
                                    session: s,
                                    stop_mode: StopMode::Atr,
                                    entry_mode: EntryMode::ObMidRetest,
                                    ob_wait_bars: ow,
                                    max_hold_bars: mh,
                                    use_regime_filter: false,
                                    require_micro_confirm: false,
                                    use_dynamic_rr: false,
                                    use_ema_distance_filter: false,
                                    use_candle_quality_filter: false,
                                    use_trend_structure_filter: false,
                                    use_loss_streak_breaker: false,
                                    ..base
                                };
                                let sname = match s { SessionFilter::All => "all", SessionFilter::NyAm => "ny", _ => "other" };
                                let label = format!("1m rr{} wick{} atr{} cap{} obw{} hold{} {}", rr, wk, am, cp, ow, mh, sname);
                                targeted_cfg_1m.push((label, cfg));
                            }
                            for s in sessions_3m {
                                for ep in ema_period_t {
                                    let cfg = RunCfg {
                                        rr,
                                        min_wick_ticks: wk,
                                        atr_floor_mult: am,
                                        max_cost_r: cp,
                                        session: s,
                                        stop_mode: StopMode::Atr,
                                        entry_mode: EntryMode::ObMidRetest,
                                        ob_wait_bars: ow,
                                        max_hold_bars: mh,
                                        use_regime_filter: false,
                                        require_micro_confirm: false,
                                        use_dynamic_rr: false,
                                        use_ema_distance_filter: false,
                                        use_candle_quality_filter: false,
                                        use_trend_structure_filter: false,
                                        use_loss_streak_breaker: false,
                                        ..base
                                    };
                                    let sname = match s {
                                        SessionFilter::NyAm => "ny",
                                        SessionFilter::NyOpen => "ny_open",
                                        SessionFilter::All => "all",
                                        _ => "other",
                                    };
                                    let label = format!("3m ema{} rr{} wick{} atr{} cap{} obw{} hold{} {}", ep, rr, wk, am, cp, ow, mh, sname);
                                    targeted_cfg_3m.push((label, cfg, ep));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let near_1m: Vec<Row> = targeted_cfg_1m
        .par_iter()
        .filter_map(|(label, cfg)| {
            let t = run(one_min_arc.as_slice(), ema200_5m_arc.as_slice(), from_ts, *cfg);
            let row = evaluate(label.clone(), &t);
            if row.trades >= 300 && row.win_rate >= Decimal::from(22) {
                Some(row)
            } else {
                None
            }
        })
        .collect();

    let near_3m: Vec<Row> = targeted_cfg_3m
        .par_iter()
        .filter_map(|(label, cfg, ep)| {
            let ema_opt = ema_sets.iter().find(|(p, _)| p == ep).map(|(_, e)| e);
            if let Some(ema) = ema_opt {
                let t = run(three_min_arc.as_slice(), ema.as_slice(), from_ts, *cfg);
                let row = evaluate(label.clone(), &t);
                if row.trades >= 250 && row.win_rate >= Decimal::from(24) {
                    Some(row)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let mut near_1m = near_1m;
    let mut near_3m = near_3m;

    near_1m.sort_by(|a, b| b.net_usd.partial_cmp(&a.net_usd).unwrap());
    near_3m.sort_by(|a, b| b.net_usd.partial_cmp(&a.net_usd).unwrap());

    println!("Top 10 TARGETED - 1m (trades>=300, win>=22%)");
    for r in near_1m.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }
    println!("Top 10 TARGETED - 3m (trades>=250, win>=24%)");
    for r in near_3m.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }

    println!("\n===== Selective Bad-Trade Filters Around New Best =====");
    let base_1m_new = RunCfg {
        rr: Decimal::from(5),
        min_wick_ticks: Decimal::from(8),
        atr_floor_mult: Decimal::new(5, 1),
        max_cost_r: Decimal::new(25, 2),
        session: SessionFilter::All,
        stop_mode: StopMode::Atr,
        entry_mode: EntryMode::ObMidRetest,
        ob_wait_bars: 6,
        max_hold_bars: 90,
        ..base
    };
    let base_3m_new = RunCfg {
        rr: Decimal::from(5),
        min_wick_ticks: Decimal::from(10),
        atr_floor_mult: Decimal::new(5, 1),
        max_cost_r: Decimal::new(25, 2),
        session: SessionFilter::NyAm,
        stop_mode: StopMode::Atr,
        entry_mode: EntryMode::ObMidRetest,
        ob_wait_bars: 6,
        max_hold_bars: 90,
        ..base
    };

    let base1_row = evaluate("base1".to_string(), &run(&one_min, &ema_sets[2].1, from_ts, base_1m_new));
    let base3_row = evaluate("base3".to_string(), &run(&three_min, &ema_sets[2].1, from_ts, base_3m_new));
    println!("baseline 1m -> trades={}, win={}%, net=${}", base1_row.trades, base1_row.win_rate.round_dp(2), base1_row.net_usd.round_dp(2));
    println!("baseline 3m -> trades={}, win={}%, net=${}", base3_row.trades, base3_row.win_rate.round_dp(2), base3_row.net_usd.round_dp(2));

    let ema_d = [Decimal::ZERO, Decimal::from(1), Decimal::from(2)];
    let body_p = [Decimal::ZERO, Decimal::from(25), Decimal::from(30)];
    let rng_t = [Decimal::ZERO, Decimal::from(5), Decimal::from(8)];
    let trend_on = [false, true];
    let loss_on = [false, true];
    let micro_on = [false, true];

    let mut sel_cfg: Vec<(String, RunCfg, RunCfg)> = Vec::new();
    for ed in ema_d {
        for bp in body_p {
            for rg in rng_t {
                for tr in trend_on {
                    for ls in loss_on {
                        for mc in micro_on {
                            let mut c1 = base_1m_new;
                            c1.use_ema_distance_filter = ed > Decimal::ZERO;
                            c1.min_close_ema_dist_ticks = ed;
                            c1.use_candle_quality_filter = bp > Decimal::ZERO || rg > Decimal::ZERO;
                            c1.min_body_pct = if bp > Decimal::ZERO { bp } else { Decimal::from(25) };
                            c1.min_range_ticks = if rg > Decimal::ZERO { rg } else { Decimal::from(5) };
                            c1.use_trend_structure_filter = tr;
                            c1.structure_lookback = 3;
                            c1.use_loss_streak_breaker = ls;
                            c1.max_losses_per_day = 2;
                            c1.require_micro_confirm = mc;

                            let mut c3 = base_3m_new;
                            c3.use_ema_distance_filter = ed > Decimal::ZERO;
                            c3.min_close_ema_dist_ticks = ed;
                            c3.use_candle_quality_filter = bp > Decimal::ZERO || rg > Decimal::ZERO;
                            c3.min_body_pct = if bp > Decimal::ZERO { bp } else { Decimal::from(25) };
                            c3.min_range_ticks = if rg > Decimal::ZERO { rg } else { Decimal::from(5) };
                            c3.use_trend_structure_filter = tr;
                            c3.structure_lookback = 3;
                            c3.use_loss_streak_breaker = ls;
                            c3.max_losses_per_day = 2;
                            c3.require_micro_confirm = mc;

                            let tag = format!(
                                "ed{} bp{} rg{} tr{} ls{} mc{}",
                                ed,
                                bp,
                                rg,
                                if tr { 1 } else { 0 },
                                if ls { 1 } else { 0 },
                                if mc { 1 } else { 0 }
                            );
                            sel_cfg.push((tag, c1, c3));
                        }
                    }
                }
            }
        }
    }

    let filt_rows: Vec<(Option<Row>, Option<Row>)> = sel_cfg
        .par_iter()
        .map(|(tag, c1, c3)| {
            let r1 = evaluate(tag.clone(), &run(one_min_arc.as_slice(), ema200_5m_arc.as_slice(), from_ts, *c1));
            let r3 = evaluate(tag.clone(), &run(three_min_arc.as_slice(), ema200_5m_arc.as_slice(), from_ts, *c3));
            let o1 = if r1.trades >= 300 && r1.win_rate >= base1_row.win_rate && r1.net_usd >= base1_row.net_usd * Decimal::new(85, 2) {
                Some(r1)
            } else {
                None
            };
            let o3 = if r3.trades >= 250 && r3.win_rate >= base3_row.win_rate && r3.net_usd >= base3_row.net_usd * Decimal::new(85, 2) {
                Some(r3)
            } else {
                None
            };
            (o1, o3)
        })
        .collect();

    let mut filt1: Vec<Row> = filt_rows.iter().filter_map(|(a, _)| a.clone()).collect();
    let mut filt3: Vec<Row> = filt_rows.iter().filter_map(|(_, b)| b.clone()).collect();

    filt1.sort_by(|a, b| b.net_usd.partial_cmp(&a.net_usd).unwrap());
    filt3.sort_by(|a, b| b.net_usd.partial_cmp(&a.net_usd).unwrap());
    println!("Top selective filters 1m (WR>=base, Net>=85%base)");
    for r in filt1.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }
    println!("Top selective filters 3m (WR>=base, Net>=85%base)");
    for r in filt3.iter().take(10) {
        println!("{} | trades={} | win={}%, net=${}, avg=${}", r.label, r.trades, r.win_rate.round_dp(2), r.net_usd.round_dp(2), r.avg_usd.round_dp(2));
    }

    println!("\n===== Monthly Breakdown: Current Best Post-Fix 3m =====");
    let current_best_3m = RunCfg {
        rr: Decimal::from(2),
        min_wick_ticks: Decimal::from(8),
        atr_floor_mult: Decimal::new(5, 1),
        max_cost_r: Decimal::new(10, 2),
        session: SessionFilter::All,
        stop_mode: StopMode::Atr,
        entry_mode: EntryMode::ObMidRetest,
        ob_wait_bars: 8,
        max_hold_bars: 120,
        use_regime_filter: false,
        require_micro_confirm: false,
        use_dynamic_rr: false,
        use_ema_distance_filter: false,
        use_candle_quality_filter: false,
        use_trend_structure_filter: false,
        use_loss_streak_breaker: false,
        ..base
    };
    let t_best_3m = run(&three_min, &ema_sets[2].1, from_ts, current_best_3m);
    print_stats("CURRENT BEST 3m (post-fix)", &t_best_3m);
    monthly_breakdown("CURRENT BEST 3m (post-fix)", &t_best_3m);
}
