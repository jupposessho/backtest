use backtest::to_new_york_time;
use chrono::{Datelike, Timelike};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::cmp::Ordering;

#[derive(Debug, Clone, Deserialize)]
struct KlineRaw {
    open_time: u64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
}

#[derive(Debug, Clone, Copy)]
struct Bar {
    open_time: i64,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    volume: Decimal,
}

#[derive(Debug, Clone, Copy)]
enum Dir {
    Long,
    Short,
}

#[derive(Debug, Clone)]
struct Trade {
    rr_realized: Decimal,
    entry: Decimal,
    risk: Decimal,
}

#[derive(Debug, Clone, Copy)]
struct Case {
    rr_target: Decimal,
    vol_sma_len: usize,
    structure_lookback: usize,
    trend_filter: TrendFilter,
    volume_rule: VolumeRule,
    min_risk_pct: Decimal,
    entry_mode: EntryMode,
    max_open_range_pct: Option<Decimal>,
}

#[derive(Debug, Clone, Copy)]
enum EntryMode {
    ImmediateClose,
    PullbackLimit {
        retrace_frac: Decimal,
        max_wait_bars: usize,
    },
}

#[derive(Debug, Clone, Copy)]
enum TrendFilter {
    None,
    Ema200,
}

#[derive(Debug, Clone, Copy)]
enum VolumeRule {
    SmaMult { mult: Decimal },
    ZScore { threshold: f64 },
    Percentile { threshold: f64 },
}

fn parse_bars(raw: &str) -> Vec<Bar> {
    let parsed: Vec<KlineRaw> = serde_json::from_str(raw).expect("valid Binance klines JSON");
    parsed
        .into_iter()
        .map(|k| Bar {
            open_time: (k.open_time / 1000) as i64,
            open: k.open.parse::<Decimal>().expect("open decimal"),
            high: k.high.parse::<Decimal>().expect("high decimal"),
            low: k.low.parse::<Decimal>().expect("low decimal"),
            close: k.close.parse::<Decimal>().expect("close decimal"),
            volume: k.volume.parse::<Decimal>().expect("volume decimal"),
        })
        .collect()
}

fn aggregate_bars_utc(src: &[Bar], tf_min: usize) -> Vec<Bar> {
    if tf_min <= 1 {
        return src.to_vec();
    }
    let tf_sec = (tf_min as i64) * 60;
    let mut out: Vec<Bar> = Vec::new();

    let mut cur_bucket: Option<i64> = None;
    let mut cur: Option<Bar> = None;

    for b in src {
        let bucket = (b.open_time / tf_sec) * tf_sec;
        match cur_bucket {
            None => {
                cur_bucket = Some(bucket);
                cur = Some(Bar {
                    open_time: bucket,
                    open: b.open,
                    high: b.high,
                    low: b.low,
                    close: b.close,
                    volume: b.volume,
                });
            }
            Some(prev) if prev == bucket => {
                if let Some(mut c) = cur {
                    if b.high > c.high {
                        c.high = b.high;
                    }
                    if b.low < c.low {
                        c.low = b.low;
                    }
                    c.close = b.close;
                    c.volume += b.volume;
                    cur = Some(c);
                }
            }
            Some(_) => {
                if let Some(c) = cur {
                    out.push(c);
                }
                cur_bucket = Some(bucket);
                cur = Some(Bar {
                    open_time: bucket,
                    open: b.open,
                    high: b.high,
                    low: b.low,
                    close: b.close,
                    volume: b.volume,
                });
            }
        }
    }

    if let Some(c) = cur {
        out.push(c);
    }
    out
}

fn mean_dec(slice: &[Decimal]) -> Decimal {
    if slice.is_empty() {
        return Decimal::ZERO;
    }
    let sum: Decimal = slice.iter().copied().sum();
    sum / Decimal::from_usize(slice.len()).unwrap()
}

fn mean_f64(slice: &[f64]) -> f64 {
    if slice.is_empty() {
        return 0.0;
    }
    slice.iter().sum::<f64>() / slice.len() as f64
}

fn stddev_f64(slice: &[f64], mean: f64) -> f64 {
    if slice.len() < 2 {
        return 0.0;
    }
    let var = slice
        .iter()
        .map(|v| {
            let d = *v - mean;
            d * d
        })
        .sum::<f64>()
        / slice.len() as f64;
    var.sqrt()
}

