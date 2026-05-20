use std::sync::Arc;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::engine::types::ExecutionConfig;
use backtest::execute;
use backtest::model::candle_stick::CandleStick;
use backtest::model::fee_config::FeeConfig;
use backtest::strategies::orb::{Orb, OrbConfig, OrbDuration, OrbEntryModel, OrbSlType};
use rayon::prelude::*;
use rust_decimal::Decimal;

const GC_POINT_VALUE: f64 = 10.0;
const COMMISSION_RT_USD: f64 = 2.20;
const SLIPPAGE_TICKS_PER_SIDE: f64 = 2.0;
const GC_TICK: f64 = 0.1;

#[derive(Clone)]
struct Row {
    strategy: &'static str,
    config: String,
    trades: usize,
    net_usd: f64,
    win_rate: f64,
    max_dd_usd: f64,
}

#[derive(Clone)]
struct RealismRow {
    strategy: String,
    config: String,
    slippage_ticks_per_side: i32,
    gross_usd: f64,
    net_usd: f64,
}

fn load_gold() -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/gold_1m_cont.parquet"))
        .expect("failed to load assets/gold_1m_cont.parquet")
}

fn resample(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if minutes <= 1 || data.is_empty() {
        return data.to_vec();
    }
    let bucket = minutes * 60;
    let mut out = Vec::new();
    let mut cur = data[0];
    let mut cur_bucket = cur.open_time / bucket;
    for c in data.iter().copied().skip(1) {
        let b = c.open_time / bucket;
        if b != cur_bucket {
            out.push(cur);
            cur = c;
            cur_bucket = b;
        } else {
            if c.high > cur.high {
                cur.high = c.high;
            }
            if c.low < cur.low {
                cur.low = c.low;
            }
            cur.close = c.close;
            cur.close_time = c.close_time;
        }
    }
    out.push(cur);
    out
}

fn close_vec(data: &[CandleStick]) -> Vec<f64> {
    data.iter().map(|c| c.close.0.to_f64().unwrap_or(0.0)).collect()
}

trait ToF64 {
    fn to_f64(&self) -> Option<f64>;
}
impl ToF64 for Decimal {
    fn to_f64(&self) -> Option<f64> {
        self.to_string().parse::<f64>().ok()
    }
}

fn sma(v: &[f64], len: usize, i: usize) -> Option<f64> {
    if i + 1 < len {
        return None;
    }
    Some(v[i + 1 - len..=i].iter().sum::<f64>() / len as f64)
}

fn backtest_cross(
    closes: &[f64],
    ma_len: usize,
    long_short: bool,
    name: &'static str,
    cfg: String,
) -> Row {
    let mut pos = 0i32;
    let mut entry = 0.0;
    let mut wins = 0usize;
    let mut pnl = 0.0;
    let mut trades = 0usize;
    let mut equity = 0.0;
    let mut peak = 0.0;
    let mut max_dd = 0.0;
    for i in ma_len..closes.len().saturating_sub(1) {
        let m = sma(closes, ma_len, i).unwrap_or(closes[i]);
        let px = closes[i + 1];
        if pos == 0 {
            if closes[i] > m {
                pos = 1;
                entry = px;
            } else if long_short && closes[i] < m {
                pos = -1;
                entry = px;
            }
        } else if (pos == 1 && closes[i] < m) || (pos == -1 && closes[i] > m) {
            let points = if pos == 1 { px - entry } else { entry - px };
            let net = points * GC_POINT_VALUE - (COMMISSION_RT_USD + 2.0 * SLIPPAGE_TICKS_PER_SIDE * GC_TICK * GC_POINT_VALUE);
            if net > 0.0 {
                wins += 1;
            }
            pnl += net;
            equity += net;
            if equity > peak {
                peak = equity;
            }
            let dd = peak - equity;
            if dd > max_dd {
                max_dd = dd;
            }
            trades += 1;
            pos = 0;
        }
    }
    Row {
        strategy: name,
        config: cfg,
        trades,
        net_usd: pnl,
        win_rate: if trades == 0 { 0.0 } else { wins as f64 / trades as f64 },
        max_dd_usd: max_dd,
    }
}

