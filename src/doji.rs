extern crate rust_decimal;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::engine::execution::run_setups_with_metrics;
use backtest::engine::types::ExecutionConfig;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade::Trade;
use backtest::model::trade_result::TradeResult;
use backtest::strategies::doji::{
    Doji, DojiConfig, DojiEntryMode, DojiTargetMode, DojiType, MaxSlMode,
};
use backtest::to_new_york_time;
use chrono::{NaiveTime, TimeZone, Timelike};
use clap::{Arg, Command};
use rayon::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

fn print_entry_time_profile(trades: &[Trade]) {
    let mut by_hour = vec![(0usize, 0usize, 0usize); 24];
    for t in trades {
        let hour = to_new_york_time(t.open_time).hour() as usize;
        let slot = &mut by_hour[hour];
        slot.0 += 1;
        match t.result {
            TradeResult::Winner => slot.1 += 1,
            TradeResult::Expense => slot.2 += 1,
            TradeResult::BreakEven => {}
        }
    }

    println!("entry_hour,trades,winners,losers,loss_rate_pct");
    for h in 0..24 {
        let (trades, winners, losers) = by_hour[h];
        if trades == 0 {
            continue;
        }
        let loss_rate = (Decimal::from(losers as u32) / Decimal::from(trades as u32)
            * Decimal::from(100))
        .round_dp(2);
        println!("{:02},{},{},{},{}", h, trades, winners, losers, loss_rate);
    }
}

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

fn load_binance_tf(instrument: &str, timeframe: i64) -> Vec<CandleStick> {
    let path = match (instrument, timeframe) {
        ("eth", 15) => "assets/binance_ETHUSDT_15m.json",
        ("eth", 60) => "assets/binance_ETHUSDT_1h.json",
        ("eth", 240) => "assets/binance_ETHUSDT_4h.json",
        _ => unreachable!(),
    };
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("failed loading {path}: {e}"));
    CandleStickLoader::load_source(CandleDataSource::BinanceJsonStr(&raw))
        .unwrap_or_else(|e| panic!("failed parsing {path}: {e}"))
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

