use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use chrono_tz::America::Chicago;
use clap::Parser;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::VecDeque;

#[path = "or_backtest_port/asia_sweep.rs"]
mod asia_sweep;
#[path = "or_backtest_port/cr.rs"]
mod cr;
#[path = "or_backtest_port/crt1m.rs"]
mod crt1m;
#[path = "or_backtest_port/pat_po3.rs"]
mod pat_po3;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum EntryFillMode {
    Boundary,
    Close,
    NextOpen,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
enum StrategyKind {
    Orb,
    MultiOrb,
    FinalBoss,
    Orb30m15m,
    AsiaSweep,
    Cr,
    PatPo3,
    Crt1m,
}

#[derive(Debug, Clone)]
struct Config {
    strategy: StrategyKind,
    csv_path: String,
    or_duration: String,
    breakout_candles: usize,
    reverse_logic: bool,
    last_entry_time: String,

    entry_fill_mode: EntryFillMode,

    position_sizing_type: String,
    fixed_contracts: f64,
    fixed_usd_risk: f64,
    tick_value: f64,
    sl_type: String,
    sl_value: f64,
    tp_type: String,
    tp_value: f64,
    atr_length: usize,
    enable_second_chance: bool,
    rth_start: String,
    rth_end: String,
    commission_round_trip_per_contract: f64,
    slippage_ticks_per_side: f64,
    tick_size: f64,
    min_price_step: f64,
}

#[derive(Debug, Parser)]
#[command(about = "OR breakout strategy backtest.")]
struct Args {
    #[arg(long, value_enum, default_value = "orb")]
    strategy: StrategyKind,
    #[arg(long = "csv", default_value = "mnq_1m.csv")]
    csv_path: String,
    #[arg(long, default_value = "15 Minutes")]
    or_duration: String,
    #[arg(long, default_value_t = 2)]
    breakout_candles: usize,
    #[arg(long)]
    reverse_logic: bool,
    #[arg(long, default_value = "")]
    last_entry_time: String,

    #[arg(long, value_enum, default_value = "next-open")]
    entry_fill_mode: EntryFillMode,

    #[arg(long, default_value = "Fixed Contracts")]
    position_sizing_type: String,
    #[arg(long, default_value_t = 1.0)]
    fixed_contracts: f64,
    #[arg(long, default_value_t = 100.0)]
    fixed_usd_risk: f64,
    #[arg(long, default_value_t = 2.0)]
    tick_value: f64,
    #[arg(long, default_value = "ATR Multiple")]
    sl_type: String,
    #[arg(long, default_value_t = 2.0)]
    sl_value: f64,
    #[arg(long, default_value = "Risk Reward")]
    tp_type: String,
    #[arg(long, default_value_t = 1.0)]
    tp_value: f64,
    #[arg(long, default_value_t = 14)]
    atr_length: usize,
    #[arg(long, default_value_t = 1.32)]
    commission_round_trip_per_contract: f64,
    #[arg(long, default_value_t = 1.0)]
    slippage_ticks_per_side: f64,
    #[arg(long, default_value_t = 0.25)]
    tick_size: f64,
    #[arg(long)]
    enable_second_chance: bool,
    #[arg(long, default_value = "")]
    save_trades: String,
    #[arg(long)]
    sweep: bool,
    #[arg(long, default_value_t = 20)]
    sweep_top: usize,
    #[arg(long, default_value_t = 200)]
    sweep_min_trades: usize,
    #[arg(long, value_enum, default_value = "score")]
    sweep_sort: SortKey,
}

#[derive(Debug, Clone, Deserialize)]
struct RawRow {
    ts_event: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    #[allow(dead_code)]
    #[serde(default)]
    symbol: String,
}

#[derive(Debug, Clone)]
struct InterimRow {
    datetime_utc: DateTime<Utc>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[derive(Debug, Clone)]
struct BarRow {
    datetime_ct: DateTime<chrono_tz::Tz>,
    date: NaiveDate,
    time: NaiveTime,
    #[allow(dead_code)]
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    #[allow(dead_code)]
    volume: f64,
    atr: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Long,
    Short,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitReason {
    Tp,
    Sl,
    Eod,
}

impl ExitReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Tp => "TP",
            Self::Sl => "SL",
            Self::Eod => "EOD",
        }
    }
}

#[derive(Debug, Clone)]
struct Trade {
    date: NaiveDate,
    direction: Direction,
    entry_time: DateTime<chrono_tz::Tz>,
    exit_time: DateTime<chrono_tz::Tz>,
    entry_price: f64,
    exit_price: f64,
    stop_loss: f64,
    take_profit: f64,
    qty: f64,
    reason: ExitReason,
    pnl_points: f64,
    pnl_usd: f64,
    risk_points: f64,
    rr: f64,
    is_second_chance: bool,
}

#[derive(Debug, Clone)]
struct Stats {
    num_trades: usize,
    winners: usize,
    losers: usize,
    breakeven: usize,
    winrate_pct: f64,
    avg_rr: f64,
    expectancy_rr: f64,
    gross_profit_usd: f64,
    gross_loss_usd: f64,
    net_profit_usd: f64,
    profit_factor: f64,
    avg_win_usd: f64,
    avg_loss_usd: f64,
    max_win_usd: f64,
    max_loss_usd: f64,
    max_drawdown_usd: f64,
    calmar_like: f64,
    score: f64,
}

#[derive(Debug, Clone)]
struct SweepRow {
    score: f64,
    net_profit_usd: f64,
    profit_factor: f64,
    max_drawdown_usd: f64,
    calmar_like: f64,
    winrate_pct: f64,
    avg_rr: f64,
    num_trades: usize,
    or_duration: String,
    breakout_candles: usize,
    reverse_logic: bool,
    enable_second_chance: bool,
    last_entry_time: String,
    sl_type: String,
    sl_value: f64,
    tp_type: String,
    tp_value: f64,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
enum SortKey {
    Score,
    NetProfitUsd,
    ProfitFactor,
    CalmarLike,
}

fn parse_hhmm(s: &str) -> Result<NaiveTime> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid HH:MM time: {s}"));
    }
    let hh: u32 = parts[0]
        .parse()
        .with_context(|| format!("Invalid hour in {s}"))?;
    let mm: u32 = parts[1]
        .parse()
        .with_context(|| format!("Invalid minute in {s}"))?;
    NaiveTime::from_hms_opt(hh, mm, 0).ok_or_else(|| anyhow!("Invalid HH:MM time: {s}"))
}

fn or_end_time_from_duration(or_duration: &str) -> Result<NaiveTime> {
    match or_duration {
        "5 Minutes" => Ok(NaiveTime::from_hms_opt(8, 35, 0).unwrap()),
        "15 Minutes" => Ok(NaiveTime::from_hms_opt(8, 45, 0).unwrap()),
        "30 Minutes" => Ok(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
        "45 Minutes" => Ok(NaiveTime::from_hms_opt(9, 15, 0).unwrap()),
        "60 Minutes" => Ok(NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
        _ => Err(anyhow!("Unsupported or_duration: {or_duration}")),
    }
}

fn parse_timestamp_utc(ts: &str) -> Result<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        return Ok(dt.with_timezone(&Utc));
    }

    let formats = [
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
    ];
    for fmt in formats {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(ts, fmt) {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
        }
    }
    Err(anyhow!("Unable to parse ts_event timestamp: {ts}"))
}

