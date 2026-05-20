use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::model::candle_stick::CandleStick;
use chrono::{DateTime, Utc};

const COMMISSION_RT_USD: f64 = 2.20;

#[derive(Clone, Copy)]
struct MarketSpec {
    name: &'static str,
    parquet: &'static str,
    point_value: f64,
    tick: f64,
}

#[derive(Clone)]
struct Case {
    strategy: &'static str,
    config: &'static str,
}

#[derive(Clone)]
struct OutRow {
    market: &'static str,
    strategy: &'static str,
    config: &'static str,
    slippage_ticks_per_side: i32,
    gross_usd: f64,
    net_usd: f64,
    trades: usize,
    max_dd_usd: f64,
    dd_from: String,
    dd_to: String,
    dataset_from: String,
    dataset_to: String,
    verdict: &'static str,
}

#[derive(Clone)]
struct EvalStats {
    gross: f64,
    net: f64,
    trades: usize,
    max_dd: f64,
    dd_from_ts: i64,
    dd_to_ts: i64,
}

struct DrawdownTracker {
    equity: f64,
    peak: f64,
    peak_ts: i64,
    max_dd: f64,
    dd_from_ts: i64,
    dd_to_ts: i64,
}

impl DrawdownTracker {
    fn new(start_ts: i64) -> Self {
        Self {
            equity: 0.0,
            peak: 0.0,
            peak_ts: start_ts,
            max_dd: 0.0,
            dd_from_ts: start_ts,
            dd_to_ts: start_ts,
        }
    }

    fn apply_trade(&mut self, pnl: f64, ts: i64) {
        self.equity += pnl;
        if self.equity > self.peak {
            self.peak = self.equity;
            self.peak_ts = ts;
        }
        let dd = self.peak - self.equity;
        if dd > self.max_dd {
            self.max_dd = dd;
            self.dd_from_ts = self.peak_ts;
            self.dd_to_ts = ts;
        }
    }
}

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .unwrap_or_else(|_| panic!("failed to load {}", path))
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

trait ToF64 {
    fn to_f64(&self) -> Option<f64>;
}

impl ToF64 for rust_decimal::Decimal {
    fn to_f64(&self) -> Option<f64> {
        self.to_string().parse::<f64>().ok()
    }
}

fn close_vec(data: &[CandleStick]) -> Vec<f64> {
    data.iter().map(|c| c.close.0.to_f64().unwrap_or(0.0)).collect()
}

fn sma(v: &[f64], len: usize, i: usize) -> Option<f64> {
    if i + 1 < len {
        return None;
    }
    Some(v[i + 1 - len..=i].iter().sum::<f64>() / len as f64)
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

fn ts_to_date(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "n/a".to_string())
}

