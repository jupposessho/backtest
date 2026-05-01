extern crate rust_decimal;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::engine::execution::run_setups_with_metrics;
use backtest::engine::types::ExecutionConfig;
use backtest::to_new_york_time;
use backtest::model::candle_stick::CandleStick;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::trade_result::TradeResult;
use backtest::strategies::doji::{Doji, DojiConfig, DojiEntryMode, DojiTargetMode, DojiType};
use clap::{Arg, Command};
use rayon::prelude::*;
use rust_decimal::Decimal;
use std::sync::Arc;

fn load_1m(instrument: &str) -> Vec<CandleStick> {
    let path = match instrument {
        "mnq" => "assets/mnq_1m_cont.parquet",
        "mes" => "assets/mes_1m_cont.parquet",
        "gc" => "assets/gold_1m_cont.parquet",
        _ => unreachable!(),
    };
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .unwrap_or_else(|e| panic!("failed loading {path}: {e}"))
}

fn resample_from_1m(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
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

fn print_stats(result: &BacktestResult) {
    let winners = result.result(TradeResult::Winner);
    let losers = result.result(TradeResult::Expense);
    let break_evens = result.result(TradeResult::BreakEven);
    let total = result.number_of_trades();
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as u32) / Decimal::from(total as u32) * Decimal::from(100)
    };
    let gross_loss_r = Decimal::from(losers as u32);
    let gross_profit_r = result.profit_in_r() + gross_loss_r;
    let profit_factor_r = if gross_loss_r > Decimal::ZERO {
        (gross_profit_r / gross_loss_r).round_dp(2)
    } else if gross_profit_r > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    };

    println!("trades      : {}", total);
    println!("winners     : {}", winners);
    println!("losers      : {}", losers);
    println!("break_evens : {}", break_evens);
    println!("win_rate%   : {}", win_rate.round_dp(2));
    println!("profit_r    : {}", result.profit_in_r());
    println!("profit_factor_r: {}", profit_factor_r);
    println!("points      : {}", result.profit_in_points());
    println!("costs_total : {}", result.costs_total());
    println!("pnl%        : {}", result.pnl());
}

fn percentile(mut vals: Vec<Decimal>, p: usize) -> Decimal {
    if vals.is_empty() {
        return Decimal::ZERO;
    }
    vals.sort();
    let idx = ((vals.len() - 1) * p) / 100;
    vals[idx]
}

fn detect_direction(doji_type: &str, lower_wick_pct: Decimal, upper_wick_pct: Decimal) -> i32 {
    if doji_type == "dragonfly" {
        return 1;
    }
    if doji_type == "gravestone" {
        return -1;
    }
    let dominance = Decimal::new(15, 1);
    if lower_wick_pct > upper_wick_pct * dominance {
        1
    } else if upper_wick_pct > lower_wick_pct * dominance {
        -1
    } else if lower_wick_pct >= upper_wick_pct {
        1
    } else {
        -1
    }
}

fn is_doji(doji_type: &str, body_pct: Decimal, upper_wick_pct: Decimal, lower_wick_pct: Decimal, body_pct_max: Decimal) -> bool {
    if doji_type == "loose" {
        return body_pct <= Decimal::from(20);
    }
    if body_pct > body_pct_max {
        return false;
    }
    match doji_type {
        "strict" => upper_wick_pct > Decimal::ZERO && lower_wick_pct > Decimal::ZERO,
        "long_legged" => upper_wick_pct >= Decimal::from(30) && lower_wick_pct >= Decimal::from(30),
        "dragonfly" => upper_wick_pct < Decimal::from(5) && lower_wick_pct >= Decimal::from(30),
        "gravestone" => lower_wick_pct < Decimal::from(5) && upper_wick_pct >= Decimal::from(30),
        _ => true,
    }
}