fn compute_atr(rows: &mut [BarRow], length: usize) {
    if length == 0 {
        for row in rows.iter_mut() {
            row.atr = f64::NAN;
        }
        return;
    }

    let mut trs = VecDeque::with_capacity(length + 1);
    let mut tr_sum = 0.0_f64;
    let mut prev_close: Option<f64> = None;

    for row in rows.iter_mut() {
        let tr = if let Some(pc) = prev_close {
            (row.high - row.low)
                .abs()
                .max((row.high - pc).abs())
                .max((row.low - pc).abs())
        } else {
            (row.high - row.low).abs()
        };
        trs.push_back(tr);
        tr_sum += tr;

        if trs.len() > length {
            if let Some(old) = trs.pop_front() {
                tr_sum -= old;
            }
        }
        row.atr = if trs.len() == length {
            tr_sum / length as f64
        } else {
            f64::NAN
        };
        prev_close = Some(row.close);
    }
}

fn get_stop_loss(
    entry_price: f64,
    is_long: bool,
    sl_type: &str,
    sl_value: f64,
    range_size: f64,
    atr_value: f64,
    session_high: f64,
    session_low: f64,
) -> Result<f64> {
    match sl_type {
        "Range %" => {
            let d = range_size * sl_value / 100.0;
            Ok(if is_long {
                entry_price - d
            } else {
                entry_price + d
            })
        }
        "ATR Multiple" => {
            let d = atr_value * sl_value;
            Ok(if is_long {
                entry_price - d
            } else {
                entry_price + d
            })
        }
        "Fixed %" => Ok(if is_long {
            entry_price * (1.0 - sl_value / 100.0)
        } else {
            entry_price * (1.0 + sl_value / 100.0)
        }),
        "Fixed Points" => Ok(if is_long {
            entry_price - sl_value
        } else {
            entry_price + sl_value
        }),
        "Opposite Range" => Ok(if is_long { session_low } else { session_high }),
        _ => Err(anyhow!("Unsupported sl_type: {sl_type}")),
    }
}

fn get_take_profit(
    entry_price: f64,
    stop_loss: f64,
    is_long: bool,
    tp_type: &str,
    tp_value: f64,
    range_size: f64,
    atr_value: f64,
) -> Result<f64> {
    let risk = (entry_price - stop_loss).abs();
    match tp_type {
        "Risk Reward" => Ok(if is_long {
            entry_price + risk * tp_value
        } else {
            entry_price - risk * tp_value
        }),
        "Range %" => {
            let d = range_size * tp_value / 100.0;
            Ok(if is_long {
                entry_price + d
            } else {
                entry_price - d
            })
        }
        "ATR Multiple" => {
            let d = atr_value * tp_value;
            Ok(if is_long {
                entry_price + d
            } else {
                entry_price - d
            })
        }
        "Fixed %" => Ok(if is_long {
            entry_price * (1.0 + tp_value / 100.0)
        } else {
            entry_price * (1.0 - tp_value / 100.0)
        }),
        "Fixed Points" => Ok(if is_long {
            entry_price + tp_value
        } else {
            entry_price - tp_value
        }),
        _ => Err(anyhow!("Unsupported tp_type: {tp_type}")),
    }
}

fn get_position_size(
    entry_price: f64,
    stop_loss: f64,
    position_sizing_type: &str,
    fixed_contracts: f64,
    fixed_usd_risk: f64,
    tick_value: f64,
) -> Result<f64> {
    match position_sizing_type {
        "Fixed Contracts" => Ok(fixed_contracts),
        "Fixed USD Risk" => {
            let stop_dist = (entry_price - stop_loss).abs();
            if stop_dist <= 0.0 {
                return Ok(fixed_contracts);
            }
            let risk_per_contract = stop_dist * tick_value;
            Ok((fixed_usd_risk / risk_per_contract).max(0.01))
        }
        _ => Err(anyhow!(
            "Unsupported position_sizing_type: {position_sizing_type}"
        )),
    }
}

fn has_valid_risk_geometry(
    direction: Direction,
    entry_price: f64,
    stop_loss: f64,
    take_profit: f64,
    min_price_step: f64,
) -> bool {
    match direction {
        Direction::Long => {
            stop_loss < entry_price - min_price_step
                && take_profit > entry_price
                && (entry_price - stop_loss).abs() >= min_price_step
        }
        Direction::Short => {
            stop_loss > entry_price + min_price_step
                && take_profit < entry_price
                && (entry_price - stop_loss).abs() >= min_price_step
        }
    }
}

fn load_deduped_data(cfg: &Config) -> Result<Vec<BarRow>> {
    let mut rdr = csv::Reader::from_path(&cfg.csv_path)
        .with_context(|| format!("Failed to open CSV: {}", cfg.csv_path))?;
    let mut rows: Vec<InterimRow> = Vec::new();

    for rec in rdr.deserialize::<RawRow>() {
        let rec = rec?;
        let dt_utc = parse_timestamp_utc(&rec.ts_event)?;
        rows.push(InterimRow {
            datetime_utc: dt_utc,
            open: rec.open,
            high: rec.high,
            low: rec.low,
            close: rec.close,
            volume: rec.volume,
        });
    }

    rows.sort_by(|a, b| match a.datetime_utc.cmp(&b.datetime_utc) {
        Ordering::Equal => b.volume.partial_cmp(&a.volume).unwrap_or(Ordering::Equal),
        ord => ord,
    });

    let mut deduped = Vec::with_capacity(rows.len());
    let mut last_dt: Option<DateTime<Utc>> = None;
    for row in rows {
        if last_dt.as_ref().is_some_and(|dt| *dt == row.datetime_utc) {
            continue;
        }
        last_dt = Some(row.datetime_utc);
        deduped.push(row);
    }

    let rth_start = parse_hhmm(&cfg.rth_start)?;
    let rth_end = parse_hhmm(&cfg.rth_end)?;
    let mut bars: Vec<BarRow> = Vec::new();

    for row in deduped {
        let dt_ct = row.datetime_utc.with_timezone(&Chicago);
        let t = NaiveTime::from_hms_opt(dt_ct.hour(), dt_ct.minute(), dt_ct.second()).unwrap();
        if t < rth_start || t > rth_end {
            continue;
        }
        bars.push(BarRow {
            datetime_ct: dt_ct,
            date: dt_ct.date_naive(),
            time: t,
            open: row.open,
            high: row.high,
            low: row.low,
            close: row.close,
            volume: row.volume,
            atr: f64::NAN,
        });
    }

    compute_atr(&mut bars, cfg.atr_length);
    Ok(bars)
}