fn run_donchian(data: &[CandleStick]) -> Row {
    let closes = close_vec(data);
    let highs: Vec<f64> = data.iter().map(|c| c.high.0.to_f64().unwrap_or(0.0)).collect();
    let lows: Vec<f64> = data.iter().map(|c| c.low.0.to_f64().unwrap_or(0.0)).collect();
    let entries = [20usize, 55, 80, 100];
    let exits = [10usize, 20, 30, 40];
    let trend_filters = [None, Some(100usize), Some(200usize)];
    let atr_lens = [14usize, 20usize];
    let atr_mults = [2.0_f64, 2.5, 3.0, 3.5];
    let mut rows: Vec<Row> = Vec::new();

    let mut tr = vec![0.0; data.len()];
    for i in 1..data.len() {
        let h = highs[i];
        let l = lows[i];
        let pc = closes[i - 1];
        tr[i] = (h - l).max((h - pc).abs()).max((l - pc).abs());
    }

    for e in entries {
        for x in exits {
            if x >= e || e + 2 >= closes.len() {
                continue;
            }
            for tf in trend_filters {
                for atr_len in atr_lens {
                    for atr_mult in atr_mults {
                        let mut pos = 0i32;
                        let mut entry = 0.0;
                        let mut stop = 0.0;
                        let mut trades = 0usize;
                        let mut wins = 0usize;
                        let mut pnl = 0.0;
                        let mut equity = 0.0;
                        let mut peak = 0.0;
                        let mut max_dd = 0.0;

                        let start = e.max(x).max(atr_len).max(tf.unwrap_or(1));
                        for i in start..closes.len().saturating_sub(1) {
                            let hh = highs[i - e..i].iter().fold(f64::MIN, |a, b| a.max(*b));
                            let ll = lows[i - e..i].iter().fold(f64::MAX, |a, b| a.min(*b));
                            let ex_h = highs[i - x..i].iter().fold(f64::MIN, |a, b| a.max(*b));
                            let ex_l = lows[i - x..i].iter().fold(f64::MAX, |a, b| a.min(*b));
                            let px = closes[i + 1];
                            let atr = tr[i + 1 - atr_len..=i].iter().sum::<f64>() / atr_len as f64;

                            let trend_ok_long = tf
                                .map(|len| closes[i] > sma(&closes, len, i).unwrap_or(closes[i]))
                                .unwrap_or(true);
                            let trend_ok_short = tf
                                .map(|len| closes[i] < sma(&closes, len, i).unwrap_or(closes[i]))
                                .unwrap_or(true);

                            if pos == 0 {
                                if closes[i] > hh && trend_ok_long {
                                    pos = 1;
                                    entry = px;
                                    stop = entry - atr_mult * atr;
                                } else if closes[i] < ll && trend_ok_short {
                                    pos = -1;
                                    entry = px;
                                    stop = entry + atr_mult * atr;
                                }
                            } else {
                                let stop_hit = (pos == 1 && lows[i] <= stop) || (pos == -1 && highs[i] >= stop);
                                let chan_exit = (pos == 1 && closes[i] < ex_l) || (pos == -1 && closes[i] > ex_h);
                                if stop_hit || chan_exit {
                                    let exit_px = if stop_hit { stop } else { px };
                                    let points = if pos == 1 { exit_px - entry } else { entry - exit_px };
                                    let net = points * GC_POINT_VALUE
                                        - (COMMISSION_RT_USD
                                            + 2.0 * SLIPPAGE_TICKS_PER_SIDE * GC_TICK * GC_POINT_VALUE);
                                    pnl += net;
                                    equity += net;
                                    if equity > peak {
                                        peak = equity;
                                    }
                                    let dd = peak - equity;
                                    if dd > max_dd {
                                        max_dd = dd;
                                    }
                                    if net > 0.0 {
                                        wins += 1;
                                    }
                                    trades += 1;
                                    pos = 0;
                                }
                            }
                        }
                        rows.push(Row {
                            strategy: "donchian_breakout",
                            config: format!(
                                "entry={e} exit={x} sma={:?} atr_len={} atr_mult={:.1}",
                                tf, atr_len, atr_mult
                            ),
                            trades,
                            net_usd: pnl,
                            win_rate: if trades == 0 { 0.0 } else { wins as f64 / trades as f64 },
                            max_dd_usd: max_dd,
                        });
                    }
                }
            }
        }
    }
    rows.sort_by(|a, b| {
        let sa = a.net_usd - 0.35 * a.max_dd_usd;
        let sb = b.net_usd - 0.35 * b.max_dd_usd;
        sb.total_cmp(&sa)
    });
    println!("donchian_top5_by_net_minus_0.35dd");
    for r in rows.iter().take(5) {
        println!(
            "cfg={},trades={},net={:.2},dd={:.2},score={:.2}",
            r.config,
            r.trades,
            r.net_usd,
            r.max_dd_usd,
            r.net_usd - 0.35 * r.max_dd_usd
        );
    }
    rows.into_iter().next().unwrap()
}