fn eval_case(data_daily: &[CandleStick], c: &Case, slip_ticks: i32, m: MarketSpec) -> OutRow {
    let closes = close_vec(data_daily);
    let slip_cost = 2.0 * slip_ticks as f64 * m.tick * m.point_value;
    let round_trip = COMMISSION_RT_USD + slip_cost;
    let start_ts = data_daily.first().map(|d| d.open_time).unwrap_or(0);
    let end_ts = data_daily.last().map(|d| d.open_time).unwrap_or(0);
    let mut stats = EvalStats {
        gross: 0.0,
        net: 0.0,
        trades: 0,
        max_dd: 0.0,
        dd_from_ts: start_ts,
        dd_to_ts: start_ts,
    };
    let mut dd = DrawdownTracker::new(start_ts);

    if c.strategy == "momentum_12m" {
        let ma_len = parse_cfg_num(c.config, "ma").unwrap_or(250);
        let long_short = c.config.contains("long_short");
        let mut pos = 0i32;
        let mut entry = 0.0;
        for i in ma_len..closes.len().saturating_sub(1) {
            let ma = sma(&closes, ma_len, i).unwrap_or(closes[i]);
            let px = closes[i + 1];
            if pos == 0 {
                if closes[i] > ma {
                    pos = 1;
                    entry = px;
                } else if long_short && closes[i] < ma {
                    pos = -1;
                    entry = px;
                }
            } else if (pos == 1 && closes[i] < ma) || (pos == -1 && closes[i] > ma) {
                let points = if pos == 1 { px - entry } else { entry - px };
                let gross_pnl = points * m.point_value;
                let net_pnl = gross_pnl - round_trip;
                stats.gross += gross_pnl;
                stats.net += net_pnl;
                stats.trades += 1;
                dd.apply_trade(net_pnl, data_daily[i + 1].open_time);
                pos = 0;
            }
        }
    } else if c.strategy == "donchian_breakout" {
        let e = parse_cfg_num(c.config, "entry").unwrap_or(55);
        let x = parse_cfg_num(c.config, "exit").unwrap_or(20);
        let sma_filter = parse_cfg_sma_opt(c.config);
        let atr_len = parse_cfg_num(c.config, "atr_len").unwrap_or(20);
        let atr_mult = parse_cfg_float(c.config, "atr_mult").unwrap_or(3.0);
        let highs: Vec<f64> = data_daily.iter().map(|d| d.high.0.to_f64().unwrap_or(0.0)).collect();
        let lows: Vec<f64> = data_daily.iter().map(|d| d.low.0.to_f64().unwrap_or(0.0)).collect();
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
                let gross_pnl = points * m.point_value;
                let net_pnl = gross_pnl - round_trip;
                stats.gross += gross_pnl;
                stats.net += net_pnl;
                stats.trades += 1;
                dd.apply_trade(net_pnl, data_daily[i + 1].open_time);
                pos = 0;
            }
        }
    } else if c.strategy == "seasonal_window" {
        use chrono::{Datelike, TimeZone};
        use chrono_tz::America::New_York;

        let mut in_pos = false;
        let mut entry = 0.0;
        for i in 200..data_daily.len().saturating_sub(1) {
            let dt = New_York.timestamp_opt(data_daily[i].open_time, 0).single().unwrap();
            let month = dt.month();
            let day = dt.day();
            let sma100 = sma(&closes, 100, i).unwrap_or(closes[i]);
            let is_entry_window = month == 7 && day >= 6;
            let is_exit_window = (month == 2 && day >= 15) || month > 2;
            if !in_pos && is_entry_window && closes[i] > sma100 {
                in_pos = true;
                entry = closes[i + 1];
            } else if in_pos && (is_exit_window || closes[i] < sma100) {
                let points = closes[i + 1] - entry;
                let gross_pnl = points * m.point_value;
                let net_pnl = gross_pnl - round_trip;
                stats.gross += gross_pnl;
                stats.net += net_pnl;
                stats.trades += 1;
                dd.apply_trade(net_pnl, data_daily[i + 1].open_time);
                in_pos = false;
            }
        }
    } else {
        let ma_len = if c.strategy == "vol_expansion_squeeze" {
            parse_cfg_num(c.config, "breakout").unwrap_or(10)
        } else {
            parse_cfg_num(c.config, "slow").unwrap_or(50)
        };
        let mut pos = 0i32;
        let mut entry = 0.0;
        for i in ma_len..closes.len().saturating_sub(1) {
            let ma = sma(&closes, ma_len, i).unwrap_or(closes[i]);
            let px = closes[i + 1];
            if pos == 0 {
                if closes[i] > ma {
                    pos = 1;
                    entry = px;
                } else if closes[i] < ma {
                    pos = -1;
                    entry = px;
                }
            } else if (pos == 1 && closes[i] < ma) || (pos == -1 && closes[i] > ma) {
                let points = if pos == 1 { px - entry } else { entry - px };
                let gross_pnl = points * m.point_value;
                let net_pnl = gross_pnl - round_trip;
                stats.gross += gross_pnl;
                stats.net += net_pnl;
                stats.trades += 1;
                dd.apply_trade(net_pnl, data_daily[i + 1].open_time);
                pos = 0;
            }
        }
    }

    stats.max_dd = dd.max_dd;
    stats.dd_from_ts = dd.dd_from_ts;
    stats.dd_to_ts = dd.dd_to_ts;

    OutRow {
        market: m.name,
        strategy: c.strategy,
        config: c.config,
        slippage_ticks_per_side: slip_ticks,
        gross_usd: stats.gross,
        net_usd: stats.net,
        trades: stats.trades,
        max_dd_usd: stats.max_dd,
        dd_from: ts_to_date(stats.dd_from_ts),
        dd_to: ts_to_date(stats.dd_to_ts),
        dataset_from: ts_to_date(start_ts),
        dataset_to: ts_to_date(end_ts),
        verdict: "",
    }
}