fn resolve_exit_for_bar(
    row: &BarRow,
    direction: Direction,
    stop_loss: f64,
    take_profit: f64,
    slippage_points_per_side: f64,
) -> Option<(f64, ExitReason)> {
    // Gap-through handling:
    // If the bar opens beyond the stop/target, assume fill at open (worsened by slippage),
    // NOT at the stop/target price. This prevents underestimating losses (and overestimating wins)
    // on fast moves and around session boundaries.
    //
    // With only OHLC, tie-breakers are still needed when both SL/TP are touched intrabar.
    let o = row.open;
    let h = row.high;
    let l = row.low;

    match direction {
        Direction::Long => {
            // Gap-down through stop
            if o <= stop_loss {
                let fill = o - slippage_points_per_side;
                return Some((fill, ExitReason::Sl));
            }
            // Gap-up through target
            if o >= take_profit {
                let fill = o - slippage_points_per_side;
                return Some((fill, ExitReason::Tp));
            }

            let sl_hit = l <= stop_loss;
            let tp_hit = h >= take_profit;

            if sl_hit && tp_hit {
                // Conservative: stop first
                Some((stop_loss - slippage_points_per_side, ExitReason::Sl))
            } else if sl_hit {
                Some((stop_loss - slippage_points_per_side, ExitReason::Sl))
            } else if tp_hit {
                Some((take_profit - slippage_points_per_side, ExitReason::Tp))
            } else {
                None
            }
        }
        Direction::Short => {
            // Gap-up through stop
            if o >= stop_loss {
                let fill = o + slippage_points_per_side;
                return Some((fill, ExitReason::Sl));
            }
            // Gap-down through target
            if o <= take_profit {
                let fill = o + slippage_points_per_side;
                return Some((fill, ExitReason::Tp));
            }

            let sl_hit = h >= stop_loss;
            let tp_hit = l <= take_profit;

            if sl_hit && tp_hit {
                // Conservative: stop first
                Some((stop_loss + slippage_points_per_side, ExitReason::Sl))
            } else if sl_hit {
                Some((stop_loss + slippage_points_per_side, ExitReason::Sl))
            } else if tp_hit {
                Some((take_profit + slippage_points_per_side, ExitReason::Tp))
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SweepSide {
    High,
    Low,
}

#[derive(Debug, Clone)]
struct ReversalSpec {
    pre_start: NaiveTime,
    pre_end: NaiveTime,
    trade_start: NaiveTime,
    trade_end: NaiveTime,
    allow_second_trade: bool,
    sweep_tolerance_pct: f64,
}

fn in_window(t: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start <= end {
        t >= start && t < end
    } else {
        t >= start || t < end
    }
}

fn strategy_is_orb(kind: StrategyKind) -> bool {
    matches!(
        kind,
        StrategyKind::Orb
            | StrategyKind::MultiOrb
            | StrategyKind::FinalBoss
            | StrategyKind::Orb30m15m
    )
}

fn strategy_reversal_spec(kind: StrategyKind) -> Option<ReversalSpec> {
    match kind {
        StrategyKind::AsiaSweep => Some(asia_sweep::reversal_spec()),
        StrategyKind::Cr => Some(cr::reversal_spec()),
        StrategyKind::PatPo3 => Some(pat_po3::reversal_spec()),
        StrategyKind::Crt1m => Some(crt1m::reversal_spec()),
        _ => None,
    }
}

fn apply_strategy_preset(cfg: &mut Config) {
    match cfg.strategy {
        StrategyKind::Orb => {}
        StrategyKind::MultiOrb => {
            cfg.or_duration = "15 Minutes".to_string();
            cfg.breakout_candles = 2;
            cfg.fixed_contracts = 1.0;
            cfg.sl_type = "Range %".to_string();
            cfg.sl_value = 50.0;
            cfg.tp_type = "Risk Reward".to_string();
            cfg.tp_value = 2.0;
            cfg.rth_start = "08:30".to_string();
            cfg.rth_end = "15:00".to_string();
        }
        StrategyKind::FinalBoss => {
            cfg.or_duration = "15 Minutes".to_string();
            cfg.breakout_candles = 2;
            cfg.fixed_contracts = 1.0;
            cfg.sl_type = "ATR Multiple".to_string();
            cfg.sl_value = 2.0;
            cfg.tp_type = "Risk Reward".to_string();
            cfg.tp_value = 1.0;
            cfg.rth_start = "08:30".to_string();
            cfg.rth_end = "15:00".to_string();
        }
        StrategyKind::Orb30m15m => {
            cfg.or_duration = "30 Minutes".to_string();
            cfg.breakout_candles = 1;
            cfg.fixed_contracts = 1.0;
            cfg.sl_type = "Range %".to_string();
            cfg.sl_value = 50.0;
            cfg.tp_type = "Risk Reward".to_string();
            cfg.tp_value = 2.0;
            cfg.rth_start = "08:30".to_string();
            cfg.rth_end = "15:00".to_string();
        }
        StrategyKind::AsiaSweep => {
            asia_sweep::apply_preset(cfg);
        }
        StrategyKind::Cr => {
            cr::apply_preset(cfg);
        }
        StrategyKind::PatPo3 => {
            pat_po3::apply_preset(cfg);
        }
        StrategyKind::Crt1m => {
            crt1m::apply_preset(cfg);
        }
    }
}

fn backtest_reversal(df: &[BarRow], cfg: &Config, spec: &ReversalSpec) -> Result<Vec<Trade>> {
    let mut trades: Vec<Trade> = Vec::new();
    let slippage_points_per_side = cfg.slippage_ticks_per_side * cfg.tick_size;
    let max_trades_per_day = if spec.allow_second_trade || cfg.enable_second_chance {
        2usize
    } else {
        1usize
    };
    let mss_lookback = cfg.breakout_candles.max(2);
    let stop_atr_mult = cfg.sl_value.max(0.1);
    let fallback_rr = cfg.tp_value.max(0.5);

    let mut i = 0usize;
    while i < df.len() {
        let date = df[i].date;
        let mut j = i + 1;
        while j < df.len() && df[j].date == date {
            j += 1;
        }
        let day = &df[i..j];
        i = j;
        if day.is_empty() {
            continue;
        }

        let mut pre_high = f64::NEG_INFINITY;
        let mut pre_low = f64::INFINITY;
        let mut has_pre = false;
        for row in day {
            if in_window(row.time, spec.pre_start, spec.pre_end) {
                has_pre = true;
                pre_high = pre_high.max(row.high);
                pre_low = pre_low.min(row.low);
            }
        }
        if !has_pre || !pre_high.is_finite() || !pre_low.is_finite() || pre_high <= pre_low {
            continue;
        }

        let trade_rows: Vec<&BarRow> = day
            .iter()
            .filter(|r| in_window(r.time, spec.trade_start, spec.trade_end))
            .collect();
        if trade_rows.len() <= mss_lookback + 1 {
            continue;
        }

        let mut in_pos = false;
        let mut pos_direction = Direction::Long;
        let mut entry_price = 0.0_f64;
        let mut stop_loss = 0.0_f64;
        let mut take_profit = 0.0_f64;
        let mut qty = 0.0_f64;
        let mut entry_time = trade_rows[0].datetime_ct;
        let mut entry_bar_idx: Option<usize> = None;
        let mut trades_taken = 0usize;
        let mut sweep_side: Option<SweepSide> = None;
        let mut sweep_extreme = f64::NAN;
        let mut second_trade_flag = false;

        for (idx, row) in trade_rows.iter().enumerate() {
            if in_pos {
                if entry_bar_idx != Some(idx) {
                    if let Some((exit_price, reason)) = resolve_exit_for_bar(
                        row,
                        pos_direction,
                        stop_loss,
                        take_profit,
                        slippage_points_per_side,
                    ) {
                        let risk_points = (entry_price - stop_loss).abs();
                        let pnl_points = match pos_direction {
                            Direction::Long => exit_price - entry_price,
                            Direction::Short => entry_price - exit_price,
                        };
                        let gross_pnl_usd = pnl_points * cfg.tick_value * qty;
                        let commission_usd = cfg.commission_round_trip_per_contract * qty;
                        let pnl_usd = gross_pnl_usd - commission_usd;
                        let rr = if risk_points > 0.0 {
                            pnl_points / risk_points
                        } else {
                            f64::NAN
                        };
                        trades.push(Trade {
                            date,
                            direction: pos_direction,
                            entry_time,
                            exit_time: row.datetime_ct,
                            entry_price,
                            exit_price,
                            stop_loss,
                            take_profit,
                            qty,
                            reason,
                            pnl_points,
                            pnl_usd,
                            risk_points,
                            rr,
                            is_second_chance: second_trade_flag,
                        });
                        in_pos = false;
                        entry_bar_idx = None;
                        if trades_taken >= max_trades_per_day {
                            break;
                        }
                        continue;
                    }
                }

                if row.time >= spec.trade_end {
                    let exit_price = if pos_direction == Direction::Long {
                        row.close - slippage_points_per_side
                    } else {
                        row.close + slippage_points_per_side
                    };
                    let risk_points = (entry_price - stop_loss).abs();
                    let pnl_points = if pos_direction == Direction::Long {
                        exit_price - entry_price
                    } else {
                        entry_price - exit_price
                    };
                    let gross_pnl_usd = pnl_points * cfg.tick_value * qty;
                    let commission_usd = cfg.commission_round_trip_per_contract * qty;
                    let pnl_usd = gross_pnl_usd - commission_usd;
                    let rr = if risk_points > 0.0 {
                        pnl_points / risk_points
                    } else {
                        f64::NAN
                    };
                    trades.push(Trade {
                        date,
                        direction: pos_direction,
                        entry_time,
                        exit_time: row.datetime_ct,
                        entry_price,
                        exit_price,
                        stop_loss,
                        take_profit,
                        qty,
                        reason: ExitReason::Eod,
                        pnl_points,
                        pnl_usd,
                        risk_points,
                        rr,
                        is_second_chance: second_trade_flag,
                    });
                    break;
                }
            }

            if in_pos || trades_taken >= max_trades_per_day || !row.atr.is_finite() {
                continue;
            }

            if sweep_side.is_none() {
                if row.high > pre_high * (1.0 + spec.sweep_tolerance_pct) && row.close < pre_high {
                    sweep_side = Some(SweepSide::High);
                    sweep_extreme = row.high;
                } else if row.low < pre_low * (1.0 - spec.sweep_tolerance_pct)
                    && row.close > pre_low
                {
                    sweep_side = Some(SweepSide::Low);
                    sweep_extreme = row.low;
                }
                continue;
            }

            if idx < mss_lookback {
                continue;
            }
            let mut recent_high = f64::NEG_INFINITY;
            let mut recent_low = f64::INFINITY;
            for prev in trade_rows.iter().take(idx).skip(idx - mss_lookback) {
                recent_high = recent_high.max(prev.high);
                recent_low = recent_low.min(prev.low);
            }

            let (direction, mss_triggered) = match sweep_side.unwrap() {
                SweepSide::High => (
                    Direction::Short,
                    row.close < recent_low && row.close < row.open && row.close < pre_high,
                ),
                SweepSide::Low => (
                    Direction::Long,
                    row.close > recent_high && row.close > row.open && row.close > pre_low,
                ),
            };
            if !mss_triggered {
                continue;
            }

            let e_intended = match cfg.entry_fill_mode {
                EntryFillMode::Boundary | EntryFillMode::Close => row.close,
                EntryFillMode::NextOpen => {
                    if idx + 1 >= trade_rows.len() {
                        continue;
                    }
                    trade_rows[idx + 1].open
                }
            };
            let e_fill = match direction {
                Direction::Long => e_intended + slippage_points_per_side,
                Direction::Short => e_intended - slippage_points_per_side,
            };

            let mut sl = match direction {
                Direction::Long => sweep_extreme.min(pre_low) - row.atr * stop_atr_mult,
                Direction::Short => sweep_extreme.max(pre_high) + row.atr * stop_atr_mult,
            };
            if direction == Direction::Long && sl >= e_fill {
                sl = e_fill - cfg.min_price_step;
            }
            if direction == Direction::Short && sl <= e_fill {
                sl = e_fill + cfg.min_price_step;
            }

            let mut tp = match direction {
                Direction::Long => pre_high,
                Direction::Short => pre_low,
            };
            if !has_valid_risk_geometry(direction, e_fill, sl, tp, cfg.min_price_step) {
                let risk = (e_fill - sl).abs();
                tp = match direction {
                    Direction::Long => e_fill + risk * fallback_rr,
                    Direction::Short => e_fill - risk * fallback_rr,
                };
            }
            if !has_valid_risk_geometry(direction, e_fill, sl, tp, cfg.min_price_step) {
                continue;
            }

            let q = get_position_size(
                e_fill,
                sl,
                &cfg.position_sizing_type,
                cfg.fixed_contracts,
                cfg.fixed_usd_risk,
                cfg.tick_value,
            )?;
            if !q.is_finite() || q <= 0.0 {
                continue;
            }

            in_pos = true;
            pos_direction = direction;
            entry_price = e_fill;
            stop_loss = sl;
            take_profit = tp;
            qty = q;
            entry_time = match cfg.entry_fill_mode {
                EntryFillMode::NextOpen => trade_rows[idx + 1].datetime_ct,
                _ => row.datetime_ct,
            };
            entry_bar_idx = Some(match cfg.entry_fill_mode {
                EntryFillMode::NextOpen => idx + 1,
                _ => idx,
            });
            second_trade_flag = trades_taken > 0;
            trades_taken += 1;
        }
    }

    Ok(trades)
}

fn backtest(df: &[BarRow], cfg: &Config) -> Result<Vec<Trade>> {
    let mut trades: Vec<Trade> = Vec::new();
    let or_end = or_end_time_from_duration(&cfg.or_duration)?;
    let slippage_points_per_side = cfg.slippage_ticks_per_side * cfg.tick_size;
    let last_entry_cutoff = if cfg.last_entry_time.is_empty() {
        None
    } else {
        Some(parse_hhmm(&cfg.last_entry_time)?)
    };
    let rth_start = parse_hhmm(&cfg.rth_start)?;
    let rth_end = parse_hhmm(&cfg.rth_end)?;

    let mut i = 0usize;
    while i < df.len() {
        let date = df[i].date;
        let mut j = i + 1;
        while j < df.len() && df[j].date == date {
            j += 1;
        }
        let day = &df[i..j];
        i = j;
        if day.is_empty() {
            continue;
        }

        let mut or_any = false;
        let mut session_high = f64::NEG_INFINITY;
        let mut session_low = f64::INFINITY;
        for row in day {
            if row.time >= rth_start && row.time < or_end {
                or_any = true;
                session_high = session_high.max(row.high);
                session_low = session_low.min(row.low);
            }
        }
        if !or_any {
            continue;
        }
        let range_size = session_high - session_low;

        let trade_rows: Vec<&BarRow> = day.iter().filter(|r| r.time >= or_end).collect();
        if trade_rows.is_empty() {
            continue;
        }

        let mut consecutive_bull = 0usize;
        let mut consecutive_bear = 0usize;
        let mut traded_today = false;
        let mut first_trade_loss = false;
        let mut first_trade_direction: Option<Direction> = None;
        let mut second_chance_taken = false;

        // position state
        let mut in_pos = false;
        let mut pos_direction = Direction::Long;
        let mut entry_price = 0.0_f64;
        let mut stop_loss = 0.0_f64;
        let mut take_profit = 0.0_f64;
        let mut qty = 0.0_f64;
        let mut entry_time = trade_rows[0].datetime_ct;
        let mut is_second_chance_trade = false;
        // Prevent same-bar entry/exit: do not evaluate stops/targets on the entry bar.
        //
        // IMPORTANT: Do NOT key this off timestamp equality in a way that might accidentally match
        // other bars. The safest option is to track the index (position) of the entry bar in the
        // per-day `trade_rows` vector and skip exit evaluation only for that exact bar.
        let mut entry_bar_idx: Option<usize> = None;

        for (i, row) in trade_rows.iter().enumerate() {
            let c = row.close;
            if c > session_high {
                consecutive_bull += 1;
                consecutive_bear = 0;
            } else if c < session_low {
                consecutive_bear += 1;
                consecutive_bull = 0;
            } else {
                consecutive_bull = 0;
                consecutive_bear = 0;
            }

            // Manage open position
            if in_pos {
                // Prevent same-bar entry/exit: skip stop/target evaluation on the entry bar.
                if entry_bar_idx == Some(i) {
                    // Still allow EOD flattening below.
                } else if let Some((exit_price, reason)) = resolve_exit_for_bar(
                    row,
                    pos_direction,
                    stop_loss,
                    take_profit,
                    slippage_points_per_side,
                ) {
                    let risk_points = (entry_price - stop_loss).abs();
                    let pnl_points = match pos_direction {
                        Direction::Long => exit_price - entry_price,
                        Direction::Short => entry_price - exit_price,
                    };
                    let gross_pnl_usd = pnl_points * cfg.tick_value * qty;
                    let commission_usd = cfg.commission_round_trip_per_contract * qty;
                    let pnl_usd = gross_pnl_usd - commission_usd;
                    let rr = if risk_points > 0.0 {
                        pnl_points / risk_points
                    } else {
                        f64::NAN
                    };

                    trades.push(Trade {
                        date,
                        direction: pos_direction,
                        entry_time,
                        exit_time: row.datetime_ct,
                        entry_price,
                        exit_price,
                        stop_loss,
                        take_profit,
                        qty,
                        reason,
                        pnl_points,
                        pnl_usd,
                        risk_points,
                        rr,
                        is_second_chance: is_second_chance_trade,
                    });

                    if !is_second_chance_trade && reason == ExitReason::Sl {
                        first_trade_loss = true;
                    }
                    in_pos = false;
                    entry_bar_idx = None;
                    continue;
                }

                if row.time == rth_end {
                    let exit_price = if pos_direction == Direction::Long {
                        c - slippage_points_per_side
                    } else {
                        c + slippage_points_per_side
                    };
                    let risk_points = (entry_price - stop_loss).abs();
                    let pnl_points = if pos_direction == Direction::Long {
                        exit_price - entry_price
                    } else {
                        entry_price - exit_price
                    };
                    let gross_pnl_usd = pnl_points * cfg.tick_value * qty;
                    let commission_usd = cfg.commission_round_trip_per_contract * qty;
                    let pnl_usd = gross_pnl_usd - commission_usd;
                    let rr = if risk_points > 0.0 {
                        pnl_points / risk_points
                    } else {
                        f64::NAN
                    };

                    trades.push(Trade {
                        date,
                        direction: pos_direction,
                        entry_time,
                        exit_time: row.datetime_ct,
                        entry_price,
                        exit_price,
                        stop_loss,
                        take_profit,
                        qty,
                        reason: ExitReason::Eod,
                        pnl_points,
                        pnl_usd,
                        risk_points,
                        rr,
                        is_second_chance: is_second_chance_trade,
                    });

                    in_pos = false;
                    entry_bar_idx = None;
                    continue;
                }
            }

            if !in_pos {
                if let Some(cutoff) = last_entry_cutoff {
                    if row.time > cutoff {
                        continue;
                    }
                }

                let atr_val = row.atr;
                if !atr_val.is_finite() {
                    continue;
                }

                if !traded_today {
                    if c > session_high && consecutive_bull >= cfg.breakout_candles {
                        let direction = if cfg.reverse_logic {
                            Direction::Short
                        } else {
                            Direction::Long
                        };
                        let is_long = direction == Direction::Long;

                        // Determine intended entry based on fill mode.
                        // - Boundary: use OR boundary (matches current behavior for non-reverse).
                        // - Close: use current close.
                        // - NextOpen: use next bar open (if available), otherwise skip signal.
                        let e_intended = match cfg.entry_fill_mode {
                            EntryFillMode::Boundary => {
                                if cfg.reverse_logic {
                                    c
                                } else {
                                    session_high
                                }
                            }
                            EntryFillMode::Close => c,
                            EntryFillMode::NextOpen => {
                                if i + 1 >= trade_rows.len() {
                                    continue;
                                }
                                trade_rows[i + 1].open
                            }
                        };

                        // Apply slippage to get actual fill.
                        let e_fill = match direction {
                            Direction::Long => e_intended + slippage_points_per_side,
                            Direction::Short => e_intended - slippage_points_per_side,
                        };

                        // Compute stop/target off fill price for internal consistency (and realism).
                        let sl = get_stop_loss(
                            e_fill,
                            is_long,
                            &cfg.sl_type,
                            cfg.sl_value,
                            range_size,
                            atr_val,
                            session_high,
                            session_low,
                        )?;
                        let tp = get_take_profit(
                            e_fill,
                            sl,
                            is_long,
                            &cfg.tp_type,
                            cfg.tp_value,
                            range_size,
                            atr_val,
                        )?;

                        // Compute qty off the fill price and stop distance.
                        let q = get_position_size(
                            e_fill,
                            sl,
                            &cfg.position_sizing_type,
                            cfg.fixed_contracts,
                            cfg.fixed_usd_risk,
                            cfg.tick_value,
                        )?;

                        if has_valid_risk_geometry(direction, e_fill, sl, tp, cfg.min_price_step) {
                            in_pos = true;
                            pos_direction = direction;
                            entry_price = e_fill;
                            stop_loss = sl;
                            take_profit = tp;
                            qty = q;

                            // Entry time should match the bar price source.
                            entry_time = match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => trade_rows[i + 1].datetime_ct,
                                _ => row.datetime_ct,
                            };
                            entry_bar_idx = Some(match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => i + 1,
                                _ => i,
                            });

                            traded_today = true;
                            first_trade_direction = Some(direction);
                            is_second_chance_trade = false;
                            continue;
                        }
                    }

                    if c < session_low && consecutive_bear >= cfg.breakout_candles {
                        let direction = if cfg.reverse_logic {
                            Direction::Long
                        } else {
                            Direction::Short
                        };
                        let is_long = direction == Direction::Long;

                        let e_intended = match cfg.entry_fill_mode {
                            EntryFillMode::Boundary => {
                                if cfg.reverse_logic {
                                    c
                                } else {
                                    session_low
                                }
                            }
                            EntryFillMode::Close => c,
                            EntryFillMode::NextOpen => {
                                if i + 1 >= trade_rows.len() {
                                    continue;
                                }
                                trade_rows[i + 1].open
                            }
                        };

                        let e_fill = match direction {
                            Direction::Long => e_intended + slippage_points_per_side,
                            Direction::Short => e_intended - slippage_points_per_side,
                        };

                        let sl = get_stop_loss(
                            e_fill,
                            is_long,
                            &cfg.sl_type,
                            cfg.sl_value,
                            range_size,
                            atr_val,
                            session_high,
                            session_low,
                        )?;
                        let tp = get_take_profit(
                            e_fill,
                            sl,
                            is_long,
                            &cfg.tp_type,
                            cfg.tp_value,
                            range_size,
                            atr_val,
                        )?;

                        let q = get_position_size(
                            e_fill,
                            sl,
                            &cfg.position_sizing_type,
                            cfg.fixed_contracts,
                            cfg.fixed_usd_risk,
                            cfg.tick_value,
                        )?;

                        if has_valid_risk_geometry(direction, e_fill, sl, tp, cfg.min_price_step) {
                            in_pos = true;
                            pos_direction = direction;
                            entry_price = e_fill;
                            stop_loss = sl;
                            take_profit = tp;
                            qty = q;

                            entry_time = match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => trade_rows[i + 1].datetime_ct,
                                _ => row.datetime_ct,
                            };
                            entry_bar_idx = Some(match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => i + 1,
                                _ => i,
                            });

                            traded_today = true;
                            first_trade_direction = Some(direction);
                            is_second_chance_trade = false;
                            continue;
                        }
                    }
                }

                if cfg.enable_second_chance
                    && first_trade_loss
                    && !second_chance_taken
                    && first_trade_direction.is_some()
                {
                    if first_trade_direction == Some(Direction::Long)
                        && c < session_low
                        && consecutive_bear >= cfg.breakout_candles
                    {
                        let direction = if cfg.reverse_logic {
                            Direction::Long
                        } else {
                            Direction::Short
                        };
                        let is_long = direction == Direction::Long;

                        // Determine intended entry based on fill mode (same rules as primary entries).
                        let e_intended = match cfg.entry_fill_mode {
                            EntryFillMode::Boundary => {
                                if cfg.reverse_logic {
                                    c
                                } else {
                                    session_low
                                }
                            }
                            EntryFillMode::Close => c,
                            EntryFillMode::NextOpen => {
                                if i + 1 >= trade_rows.len() {
                                    continue;
                                }
                                trade_rows[i + 1].open
                            }
                        };

                        // Apply slippage to get actual fill.
                        let e_fill = match direction {
                            Direction::Long => e_intended + slippage_points_per_side,
                            Direction::Short => e_intended - slippage_points_per_side,
                        };

                        // Compute stop/target off fill price for internal consistency (and realism).
                        let sl = get_stop_loss(
                            e_fill,
                            is_long,
                            &cfg.sl_type,
                            cfg.sl_value,
                            range_size,
                            atr_val,
                            session_high,
                            session_low,
                        )?;
                        let tp = get_take_profit(
                            e_fill,
                            sl,
                            is_long,
                            &cfg.tp_type,
                            cfg.tp_value,
                            range_size,
                            atr_val,
                        )?;

                        // Compute qty off the fill price and stop distance.
                        let q = get_position_size(
                            e_fill,
                            sl,
                            &cfg.position_sizing_type,
                            cfg.fixed_contracts,
                            cfg.fixed_usd_risk,
                            cfg.tick_value,
                        )?;

                        if has_valid_risk_geometry(direction, e_fill, sl, tp, cfg.min_price_step) {
                            in_pos = true;
                            pos_direction = direction;
                            entry_price = e_fill;
                            stop_loss = sl;
                            take_profit = tp;
                            qty = q;

                            entry_time = match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => trade_rows[i + 1].datetime_ct,
                                _ => row.datetime_ct,
                            };
                            entry_bar_idx = Some(match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => i + 1,
                                _ => i,
                            });

                            second_chance_taken = true;
                            is_second_chance_trade = true;
                            continue;
                        }
                    }

                    if first_trade_direction == Some(Direction::Short)
                        && c > session_high
                        && consecutive_bull >= cfg.breakout_candles
                    {
                        let direction = if cfg.reverse_logic {
                            Direction::Short
                        } else {
                            Direction::Long
                        };
                        let is_long = direction == Direction::Long;

                        // Determine intended entry based on fill mode (same rules as primary entries).
                        let e_intended = match cfg.entry_fill_mode {
                            EntryFillMode::Boundary => {
                                if cfg.reverse_logic {
                                    c
                                } else {
                                    session_high
                                }
                            }
                            EntryFillMode::Close => c,
                            EntryFillMode::NextOpen => {
                                if i + 1 >= trade_rows.len() {
                                    continue;
                                }
                                trade_rows[i + 1].open
                            }
                        };

                        // Apply slippage to get actual fill.
                        let e_fill = match direction {
                            Direction::Long => e_intended + slippage_points_per_side,
                            Direction::Short => e_intended - slippage_points_per_side,
                        };

                        // Compute stop/target off fill price for internal consistency (and realism).
                        let sl = get_stop_loss(
                            e_fill,
                            is_long,
                            &cfg.sl_type,
                            cfg.sl_value,
                            range_size,
                            atr_val,
                            session_high,
                            session_low,
                        )?;
                        let tp = get_take_profit(
                            e_fill,
                            sl,
                            is_long,
                            &cfg.tp_type,
                            cfg.tp_value,
                            range_size,
                            atr_val,
                        )?;

                        // Compute qty off the fill price and stop distance.
                        let q = get_position_size(
                            e_fill,
                            sl,
                            &cfg.position_sizing_type,
                            cfg.fixed_contracts,
                            cfg.fixed_usd_risk,
                            cfg.tick_value,
                        )?;

                        if has_valid_risk_geometry(direction, e_fill, sl, tp, cfg.min_price_step) {
                            in_pos = true;
                            pos_direction = direction;
                            entry_price = e_fill;
                            stop_loss = sl;
                            take_profit = tp;
                            qty = q;

                            entry_time = match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => trade_rows[i + 1].datetime_ct,
                                _ => row.datetime_ct,
                            };
                            entry_bar_idx = Some(match cfg.entry_fill_mode {
                                EntryFillMode::NextOpen => i + 1,
                                _ => i,
                            });

                            second_chance_taken = true;
                            is_second_chance_trade = true;
                            continue;
                        }
                    }
                }
            }
        }
    }

    Ok(trades)
}