fn run_momentum(closes: &[f64]) -> Row {
    let lens = [100usize, 150, 200, 250, 300];
    let mut rows: Vec<Row> = lens
        .iter()
        .flat_map(|l| {
            [
                backtest_cross(closes, *l, false, "momentum_12m", format!("ma={} mode=long_only", l)),
                backtest_cross(closes, *l, true, "momentum_12m", format!("ma={} mode=long_short", l)),
            ]
        })
        .collect();
    rows.sort_by(|a, b| b.net_usd.total_cmp(&a.net_usd));
    rows[0].clone()
}

fn run_ema_pullback(data: &[CandleStick]) -> Row {
    let closes = close_vec(data);
    let mut rows = Vec::new();
    for f in [10usize, 20, 30] {
        for s in [40usize, 50, 60] {
            if f >= s {
                continue;
            }
            rows.push(backtest_cross(&closes, s, true, "ema_pullback_continuation", format!("fast={} slow={}", f, s)));
        }
    }
    rows.into_iter().max_by(|a, b| a.net_usd.total_cmp(&b.net_usd)).unwrap()
}

fn run_seasonal(daily: &[CandleStick]) -> Row {
    use chrono::{Datelike, TimeZone};
    use chrono_tz::America::New_York;

    let closes = close_vec(daily);
    let mut in_pos = false;
    let mut entry = 0.0;
    let mut trades = 0usize;
    let mut wins = 0usize;
    let mut pnl = 0.0;
    for i in 200..daily.len().saturating_sub(1) {
        let dt = New_York.timestamp_opt(daily[i].open_time, 0).single().unwrap();
        let m = dt.month();
        let d = dt.day();
        let sma100 = sma(&closes, 100, i).unwrap_or(closes[i]);
        let is_entry_window = m == 7 && d >= 6;
        let is_exit_window = (m == 2 && d >= 15) || m > 2;
        if !in_pos && is_entry_window && closes[i] > sma100 {
            in_pos = true;
            entry = closes[i + 1];
        } else if in_pos && (is_exit_window || closes[i] < sma100) {
            let points = closes[i + 1] - entry;
            let net = points * GC_POINT_VALUE - (COMMISSION_RT_USD + 2.0 * SLIPPAGE_TICKS_PER_SIDE * GC_TICK * GC_POINT_VALUE);
            pnl += net;
            trades += 1;
            if net > 0.0 { wins += 1; }
            in_pos = false;
        }
    }
    Row {
        strategy: "seasonal_window",
        config: "entry=after_jul5 exit=feb15+ sma100_filter".to_string(),
        trades,
        net_usd: pnl,
        win_rate: if trades == 0 { 0.0 } else { wins as f64 / trades as f64 },
        max_dd_usd: 0.0,
    }
}

fn run_orb(data_1m: &[CandleStick]) -> Row {
    let mut best = Row {
        strategy: "intraday_orb",
        config: String::new(),
        trades: 0,
        net_usd: f64::MIN,
        win_rate: 0.0,
        max_dd_usd: 0.0,
    };
    for d in [OrbDuration::Minutes15, OrbDuration::Minutes30] {
        for rr in [Decimal::new(15, 1), Decimal::from(2), Decimal::new(25, 1)] {
            let cfg = OrbConfig {
                duration: d,
                active_window_minutes: 240,
                sl_type: OrbSlType::OppositeRange,
                rr_target: rr,
                eod_close: true,
                max_hold_bars: None,
                retest_mode: false,
                retest_max_bars: 12,
                entry_model: OrbEntryModel::NextBarOpen,
                conservative_intrabar: true,
                execution: ExecutionConfig {
                    commission_rate_per_side: Decimal::ZERO,
                    fee_rate_per_side: Decimal::ZERO,
                    slippage_ticks_per_side: 2,
                    tick_size: Decimal::new(1, 1),
                },
                fee_config: FeeConfig::zero(),
            };
            let res = execute(Orb { data: data_1m.to_vec(), config: cfg });
            let mut wins = 0usize;
            let mut pnl = 0.0;
            for t in &res.trades {
                let p = t.points().0.to_f64().unwrap_or(0.0) * GC_POINT_VALUE - COMMISSION_RT_USD;
                if p > 0.0 {
                    wins += 1;
                }
                pnl += p;
            }
            let row = Row {
                strategy: "intraday_orb",
                config: format!("dur={:?} rr={}", d, rr),
                trades: res.trades.len(),
                net_usd: pnl,
                win_rate: if res.trades.is_empty() { 0.0 } else { wins as f64 / res.trades.len() as f64 },
                max_dd_usd: 0.0,
            };
            if row.net_usd > best.net_usd {
                best = row;
            }
        }
    }
    best
}

