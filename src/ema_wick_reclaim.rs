use backtest::candle_stick_loader::CandleStickLoader;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::decimal::DecimalVec;
use backtest::model::position_direction::PositionDirection;
use backtest::model::trade::Trade;
use backtest::model::trade_result::TradeResult;
use backtest::to_new_york_time;
use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct EmaPoint {
    ts: i64,
    value: Decimal,
}

fn load_btc_1m() -> Vec<CandleStick> {
    let raw = include_str!("../assets/binance_BTCUSDT_1m.json");
    CandleStickLoader::load_binance(raw)
}

fn resample(candles: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if candles.is_empty() {
        return Vec::new();
    }

    let bucket = minutes * 60;
    let mut out = Vec::new();

    let mut cur_start = candles[0].open_time - (candles[0].open_time % bucket);
    let mut open = candles[0].open;
    let mut high = candles[0].high;
    let mut low = candles[0].low;
    let mut close = candles[0].close;

    for c in candles.iter().copied() {
        let start = c.open_time - (c.open_time % bucket);
        if start != cur_start {
            out.push(CandleStick {
                open_time: cur_start,
                close_time: cur_start + bucket,
                open,
                high,
                low,
                close,
            });

            cur_start = start;
            open = c.open;
            high = c.high;
            low = c.low;
            close = c.close;
        } else {
            if c.high > high {
                high = c.high;
            }
            if c.low < low {
                low = c.low;
            }
            close = c.close;
        }
    }

    out.push(CandleStick {
        open_time: cur_start,
        close_time: cur_start + bucket,
        open,
        high,
        low,
        close,
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

fn date_cutoff_utc(date: &str) -> i64 {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("invalid date");
    Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).expect("midnight"))
        .timestamp()
}

fn run_wick_reclaim(
    tf_candles: &[CandleStick],
    ema_5m: &[EmaPoint],
    from_ts: i64,
    rr: Decimal,
) -> Vec<Trade> {
    run_wick_reclaim_cfg(
        tf_candles,
        ema_5m,
        from_ts,
        rr,
        false,
        SessionFilter::All,
        0,
        Decimal::from(0),
        Decimal::new(1, 2),
        Decimal::ZERO,
        14,
        Decimal::from(1000),
        Decimal::ZERO,
        Decimal::ZERO,
    )
}

#[derive(Clone, Copy)]
enum SessionFilter {
    All,
    London,
    Ny,
}

fn in_session(ts: i64, session: SessionFilter) -> bool {
    let t = to_new_york_time(ts).time();
    let hm = (t.hour(), t.minute());
    match session {
        SessionFilter::All => true,
        SessionFilter::London => hm >= (3, 0) && hm <= (5, 0),
        SessionFilter::Ny => hm >= (9, 30) && hm <= (11, 30),
    }
}

fn run_wick_reclaim_cfg(
    tf_candles: &[CandleStick],
    ema_5m: &[EmaPoint],
    from_ts: i64,
    rr: Decimal,
    ema_side_bias: bool,
    session: SessionFilter,
    max_hold_bars: usize,
    min_stop_ticks: Decimal,
    tick_size: Decimal,
    atr_floor_mult: Decimal,
    atr_period: usize,
    max_cost_r: Decimal,
    round_trip_bps: Decimal,
    slippage_bps: Decimal,
) -> Vec<Trade> {
    let mut trades = Vec::new();
    let mut i = 1usize;
    let mut eidx = 0usize;

    while i < tf_candles.len() {
        let c = tf_candles[i];
        if c.open_time < from_ts {
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

        if !in_session(c.open_time, session) {
            i += 1;
            continue;
        }

        let ema = ema_5m[eidx].value;
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

        if ema_side_bias {
            let allowed = match direction {
                PositionDirection::Long => c.close.0 > ema,
                PositionDirection::Short => c.close.0 < ema,
            };
            if !allowed {
                i += 1;
                continue;
            }
        }

        let entry = c.close;
        let mut risk = if direction == PositionDirection::Long {
            entry.0 - c.low.0
        } else {
            c.high.0 - entry.0
        };
        let min_stop = min_stop_ticks * tick_size;
        if i >= atr_period {
            let mut tr_sum = Decimal::ZERO;
            let start = i - atr_period + 1;
            for k in start..=i {
                let cur = tf_candles[k];
                let prev_close = if k > 0 {
                    tf_candles[k - 1].close.0
                } else {
                    cur.close.0
                };
                let tr1 = cur.high.0 - cur.low.0;
                let tr2 = (cur.high.0 - prev_close).abs();
                let tr3 = (cur.low.0 - prev_close).abs();
                let tr = tr1.max(tr2).max(tr3);
                tr_sum += tr;
            }
            let atr = tr_sum / Decimal::from_usize(atr_period).unwrap();
            let atr_floor = atr * atr_floor_mult;
            if atr_floor > risk {
                risk = atr_floor;
            }
        }
        if min_stop > risk {
            risk = min_stop;
        }
        if risk <= Decimal::ZERO {
            i += 1;
            continue;
        }
        let total_bps = round_trip_bps + slippage_bps;
        let bps_to_frac = Decimal::new(1, 4);
        let cost_r = (entry.0 * (total_bps * bps_to_frac)) / risk;
        if cost_r > max_cost_r {
            i += 1;
            continue;
        }
        let sl = if direction == PositionDirection::Long {
            DecimalVec(entry.0 - risk)
        } else {
            DecimalVec(entry.0 + risk)
        };
        let tp = if direction == PositionDirection::Long {
            DecimalVec(entry.0 + risk * rr)
        } else {
            DecimalVec(entry.0 - risk * rr)
        };

        let mut result = TradeResult::BreakEven;
        let mut close_time = c.close_time;
        let mut j = i + 1;
        let mut held = 0usize;
        while j < tf_candles.len() {
            let nx = tf_candles[j];
            let hit_sl = if direction == PositionDirection::Long {
                nx.low.0 <= sl.0
            } else {
                nx.high.0 >= sl.0
            };
            let hit_tp = if direction == PositionDirection::Long {
                nx.high.0 >= tp.0
            } else {
                nx.low.0 <= tp.0
            };

            if hit_sl && hit_tp {
                result = TradeResult::Expense;
                close_time = nx.close_time;
                break;
            }
            if hit_sl {
                result = TradeResult::Expense;
                close_time = nx.close_time;
                break;
            }
            if hit_tp {
                result = TradeResult::Winner;
                close_time = nx.close_time;
                break;
            }
            held += 1;
            if max_hold_bars > 0 && held >= max_hold_bars {
                close_time = nx.close_time;
                break;
            }
            j += 1;
        }

        trades.push(Trade {
            direction,
            open_time: c.close_time,
            close_time,
            entry,
            sl,
            tp,
            result,
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO,
            fees: Decimal::ZERO,
        });

        i = if j > i { j } else { i + 1 };
    }

    trades
}

fn print_results(label: &str, trades: &[Trade]) {
    let total = trades.len();
    let wins = trades
        .iter()
        .filter(|t| t.result == TradeResult::Winner)
        .count();
    let losses = trades
        .iter()
        .filter(|t| t.result == TradeResult::Expense)
        .count();
    let breakeven = trades
        .iter()
        .filter(|t| t.result == TradeResult::BreakEven)
        .count();

    let win_rate = if total > 0 {
        Decimal::from_usize(wins).unwrap() * Decimal::from(100)
            / Decimal::from_usize(total).unwrap()
    } else {
        Decimal::ZERO
    };

    let gross_r: Decimal = trades.iter().map(|t| t.gross_r()).sum();
    let expectancy = if total > 0 {
        gross_r / Decimal::from_usize(total).unwrap()
    } else {
        Decimal::ZERO
    };
    let start_capital = Decimal::from(1000);
    let fixed_risk_usd = Decimal::from(10);
    let fixed_net_usd = gross_r * fixed_risk_usd;
    let bt = BacktestResult {
        trades: trades.to_vec(),
        capital: start_capital,
    };
    let pnl_pct = bt.pnl();
    let compounded_net_usd = start_capital * pnl_pct / Decimal::from(100);

    println!("\n{}", label);
    println!("trades: {}", total);
    println!("wins/losses/be: {}/{}/{}", wins, losses, breakeven);
    println!("win rate: {}%", win_rate.round_dp(2));
    println!("net R: {}", gross_r.round_dp(2));
    println!("expectancy R/trade: {}", expectancy.round_dp(4));
    println!("net USD (fixed $10/R): ${}", fixed_net_usd.round_dp(2));
    println!(
        "net USD (compounded, start $1000): ${}",
        compounded_net_usd.round_dp(2)
    );
}

fn net_r_after_costs(trades: &[Trade], round_trip_bps: Decimal, slippage_bps: Decimal) -> Decimal {
    let mut net = Decimal::ZERO;
    let bps_to_frac = Decimal::new(1, 4);
    let total_bps = round_trip_bps + slippage_bps;
    for t in trades {
        let risk = match t.direction {
            PositionDirection::Long => t.entry.0 - t.sl.0,
            PositionDirection::Short => t.sl.0 - t.entry.0,
        };
        if risk <= Decimal::ZERO {
            continue;
        }
        let cost_r = (t.entry.0 * (total_bps * bps_to_frac)) / risk;
        net += t.gross_r() - cost_r;
    }
    net
}

fn max_drawdown_pct_from_r(
    trades: &[Trade],
    round_trip_bps: Decimal,
    slippage_bps: Decimal,
) -> Decimal {
    let mut bal = Decimal::from(1000);
    let risk_pct = Decimal::new(1, 2);
    let mut peak = bal;
    let mut max_dd = Decimal::ZERO;
    let bps_to_frac = Decimal::new(1, 4);
    let total_bps = round_trip_bps + slippage_bps;
    for t in trades {
        let risk = match t.direction {
            PositionDirection::Long => t.entry.0 - t.sl.0,
            PositionDirection::Short => t.sl.0 - t.entry.0,
        };
        if risk <= Decimal::ZERO {
            continue;
        }
        let cost_r = (t.entry.0 * (total_bps * bps_to_frac)) / risk;
        let net_r = t.gross_r() - cost_r;
        bal += bal * risk_pct * net_r;
        if bal > peak {
            peak = bal;
        }
        if peak > Decimal::ZERO {
            let dd = (peak - bal) / peak * Decimal::from(100);
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd.round_dp(2)
}

fn monthly_validation(
    label: &str,
    trades: &[Trade],
    round_trip_bps: Decimal,
    slippage_bps: Decimal,
) {
    let mut monthly: BTreeMap<String, Decimal> = BTreeMap::new();
    let bps_to_frac = Decimal::new(1, 4);
    let total_bps = round_trip_bps + slippage_bps;
    for t in trades {
        let dt = to_new_york_time(t.close_time);
        let key = format!("{:04}-{:02}", dt.year(), dt.month());
        let risk = match t.direction {
            PositionDirection::Long => t.entry.0 - t.sl.0,
            PositionDirection::Short => t.sl.0 - t.entry.0,
        };
        if risk <= Decimal::ZERO {
            continue;
        }
        let cost_r = (t.entry.0 * (total_bps * bps_to_frac)) / risk;
        let nr = t.gross_r() - cost_r;
        *monthly.entry(key).or_insert(Decimal::ZERO) += nr;
    }

    let mut pos = 0usize;
    let mut neg = 0usize;
    for v in monthly.values() {
        if *v > Decimal::ZERO {
            pos += 1;
        } else if *v < Decimal::ZERO {
            neg += 1;
        }
    }
    println!("\n{} monthly validation (net R after costs)", label);
    println!(
        "positive months: {} | negative months: {} | total months: {}",
        pos,
        neg,
        monthly.len()
    );
    for (k, v) in monthly.iter() {
        println!("{}: {}R", k, v.round_dp(2));
    }
}

fn session_name(s: SessionFilter) -> &'static str {
    match s {
        SessionFilter::All => "all",
        SessionFilter::London => "london",
        SessionFilter::Ny => "ny_am",
    }
}

fn main() {
    let one_min = load_btc_1m();
    let three_min = resample(&one_min, 3);
    let five_min = resample(&one_min, 5);
    let fifteen_min = resample(&one_min, 15);
    let ema200_5m = ema_series(&five_min, 200);
    let ema200_15m = ema_series(&fifteen_min, 200);

    let from_ts = date_cutoff_utc("2025-01-01");
    println!("EMA wick reclaim strategy");
    println!(
        "rule: wick through EMA200(5m) then close back, SL=signal wick, one position at a time"
    );
    println!("start date: 2025-01-01");

    let rrs = [
        Decimal::ONE,
        Decimal::new(125, 2),
        Decimal::new(15, 1),
        Decimal::new(2, 0),
        Decimal::new(25, 1),
        Decimal::new(3, 0),
        Decimal::new(4, 0),
        Decimal::new(5, 0),
        Decimal::new(6, 0),
    ];
    let biases = [false, true];
    let sessions = [SessionFilter::All, SessionFilter::London, SessionFilter::Ny];
    let max_hold_bars = 120usize;
    let min_stop_ticks = Decimal::from(8);
    let tick_size = Decimal::new(1, 2);
    let atr_floor_mult = Decimal::new(5, 1);
    let atr_period = 14usize;
    let max_cost_r = Decimal::new(15, 2);
    let fees_rt_bps = Decimal::from(12);
    let slippage_rt_bps = Decimal::from(2);

    for rr in rrs {
        for bias in biases {
            for session in sessions {
                let label_1m = format!(
                    "1m | rr={} | ema_bias={} | session={}",
                    rr,
                    if bias { 1 } else { 0 },
                    session_name(session)
                );
                let label_3m = format!(
                    "3m | rr={} | ema_bias={} | session={}",
                    rr,
                    if bias { 1 } else { 0 },
                    session_name(session)
                );
                let r1m = run_wick_reclaim_cfg(
                    &one_min,
                    &ema200_5m,
                    from_ts,
                    rr,
                    bias,
                    session,
                    max_hold_bars,
                    min_stop_ticks,
                    tick_size,
                    atr_floor_mult,
                    atr_period,
                    max_cost_r,
                    fees_rt_bps,
                    slippage_rt_bps,
                );
                let r3m = run_wick_reclaim_cfg(
                    &three_min,
                    &ema200_5m,
                    from_ts,
                    rr,
                    bias,
                    session,
                    max_hold_bars,
                    min_stop_ticks,
                    tick_size,
                    atr_floor_mult,
                    atr_period,
                    max_cost_r,
                    fees_rt_bps,
                    slippage_rt_bps,
                );
                print_results(&label_1m, &r1m);
                print_results(&label_3m, &r3m);
            }
        }
    }

    println!("\n===== Reality Validation (finalists) =====");
    println!(
        "assumptions: round-trip fees={} bps, round-trip slippage={} bps",
        fees_rt_bps, slippage_rt_bps
    );
    println!(
        "filters: min_stop_ticks={}, tick_size={}, atr_floor_mult={}, atr_period={}, max_cost_r={}",
        min_stop_ticks, tick_size, atr_floor_mult, atr_period, max_cost_r
    );

    let t1 = run_wick_reclaim_cfg(
        &one_min,
        &ema200_5m,
        from_ts,
        Decimal::from(4),
        false,
        SessionFilter::All,
        120,
        min_stop_ticks,
        tick_size,
        atr_floor_mult,
        atr_period,
        max_cost_r,
        fees_rt_bps,
        slippage_rt_bps,
    );
    let t3 = run_wick_reclaim_cfg(
        &three_min,
        &ema200_5m,
        from_ts,
        Decimal::from(4),
        false,
        SessionFilter::All,
        120,
        min_stop_ticks,
        tick_size,
        atr_floor_mult,
        atr_period,
        max_cost_r,
        fees_rt_bps,
        slippage_rt_bps,
    );

    let net_r_1m = net_r_after_costs(&t1, fees_rt_bps, slippage_rt_bps);
    let net_r_3m = net_r_after_costs(&t3, fees_rt_bps, slippage_rt_bps);
    println!(
        "1m rr=4 all | net after costs: {}R | fixed-risk USD: ${}",
        net_r_1m.round_dp(2),
        (net_r_1m * Decimal::from(10)).round_dp(2)
    );
    println!(
        "3m rr=4 all | net after costs: {}R | fixed-risk USD: ${}",
        net_r_3m.round_dp(2),
        (net_r_3m * Decimal::from(10)).round_dp(2)
    );
    println!(
        "1m rr=4 all | max drawdown: {}%",
        max_drawdown_pct_from_r(&t1, fees_rt_bps, slippage_rt_bps)
    );
    println!(
        "3m rr=4 all | max drawdown: {}%",
        max_drawdown_pct_from_r(&t3, fees_rt_bps, slippage_rt_bps)
    );

    monthly_validation("1m rr=4 all", &t1, fees_rt_bps, slippage_rt_bps);
    monthly_validation("3m rr=4 all", &t3, fees_rt_bps, slippage_rt_bps);

    println!("\n===== Higher TF Matrix (rr=4, all session) =====");
    println!("columns: entry_tf | ema_tf | trades | win_rate | netR_after_costs | netUSD_fixed | maxDD | +months/-months");

    let t_5m_ema5m = run_wick_reclaim_cfg(
        &five_min,
        &ema200_5m,
        from_ts,
        Decimal::from(4),
        false,
        SessionFilter::All,
        120,
        min_stop_ticks,
        tick_size,
        atr_floor_mult,
        atr_period,
        max_cost_r,
        fees_rt_bps,
        slippage_rt_bps,
    );
    let t_15m_ema15m = run_wick_reclaim_cfg(
        &fifteen_min,
        &ema200_15m,
        from_ts,
        Decimal::from(4),
        false,
        SessionFilter::All,
        120,
        min_stop_ticks,
        tick_size,
        atr_floor_mult,
        atr_period,
        max_cost_r,
        fees_rt_bps,
        slippage_rt_bps,
    );
    let t_15m_ema5m = run_wick_reclaim_cfg(
        &fifteen_min,
        &ema200_5m,
        from_ts,
        Decimal::from(4),
        false,
        SessionFilter::All,
        120,
        min_stop_ticks,
        tick_size,
        atr_floor_mult,
        atr_period,
        max_cost_r,
        fees_rt_bps,
        slippage_rt_bps,
    );

    let cases = vec![
        ("5m", "ema200_5m", t_5m_ema5m),
        ("15m", "ema200_15m", t_15m_ema15m),
        ("15m", "ema200_5m", t_15m_ema5m),
    ];

    for (entry_tf, ema_tf, trades) in cases {
        let wins = trades
            .iter()
            .filter(|t| t.result == TradeResult::Winner)
            .count();
        let total = trades.len();
        let win_rate = if total > 0 {
            Decimal::from_usize(wins).unwrap() * Decimal::from(100)
                / Decimal::from_usize(total).unwrap()
        } else {
            Decimal::ZERO
        };
        let net_r = net_r_after_costs(&trades, fees_rt_bps, slippage_rt_bps);
        let net_usd = (net_r * Decimal::from(10)).round_dp(2);
        let max_dd = max_drawdown_pct_from_r(&trades, fees_rt_bps, slippage_rt_bps);

        let mut monthly: BTreeMap<String, Decimal> = BTreeMap::new();
        let bps_to_frac = Decimal::new(1, 4);
        let total_bps = fees_rt_bps + slippage_rt_bps;
        for t in &trades {
            let dt = to_new_york_time(t.close_time);
            let key = format!("{:04}-{:02}", dt.year(), dt.month());
            let risk = match t.direction {
                PositionDirection::Long => t.entry.0 - t.sl.0,
                PositionDirection::Short => t.sl.0 - t.entry.0,
            };
            if risk <= Decimal::ZERO {
                continue;
            }
            let cost_r = (t.entry.0 * (total_bps * bps_to_frac)) / risk;
            *monthly.entry(key).or_insert(Decimal::ZERO) += t.gross_r() - cost_r;
        }
        let mut pos = 0usize;
        let mut neg = 0usize;
        for v in monthly.values() {
            if *v > Decimal::ZERO {
                pos += 1;
            } else if *v < Decimal::ZERO {
                neg += 1;
            }
        }

        println!(
            "{} | {} | {} | {}% | {}R | ${} | {}% | {}/{}",
            entry_tf,
            ema_tf,
            total,
            win_rate.round_dp(2),
            net_r.round_dp(2),
            net_usd,
            max_dd,
            pos,
            neg
        );
    }
}