fn summarize(trades: &[Trade]) -> Stats {
    if trades.is_empty() {
        return Stats {
            num_trades: 0,
            winners: 0,
            losers: 0,
            breakeven: 0,
            winrate_pct: f64::NAN,
            avg_rr: f64::NAN,
            expectancy_rr: f64::NAN,
            gross_profit_usd: 0.0,
            gross_loss_usd: 0.0,
            net_profit_usd: 0.0,
            profit_factor: f64::NAN,
            avg_win_usd: f64::NAN,
            avg_loss_usd: f64::NAN,
            max_win_usd: f64::NAN,
            max_loss_usd: f64::NAN,
            max_drawdown_usd: f64::NAN,
            calmar_like: f64::NAN,
            score: f64::NAN,
        };
    }

    let pnl: Vec<f64> = trades.iter().map(|t| t.pnl_usd).collect();
    let winners = pnl.iter().filter(|&&x| x > 0.0).count();
    let losers = pnl.iter().filter(|&&x| x < 0.0).count();
    let breakeven = pnl.iter().filter(|&&x| x == 0.0).count();
    let n = trades.len();

    let gross_profit: f64 = pnl.iter().filter(|&&x| x > 0.0).sum();
    let gross_loss: f64 = -pnl.iter().filter(|&&x| x < 0.0).sum::<f64>();
    let net_profit: f64 = pnl.iter().sum();

    let mut equity = 0.0_f64;
    let mut running_max = 0.0_f64;
    let mut max_dd = 0.0_f64;
    for p in &pnl {
        equity += p;
        if equity > running_max {
            running_max = equity;
        }
        let dd = equity - running_max;
        if dd < max_dd {
            max_dd = dd;
        }
    }

    let rr_vals: Vec<f64> = trades
        .iter()
        .map(|t| t.rr)
        .filter(|x| x.is_finite())
        .collect();
    let avg_rr = mean(&rr_vals);
    let expectancy_rr = mean(&rr_vals);
    let winrate = if n > 0 {
        winners as f64 / n as f64 * 100.0
    } else {
        f64::NAN
    };

    let wins: Vec<f64> = pnl.iter().copied().filter(|x| *x > 0.0).collect();
    let losses: Vec<f64> = pnl.iter().copied().filter(|x| *x < 0.0).collect();
    let avg_win = mean(&wins);
    let avg_loss = mean(&losses);

    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else {
        f64::NAN
    };
    let calmar_like = if max_dd < 0.0 {
        net_profit / max_dd.abs()
    } else {
        f64::NAN
    };
    let score = if profit_factor.is_finite() && profit_factor > 0.0 && max_dd.is_finite() {
        let dd_penalty = 1.0 + (max_dd.abs() / 10000.0);
        (net_profit * profit_factor) / dd_penalty
    } else {
        f64::NAN
    };

    Stats {
        num_trades: n,
        winners,
        losers,
        breakeven,
        winrate_pct: winrate,
        avg_rr,
        expectancy_rr,
        gross_profit_usd: gross_profit,
        gross_loss_usd: gross_loss,
        net_profit_usd: net_profit,
        profit_factor,
        avg_win_usd: avg_win,
        avg_loss_usd: avg_loss,
        max_win_usd: pnl.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        max_loss_usd: pnl.iter().copied().fold(f64::INFINITY, f64::min),
        max_drawdown_usd: max_dd,
        calmar_like,
        score,
    }
}