fn measure_reversal_distance(data: &[CandleStick], doji_type: &str, body_pct_max: Decimal, lookahead_bars: usize) {
    let mut moves: Vec<Decimal> = Vec::new();
    let mut unresolved = 0usize;
    for i in 0..data.len().saturating_sub(1) {
        let c = data[i];
        let ny_time = to_new_york_time(c.open_time).time();
        if ny_time < chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap() || ny_time >= chrono::NaiveTime::from_hms_opt(15, 30, 0).unwrap() {
            continue;
        }
        let range = c.high.0 - c.low.0;
        if range <= Decimal::ZERO {
            continue;
        }
        let body = (c.close.0 - c.open.0).abs();
        let body_pct = body / range * Decimal::from(100);
        let upper_wick = c.high.0 - c.open.0.max(c.close.0);
        let lower_wick = c.open.0.min(c.close.0) - c.low.0;
        let upper_wick_pct = upper_wick / range * Decimal::from(100);
        let lower_wick_pct = lower_wick / range * Decimal::from(100);
        if !is_doji(doji_type, body_pct, upper_wick_pct, lower_wick_pct, body_pct_max) {
            continue;
        }

        let dir = detect_direction(doji_type, lower_wick_pct, upper_wick_pct);
        let ref_close = c.close.0;
        let end = (i + 1 + lookahead_bars).min(data.len());
        let mut best = Decimal::ZERO;
        let mut reversed = false;

        for f in &data[i + 1..end] {
            if dir > 0 {
                let fav = f.high.0 - ref_close;
                if fav > best {
                    best = fav;
                }
                if f.low.0 <= ref_close {
                    reversed = true;
                    break;
                }
            } else {
                let fav = ref_close - f.low.0;
                if fav > best {
                    best = fav;
                }
                if f.high.0 >= ref_close {
                    reversed = true;
                    break;
                }
            }
        }
        if reversed {
            moves.push(best.max(Decimal::ZERO));
        } else {
            unresolved += 1;
        }
    }

    let count = moves.len();
    let mean = if count == 0 { Decimal::ZERO } else { moves.iter().copied().sum::<Decimal>() / Decimal::from(count as u32) };
    println!("Reversal-distance study");
    println!("resolved_signals: {} | unresolved_within_lookahead: {}", count, unresolved);
    println!("mean_pts: {}", mean.round_dp(2));
    println!("p25_pts: {}", percentile(moves.clone(), 25).round_dp(2));
    println!("p50_pts: {}", percentile(moves.clone(), 50).round_dp(2));
    println!("p75_pts: {}", percentile(moves.clone(), 75).round_dp(2));
    println!("p90_pts: {}", percentile(moves, 90).round_dp(2));
}

fn measure_market_risk_profile(
    data: &[CandleStick],
    doji_type: &str,
    body_pct_max: Decimal,
    stop_buffer_ticks: i32,
    tick_size: Decimal,
) {
    let mut risks: Vec<Decimal> = Vec::new();
    for c in data.iter().copied() {
        let ny_time = to_new_york_time(c.open_time).time();
        if ny_time < chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap() || ny_time >= chrono::NaiveTime::from_hms_opt(15, 30, 0).unwrap() {
            continue;
        }
        let range = c.high.0 - c.low.0;
        if range <= Decimal::ZERO {
            continue;
        }
        let body = (c.close.0 - c.open.0).abs();
        let body_pct = body / range * Decimal::from(100);
        let upper_wick = c.high.0 - c.open.0.max(c.close.0);
        let lower_wick = c.open.0.min(c.close.0) - c.low.0;
        let upper_wick_pct = upper_wick / range * Decimal::from(100);
        let lower_wick_pct = lower_wick / range * Decimal::from(100);
        if !is_doji(doji_type, body_pct, upper_wick_pct, lower_wick_pct, body_pct_max) {
            continue;
        }
        let dir = detect_direction(doji_type, lower_wick_pct, upper_wick_pct);
        let buffer = Decimal::from(stop_buffer_ticks) * tick_size;
        let risk = if dir > 0 {
            c.close.0 - (c.low.0 - buffer)
        } else {
            (c.high.0 + buffer) - c.close.0
        };
        if risk > Decimal::ZERO {
            risks.push(risk);
        }
    }

    let n = risks.len();
    let mean = if n == 0 {
        Decimal::ZERO
    } else {
        risks.iter().copied().sum::<Decimal>() / Decimal::from(n as u32)
    };
    let gt_25 = risks.iter().filter(|x| **x > Decimal::from(25)).count();
    let gt_40 = risks.iter().filter(|x| **x > Decimal::from(40)).count();
    println!("Market-entry risk profile");
    println!("signals: {}", n);
    println!("mean_risk_pts: {}", mean.round_dp(2));
    println!("p25_risk: {} | p50_risk: {} | p75_risk: {} | p90_risk: {}",
        percentile(risks.clone(), 25).round_dp(2),
        percentile(risks.clone(), 50).round_dp(2),
        percentile(risks.clone(), 75).round_dp(2),
        percentile(risks.clone(), 90).round_dp(2),
    );
    if n > 0 {
        let n_dec = Decimal::from(n as u32);
        println!(">25pts: {} ({}%) | >40pts: {} ({}%)",
            gt_25,
            (Decimal::from(gt_25 as u32) / n_dec * Decimal::from(100)).round_dp(2),
            gt_40,
            (Decimal::from(gt_40 as u32) / n_dec * Decimal::from(100)).round_dp(2)
        );
    }
}