fn percentile_f64(slice: &[f64], q: f64) -> f64 {
    if slice.is_empty() {
        return 0.0;
    }
    let mut v = slice.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let clamped_q = q.clamp(0.0, 1.0);
    let idx = ((v.len() - 1) as f64 * clamped_q).round() as usize;
    v[idx]
}

fn ema_series(values: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    let mut out = vec![None; values.len()];
    if values.len() < period || period == 0 {
        return out;
    }
    let seed = mean_dec(&values[..period]);
    let two = Decimal::from(2);
    let alpha = two / (Decimal::from_usize(period).unwrap() + Decimal::ONE);
    let mut ema = seed;
    out[period - 1] = Some(ema);
    for i in period..values.len() {
        ema = values[i] * alpha + ema * (Decimal::ONE - alpha);
        out[i] = Some(ema);
    }
    out
}

fn run_case(bars: &[Bar], cfg: Case) -> Vec<Trade> {
    let mut trades = Vec::new();
    let mut i = cfg.vol_sma_len.max(cfg.structure_lookback) + 1;
    let closes: Vec<Decimal> = bars.iter().map(|b| b.close).collect();
    let ema200 = ema_series(&closes, 200);

    while i + 1 < bars.len() {
        let dt = to_new_york_time(bars[i].open_time);
        let y = dt.year();
        let m = dt.month();
        let d = dt.day();

        let mut j = i;
        while j < bars.len() {
            let t = to_new_york_time(bars[j].open_time);
            if t.year() != y || t.month() != m || t.day() != d {
                break;
            }
            j += 1;
        }
        let day_end = j;

        let mut signal_idx: Option<usize> = None;
        let mut open_range_pct: Option<Decimal> = None;
        for idx in i..day_end {
            let t = to_new_york_time(bars[idx].open_time);
            let hour = t.hour();
            let minute = t.minute();
            let in_window = hour == 9 && (30..45).contains(&minute);
            if !in_window {
                continue;
            }

            if open_range_pct.is_none() && hour == 9 && minute == 30 {
                let b = bars[idx];
                if b.open > Decimal::ZERO {
                    open_range_pct = Some(((b.high - b.low) / b.open) * Decimal::from(100));
                }
            }

            let vol_hist_start = idx.saturating_sub(cfg.vol_sma_len);
            if idx <= vol_hist_start {
                continue;
            }
            let vol_hist_dec: Vec<Decimal> =
                bars[vol_hist_start..idx].iter().map(|b| b.volume).collect();
            let vol_hist_f64: Vec<f64> = vol_hist_dec.iter().filter_map(|v| v.to_f64()).collect();
            if vol_hist_f64.is_empty() {
                continue;
            }

            let cur_vol = bars[idx].volume.to_f64().unwrap_or(0.0);
            let high_vol_ok = match cfg.volume_rule {
                VolumeRule::SmaMult { mult } => {
                    let vol_sma = mean_dec(&vol_hist_dec);
                    vol_sma > Decimal::ZERO && bars[idx].volume >= vol_sma * mult
                }
                VolumeRule::ZScore { threshold } => {
                    let mu = mean_f64(&vol_hist_f64);
                    let sd = stddev_f64(&vol_hist_f64, mu);
                    sd > 0.0 && ((cur_vol - mu) / sd) >= threshold
                }
                VolumeRule::Percentile { threshold } => {
                    let cut = percentile_f64(&vol_hist_f64, threshold);
                    cur_vol >= cut
                }
            };

            if !high_vol_ok {
                continue;
            }

            let trend_ok = match cfg.trend_filter {
                TrendFilter::None => true,
                TrendFilter::Ema200 => {
                    if let Some(e) = ema200[idx] {
                        (bars[idx].close > bars[idx].open && bars[idx].close > e)
                            || (bars[idx].close < bars[idx].open && bars[idx].close < e)
                    } else {
                        false
                    }
                }
            };

            if trend_ok {
                signal_idx = Some(idx);
                break;
            }
        }

        if let Some(sidx) = signal_idx {
            if let Some(max_pct) = cfg.max_open_range_pct {
                if let Some(or_pct) = open_range_pct {
                    if or_pct > max_pct {
                        i = day_end;
                        continue;
                    }
                }
            }

            let sbar = bars[sidx];
            let dir = if sbar.close > sbar.open {
                Dir::Long
            } else if sbar.close < sbar.open {
                Dir::Short
            } else {
                i = day_end;
                continue;
            };

            let structure_start = sidx.saturating_sub(cfg.structure_lookback);
            let window = &bars[structure_start..=sidx];
            let structure_low = window.iter().map(|b| b.low).min().unwrap_or(sbar.low);
            let structure_high = window.iter().map(|b| b.high).max().unwrap_or(sbar.high);

            let signal_entry = sbar.close;
            let signal_risk = match dir {
                Dir::Long => signal_entry - structure_low,
                Dir::Short => structure_high - signal_entry,
            };
            if signal_risk <= Decimal::ZERO {
                i = day_end;
                continue;
            }

            let mut entry = signal_entry;
            let mut fill_idx = sidx;
            if let EntryMode::PullbackLimit {
                retrace_frac,
                max_wait_bars,
            } = cfg.entry_mode
            {
                let pullback = signal_risk * retrace_frac;
                let limit = match dir {
                    Dir::Long => signal_entry - pullback,
                    Dir::Short => signal_entry + pullback,
                };
                let mut filled = None;
                let start = sidx + 1;
                let end = (sidx + 1 + max_wait_bars).min(day_end);
                for (k, nx) in bars.iter().enumerate().take(end).skip(start) {
                    let touched = match dir {
                        Dir::Long => nx.low <= limit,
                        Dir::Short => nx.high >= limit,
                    };
                    if touched {
                        filled = Some(k);
                        break;
                    }
                }
                if let Some(k) = filled {
                    entry = limit;
                    fill_idx = k;
                } else {
                    i = day_end;
                    continue;
                }
            }

            let risk = match dir {
                Dir::Long => entry - structure_low,
                Dir::Short => structure_high - entry,
            };
            if risk <= Decimal::ZERO {
                i = day_end;
                continue;
            }
            let risk_pct = if entry > Decimal::ZERO {
                (risk / entry) * Decimal::from(100)
            } else {
                Decimal::ZERO
            };
            if risk_pct < cfg.min_risk_pct {
                i = day_end;
                continue;
            }

            let sl = match dir {
                Dir::Long => structure_low,
                Dir::Short => structure_high,
            };
            let tp = match dir {
                Dir::Long => entry + risk * cfg.rr_target,
                Dir::Short => entry - risk * cfg.rr_target,
            };

            let mut closed = false;
            let mut realized_rr = Decimal::ZERO;

            for nx in bars.iter().take(day_end).skip(fill_idx + 1) {
                let hit_sl = match dir {
                    Dir::Long => nx.low <= sl,
                    Dir::Short => nx.high >= sl,
                };
                let hit_tp = match dir {
                    Dir::Long => nx.high >= tp,
                    Dir::Short => nx.low <= tp,
                };

                // Conservative fill when both touched in same bar: stop-loss first.
                if hit_sl {
                    realized_rr = Decimal::from(-1);
                    closed = true;
                    break;
                }
                if hit_tp {
                    realized_rr = cfg.rr_target;
                    closed = true;
                    break;
                }

                let t = to_new_york_time(nx.open_time);
                if t.hour() >= 16 {
                    let pnl = match dir {
                        Dir::Long => nx.close - entry,
                        Dir::Short => entry - nx.close,
                    };
                    realized_rr = pnl / risk;
                    closed = true;
                    break;
                }
            }

            if !closed {
                let last = bars[day_end - 1];
                let pnl = match dir {
                    Dir::Long => last.close - entry,
                    Dir::Short => entry - last.close,
                };
                realized_rr = pnl / risk;
            }

            trades.push(Trade {
                rr_realized: realized_rr,
                entry,
                risk,
            });
        }

        i = day_end;
    }

    trades
}