fn run_vol_expansion(data: &[CandleStick]) -> Row {
    let closes = close_vec(data);
    let mut rows = Vec::new();
    for b in [10usize, 20, 40] {
        rows.push(backtest_cross(&closes, b, true, "vol_expansion_squeeze", format!("breakout={b}")));
    }
    rows.into_iter().max_by(|a, b| a.net_usd.total_cmp(&b.net_usd)).unwrap()
}

fn main() {
    let data_1m = Arc::new(load_gold());
    let data_daily = Arc::new(resample(&data_1m, 60 * 24));
    let closes_daily = Arc::new(close_vec(&data_daily));

    let rows: Vec<Row> = vec![
        run_donchian(&data_daily),
        run_momentum(&closes_daily),
        run_ema_pullback(&data_daily),
        run_seasonal(&data_daily),
        run_orb(&data_1m),
        run_vol_expansion(&data_daily),
    ]
    .into_par_iter()
    .collect();

    let mut sorted = rows;
    sorted.sort_by(|a, b| b.net_usd.total_cmp(&a.net_usd));
    println!("strategy,config,trades,win_rate,net_usd");
    for r in &sorted {
        println!(
            "{},{},{},{:.2},{:.2}",
            r.strategy,
            r.config,
            r.trades,
            r.win_rate * 100.0,
            r.net_usd
        );
    }
    if let Some(w) = sorted.first() {
        println!("WINNER={},net_usd={:.2},cfg={}", w.strategy, w.net_usd, w.config);

        let realism = realism_matrix(&data_1m, &data_daily, w);
        println!("\nrealism_matrix,strategy,config,slippage_ticks_per_side,gross_usd,net_usd");
        for r in &realism {
            println!(
                "realism,{},{},{},{:.2},{:.2}",
                r.strategy, r.config, r.slippage_ticks_per_side, r.gross_usd, r.net_usd
            );
        }
        let verdict = realism_verdict(&realism);
        println!("REALISM_VERDICT={}", verdict);
    }

    let mut all_realism = Vec::<RealismRow>::new();
    let mut verdict_rows: Vec<(String, String)> = Vec::new();
    for r in &sorted {
        let mat = realism_matrix(&data_1m, &data_daily, r);
        let verdict = realism_verdict(&mat).to_string();
        verdict_rows.push((r.strategy.to_string(), verdict));
        all_realism.extend(mat);
    }

    let _ = std::fs::create_dir_all("reports");
    let mut out = String::from("strategy,config,slippage_ticks_per_side,gross_usd,net_usd,verdict\n");
    for rr in &all_realism {
        let v = verdict_rows
            .iter()
            .find(|(s, _)| s == &rr.strategy)
            .map(|(_, v)| v.as_str())
            .unwrap_or("INSUFFICIENT_DATA");
        out.push_str(&format!(
            "{},{},{},{:.2},{:.2},{}\n",
            rr.strategy, rr.config, rr.slippage_ticks_per_side, rr.gross_usd, rr.net_usd, v
        ));
    }
    let path = "reports/gold_mechanical_realism_matrix.csv";
    std::fs::write(path, out).expect("failed writing realism csv");
    println!("REALISM_CSV={}", path);
}

fn realism_matrix(data_1m: &[CandleStick], data_daily: &[CandleStick], winner: &Row) -> Vec<RealismRow> {
    let mut rows = Vec::new();
    for slip in [1_i32, 2, 3] {
        let row = if winner.strategy == "intraday_orb" {
            eval_orb_config(data_1m, &winner.config, slip)
        } else {
            eval_daily_config(data_daily, winner, slip)
        };
        rows.push(row);
    }
    rows
}