fn print_summary(stats: &Stats) {
    println!("\n=== Opening Range Backtest Summary ===");
    println!("{:20}: {}", "num_trades", stats.num_trades);
    println!("{:20}: {}", "winners", stats.winners);
    println!("{:20}: {}", "losers", stats.losers);
    println!("{:20}: {}", "breakeven", stats.breakeven);
    print_float("winrate_pct", stats.winrate_pct);
    print_float("avg_rr", stats.avg_rr);
    print_float("expectancy_rr", stats.expectancy_rr);
    print_float("gross_profit_usd", stats.gross_profit_usd);
    print_float("gross_loss_usd", stats.gross_loss_usd);
    print_float("net_profit_usd", stats.net_profit_usd);
    print_float("profit_factor", stats.profit_factor);
    print_float("avg_win_usd", stats.avg_win_usd);
    print_float("avg_loss_usd", stats.avg_loss_usd);
    print_float("max_win_usd", stats.max_win_usd);
    print_float("max_loss_usd", stats.max_loss_usd);
    print_float("max_drawdown_usd", stats.max_drawdown_usd);
}

fn print_float(name: &str, value: f64) {
    if value.is_nan() {
        println!("{:20}: nan", name);
    } else {
        println!("{:20}: {:.4}", name, value);
    }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        f64::NAN
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

fn iter_reasonable_variations(base: &Config) -> Vec<Config> {
    let or_durations = [
        "5 Minutes",
        "15 Minutes",
        "30 Minutes",
        "45 Minutes",
        "60 Minutes",
    ];
    let breakout_candles = [1usize, 2, 3, 4];
    let reverse_logic = [false, true];
    let enable_second_chance = [false, true];
    let last_entry_times = ["", "10:30", "11:00", "12:00"];
    let sl_types = ["ATR Multiple", "Opposite Range", "Fixed Points", "Range %"];
    let tp_types = ["Risk Reward", "Fixed Points", "Range %"];

    let mut all = Vec::new();
    for od in or_durations {
        for bc in breakout_candles {
            for rev in reverse_logic {
                for sc in enable_second_chance {
                    for letime in last_entry_times {
                        for slt in sl_types {
                            let sl_vals: &[f64] = match slt {
                                "ATR Multiple" => &[1.0, 1.5, 2.0, 2.5],
                                "Opposite Range" => &[1.0],
                                "Fixed Points" => &[5.0, 10.0, 15.0, 20.0],
                                "Range %" => &[25.0, 50.0, 75.0, 100.0],
                                _ => &[],
                            };
                            for slv in sl_vals {
                                for tpt in tp_types {
                                    let tp_vals: &[f64] = match tpt {
                                        "Risk Reward" => &[0.5, 0.75, 1.0, 1.25, 1.5, 2.0],
                                        "Fixed Points" => &[5.0, 10.0, 15.0, 20.0],
                                        "Range %" => &[25.0, 50.0, 75.0, 100.0, 150.0],
                                        _ => &[],
                                    };
                                    for tpv in tp_vals {
                                        let mut cfg = base.clone();
                                        cfg.or_duration = od.to_string();
                                        cfg.breakout_candles = bc;
                                        cfg.reverse_logic = rev;
                                        cfg.enable_second_chance = sc;
                                        cfg.last_entry_time = letime.to_string();
                                        cfg.sl_type = slt.to_string();
                                        cfg.sl_value = *slv;
                                        cfg.tp_type = tpt.to_string();
                                        cfg.tp_value = *tpv;
                                        all.push(cfg);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    all
}

fn metric_for_sort(row: &SweepRow, key: SortKey) -> f64 {
    match key {
        SortKey::Score => row.score,
        SortKey::NetProfitUsd => row.net_profit_usd,
        SortKey::ProfitFactor => row.profit_factor,
        SortKey::CalmarLike => row.calmar_like,
    }
}

fn compare_desc_nan_last(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
    }
}

fn run_sweep(
    df: &[BarRow],
    base_cfg: &Config,
    top_n: usize,
    min_trades: usize,
    sort_key: SortKey,
) -> Result<()> {
    let mut rows: Vec<SweepRow> = Vec::new();

    // Extra realism guardrails for sweep ranking:
    // - PF can explode if there are extremely few losses; require a minimum number of losers.
    // - Reject configs with tiny average loss magnitude (often indicates a fill/assumption artifact).
    // - Use a more conservative score that penalizes deep DD but doesn't let tiny DD + huge PF dominate.
    const MIN_LOSERS: usize = 10;
    const MIN_AVG_LOSS_MAG_USD: f64 = 200.0; // must lose at least ~$200 on average when losing (net); adjust as desired
    const DD_SCALE: f64 = 10_000.0;

    for cfg in iter_reasonable_variations(base_cfg) {
        let trades = backtest(df, &cfg)?;
        let stats = summarize(&trades);
        if stats.num_trades < min_trades {
            continue;
        }

        // Recompute losers/avg loss directly from trades to avoid relying on summary fields.
        let mut losers = 0usize;
        let mut loss_sum = 0.0_f64;
        for t in &trades {
            if t.pnl_usd < 0.0 {
                losers += 1;
                loss_sum += t.pnl_usd; // negative
            }
        }
        let avg_loss = if losers > 0 {
            loss_sum / losers as f64
        } else {
            f64::NAN
        };
        let avg_loss_mag = if avg_loss.is_finite() {
            avg_loss.abs()
        } else {
            f64::NAN
        };

        if losers < MIN_LOSERS {
            continue;
        }
        if !avg_loss_mag.is_finite() || avg_loss_mag < MIN_AVG_LOSS_MAG_USD {
            continue;
        }

        // Build a more conservative score for sweep sorting/printing.
        // Keep the original stats.score around (it is still printed), but prefer this for ranking when --sweep-sort score.
        let dd = stats.max_drawdown_usd.abs();
        let dd_penalty = 1.0 + (dd / DD_SCALE);
        let conservative_score = (stats.net_profit_usd * stats.profit_factor) / dd_penalty;

        rows.push(SweepRow {
            score: if sort_key == SortKey::Score {
                conservative_score
            } else {
                stats.score
            },
            net_profit_usd: stats.net_profit_usd,
            profit_factor: stats.profit_factor,
            max_drawdown_usd: stats.max_drawdown_usd,
            calmar_like: stats.calmar_like,
            winrate_pct: stats.winrate_pct,
            avg_rr: stats.avg_rr,
            num_trades: stats.num_trades,
            or_duration: cfg.or_duration.clone(),
            breakout_candles: cfg.breakout_candles,
            reverse_logic: cfg.reverse_logic,
            enable_second_chance: cfg.enable_second_chance,
            last_entry_time: cfg.last_entry_time.clone(),
            sl_type: cfg.sl_type.clone(),
            sl_value: cfg.sl_value,
            tp_type: cfg.tp_type.clone(),
            tp_value: cfg.tp_value,
        });
    }

    if rows.is_empty() {
        println!("\nSweep produced no results (try lowering --sweep-min-trades).");
        return Ok(());
    }

    rows.sort_by(|a, b| {
        compare_desc_nan_last(metric_for_sort(a, sort_key), metric_for_sort(b, sort_key))
    });

    println!("\n=== Sweep Results (top configs) ===");
    println!(
        "score,net_profit_usd,profit_factor,max_drawdown_usd,calmar_like,num_trades,winrate_pct,avg_rr,or_duration,breakout_candles,reverse_logic,enable_second_chance,last_entry_time,sl_type,sl_value,tp_type,tp_value"
    );
    for row in rows.iter().take(top_n) {
        println!(
            "{},{:.6},{:.6},{:.6},{:.6},{},{:.4},{:.4},{},{},{},{},{},{},{:.4},{},{:.4}",
            row.score,
            row.net_profit_usd,
            row.profit_factor,
            row.max_drawdown_usd,
            row.calmar_like,
            row.num_trades,
            row.winrate_pct,
            row.avg_rr,
            row.or_duration,
            row.breakout_candles,
            row.reverse_logic,
            row.enable_second_chance,
            row.last_entry_time,
            row.sl_type,
            row.sl_value,
            row.tp_type,
            row.tp_value
        );
    }

    let best = &rows[0];
    println!("\n=== Best Config ===");
    println!("{:20}: {:.6}", "score", best.score);
    println!("{:20}: {:.6}", "net_profit_usd", best.net_profit_usd);
    println!("{:20}: {:.6}", "profit_factor", best.profit_factor);
    println!("{:20}: {:.6}", "max_drawdown_usd", best.max_drawdown_usd);
    println!("{:20}: {:.6}", "calmar_like", best.calmar_like);
    println!("{:20}: {}", "num_trades", best.num_trades);
    println!("{:20}: {:.4}", "winrate_pct", best.winrate_pct);
    println!("{:20}: {:.4}", "avg_rr", best.avg_rr);
    println!("{:20}: {}", "or_duration", best.or_duration);
    println!("{:20}: {}", "breakout_candles", best.breakout_candles);
    println!("{:20}: {}", "reverse_logic", best.reverse_logic);
    println!(
        "{:20}: {}",
        "enable_second_chance", best.enable_second_chance
    );
    println!("{:20}: {}", "last_entry_time", best.last_entry_time);
    println!("{:20}: {}", "sl_type", best.sl_type);
    println!("{:20}: {:.4}", "sl_value", best.sl_value);
    println!("{:20}: {}", "tp_type", best.tp_type);
    println!("{:20}: {:.4}", "tp_value", best.tp_value);

    Ok(())
}

fn save_trades(path: &str, trades: &[Trade]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)
        .with_context(|| format!("Failed to open output CSV: {path}"))?;
    wtr.write_record([
        "date",
        "direction",
        "entry_time",
        "exit_time",
        "entry_price",
        "exit_price",
        "stop_loss",
        "take_profit",
        "qty",
        "reason",
        "pnl_points",
        "pnl_usd",
        "risk_points",
        "rr",
        "is_second_chance",
    ])?;

    for t in trades {
        wtr.write_record([
            t.date.to_string(),
            t.direction.as_str().to_string(),
            t.entry_time.to_rfc3339(),
            t.exit_time.to_rfc3339(),
            format!("{}", t.entry_price),
            format!("{}", t.exit_price),
            format!("{}", t.stop_loss),
            format!("{}", t.take_profit),
            format!("{}", t.qty),
            t.reason.as_str().to_string(),
            format!("{}", t.pnl_points),
            format!("{}", t.pnl_usd),
            format!("{}", t.risk_points),
            format!("{}", t.rr),
            format!("{}", t.is_second_chance),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut cfg = Config {
        strategy: args.strategy,
        csv_path: args.csv_path.clone(),
        or_duration: args.or_duration,
        breakout_candles: args.breakout_candles,
        reverse_logic: args.reverse_logic,
        last_entry_time: args.last_entry_time,
        entry_fill_mode: args.entry_fill_mode,
        position_sizing_type: args.position_sizing_type,
        fixed_contracts: args.fixed_contracts,
        fixed_usd_risk: args.fixed_usd_risk,
        tick_value: args.tick_value,
        sl_type: args.sl_type,
        sl_value: args.sl_value,
        tp_type: args.tp_type,
        tp_value: args.tp_value,
        atr_length: args.atr_length,
        enable_second_chance: args.enable_second_chance,
        rth_start: "08:30".to_string(),
        rth_end: "15:00".to_string(),
        commission_round_trip_per_contract: args.commission_round_trip_per_contract,
        slippage_ticks_per_side: args.slippage_ticks_per_side,
        tick_size: args.tick_size,
        min_price_step: 0.25,
    };
    apply_strategy_preset(&mut cfg);

    let df = load_deduped_data(&cfg)?;
    if args.sweep {
        if strategy_is_orb(cfg.strategy) {
            run_sweep(
                &df,
                &cfg,
                args.sweep_top,
                args.sweep_min_trades,
                args.sweep_sort,
            )?;
        } else {
            println!("\nSweep is currently supported for ORB-family strategies only.");
        }
        return Ok(());
    }

    let trades = if strategy_is_orb(cfg.strategy) {
        backtest(&df, &cfg)?
    } else {
        let spec = strategy_reversal_spec(cfg.strategy)
            .ok_or_else(|| anyhow!("Missing reversal strategy spec"))?;
        backtest_reversal(&df, &cfg, &spec)?
    };
    let stats = summarize(&trades);
    print_summary(&stats);
    if !args.save_trades.is_empty() {
        save_trades(&args.save_trades, &trades)?;
        println!("\nSaved trades -> {}", args.save_trades);
    }
    Ok(())
}