#[derive(Clone)]
struct SweepCase {
    instrument: String,
    entry: String,
    tp_mode: String,
    tp_value: Decimal,
    doji_type: String,
    stop_buffer_ticks: i32,
    trail_activate: Decimal,
    trail_distance: Decimal,
}

#[derive(Clone)]
struct SweepRow {
    case: SweepCase,
    trades: usize,
    win_rate: Decimal,
    profit_r: Decimal,
    pnl_pct: Decimal,
    costs: Decimal,
}

fn run_case(data: Arc<Vec<CandleStick>>, case: &SweepCase, timeframe: i64) -> SweepRow {
    let tick_size = match case.instrument.as_str() {
        "mnq" | "mes" => Decimal::new(25, 2),
        "gc" => Decimal::new(1, 1),
        _ => Decimal::new(25, 2),
    };
    let strategy = Doji {
        data: (*data).clone(),
        config: DojiConfig {
            doji_type: match case.doji_type.as_str() {
                "classic" => DojiType::Classic,
                "strict" => DojiType::Strict,
                "long_legged" => DojiType::LongLegged,
                "dragonfly" => DojiType::Dragonfly,
                "gravestone" => DojiType::Gravestone,
                "loose" => DojiType::Loose,
                _ => DojiType::Classic,
            },
            stop_buffer_ticks: case.stop_buffer_ticks,
            trail_activate_points: case.trail_activate,
            trail_distance_points: case.trail_distance,
            entry_mode: match case.entry.as_str() {
                "market_close" => DojiEntryMode::MarketClose,
                _ => DojiEntryMode::MidpointLimit,
            },
            target_mode: if case.tp_mode == "fixed_r" {
                DojiTargetMode::RunnerR(case.tp_value)
            } else {
                DojiTargetMode::FixedPoints(case.tp_value)
            },
            execution: ExecutionConfig {
                commission_rate_per_side: Decimal::ZERO,
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: 0,
                tick_size,
            },
            ..DojiConfig::default()
        },
    };
    let setups = strategy.detect_setups();
    let trades = backtest::engine::execution::run_setups(&strategy.data, &setups, &strategy.config.execution);
    let result = BacktestResult {
        trades,
        capital: Decimal::from(1000),
    };
    let trades = result.number_of_trades();
    let winners = result.result(TradeResult::Winner);
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as u32) / Decimal::from(trades as u32) * Decimal::from(100)
    };
    let _ = timeframe;
    SweepRow {
        case: case.clone(),
        trades,
        win_rate: win_rate.round_dp(2),
        profit_r: result.profit_in_r(),
        pnl_pct: result.pnl(),
        costs: result.costs_total(),
    }
}