fn realism_verdict(rows: &[RealismRow]) -> &'static str {
    if rows.is_empty() {
        return "INSUFFICIENT_DATA";
    }
    let all_positive = rows.iter().all(|r| r.net_usd > 0.0);
    let monotonic_down = rows.windows(2).all(|w| w[1].net_usd <= w[0].net_usd + 1e-9);
    if all_positive && monotonic_down {
        "PASS"
    } else if rows.iter().any(|r| r.net_usd > 0.0) {
        "PARTIAL"
    } else {
        "FAIL"
    }
}

fn parse_cfg_num(cfg: &str, key: &str) -> Option<usize> {
    for tok in cfg.split_whitespace() {
        if let Some((k, v)) = tok.split_once('=') {
            if k == key {
                let v2 = v.trim_end_matches(',');
                if let Ok(n) = v2.parse::<usize>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn parse_cfg_float(cfg: &str, key: &str) -> Option<f64> {
    for tok in cfg.split_whitespace() {
        if let Some((k, v)) = tok.split_once('=') {
            if k == key {
                let v2 = v.trim_end_matches(',');
                if let Ok(n) = v2.parse::<f64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn parse_cfg_sma_opt(cfg: &str) -> Option<usize> {
    for tok in cfg.split_whitespace() {
        if let Some((k, v)) = tok.split_once('=') {
            if k == "sma" {
                if v == "None" {
                    return None;
                }
                if let Some(inner) = v.strip_prefix("Some(").and_then(|x| x.strip_suffix(')')) {
                    if let Ok(n) = inner.parse::<usize>() {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

fn eval_daily_config(data_daily: &[CandleStick], winner: &Row, slip_ticks: i32) -> RealismRow {
    let closes = close_vec(data_daily);
    let slip_cost = 2.0 * slip_ticks as f64 * GC_TICK * GC_POINT_VALUE;
    let round_trip = COMMISSION_RT_USD + slip_cost;
    let mut gross = 0.0;
    let mut net = 0.0;

    if winner.strategy == "momentum_12m" {
        let ma_len = parse_cfg_num(&winner.config, "ma").unwrap_or(250);
        let long_short = winner.config.contains("long_short");
        let mut pos = 0i32;
        let mut entry = 0.0;
        for i in ma_len..closes.len().saturating_sub(1) {
            let m = sma(&closes, ma_len, i).unwrap_or(closes[i]);
            let px = closes[i + 1];
            if pos == 0 {
                if closes[i] > m {
                    pos = 1;
                    entry = px;
                } else if long_short && closes[i] < m {
                    pos = -1;
                    entry = px;
                }
            } else if (pos == 1 && closes[i] < m) || (pos == -1 && closes[i] > m) {
                let points = if pos == 1 { px - entry } else { entry - px };
                gross += points * GC_POINT_VALUE;
                net += points * GC_POINT_VALUE - round_trip;
                pos = 0;
            }
        }
    } else {
        let row = if winner.strategy == "donchian_breakout" {
            let e = parse_cfg_num(&winner.config, "entry").unwrap_or(55);
            let x = parse_cfg_num(&winner.config, "exit").unwrap_or(20);
            let sma_filter = parse_cfg_sma_opt(&winner.config);
            let atr_len = parse_cfg_num(&winner.config, "atr_len").unwrap_or(20);
            let atr_mult = parse_cfg_float(&winner.config, "atr_mult").unwrap_or(3.0);
            let highs: Vec<f64> = data_daily.iter().map(|c| c.high.0.to_f64().unwrap_or(0.0)).collect();
            let lows: Vec<f64> = data_daily.iter().map(|c| c.low.0.to_f64().unwrap_or(0.0)).collect();
            let mut tr = vec![0.0; data_daily.len()];
            for i in 1..data_daily.len() {
                let h = highs[i];
                let l = lows[i];
                let pc = closes[i - 1];
                tr[i] = (h - l).max((h - pc).abs()).max((l - pc).abs());
            }
            let mut pos = 0i32;
            let mut entry = 0.0;
            let mut stop = 0.0;
            let mut g = 0.0;
            let mut n = 0.0;
            let start = e.max(x).max(atr_len).max(sma_filter.unwrap_or(1));
            for i in start..closes.len().saturating_sub(1) {
                let hh = highs[i - e..i].iter().fold(f64::MIN, |a, b| a.max(*b));
                let ll = lows[i - e..i].iter().fold(f64::MAX, |a, b| a.min(*b));
                let ex_h = highs[i - x..i].iter().fold(f64::MIN, |a, b| a.max(*b));
                let ex_l = lows[i - x..i].iter().fold(f64::MAX, |a, b| a.min(*b));
                let px = closes[i + 1];
                let atr = tr[i + 1 - atr_len..=i].iter().sum::<f64>() / atr_len as f64;
                let trend_ok_long = sma_filter
                    .map(|len| closes[i] > sma(&closes, len, i).unwrap_or(closes[i]))
                    .unwrap_or(true);
                let trend_ok_short = sma_filter
                    .map(|len| closes[i] < sma(&closes, len, i).unwrap_or(closes[i]))
                    .unwrap_or(true);
                if pos == 0 {
                    if closes[i] > hh && trend_ok_long {
                        pos = 1;
                        entry = px;
                        stop = entry - atr_mult * atr;
                    } else if closes[i] < ll && trend_ok_short {
                        pos = -1;
                        entry = px;
                        stop = entry + atr_mult * atr;
                    }
                } else {
                    let stop_hit = (pos == 1 && lows[i] <= stop) || (pos == -1 && highs[i] >= stop);
                    let chan_exit = (pos == 1 && closes[i] < ex_l) || (pos == -1 && closes[i] > ex_h);
                    if !stop_hit && !chan_exit {
                        continue;
                    }
                    let exit_px = if stop_hit { stop } else { px };
                    let points = if pos == 1 { exit_px - entry } else { entry - exit_px };
                    g += points * GC_POINT_VALUE;
                    n += points * GC_POINT_VALUE - round_trip;
                    pos = 0;
                }
            }
            (g, n)
        } else {
            let ma_len = parse_cfg_num(&winner.config, "slow").unwrap_or(50);
            let mut pos = 0i32;
            let mut entry = 0.0;
            let mut g = 0.0;
            let mut n = 0.0;
            for i in ma_len..closes.len().saturating_sub(1) {
                let m = sma(&closes, ma_len, i).unwrap_or(closes[i]);
                let px = closes[i + 1];
                if pos == 0 {
                    if closes[i] > m {
                        pos = 1;
                        entry = px;
                    } else if closes[i] < m {
                        pos = -1;
                        entry = px;
                    }
                } else if (pos == 1 && closes[i] < m) || (pos == -1 && closes[i] > m) {
                    let points = if pos == 1 { px - entry } else { entry - px };
                    g += points * GC_POINT_VALUE;
                    n += points * GC_POINT_VALUE - round_trip;
                    pos = 0;
                }
            }
            (g, n)
        };
        gross = row.0;
        net = row.1;
    }

    RealismRow {
        strategy: winner.strategy.to_string(),
        config: winner.config.clone(),
        slippage_ticks_per_side: slip_ticks,
        gross_usd: gross,
        net_usd: net,
    }
}

fn eval_orb_config(data_1m: &[CandleStick], cfg: &str, slip_ticks: i32) -> RealismRow {
    let duration = if cfg.contains("Minutes15") {
        OrbDuration::Minutes15
    } else {
        OrbDuration::Minutes30
    };
    let rr = if cfg.contains("rr=1.5") {
        Decimal::new(15, 1)
    } else if cfg.contains("rr=2.5") {
        Decimal::new(25, 1)
    } else {
        Decimal::from(2)
    };
    let orb_cfg = OrbConfig {
        duration,
        active_window_minutes: 240,
        sl_type: OrbSlType::OppositeRange,
        rr_target: rr,
        eod_close: true,
        max_hold_bars: None,
        retest_mode: false,
        retest_max_bars: 12,
        entry_model: OrbEntryModel::NextBarOpen,
        conservative_intrabar: true,
        execution: ExecutionConfig {
            commission_rate_per_side: Decimal::ZERO,
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: slip_ticks,
            tick_size: Decimal::new(1, 1),
        },
        fee_config: FeeConfig::zero(),
    };
    let res = execute(Orb {
        data: data_1m.to_vec(),
        config: orb_cfg,
    });
    let mut gross = 0.0;
    let mut net = 0.0;
    for t in &res.trades {
        let p = t.points().0.to_f64().unwrap_or(0.0) * GC_POINT_VALUE;
        gross += p;
        net += p - COMMISSION_RT_USD;
    }
    RealismRow {
        strategy: "intraday_orb".to_string(),
        config: cfg.to_string(),
        slippage_ticks_per_side: slip_ticks,
        gross_usd: gross,
        net_usd: net,
    }
}