fn verdict_for(rows: &[OutRow]) -> &'static str {
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

fn main() {
    let markets = [
        MarketSpec {
            name: "mnq",
            parquet: "assets/mnq_1m_cont.parquet",
            point_value: 2.0,
            tick: 0.25,
        },
        MarketSpec {
            name: "mes",
            parquet: "assets/mes_1m_cont.parquet",
            point_value: 5.0,
            tick: 0.25,
        },
    ];

    let profitable_gold_cases = [
        Case {
            strategy: "donchian_breakout",
            config: "entry=55 exit=20 sma=Some(200) atr_len=20 atr_mult=3.5",
        },
        Case {
            strategy: "momentum_12m",
            config: "ma=100 mode=long_only",
        },
        Case {
            strategy: "vol_expansion_squeeze",
            config: "breakout=10",
        },
        Case {
            strategy: "ema_pullback_continuation",
            config: "fast=30 slow=50",
        },
        Case {
            strategy: "seasonal_window",
            config: "entry=after_jul5 exit=feb15+ sma100_filter",
        },
    ];

    let mut all_rows = Vec::<OutRow>::new();
    for market in markets {
        let data_1m = load_parquet(market.parquet);
        let data_daily = resample(&data_1m, 60 * 24);
        for c in &profitable_gold_cases {
            let mut per_case = Vec::<OutRow>::new();
            for slip in [1_i32, 2, 3] {
                per_case.push(eval_case(&data_daily, c, slip, market));
            }
            let verdict = verdict_for(&per_case);
            for mut r in per_case {
                r.verdict = verdict;
                all_rows.push(r);
            }
        }
    }

    all_rows.sort_by(|a, b| {
        let key_a = format!("{}:{}", a.market, a.strategy);
        let key_b = format!("{}:{}", b.market, b.strategy);
        key_a.cmp(&key_b).then(a.slippage_ticks_per_side.cmp(&b.slippage_ticks_per_side))
    });

    let mut out = String::from(
        "market,strategy,config,slippage_ticks_per_side,trades,gross_usd,net_usd,max_dd_usd,dd_from,dd_to,dataset_from,dataset_to,verdict\n",
    );
    for r in &all_rows {
        out.push_str(&format!(
            "{},{},{},{},{},{:.2},{:.2},{:.2},{},{},{},{},{}\n",
            r.market,
            r.strategy,
            r.config,
            r.slippage_ticks_per_side,
            r.trades,
            r.gross_usd,
            r.net_usd,
            r.max_dd_usd,
            r.dd_from,
            r.dd_to,
            r.dataset_from,
            r.dataset_to,
            r.verdict
        ));
    }

    let path = "reports/gold_mechanical_on_mnq_mes.csv";
    std::fs::write(path, out).expect("failed writing cross asset report");
    println!("WROTE={}", path);

    println!("market,strategy,slip,trades,net_usd,max_dd_usd,dd_from,dd_to,dataset_from,dataset_to,verdict");
    for r in &all_rows {
        println!(
            "{},{},{},{},{:.2},{:.2},{},{},{},{},{}",
            r.market,
            r.strategy,
            r.slippage_ticks_per_side,
            r.trades,
            r.net_usd,
            r.max_dd_usd,
            r.dd_from,
            r.dd_to,
            r.dataset_from,
            r.dataset_to,
            r.verdict
        );
    }

    let mut ranked: Vec<OutRow> = all_rows
        .iter()
        .filter(|r| r.slippage_ticks_per_side == 2)
        .cloned()
        .collect();
    ranked.sort_by(|a, b| b.net_usd.total_cmp(&a.net_usd));
    println!("\nRANKING_SLIP2");
    println!("rank,market,strategy,trades,net_usd,max_dd_usd,dd_from,dd_to,dataset_from,dataset_to");
    for (i, r) in ranked.iter().enumerate() {
        println!(
            "{},{},{},{},{:.2},{:.2},{},{},{},{}",
            i + 1,
            r.market,
            r.strategy,
            r.trades,
            r.net_usd,
            r.max_dd_usd,
            r.dd_from,
            r.dd_to,
            r.dataset_from,
            r.dataset_to
        );
    }
}