fn is_doji(
    doji_type: &str,
    body_pct: Decimal,
    upper_wick_pct: Decimal,
    lower_wick_pct: Decimal,
    body_pct_max: Decimal,
) -> bool {
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

fn measure_reversal_distance(
    data: &[CandleStick],
    doji_type: &str,
    body_pct_max: Decimal,
    lookahead_bars: usize,
) {
    let mut moves: Vec<Decimal> = Vec::new();
    let mut unresolved = 0usize;
    for i in 0..data.len().saturating_sub(1) {
        let c = data[i];
        let ny_time = to_new_york_time(c.open_time).time();
        if ny_time < chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap()
            || ny_time >= chrono::NaiveTime::from_hms_opt(15, 30, 0).unwrap()
        {
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
        if !is_doji(
            doji_type,
            body_pct,
            upper_wick_pct,
            lower_wick_pct,
            body_pct_max,
        ) {
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
    let mean = if count == 0 {
        Decimal::ZERO
    } else {
        moves.iter().copied().sum::<Decimal>() / Decimal::from(count as u32)
    };
    println!("Reversal-distance study");
    println!(
        "resolved_signals: {} | unresolved_within_lookahead: {}",
        count, unresolved
    );
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
        if ny_time < chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap()
            || ny_time >= chrono::NaiveTime::from_hms_opt(15, 30, 0).unwrap()
        {
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
        if !is_doji(
            doji_type,
            body_pct,
            upper_wick_pct,
            lower_wick_pct,
            body_pct_max,
        ) {
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
    println!(
        "p25_risk: {} | p50_risk: {} | p75_risk: {} | p90_risk: {}",
        percentile(risks.clone(), 25).round_dp(2),
        percentile(risks.clone(), 50).round_dp(2),
        percentile(risks.clone(), 75).round_dp(2),
        percentile(risks.clone(), 90).round_dp(2),
    );
    if n > 0 {
        let n_dec = Decimal::from(n as u32);
        println!(
            ">25pts: {} ({}%) | >40pts: {} ({}%)",
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
    max_sl_mode: String,
    max_sl_points: Decimal,
    tp_mode: String,
    tp_value: Decimal,
    doji_type: String,
    max_trades_per_day: usize,
    session_start: NaiveTime,
    session_end: NaiveTime,
    stop_buffer_ticks: i32,
    trail_activate: Decimal,
    trail_distance: Decimal,
    slippage_ticks: i32,
    commission_rt: Decimal,
}

#[derive(Clone)]
struct SweepRow {
    case: SweepCase,
    trades: usize,
    win_rate: Decimal,
    profit_factor_r: Decimal,
    profit_r: Decimal,
    points: Decimal,
    pnl_usd_net_est: Decimal,
    fill_rate_pct: Decimal,
    costs: Decimal,
}

fn sweep_case_key(case: &SweepCase) -> String {
    format!(
        "entry={};max_sl_mode={};max_sl={};doji={};max_trades/day={};session={}-{};stop_buffer={};trail={}/{};tp_mode={};tp={}",
        case.entry,
        case.max_sl_mode,
        case.max_sl_points,
        case.doji_type,
        case.max_trades_per_day,
        case.session_start,
        case.session_end,
        case.stop_buffer_ticks,
        case.trail_activate,
        case.trail_distance,
        case.tp_mode,
        case.tp_value
    )
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
            max_trades_per_day: case.max_trades_per_day,
            session_start: case.session_start,
            session_end: case.session_end,
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
            max_sl_points: Some(case.max_sl_points),
            max_sl_mode: match case.max_sl_mode.as_str() {
                "limit_reprice" => MaxSlMode::LimitReprice,
                _ => MaxSlMode::MarketStopCap,
            },
            execution: ExecutionConfig {
                commission_rate_per_side: Decimal::ZERO,
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: case.slippage_ticks,
                tick_size,
            },
            ..DojiConfig::default()
        },
    };
    let setups = strategy.detect_setups();
    let (trades, metrics) =
        run_setups_with_metrics(&strategy.data, &setups, &strategy.config.execution);
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
    let losers = result.result(TradeResult::Expense);
    let gross_loss_r = Decimal::from(losers as u32);
    let gross_profit_r = result.profit_in_r() + gross_loss_r;
    let profit_factor_r = if gross_loss_r > Decimal::ZERO {
        (gross_profit_r / gross_loss_r).round_dp(2)
    } else if gross_profit_r > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    };
    let point_value = match case.instrument.as_str() {
        "mnq" => Decimal::from(2),
        "mes" => Decimal::from(5),
        "gc" => Decimal::from(10),
        "eth" => Decimal::ONE,
        _ => Decimal::ONE,
    };
    let points = result.profit_in_points();
    let pnl_usd_gross = (points * point_value).round_dp(2);
    let commissions_total = (case.commission_rt * Decimal::from(trades as u32)).round_dp(2);
    let pnl_usd_net_est = (pnl_usd_gross - commissions_total).round_dp(2);
    let fill_rate_pct = if metrics.limit_orders_placed == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from(metrics.limit_orders_filled as u32)
            / Decimal::from(metrics.limit_orders_placed as u32)
            * Decimal::from(100))
        .round_dp(2)
    };
    let _ = timeframe;
    SweepRow {
        case: case.clone(),
        trades,
        win_rate: win_rate.round_dp(2),
        profit_factor_r,
        profit_r: result.profit_in_r(),
        points,
        pnl_usd_net_est,
        fill_rate_pct,
        costs: result.costs_total(),
    }
}

fn main() {
    let matches = Command::new("Doji Runner")
        .arg(
            Arg::new("instrument")
                .long("instrument")
                .value_parser(["mnq", "mes", "gc", "eth"])
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
                .value_parser([
                    "classic",
                    "strict",
                    "long_legged",
                    "dragonfly",
                    "gravestone",
                    "loose",
                ])
                .default_value("classic"),
        )
        .arg(
            Arg::new("entry")
                .long("entry")
                .value_parser(["midpoint_limit", "market_close"])
                .default_value("midpoint_limit"),
        )
        .arg(
            Arg::new("body-pct-max")
                .long("body-pct-max")
                .value_parser(clap::value_parser!(f64))
                .default_value("5"),
        )
        .arg(
            Arg::new("stop-buffer-ticks")
                .long("stop-buffer-ticks")
                .value_parser(clap::value_parser!(i32))
                .default_value("1"),
        )
        .arg(
            Arg::new("limit-timeout")
                .long("limit-timeout")
                .value_parser(clap::value_parser!(usize))
                .default_value("5"),
        )
        .arg(
            Arg::new("trail-activate")
                .long("trail-activate")
                .value_parser(clap::value_parser!(f64))
                .default_value("10"),
        )
        .arg(
            Arg::new("trail-distance")
                .long("trail-distance")
                .value_parser(clap::value_parser!(f64))
                .default_value("10"),
        )
        .arg(
            Arg::new("max-trades-per-day")
                .long("max-trades-per-day")
                .value_parser(clap::value_parser!(usize))
                .default_value("3"),
        )
        .arg(
            Arg::new("tp-points")
                .long("tp-points")
                .value_parser(clap::value_parser!(f64))
                .required(false),
        )
        .arg(
            Arg::new("tp-runner-r")
                .long("tp-runner-r")
                .value_parser(clap::value_parser!(f64))
                .default_value("100"),
        )
        .arg(
            Arg::new("max-sl-points")
                .long("max-sl-points")
                .value_parser(clap::value_parser!(f64))
                .required(false),
        )
        .arg(
            Arg::new("max-sl-mode")
                .long("max-sl-mode")
                .value_parser(["market_stop_cap", "limit_reprice"])
                .default_value("market_stop_cap"),
        )
        .arg(
            Arg::new("commission-rt")
                .long("commission-rt")
                .value_parser(clap::value_parser!(f64))
                .required(false),
        )
        .arg(
            Arg::new("slippage-ticks")
                .long("slippage-ticks")
                .value_parser(clap::value_parser!(i32))
                .default_value("1"),
        )
        .arg(
            Arg::new("from-ts")
                .long("from-ts")
                .value_parser(clap::value_parser!(i64))
                .required(false),
        )
        .arg(
            Arg::new("to-ts")
                .long("to-ts")
                .value_parser(clap::value_parser!(i64))
                .required(false),
        )
        .arg(
            Arg::new("measure-reversal")
                .long("measure-reversal")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("measure-market-risk")
                .long("measure-market-risk")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("entry-time-profile")
                .long("entry-time-profile")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("lookahead-bars")
                .long("lookahead-bars")
                .value_parser(clap::value_parser!(usize))
                .default_value("100"),
        )
        .arg(
            Arg::new("session-start")
                .long("session-start")
                .value_parser(clap::value_parser!(String))
                .default_value("09:30"),
        )
        .arg(
            Arg::new("session-end")
                .long("session-end")
                .value_parser(clap::value_parser!(String))
                .default_value("15:30"),
        )
        .arg(
            Arg::new("sweep")
                .long("sweep")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("hunt-2025")
                .long("hunt-2025")
                .help("Run 2025+ objective view: points/week with slip 1/2/3 robustness gate")
                .action(clap::ArgAction::SetTrue),
        )
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
    let session_start = matches
        .get_one::<String>("session-start")
        .and_then(|v| NaiveTime::parse_from_str(v, "%H:%M").ok())
        .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 30, 0).unwrap());
    let session_end = matches
        .get_one::<String>("session-end")
        .and_then(|v| NaiveTime::parse_from_str(v, "%H:%M").ok())
        .unwrap_or_else(|| NaiveTime::from_hms_opt(15, 30, 0).unwrap());

    let mut data = if instrument == "eth" {
        load_binance_tf(instrument, timeframe)
    } else {
        let base = load_1m(instrument);
        resample_from_1m(&base, timeframe)
    };
    if let Some(from_ts) = matches.get_one::<i64>("from-ts") {
        data.retain(|c| c.open_time >= *from_ts);
    }
    if let Some(to_ts) = matches.get_one::<i64>("to-ts") {
        data.retain(|c| c.open_time <= *to_ts);
    }
    if matches.get_flag("measure-reversal") {
        let lookahead = *matches
            .get_one::<usize>("lookahead-bars")
            .unwrap_or(&100usize);
        let body_pct_max =
            Decimal::from_f64_retain(*matches.get_one::<f64>("body-pct-max").unwrap_or(&5.0))
                .unwrap_or(Decimal::from(5));
        measure_reversal_distance(&data, doji_type, body_pct_max, lookahead);
        return;
    }

    if matches.get_flag("measure-market-risk") {
        let body_pct_max =
            Decimal::from_f64_retain(*matches.get_one::<f64>("body-pct-max").unwrap_or(&5.0))
                .unwrap_or(Decimal::from(5));
        let stop_buffer_ticks = *matches.get_one::<i32>("stop-buffer-ticks").unwrap_or(&1);
        let tick_size = match instrument {
            "mnq" | "mes" => Decimal::new(25, 2),
            "gc" => Decimal::new(1, 1),
            "eth" => Decimal::new(1, 2),
            _ => Decimal::new(25, 2),
        };
        measure_market_risk_profile(&data, doji_type, body_pct_max, stop_buffer_ticks, tick_size);
        return;
    }

    if matches.get_flag("sweep") {
        let hunt_2025 = matches.get_flag("hunt-2025");
        let from_ts = if hunt_2025 {
            Some(
                chrono_tz::America::New_York
                    .with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
                    .single()
                    .unwrap()
                    .timestamp(),
            )
        } else {
            matches.get_one::<i64>("from-ts").copied()
        };
        let instruments = ["mnq"];
        let entries = ["market_close"];
        let max_sl_modes = ["limit_reprice"];
        let doji_types = ["classic"];
        let max_trades_caps = [10usize];
        let sessions = [(
            "winner_window",
            NaiveTime::from_hms_opt(4, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        )];
        let stop_buffers = [1i32];
        let trail_pairs = [(6i64, 6i64), (8i64, 8i64), (10i64, 10i64)];
        let max_sls = [10i64, 11, 12, 13, 14];
        let tp_points = [225i64, 250, 275, 300];
        let slippage_ticks = [1i32, 2, 3];
        let commission_rt = matches
            .get_one::<f64>("commission-rt")
            .and_then(|v| Decimal::from_f64_retain(*v))
            .unwrap_or_else(|| {
                Decimal::from_f64_retain(1.32).unwrap_or(Decimal::from(132) / Decimal::from(100))
            });

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
                    for cap in max_trades_caps {
                        for (_, sess_start, sess_end) in sessions {
                            for sb in stop_buffers {
                                for (ta, td) in trail_pairs {
                                    for msm in max_sl_modes {
                                        for msl in max_sls {
                                            for tp in tp_points {
                                                for slip in slippage_ticks {
                                                    cases.push(SweepCase {
                                                        instrument: ins.to_string(),
                                                        entry: ent.to_string(),
                                                        max_sl_mode: msm.to_string(),
                                                        max_sl_points: Decimal::from(msl),
                                                        tp_mode: "fixed_points".to_string(),
                                                        tp_value: Decimal::from(tp),
                                                        doji_type: dt.to_string(),
                                                        max_trades_per_day: cap,
                                                        session_start: sess_start,
                                                        session_end: sess_end,
                                                        stop_buffer_ticks: sb,
                                                        trail_activate: Decimal::from(ta),
                                                        trail_distance: Decimal::from(td),
                                                        slippage_ticks: slip,
                                                        commission_rt,
                                                    });
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

        if hunt_2025 {
            let sample = datasets
                .first()
                .map(|(_, d)| Arc::clone(d))
                .expect("missing dataset for week span");
            let first_ts = sample.first().map(|c| c.open_time).unwrap_or(0);
            let last_ts = sample.last().map(|c| c.close_time).unwrap_or(first_ts + 1);
            let span_secs = (last_ts - first_ts).max(1);
            let weeks = Decimal::from(span_secs) / Decimal::from(7 * 24 * 60 * 60);

            println!(
                "2025+ hunt mode: bars={} span_weeks={}",
                sample.len(),
                weeks.round_dp(2)
            );
            println!("Top 20 rows by points/week:");
            let mut ranked = rows.clone();
            ranked.sort_by(|a, b| {
                let apw = if weeks > Decimal::ZERO {
                    a.points / weeks
                } else {
                    Decimal::ZERO
                };
                let bpw = if weeks > Decimal::ZERO {
                    b.points / weeks
                } else {
                    Decimal::ZERO
                };
                bpw.cmp(&apw)
                    .then(b.pnl_usd_net_est.cmp(&a.pnl_usd_net_est))
                    .then(b.profit_factor_r.cmp(&a.profit_factor_r))
            });
            for r in ranked.iter().take(20) {
                let pw = if weeks > Decimal::ZERO {
                    r.points / weeks
                } else {
                    Decimal::ZERO
                };
                let uw = if weeks > Decimal::ZERO {
                    r.pnl_usd_net_est / weeks
                } else {
                    Decimal::ZERO
                };
                println!(
                    "{} | slip={} trades={} win%={} pf_r={} points/wk={} net_usd/wk={} net_usd={}",
                    sweep_case_key(&r.case),
                    r.case.slippage_ticks,
                    r.trades,
                    r.win_rate,
                    r.profit_factor_r,
                    pw.round_dp(2),
                    uw.round_dp(2),
                    r.pnl_usd_net_est
                );
            }

            let mut grouped: HashMap<String, HashMap<i32, SweepRow>> = HashMap::new();
            for r in ranked {
                grouped
                    .entry(sweep_case_key(&r.case))
                    .or_default()
                    .insert(r.case.slippage_ticks, r);
            }
            let mut robust = Vec::new();
            for (k, by_slip) in grouped {
                if let (Some(s1), Some(s2), Some(s3)) =
                    (by_slip.get(&1), by_slip.get(&2), by_slip.get(&3))
                {
                    let p1 = if weeks > Decimal::ZERO {
                        s1.points / weeks
                    } else {
                        Decimal::ZERO
                    };
                    let p2 = if weeks > Decimal::ZERO {
                        s2.points / weeks
                    } else {
                        Decimal::ZERO
                    };
                    let p3 = if weeks > Decimal::ZERO {
                        s3.points / weeks
                    } else {
                        Decimal::ZERO
                    };
                    let min_p = p1.min(p2).min(p3);
                    if min_p >= Decimal::from(100) {
                        robust.push((k, min_p));
                    }
                }
            }
            robust.sort_by(|a, b| b.1.cmp(&a.1));
            println!("\nRobustness gate (slip 1/2/3 min points/week >= 100):");
            if robust.is_empty() {
                println!("No candidate passed.");
            } else {
                for (i, (k, p)) in robust.iter().take(20).enumerate() {
                    println!("{}. {} | min_points/week={}", i + 1, k, p.round_dp(2));
                }
            }
            return;
        }

        println!("instrument,entry,max_sl_mode,max_sl_points,doji_type,max_trades_per_day,session_start,session_end,stop_buffer,trail_activate,trail_distance,tp_mode,tp_value,slippage_ticks,trades,win_rate,profit_factor_r,profit_r,points,pnl_usd_net_est,fill_rate_pct,costs");
        for r in &rows {
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                r.case.instrument,
                r.case.entry,
                r.case.max_sl_mode,
                r.case.max_sl_points,
                r.case.doji_type,
                r.case.max_trades_per_day,
                r.case.session_start,
                r.case.session_end,
                r.case.stop_buffer_ticks,
                r.case.trail_activate,
                r.case.trail_distance,
                r.case.tp_mode,
                r.case.tp_value,
                r.case.slippage_ticks,
                r.trades,
                r.win_rate,
                r.profit_factor_r,
                r.profit_r,
                r.points,
                r.pnl_usd_net_est,
                r.fill_rate_pct,
                r.costs
            );
        }
        return;
    }

    let tick_size = match instrument {
        "mnq" | "mes" => Decimal::new(25, 2),
        "gc" => Decimal::new(1, 1),
        "eth" => Decimal::new(1, 2),
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
            body_pct_max: Decimal::from_f64_retain(
                *matches.get_one::<f64>("body-pct-max").unwrap_or(&5.0),
            )
            .unwrap_or(Decimal::from(5)),
            stop_buffer_ticks: *matches.get_one::<i32>("stop-buffer-ticks").unwrap_or(&1),
            limit_timeout_bars: *matches.get_one::<usize>("limit-timeout").unwrap_or(&5),
            trail_activate_points: Decimal::from_f64_retain(
                *matches.get_one::<f64>("trail-activate").unwrap_or(&10.0),
            )
            .unwrap_or(Decimal::from(10)),
            trail_distance_points: Decimal::from_f64_retain(
                *matches.get_one::<f64>("trail-distance").unwrap_or(&10.0),
            )
            .unwrap_or(Decimal::from(10)),
            max_trades_per_day: *matches.get_one::<usize>("max-trades-per-day").unwrap_or(&3),
            session_start,
            session_end,
            entry_mode: match entry {
                "market_close" => DojiEntryMode::MarketClose,
                _ => DojiEntryMode::MidpointLimit,
            },
            target_mode,
            max_sl_points: matches
                .get_one::<f64>("max-sl-points")
                .and_then(|v| Decimal::from_f64_retain(*v)),
            max_sl_mode: match matches
                .get_one::<String>("max-sl-mode")
                .map(|s| s.as_str())
                .unwrap_or("market_stop_cap")
            {
                "limit_reprice" => MaxSlMode::LimitReprice,
                _ => MaxSlMode::MarketStopCap,
            },
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
    let (trades, metrics) =
        run_setups_with_metrics(&strategy.data, &setups, &strategy.config.execution);
    let result = BacktestResult {
        trades,
        capital: Decimal::from(1000),
    };
    let point_value = match instrument {
        "mnq" => Decimal::from(2),
        "mes" => Decimal::from(5),
        "gc" => Decimal::from(10),
        "eth" => Decimal::ONE,
        _ => Decimal::ONE,
    };
    let pnl_usd_gross = (result.profit_in_points() * point_value).round_dp(2);
    let commission_rt = matches
        .get_one::<f64>("commission-rt")
        .and_then(|v| Decimal::from_f64_retain(*v))
        .unwrap_or_else(|| match instrument {
            "mnq" | "mes" => Decimal::from_f64_retain(1.32).unwrap(),
            "gc" => Decimal::from_f64_retain(2.20).unwrap(),
            "eth" => Decimal::ZERO,
            _ => Decimal::ZERO,
        });
    let commissions_total =
        (commission_rt * Decimal::from(result.number_of_trades() as u32)).round_dp(2);
    let pnl_usd_net = (pnl_usd_gross - commissions_total).round_dp(2);
    println!(
        "Doji strategy: {} {}m",
        instrument.to_uppercase(),
        timeframe
    );
    println!(
        "doji_type={} entry={} tp_mode={}",
        doji_type,
        entry,
        if matches.get_one::<f64>("tp-points").is_some() {
            "fixed_points"
        } else {
            "runner_r"
        }
    );
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
    if matches.get_flag("entry-time-profile") {
        print_entry_time_profile(&result.trades);
    }
    println!("pnl_usd_gross_est : {}", pnl_usd_gross);
    println!(
        "commission_rt_used: {} | commissions_total_est: {}",
        commission_rt, commissions_total
    );
    println!("pnl_usd_net_est   : {}", pnl_usd_net);
}