fn print_case(label: &str, trades: &[Trade], fee_bps_rt: Decimal, slip_bps_rt: Decimal) {
    let n = trades.len();
    if n == 0 {
        println!(
            "{:<44} {:>6} {:>8} {:>10} {:>10}",
            label, 0, "0.0%", "0.00", "0.00"
        );
        return;
    }

    let bps_frac = Decimal::new(1, 4);
    let total_bps = fee_bps_rt + slip_bps_rt;

    let mut adj_rrs: Vec<Decimal> = Vec::with_capacity(n);
    for t in trades {
        let cost_r = if t.risk > Decimal::ZERO {
            (t.entry * (total_bps * bps_frac)) / t.risk
        } else {
            Decimal::ZERO
        };
        adj_rrs.push(t.rr_realized - cost_r);
    }

    let wins = adj_rrs.iter().filter(|r| **r > Decimal::ZERO).count();
    let wr = (wins as f64) * 100.0 / (n as f64);
    let sum_rr: Decimal = adj_rrs.iter().copied().sum();
    let avg_rr = (sum_rr / Decimal::from_usize(n).unwrap())
        .to_f64()
        .unwrap_or(0.0);

    let mut bal = Decimal::from(1000);
    let risk_frac = Decimal::from_f64(0.01).unwrap();
    for rr in adj_rrs {
        bal += bal * risk_frac * rr;
    }
    let gain_x = (bal / Decimal::from(1000)).to_f64().unwrap_or(0.0);

    println!(
        "{:<44} {:>6} {:>7.1}% {:>10.2} {:>10.2}x",
        label, n, wr, avg_rr, gain_x
    );
}