fn main() {
    let matches = Command::new("Doji Runner")
        .arg(
            Arg::new("instrument")
                .long("instrument")
                .value_parser(["mnq", "mes", "gc"])
                .default_value("mnq"),
        )
        .arg(
            Arg::new("timeframe")
                .long("timeframe")
                .value_parser(clap::value_parser!(i64))
                .default_value("15"),
        )
        .arg(
            Arg::new("doji-type")
                .long("doji-type")
                .value_parser(["classic", "strict", "long_legged", "dragonfly", "gravestone", "loose"])
                .default_value("classic"),
        )
        .arg(
            Arg::new("entry")
                .long("entry")
                .value_parser(["midpoint_limit", "market_close"])
                .default_value("midpoint_limit"),
        )
        .arg(Arg::new("body-pct-max").long("body-pct-max").value_parser(clap::value_parser!(f64)).default_value("5"))
        .arg(Arg::new("stop-buffer-ticks").long("stop-buffer-ticks").value_parser(clap::value_parser!(i32)).default_value("1"))
        .arg(Arg::new("limit-timeout").long("limit-timeout").value_parser(clap::value_parser!(usize)).default_value("5"))
        .arg(Arg::new("trail-activate").long("trail-activate").value_parser(clap::value_parser!(f64)).default_value("10"))
        .arg(Arg::new("trail-distance").long("trail-distance").value_parser(clap::value_parser!(f64)).default_value("10"))
        .arg(Arg::new("max-trades-per-day").long("max-trades-per-day").value_parser(clap::value_parser!(usize)).default_value("3"))
        .arg(Arg::new("tp-points").long("tp-points").value_parser(clap::value_parser!(f64)).required(false))
        .arg(Arg::new("tp-runner-r").long("tp-runner-r").value_parser(clap::value_parser!(f64)).default_value("100"))
        .arg(Arg::new("max-sl-points").long("max-sl-points").value_parser(clap::value_parser!(f64)).required(false))
        .arg(Arg::new("commission-rt").long("commission-rt").value_parser(clap::value_parser!(f64)).required(false))
        .arg(Arg::new("slippage-ticks").long("slippage-ticks").value_parser(clap::value_parser!(i32)).default_value("1"))
        .arg(Arg::new("from-ts").long("from-ts").value_parser(clap::value_parser!(i64)).required(false))
        .arg(Arg::new("to-ts").long("to-ts").value_parser(clap::value_parser!(i64)).required(false))
        .arg(Arg::new("measure-reversal").long("measure-reversal").action(clap::ArgAction::SetTrue))
        .arg(Arg::new("measure-market-risk").long("measure-market-risk").action(clap::ArgAction::SetTrue))
        .arg(Arg::new("lookahead-bars").long("lookahead-bars").value_parser(clap::value_parser!(usize)).default_value("100"))
        .arg(Arg::new("sweep").long("sweep").action(clap::ArgAction::SetTrue))
        .get_matches();

    let instrument = matches
        .get_one::<String>("instrument")
        .map(|s| s.as_str())
        .unwrap_or("mnq");
    let timeframe = *matches.get_one::<i64>("timeframe").unwrap_or(&15);
    let doji_type = matches
        .get_one::<String>("doji-type")
        .map(|s| s.as_str())
        .unwrap_or("classic");
    let entry = matches
        .get_one::<String>("entry")
        .map(|s| s.as_str())
        .unwrap_or("midpoint_limit");

    let mut data = load_1m(instrument);
    if let Some(from_ts) = matches.get_one::<i64>("from-ts") {
        data.retain(|c| c.open_time >= *from_ts);
    }
    if let Some(to_ts) = matches.get_one::<i64>("to-ts") {
        data.retain(|c| c.open_time <= *to_ts);
    }
    data = resample_from_1m(&data, timeframe);

    if matches.get_flag("measure-reversal") {
        let lookahead = *matches.get_one::<usize>("lookahead-bars").unwrap_or(&100usize);
        let body_pct_max = Decimal::from_f64_retain(*matches.get_one::<f64>("body-pct-max").unwrap_or(&5.0))
            .unwrap_or(Decimal::from(5));
        measure_reversal_distance(&data, doji_type, body_pct_max, lookahead);
        return;
    }

    if matches.get_flag("measure-market-risk") {
        let body_pct_max = Decimal::from_f64_retain(*matches.get_one::<f64>("body-pct-max").unwrap_or(&5.0))
            .unwrap_or(Decimal::from(5));
        let stop_buffer_ticks = *matches.get_one::<i32>("stop-buffer-ticks").unwrap_or(&1);
        let tick_size = match instrument {
            "mnq" | "mes" => Decimal::new(25, 2),
            "gc" => Decimal::new(1, 1),
            _ => Decimal::new(25, 2),
        };
        measure_market_risk_profile(&data, doji_type, body_pct_max, stop_buffer_ticks, tick_size);
        return;
    }

    if matches.get_flag("sweep") {
        let from_ts = matches.get_one::<i64>("from-ts").copied();
        let instruments = ["mnq"];
        let entries = ["market_close"];
        let doji_types = ["classic", "strict", "long_legged", "dragonfly", "gravestone", "loose"];
        let stop_buffers = [0i32, 1, 2, 3];
        let trail_activates = [10i64, 20];
        let trail_distances = [10i64, 20];
        let mnq_points = [10i64, 20, 30, 50, 75, 100, 125, 150];
        let mes_points = [10i64, 20, 30, 40, 50, 60, 75];
        let gc_points = [5i64, 10, 15, 20, 30, 40, 50];
        let r_values = [1i64, 2, 3, 4, 5, 8, 10, 15, 20];

        let mut datasets: Vec<(String, Arc<Vec<CandleStick>>)> = Vec::new();
        for ins in instruments {
            let mut d = load_1m(ins);
            if let Some(ts) = from_ts {
                d.retain(|c| c.open_time >= ts);
            }
            let d = Arc::new(resample_from_1m(&d, timeframe));
            datasets.push((ins.to_string(), d));
        }

        let mut cases = Vec::new();
        for ins in instruments {
            for ent in entries {
                for dt in doji_types {
                    for sb in stop_buffers {
                        for ta in trail_activates {
                            for td in trail_distances {
                                cases.push(SweepCase {
                                    instrument: ins.to_string(),
                                    entry: ent.to_string(),
                                    tp_mode: "fixed_points".to_string(),
                                    tp_value: Decimal::from(150),
                                    doji_type: dt.to_string(),
                                    stop_buffer_ticks: sb,
                                    trail_activate: Decimal::from(ta),
                                    trail_distance: Decimal::from(td),
                                });
                            }
                        }
                    }
                }
            }
        }

        let rows: Vec<SweepRow> = cases
            .par_iter()
            .map(|case| {
                let data = datasets
                    .iter()
                    .find(|(name, _)| name == &case.instrument)
                    .map(|(_, d)| Arc::clone(d))
                    .expect("missing dataset");
                run_case(data, case, timeframe)
            })
            .collect();

        println!("instrument,entry,doji_type,stop_buffer,trail_activate,trail_distance,tp_mode,tp_value,trades,win_rate,profit_r,pnl_pct,costs");
        for r in &rows {
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{}",
                r.case.instrument,
                r.case.entry,
                r.case.doji_type,
                r.case.stop_buffer_ticks,
                r.case.trail_activate,
                r.case.trail_distance,
                r.case.tp_mode,
                r.case.tp_value,
                r.trades,
                r.win_rate,
                r.profit_r,
                r.pnl_pct,
                r.costs
            );
        }
        return;
    }

    let tick_size = match instrument {
        "mnq" | "mes" => Decimal::new(25, 2),
        "gc" => Decimal::new(1, 1),
        _ => unreachable!(),
    };

    let target_mode = if let Some(points) = matches.get_one::<f64>("tp-points") {
        DojiTargetMode::FixedPoints(Decimal::from_f64_retain(*points).unwrap_or(Decimal::from(10)))
    } else {
        let r = *matches.get_one::<f64>("tp-runner-r").unwrap_or(&100.0);
        DojiTargetMode::RunnerR(Decimal::from_f64_retain(r).unwrap_or(Decimal::from(100)))
    };

    let strategy = Doji {
        data,
        config: DojiConfig {
            doji_type: match doji_type {
                "classic" => DojiType::Classic,
                "strict" => DojiType::Strict,
                "long_legged" => DojiType::LongLegged,
                "dragonfly" => DojiType::Dragonfly,
                "gravestone" => DojiType::Gravestone,
                "loose" => DojiType::Loose,
                _ => DojiType::Classic,
            },
            body_pct_max: Decimal::from_f64_retain(*matches.get_one::<f64>("body-pct-max").unwrap_or(&5.0))
                .unwrap_or(Decimal::from(5)),
            stop_buffer_ticks: *matches.get_one::<i32>("stop-buffer-ticks").unwrap_or(&1),
            limit_timeout_bars: *matches.get_one::<usize>("limit-timeout").unwrap_or(&5),
            trail_activate_points: Decimal::from_f64_retain(*matches.get_one::<f64>("trail-activate").unwrap_or(&10.0))
                .unwrap_or(Decimal::from(10)),
            trail_distance_points: Decimal::from_f64_retain(*matches.get_one::<f64>("trail-distance").unwrap_or(&10.0))
                .unwrap_or(Decimal::from(10)),
            max_trades_per_day: *matches.get_one::<usize>("max-trades-per-day").unwrap_or(&3),
            entry_mode: match entry {
                "market_close" => DojiEntryMode::MarketClose,
                _ => DojiEntryMode::MidpointLimit,
            },
            target_mode,
            max_sl_points: matches
                .get_one::<f64>("max-sl-points")
                .and_then(|v| Decimal::from_f64_retain(*v)),
            execution: ExecutionConfig {
                commission_rate_per_side: Decimal::ZERO,
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: *matches.get_one::<i32>("slippage-ticks").unwrap_or(&1),
                tick_size,
            },
            ..DojiConfig::default()
        },
    };
    let setups = strategy.detect_setups();
    let (trades, metrics) = run_setups_with_metrics(&strategy.data, &setups, &strategy.config.execution);
    let result = BacktestResult {
        trades,
        capital: Decimal::from(1000),
    };
    let point_value = match instrument {
        "mnq" => Decimal::from(2),
        "mes" => Decimal::from(5),
        "gc" => Decimal::from(10),
        _ => Decimal::ONE,
    };
    let pnl_usd_gross = (result.profit_in_points() * point_value).round_dp(2);
    let commission_rt = matches
        .get_one::<f64>("commission-rt")
        .and_then(|v| Decimal::from_f64_retain(*v))
        .unwrap_or_else(|| match instrument {
            "mnq" | "mes" => Decimal::from_f64_retain(1.32).unwrap(),
            "gc" => Decimal::from_f64_retain(2.20).unwrap(),
            _ => Decimal::ZERO,
        });
    let commissions_total = (commission_rt * Decimal::from(result.number_of_trades() as u32)).round_dp(2);
    let pnl_usd_net = (pnl_usd_gross - commissions_total).round_dp(2);
    println!("Doji strategy: {} {}m", instrument.to_uppercase(), timeframe);
    println!("doji_type={} entry={} tp_mode={}", doji_type, entry, if matches.get_one::<f64>("tp-points").is_some() { "fixed_points" } else { "runner_r" });
    println!(
        "signals: {} | limit_placed: {} | limit_filled: {} | limit_expired: {} | skipped_same_dir_open: {} | skipped_opposite_open: {} | skipped_other: {} | fill_rate: {}%",
        metrics.setup_count,
        metrics.limit_orders_placed,
        metrics.limit_orders_filled,
        metrics.limit_orders_expired,
        metrics.skipped_open_same_dir,
        metrics.skipped_open_opposite_dir,
        metrics.skipped_other,
        if metrics.limit_orders_placed == 0 {
            Decimal::ZERO
        } else {
            (Decimal::from(metrics.limit_orders_filled as u32)
                / Decimal::from(metrics.limit_orders_placed as u32)
                * Decimal::from(100))
                .round_dp(2)
        }
    );
    print_stats(&result);
    println!("pnl_usd_gross_est : {}", pnl_usd_gross);
    println!("commission_rt_used: {} | commissions_total_est: {}", commission_rt, commissions_total);
    println!("pnl_usd_net_est   : {}", pnl_usd_net);
}