fn main() {
    let bars_1m = parse_bars(include_str!("../assets/binance_BTCUSDT_1m.json"));

    let cost_scenarios = [
        ("No costs", Decimal::ZERO, Decimal::ZERO),
        ("Low realistic", Decimal::from(8), Decimal::from(2)),
        ("Med realistic", Decimal::from(12), Decimal::from(5)),
    ];

    println!("\nHigh-volume 9:30-9:45 NY first-candle focused re-check (BTCUSDT)");
    println!("Rules: first qualifying candle in window, entry at close, one trade/day, SL at structure, TP=RR target, EOD close @16:00 NY");
    println!("Filters: trend=None/EMA200 and high-volume via SMA-mult, z-score, percentile");
    let timeframes = [5usize];
    let rr_grid = [
        Decimal::new(30, 1),
        Decimal::new(32, 1),
        Decimal::new(35, 1),
        Decimal::new(38, 1),
        Decimal::from(4),
    ];
    let vol_mult_grid = [
        Decimal::new(18, 1),
        Decimal::new(20, 1),
        Decimal::new(22, 1),
        Decimal::new(24, 1),
    ];
    for tf in timeframes {
        let bars = aggregate_bars_utc(&bars_1m, tf);
        println!("\n=== Signal timeframe: {}m ({} bars) ===", tf, bars.len());
        println!(
            "Focused sweep: entry=close, min_risk=0.40%, or<=0.30%, RR 3.0-4.0, vol_mult 1.8-2.4"
        );

        let mut sweep_cases: Vec<(String, Case)> = Vec::new();
        for rr in rr_grid {
            for vm in vol_mult_grid {
                let label = format!(
                    "rr={:.1} vol>=sma*{:.1} sma40 lb=8 entry=close or<=0.30%",
                    rr.to_f64().unwrap_or(0.0),
                    vm.to_f64().unwrap_or(0.0)
                );
                sweep_cases.push((
                    label,
                    Case {
                        rr_target: rr,
                        vol_sma_len: 40,
                        structure_lookback: 8,
                        trend_filter: TrendFilter::None,
                        volume_rule: VolumeRule::SmaMult { mult: vm },
                        min_risk_pct: Decimal::new(40, 2),
                        entry_mode: EntryMode::ImmediateClose,
                        max_open_range_pct: Some(Decimal::new(30, 2)),
                    },
                ));
            }
        }

        for (scenario, fee_bps, slip_bps) in cost_scenarios {
            println!(
                "\nScenario: {} (fee_rt={}bps, slip_rt={}bps)",
                scenario, fee_bps, slip_bps
            );
            println!(
                "{:<44} {:>6} {:>8} {:>10} {:>10}",
                "case", "trades", "win%", "avg R", "gain"
            );
            println!("{}", "-".repeat(86));

            let mut rows: Vec<(String, Vec<Trade>)> = sweep_cases
                .iter()
                .map(|(label, cfg)| (label.to_string(), run_case(&bars, *cfg)))
                .collect();

            rows.sort_by(|a, b| {
                let sa: Decimal = a.1.iter().map(|t| t.rr_realized).sum();
                let sb: Decimal = b.1.iter().map(|t| t.rr_realized).sum();
                sb.cmp(&sa)
            });

            for (label, trades) in rows.iter().take(20) {
                print_case(label, trades, fee_bps, slip_bps);
            }
        }
    }
}
