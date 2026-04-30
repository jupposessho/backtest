/// ICT 2022 Mentorship Strategy — Rust port
///
/// Multi-timeframe flow:
///   4h  → HTF bias + periodic levels (PDH/PDL/PWH/PWL/PMH/PML)
///   1h  → Session H/L — intraday developing values (ffill semantics, matches Python)
///   15m → MSS (Market Structure Shift) confirmation
///   5m  → Sweep detection + PD array (FVG / OB) + limit entry simulation
use crate::to_new_york_time;
use chrono::{Datelike, NaiveDate, Timelike};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;

// ── Config ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PdArrayFilter {
    Both,
    FvgOnly,
    ObOnly,
}

#[derive(Clone, Debug)]
pub struct IctConfig {
    pub sweep_tolerance: Decimal,    // 0.001 (0.1%)
    pub mss_swing_lookback: usize,   // bars each side for swing pivot
    pub mss_lookforward_bars: usize, // max 15m bars forward to search for MSS
    pub min_fvg_size: Decimal,       // min gap as fraction of close
    pub min_displacement: Decimal,   // min body pct on next candle for OB displacement
    pub ote_low_fib: Decimal,        // 0.618
    pub ote_high_fib: Decimal,       // 0.786
    pub risk_pct: Decimal,           // 0.01
    pub starting_capital: Decimal,
    pub min_rr: Decimal,
    pub sl_buffer: Decimal,          // 0.002
    pub fee_rate: Decimal,           // 0.0006 per side
    pub max_trades_per_day: u32,
    pub breakeven_at_1r: bool,
    pub entry_fill_bars: usize,      // 5m bars after MSS to wait for limit fill
    pub use_bias_filter: bool,
    pub use_kill_zone_filter: bool,
    pub use_mss_filter: bool,
    pub use_ote_filter: bool,
    pub pd_array_filter: PdArrayFilter,
    pub track_pdh_pdl: bool,
    pub track_pwh_pwl: bool,
    pub track_pmh_pml: bool,
    pub track_session_levels: bool,
    /// Print debug rejection counts to stderr
    pub debug: bool,
}

impl Default for IctConfig {
    fn default() -> Self {
        Self {
            sweep_tolerance: dec(0.001),
            mss_swing_lookback: 3,
            mss_lookforward_bars: 48,
            min_fvg_size: dec(0.0005),
            min_displacement: dec(0.001),
            ote_low_fib: dec(0.618),
            ote_high_fib: dec(0.786),
            risk_pct: dec(0.01),
            starting_capital: Decimal::from(10_000),
            min_rr: dec(3.0),
            sl_buffer: dec(0.002),
            fee_rate: dec(0.0006),
            max_trades_per_day: 2,
            breakeven_at_1r: true,
            entry_fill_bars: 96,
            use_bias_filter: true,
            use_kill_zone_filter: true,
            use_mss_filter: true,
            use_ote_filter: true,
            pd_array_filter: PdArrayFilter::Both,
            track_pdh_pdl: true,
            track_pwh_pwl: true,
            track_pmh_pml: false,
            track_session_levels: true,
            debug: false,
        }
    }
}

fn dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap()
}

// ── Domain types ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HtfBias {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TradeDir {
    Bullish,
    Bearish,
}

#[derive(Clone, Debug)]
pub enum ExitReason {
    Sl,
    Tp1Only,
    Tp2,
    Timeout,
}

#[derive(Clone, Debug)]
pub struct IctTrade {
    pub direction: TradeDir,
    pub level_type: &'static str,
    pub kill_zone: &'static str,
    pub pd_array_type: &'static str,
    pub entry_ts: i64,
    pub exit_ts: i64,
    pub entry: Decimal,
    pub sl: Decimal,
    pub tp1: Decimal,
    pub tp2: Decimal,
    pub exit_price: Decimal,
    pub exit_reason: ExitReason,
    pub r_multiple: Decimal,
    pub pnl_usd: Decimal,
    pub tp1_hit: bool,
    pub be_triggered: bool,
    pub equity: Decimal,
}

// ── Internal structs ──────────────────────────────────────────────────────────

/// Periodic levels per trading date (4h-derived: PDH/PDL/PWH/PWL/PMH/PML only)
#[derive(Default, Clone)]
struct PeriodicLevels {
    pdh: Option<Decimal>,
    pdl: Option<Decimal>,
    pwh: Option<Decimal>,
    pwl: Option<Decimal>,
    pmh: Option<Decimal>,
    pml: Option<Decimal>,
}

/// Developing session H/L at a specific 1h bar timestamp (Python ffill equivalent).
/// Each field reflects the max/min seen in that session up to (and including) this bar.
#[derive(Default, Clone, Debug)]
struct SessionSlice {
    asia_h: Option<Decimal>,
    asia_l: Option<Decimal>,
    london_h: Option<Decimal>,
    london_l: Option<Decimal>,
    nyam_h: Option<Decimal>,
    nyam_l: Option<Decimal>,
}

#[derive(Clone)]
struct SweepEvent {
    ts_5m: i64,
    idx_5m: usize,
    direction: TradeDir,
    level_type: &'static str,
    level_price: Decimal,
    sweep_extreme: Decimal,
    kill_zone: &'static str,
    bias: HtfBias,
}

#[derive(Clone, Copy)]
struct PdArray {
    kind: &'static str,
    bottom: Decimal,
    top: Decimal,
    #[allow(dead_code)]
    midpoint_idx: usize,
}

// ── Kill zone helpers ──────────────────────────────────────────────────────────

fn kill_zone(h: u32, m: u32) -> &'static str {
    let mins = h * 60 + m;
    if mins >= 120 && mins < 300 { return "london_open"; }
    if mins >= 420 && mins < 600 { return "ny_am"; }
    if mins >= 600 && mins < 720 { return "london_close"; }
    ""
}

fn is_kill_zone(h: u32, m: u32) -> bool {
    !kill_zone(h, m).is_empty()
}

// ── NY trading date ─────────────────────────────────────────────────────────

/// Asia session (≥17:00 NY) belongs to the next calendar date.
fn trading_date(ts: i64) -> NaiveDate {
    let dt = to_new_york_time(ts);
    let date = dt.date_naive();
    if dt.hour() >= 17 {
        date.succ_opt().unwrap_or(date)
    } else {
        date
    }
}

// ── Phase 1: 4h bias + periodic levels ────────────────────────────────────────

fn compute_4h_data(
    candles_4h: &[crate::model::candle_stick::CandleStick],
    cfg: &IctConfig,
) -> (Vec<HtfBias>, HashMap<NaiveDate, PeriodicLevels>) {
    let n = candles_4h.len();
    let mut biases = vec![HtfBias::Neutral; n];
    for i in 3..n {
        biases[i] = if candles_4h[i].close.0 > candles_4h[i - 3].close.0 {
            HtfBias::Bullish
        } else if candles_4h[i].close.0 < candles_4h[i - 3].close.0 {
            HtfBias::Bearish
        } else {
            HtfBias::Neutral
        };
    }

    let mut levels_map: HashMap<NaiveDate, PeriodicLevels> = HashMap::new();

    let mut day_h = Decimal::ZERO;
    let mut day_l = Decimal::MAX;
    let mut week_h = Decimal::ZERO;
    let mut week_l = Decimal::MAX;
    let mut month_h = Decimal::ZERO;
    let mut month_l = Decimal::MAX;

    let mut prev_day_h: Option<Decimal> = None;
    let mut prev_day_l: Option<Decimal> = None;
    let mut prev_week_h: Option<Decimal> = None;
    let mut prev_week_l: Option<Decimal> = None;
    let mut prev_month_h: Option<Decimal> = None;
    let mut prev_month_l: Option<Decimal> = None;

    let mut cur_day: Option<NaiveDate> = None;
    let mut cur_week: Option<u32> = None;
    let mut cur_month: Option<u32> = None;

    for c in candles_4h.iter() {
        let dt = to_new_york_time(c.open_time);
        let date = dt.date_naive();
        let week = dt.iso_week().week();
        let month = dt.month();

        if cur_day != Some(date) {
            if cur_day.is_some() && cfg.track_pdh_pdl {
                prev_day_h = Some(day_h);
                prev_day_l = Some(day_l);
            }
            day_h = c.high.0;
            day_l = c.low.0;
            cur_day = Some(date);
        }

        if cur_week != Some(week) {
            if cur_week.is_some() && cfg.track_pwh_pwl {
                prev_week_h = Some(week_h);
                prev_week_l = Some(week_l);
            }
            week_h = c.high.0;
            week_l = c.low.0;
            cur_week = Some(week);
        }

        if cur_month != Some(month) {
            if cur_month.is_some() && cfg.track_pmh_pml {
                prev_month_h = Some(month_h);
                prev_month_l = Some(month_l);
            }
            month_h = c.high.0;
            month_l = c.low.0;
            cur_month = Some(month);
        }

        if c.high.0 > day_h { day_h = c.high.0; }
        if c.low.0 < day_l { day_l = c.low.0; }
        if c.high.0 > week_h { week_h = c.high.0; }
        if c.low.0 < week_l { week_l = c.low.0; }
        if c.high.0 > month_h { month_h = c.high.0; }
        if c.low.0 < month_l { month_l = c.low.0; }

        let entry = levels_map.entry(date).or_default();
        if cfg.track_pdh_pdl {
            if prev_day_h.is_some() { entry.pdh = prev_day_h; }
            if prev_day_l.is_some() { entry.pdl = prev_day_l; }
        }
        if cfg.track_pwh_pwl {
            if prev_week_h.is_some() { entry.pwh = prev_week_h; }
            if prev_week_l.is_some() { entry.pwl = prev_week_l; }
        }
        if cfg.track_pmh_pml {
            if prev_month_h.is_some() { entry.pmh = prev_month_h; }
            if prev_month_l.is_some() { entry.pml = prev_month_l; }
        }
    }

    (biases, levels_map)
}

fn bias_at(
    ts: i64,
    candles_4h: &[crate::model::candle_stick::CandleStick],
    biases_4h: &[HtfBias],
) -> HtfBias {
    let idx = candles_4h.partition_point(|c| c.open_time <= ts);
    if idx == 0 { return HtfBias::Neutral; }
    biases_4h[idx - 1]
}

// ── Phase 2: Intraday session H/L (matches Python compute_session_levels_intraday) ──
//
// Produces a sorted Vec<(i64, SessionSlice)> where each entry records the
// developing session H/L at each 1h session bar.  Non-current session fields
// carry forward the last known value (ffill semantics, exactly like Python).
//
// At any 5m bar at time T: look up the last entry with ts ≤ T to get the
// equivalent of Python's df_tagged.loc[T, 'LONDON_H'] etc.

fn compute_intraday_session_levels(
    candles_1h: &[crate::model::candle_stick::CandleStick],
) -> Vec<(i64, SessionSlice)> {
    // Developing H/L keyed by (trading_date_for_asia / calendar_date_for_lon_nyam, session_id)
    let mut trackers_asia:  HashMap<NaiveDate, (Decimal, Decimal)> = HashMap::new();
    let mut trackers_lon:   HashMap<NaiveDate, (Decimal, Decimal)> = HashMap::new();
    let mut trackers_nyam:  HashMap<NaiveDate, (Decimal, Decimal)> = HashMap::new();

    let mut last = SessionSlice::default();
    let mut entries: Vec<(i64, SessionSlice)> = Vec::new();

    for c in candles_1h.iter() {
        let dt = to_new_york_time(c.open_time);
        let h = dt.hour();
        let date = dt.date_naive();        // NY calendar date
        let td   = trading_date(c.open_time); // rolls at 17:00 for Asia

        let high = c.high.0;
        let low  = c.low.0;

        if h >= 20 || h < 3 {
            // Asia session — keyed by trading_date (next calendar day)
            let e = trackers_asia.entry(td).or_insert((high, low));
            if high > e.0 { e.0 = high; }
            if low  < e.1 { e.1 = low; }
            let mut slice = last.clone();
            slice.asia_h = Some(e.0);
            slice.asia_l = Some(e.1);
            last = slice.clone();
            entries.push((c.open_time, slice));
        } else if h >= 3 && h < 8 {
            // London session — keyed by calendar date
            let e = trackers_lon.entry(date).or_insert((high, low));
            if high > e.0 { e.0 = high; }
            if low  < e.1 { e.1 = low; }
            let mut slice = last.clone();
            slice.london_h = Some(e.0);
            slice.london_l = Some(e.1);
            last = slice.clone();
            entries.push((c.open_time, slice));
        } else if h >= 8 && h < 12 {
            // NYAM session — keyed by calendar date
            let e = trackers_nyam.entry(date).or_insert((high, low));
            if high > e.0 { e.0 = high; }
            if low  < e.1 { e.1 = low; }
            let mut slice = last.clone();
            slice.nyam_h = Some(e.0);
            slice.nyam_l = Some(e.1);
            last = slice.clone();
            entries.push((c.open_time, slice));
        }
        // Non-session hours (12:00–20:00 NY): no entry — last values propagate via ffill
    }

    // Entries are already in ascending timestamp order (we iterate candles in order)
    entries
}

/// Return developing session levels at time `ts` (last entry with ts ≤ ts — ffill).
fn session_levels_at<'a>(
    entries: &'a [(i64, SessionSlice)],
    ts: i64,
) -> Option<&'a SessionSlice> {
    let idx = entries.partition_point(|(t, _)| *t <= ts);
    if idx == 0 { None } else { Some(&entries[idx - 1].1) }
}

// ── Phase 3: Sweep detection ──────────────────────────────────────────────────

/// Returns (name, price, is_high_level).
/// is_high_level=true → only BSL (bearish) sweep; false → only SSL (bullish) sweep.
/// Matches Python's level_specs: high levels are 'bearish', low levels are 'bullish'.
fn active_levels<'a>(
    periodic: &'a PeriodicLevels,
    session: Option<&'a SessionSlice>,
    cfg: &IctConfig,
) -> Vec<(&'static str, Decimal, bool)> {
    let mut out: Vec<(&'static str, Decimal, bool)> = Vec::new();
    macro_rules! push_h { ($field:expr, $name:literal) => { if let Some(p) = $field { out.push(($name, p, true));  } }; }
    macro_rules! push_l { ($field:expr, $name:literal) => { if let Some(p) = $field { out.push(($name, p, false)); } }; }
    if cfg.track_pdh_pdl {
        push_h!(periodic.pdh, "PDH");
        push_l!(periodic.pdl, "PDL");
    }
    if cfg.track_pwh_pwl {
        push_h!(periodic.pwh, "PWH");
        push_l!(periodic.pwl, "PWL");
    }
    if cfg.track_pmh_pml {
        push_h!(periodic.pmh, "PMH");
        push_l!(periodic.pml, "PML");
    }
    if cfg.track_session_levels {
        if let Some(s) = session {
            push_h!(s.asia_h,   "ASIA_H");
            push_l!(s.asia_l,   "ASIA_L");
            push_h!(s.london_h, "LONDON_H");
            push_l!(s.london_l, "LONDON_L");
            push_h!(s.nyam_h,   "NYAM_H");
            push_l!(s.nyam_l,   "NYAM_L");
        }
    }
    out
}

fn detect_sweeps(
    candles_5m: &[crate::model::candle_stick::CandleStick],
    candles_4h: &[crate::model::candle_stick::CandleStick],
    biases_4h: &[HtfBias],
    periodic_map: &HashMap<NaiveDate, PeriodicLevels>,
    intraday_session: &[(i64, SessionSlice)],
    cfg: &IctConfig,
) -> Vec<SweepEvent> {
    let mut sweeps: Vec<SweepEvent> = Vec::new();
    let mut swept: HashMap<(NaiveDate, &'static str), bool> = HashMap::new();
    let mut sweep_counts: HashMap<&'static str, u32> = HashMap::new();

    for (idx, c) in candles_5m.iter().enumerate() {
        let dt = to_new_york_time(c.open_time);
        let h = dt.hour();
        let m = dt.minute();
        let td = trading_date(c.open_time);

        if cfg.use_kill_zone_filter && !is_kill_zone(h, m) {
            continue;
        }

        let kz = kill_zone(h, m);
        let periodic = match periodic_map.get(&td) {
            Some(l) => l,
            None => continue,
        };

        // Intraday session levels at the time of this 5m bar (ffill semantics)
        let session = session_levels_at(intraday_session, c.open_time);

        let bias = bias_at(c.open_time, candles_4h, biases_4h);
        let tol = cfg.sweep_tolerance;

        for (name, level, is_high_level) in active_levels(periodic, session, cfg) {
            let key = (td, name);
            if swept.contains_key(&key) { continue; }

            // Python: high levels → BSL (bearish) only; low levels → SSL (bullish) only
            if is_high_level {
                // BSL sweep (bearish): wick above level, close below
                let is_bsl = c.high.0 > level * (Decimal::ONE + tol) && c.close.0 < level;
                if is_bsl {
                    swept.insert(key, true);
                    *sweep_counts.entry(name).or_insert(0) += 1;
                    sweeps.push(SweepEvent {
                        ts_5m: c.open_time,
                        idx_5m: idx,
                        direction: TradeDir::Bearish,
                        level_type: name,
                        level_price: level,
                        sweep_extreme: c.high.0,
                        kill_zone: kz,
                        bias,
                    });
                }
            } else {
                // SSL sweep (bullish): wick below level, close above
                let is_ssl = c.low.0 < level * (Decimal::ONE - tol) && c.close.0 > level;
                if is_ssl {
                    swept.insert(key, true);
                    *sweep_counts.entry(name).or_insert(0) += 1;
                    sweeps.push(SweepEvent {
                        ts_5m: c.open_time,
                        idx_5m: idx,
                        direction: TradeDir::Bullish,
                        level_type: name,
                        level_price: level,
                        sweep_extreme: c.low.0,
                        kill_zone: kz,
                        bias,
                    });
                }
            }
        }
    }

    if cfg.debug {
        let mut counts: Vec<_> = sweep_counts.iter().collect();
        counts.sort_by_key(|(k, _)| k.to_string());
        eprint!("[ICT Sweeps by type]");
        for (name, cnt) in &counts {
            eprint!(" {}={}", name, cnt);
        }
        eprintln!(" TOTAL={}", sweeps.len());
    }

    sweeps
}

// ── Phase 4a: MSS detection on 15m ─────────────────────────────────────────────
//
// Matches Python detect_mss():
//   • pre_sweep = df_15m.loc[:sweep_ts]  (bars with open_time ≤ sweep_ts)
//   • scan      = df_15m.loc[sweep_ts:].iloc[1:n_bars+1]  (skip the first bar at sweep_ts)
//   • Find most recent swing pivot within pre_sweep
//   • Scan forward for close breaking the pivot

fn detect_mss(
    candles_15m: &[crate::model::candle_stick::CandleStick],
    sweep_ts: i64,
    direction: TradeDir,
    cfg: &IctConfig,
) -> Option<(Decimal, i64)> {
    let lb = cfg.mss_swing_lookback;
    let n_bars = cfg.mss_lookforward_bars;

    // Index of the first 15m bar with open_time >= sweep_ts
    // (same as Python's df_15m.loc[sweep_ts:] start index)
    let sweep_15m_idx = candles_15m.partition_point(|c| c.open_time < sweep_ts);
    if sweep_15m_idx < lb * 2 + 1 {
        return None;
    }

    // pre_sweep: bars [0, sweep_15m_idx)  — open_time < sweep_ts
    // Python: pre_sweep = df_15m.loc[:sweep_ts]  includes the bar AT sweep_ts if aligned,
    // but in practice the sweep bar is a 5m bar and 15m bars align at 3× their interval,
    // so the pre-sweep window is [0, sweep_15m_idx).

    let pivot: Decimal;

    match direction {
        TradeDir::Bullish => {
            // Last swing HIGH: high[i] == max(high[i-lb..=i+lb]) within pre_sweep window
            let mut found = None;
            if sweep_15m_idx > lb {
                let end = sweep_15m_idx - 1;
                let search_end = if end >= lb { end - lb } else { 0 };
                if search_end >= lb {
                    'outer: for i in (lb..=search_end).rev() {
                        let h = candles_15m[i].high.0;
                        let win_start = i - lb;
                        let win_end = (i + lb).min(sweep_15m_idx - 1);
                        let mut is_max = true;
                        for j in win_start..=win_end {
                            if candles_15m[j].high.0 > h {
                                is_max = false;
                                break;
                            }
                        }
                        if is_max {
                            found = Some(h);
                            break 'outer;
                        }
                    }
                }
            }
            // Fallback: highest high in pre_sweep window (matches Python's pre_sweep['high'].max())
            let fallback = candles_15m[..sweep_15m_idx]
                .iter()
                .map(|c| c.high.0)
                .max()
                .unwrap_or(Decimal::ZERO);
            pivot = found.unwrap_or(fallback);
            if pivot == Decimal::ZERO { return None; }
        }
        TradeDir::Bearish => {
            let mut found = None;
            if sweep_15m_idx > lb {
                let end = sweep_15m_idx - 1;
                let search_end = if end >= lb { end - lb } else { 0 };
                if search_end >= lb {
                    'outer: for i in (lb..=search_end).rev() {
                        let l = candles_15m[i].low.0;
                        let win_start = i - lb;
                        let win_end = (i + lb).min(sweep_15m_idx - 1);
                        let mut is_min = true;
                        for j in win_start..=win_end {
                            if candles_15m[j].low.0 < l {
                                is_min = false;
                                break;
                            }
                        }
                        if is_min {
                            found = Some(l);
                            break 'outer;
                        }
                    }
                }
            }
            let fallback = candles_15m[..sweep_15m_idx]
                .iter()
                .map(|c| c.low.0)
                .min()
                .unwrap_or(Decimal::MAX);
            pivot = found.unwrap_or(fallback);
            if pivot == Decimal::MAX { return None; }
        }
    }

    // Python: scan = df_15m.loc[sweep_ts:].iloc[1:n_bars+1]
    // df_15m.loc[sweep_ts:] → bars at indices [sweep_15m_idx, ...)
    // .iloc[1:n_bars+1]     → skip first (sweep_15m_idx), scan next n_bars bars
    let scan_start = sweep_15m_idx + 1;
    let end_idx = (scan_start + n_bars).min(candles_15m.len());

    for i in scan_start..end_idx {
        let confirms = match direction {
            TradeDir::Bullish => candles_15m[i].close.0 > pivot,
            TradeDir::Bearish => candles_15m[i].close.0 < pivot,
        };
        if confirms {
            return Some((pivot, candles_15m[i].open_time));
        }
    }

    None
}

// ── Phase 4b: PD array detection on 5m ────────────────────────────────────────
//
// Matches Python detect_fvg() and detect_ob() exactly.
// OB range is body-only: [min(open,close), max(open,close)]
// OB: last pair (i, i+1) where candle i is opposing and next has body_pct >= threshold

fn find_pd_arrays(
    candles_5m: &[crate::model::candle_stick::CandleStick],
    sweep_idx: usize,
    mss_ts: i64,
    direction: TradeDir,
    cfg: &IctConfig,
) -> Vec<PdArray> {
    // Python: window = df_5m.loc[sweep_ts:mss_ts]  (inclusive both ends)
    let mss_idx = candles_5m.partition_point(|c| c.open_time <= mss_ts);
    let from_idx = sweep_idx;
    let to_idx = mss_idx; // exclusive: bars [from_idx, to_idx)

    if to_idx <= from_idx + 2 {
        return vec![];
    }

    let mut arrays: Vec<PdArray> = Vec::new();

    // FVG: 3-bar imbalance
    if cfg.pd_array_filter != PdArrayFilter::ObOnly {
        for i in (from_idx + 1)..to_idx.saturating_sub(1) {
            if i + 1 >= candles_5m.len() { break; }
            let prev = &candles_5m[i - 1];
            let curr = &candles_5m[i];
            let next = &candles_5m[i + 1];
            match direction {
                TradeDir::Bullish => {
                    if prev.high.0 < next.low.0 {
                        let gap = next.low.0 - prev.high.0;
                        if curr.close.0 > Decimal::ZERO && gap / curr.close.0 >= cfg.min_fvg_size {
                            arrays.push(PdArray {
                                kind: "FVG",
                                bottom: prev.high.0,
                                top: next.low.0,
                                midpoint_idx: i,
                            });
                        }
                    }
                }
                TradeDir::Bearish => {
                    if prev.low.0 > next.high.0 {
                        let gap = prev.low.0 - next.high.0;
                        if curr.close.0 > Decimal::ZERO && gap / curr.close.0 >= cfg.min_fvg_size {
                            arrays.push(PdArray {
                                kind: "FVG",
                                bottom: next.high.0,
                                top: prev.low.0,
                                midpoint_idx: i,
                            });
                        }
                    }
                }
            }
        }
    }

    // OB: last opposing candle where next candle has body_pct >= min_displacement
    // Matches Python detect_ob(): returns obs[-1:] — the most recent qualifying pair
    if cfg.pd_array_filter != PdArrayFilter::FvgOnly {
        let mut last_ob: Option<PdArray> = None;
        for i in from_idx..to_idx.saturating_sub(1) {
            if i + 1 >= candles_5m.len() { break; }
            let c = &candles_5m[i];
            let next = &candles_5m[i + 1];
            let is_opposing = match direction {
                TradeDir::Bullish => c.close.0 < c.open.0,  // bearish candle
                TradeDir::Bearish => c.close.0 > c.open.0,  // bullish candle
            };
            if !is_opposing { continue; }
            if next.open.0 == Decimal::ZERO { continue; }
            let body_pct = match direction {
                TradeDir::Bullish => (next.close.0 - next.open.0) / next.open.0,
                TradeDir::Bearish => (next.open.0 - next.close.0) / next.open.0,
            };
            if body_pct >= cfg.min_displacement {
                // OB range is body only: [min(open,close), max(open,close)]
                last_ob = Some(PdArray {
                    kind: "OB",
                    bottom: c.open.0.min(c.close.0),
                    top: c.open.0.max(c.close.0),
                    midpoint_idx: i,
                });
            }
        }
        if let Some(ob) = last_ob {
            arrays.push(ob);
        }
    }

    arrays
}

// ── Phase 4c: Best array selection ────────────────────────────────────────────

fn find_best_array_ote(
    arrays: &[PdArray],
    sweep_extreme: Decimal,
    mss_pivot: Decimal,
    direction: TradeDir,
    cfg: &IctConfig,
) -> Option<PdArray> {
    let range = (mss_pivot - sweep_extreme).abs();
    if range == Decimal::ZERO { return None; }

    let (ote_low, ote_high) = match direction {
        TradeDir::Bullish => {
            let lo = mss_pivot - range * cfg.ote_high_fib;
            let hi = mss_pivot - range * cfg.ote_low_fib;
            (lo, hi)
        }
        TradeDir::Bearish => {
            let lo = mss_pivot + range * cfg.ote_low_fib;
            let hi = mss_pivot + range * cfg.ote_high_fib;
            (lo, hi)
        }
    };

    let target = match direction {
        TradeDir::Bullish => mss_pivot - range * dec(0.705),
        TradeDir::Bearish => mss_pivot + range * dec(0.705),
    };

    let overlaps = |arr: &PdArray| -> bool {
        arr.bottom < ote_high && arr.top > ote_low
    };

    let score = |arr: &PdArray| -> Decimal {
        let mid = (arr.bottom + arr.top) / Decimal::from(2);
        (mid - target).abs()
    };

    // FVG preferred over OB
    let best_fvg = arrays
        .iter()
        .filter(|a| a.kind == "FVG" && overlaps(a))
        .min_by(|a, b| score(a).cmp(&score(b)));

    if let Some(a) = best_fvg {
        return Some(*a);
    }

    arrays
        .iter()
        .filter(|a| a.kind == "OB" && overlaps(a))
        .min_by(|a, b| score(a).cmp(&score(b)))
        .copied()
}

// ── Phase 4d: TP targets ──────────────────────────────────────────────────────
//
// Matches Python find_tp_targets():
//   • Looks for opposite levels above/below entry that meet min_rr.
//   • tp2 = tp1 when only one level available.
//   • Returns None if no level meets min_rr (caller uses fallback).
//   • Uses session levels from the SWEEP bar (intraday developing values).

fn find_tp_levels(
    entry: Decimal,
    direction: TradeDir,
    periodic: &PeriodicLevels,
    session: Option<&SessionSlice>,
    sl: Decimal,
    min_rr: Decimal,
) -> Option<(Decimal, Decimal)> {
    let risk = (entry - sl).abs();
    if risk == Decimal::ZERO { return None; }

    let mut candidates: Vec<Decimal> = Vec::new();
    let push_level = |v: &mut Vec<Decimal>, l: Option<Decimal>| {
        if let Some(p) = l { v.push(p); }
    };

    // Matches Python level_cols_long/short: ['PDH','PWH','ASIA_H','LONDON_H','NYAM_H']
    match direction {
        TradeDir::Bullish => {
            push_level(&mut candidates, periodic.pdh);
            push_level(&mut candidates, periodic.pwh);
            if let Some(s) = session {
                push_level(&mut candidates, s.asia_h);
                push_level(&mut candidates, s.london_h);
                push_level(&mut candidates, s.nyam_h);
            }
        }
        TradeDir::Bearish => {
            push_level(&mut candidates, periodic.pdl);
            push_level(&mut candidates, periodic.pwl);
            if let Some(s) = session {
                push_level(&mut candidates, s.asia_l);
                push_level(&mut candidates, s.london_l);
                push_level(&mut candidates, s.nyam_l);
            }
        }
    }

    let mut valid: Vec<Decimal> = candidates
        .into_iter()
        .filter(|&p| match direction {
            TradeDir::Bullish => p > entry && (p - entry) / risk >= min_rr,
            TradeDir::Bearish => p < entry && (entry - p) / risk >= min_rr,
        })
        .collect();

    if valid.is_empty() { return None; }

    match direction {
        TradeDir::Bullish => valid.sort_by(|a, b| a.cmp(b)),
        TradeDir::Bearish => valid.sort_by(|a, b| b.cmp(a)),
    }

    let tp1 = valid[0];
    // Python: tp2 = targets[1] if len(targets) > 1 else tp1
    let tp2 = if valid.len() > 1 { valid[1] } else { tp1 };

    Some((tp1, tp2))
}

// ── Phase 5: Trade simulation ─────────────────────────────────────────────────
//
// Matches Python simulate_trade_ict() exactly:
//   • TP1/TP2 trigger booleans are computed at the TOP of each bar iteration
//     (before any modification of tp1_hit).  This means TP2 CANNOT trigger on
//     the same bar as TP1 (unlike the previous Rust version).
//   • SL checked first; TP1 then TP2 on separate bars.
//   • BE move happens within TP1 block on the same bar.

fn simulate_trade(
    fill_start_idx: usize,
    direction: TradeDir,
    entry: Decimal,
    sl_raw: Decimal,
    tp1: Decimal,
    tp2: Decimal,
    candles_5m: &[crate::model::candle_stick::CandleStick],
    capital: Decimal,
    cfg: &IctConfig,
) -> Option<(Decimal, Decimal, i64, ExitReason, bool, bool)> {
    let risk = (entry - sl_raw).abs();
    if risk == Decimal::ZERO { return None; }

    let pos_size = capital * cfg.risk_pct / risk;
    let half_size = pos_size / Decimal::from(2);
    let fee_rate = cfg.fee_rate;

    // Entry fee on full position
    let mut fees_paid = pos_size * entry * fee_rate;

    let n = candles_5m.len();
    let fill_end = (fill_start_idx + cfg.entry_fill_bars).min(n);

    // Stage 1: find limit fill (Python: scan_window = future.iloc[:fill_bars])
    let mut fill_idx = None;
    for i in fill_start_idx..fill_end {
        let c = &candles_5m[i];
        let fills = match direction {
            TradeDir::Bullish => c.low.0 <= entry && c.low.0 > sl_raw,
            TradeDir::Bearish => c.high.0 >= entry && c.high.0 < sl_raw,
        };
        if fills {
            fill_idx = Some(i);
            break;
        }
        // SL blown before fill
        let blown = match direction {
            TradeDir::Bullish => c.low.0 <= sl_raw,
            TradeDir::Bearish => c.high.0 >= sl_raw,
        };
        if blown { break; }
    }

    let start_sim = fill_idx?;

    // Stage 2: bar-by-bar simulation
    // Python computes sl_triggered, tp1_triggered, tp2_triggered at the TOP of each
    // iteration — before any modification of tp1_hit.  This means tp2_triggered uses
    // the OLD tp1_hit and therefore CANNOT be True on the same bar as TP1.
    let mut sl_current = sl_raw;
    let mut tp1_hit = false;
    let mut partial_pnl = Decimal::ZERO;
    let mut be_triggered = false;

    let sim_end = n;

    for i in start_sim..sim_end {
        let c = &candles_5m[i];
        let low  = c.low.0;
        let high = c.high.0;

        // All triggers computed with OLD tp1_hit (matches Python's top-of-loop computation)
        let sl_triggered  = match direction {
            TradeDir::Bullish => low  <= sl_current,
            TradeDir::Bearish => high >= sl_current,
        };
        let tp1_triggered = !tp1_hit && match direction {
            TradeDir::Bullish => high >= tp1,
            TradeDir::Bearish => low  <= tp1,
        };
        let tp2_triggered = tp1_hit && match direction {
            TradeDir::Bullish => high >= tp2,
            TradeDir::Bearish => low  <= tp2,
        };

        // SL has highest priority
        if sl_triggered {
            let close_size = if tp1_hit { half_size } else { pos_size };
            fees_paid += close_size * sl_current * fee_rate;
            let sl_pnl = match direction {
                TradeDir::Bullish => close_size * (sl_current - entry),
                TradeDir::Bearish => close_size * (entry - sl_current),
            };
            let full_pnl = partial_pnl + sl_pnl - fees_paid;
            let r = full_pnl / (capital * cfg.risk_pct);
            let reason = if tp1_hit { ExitReason::Tp1Only } else { ExitReason::Sl };
            return Some((full_pnl, r, c.open_time, reason, tp1_hit, be_triggered));
        }

        // TP1 — modifies tp1_hit for SUBSEQUENT bars
        if tp1_triggered {
            tp1_hit = true;
            fees_paid += half_size * tp1 * fee_rate;
            partial_pnl = match direction {
                TradeDir::Bullish => half_size * (tp1 - entry),
                TradeDir::Bearish => half_size * (entry - tp1),
            };
            if cfg.breakeven_at_1r {
                sl_current = entry;
                be_triggered = true;
            }
        }

        // TP2 — uses tp2_triggered computed with OLD tp1_hit: cannot fire same bar as TP1
        if tp2_triggered {
            fees_paid += half_size * tp2 * fee_rate;
            let tp2_pnl = match direction {
                TradeDir::Bullish => half_size * (tp2 - entry),
                TradeDir::Bearish => half_size * (entry - tp2),
            };
            let full_pnl = partial_pnl + tp2_pnl - fees_paid;
            let r = full_pnl / (capital * cfg.risk_pct);
            return Some((full_pnl, r, c.open_time, ExitReason::Tp2, true, be_triggered));
        }
    }

    // Timeout: close at last bar's close
    let last = &candles_5m[sim_end - 1];
    let ep = last.close.0;
    let close_size = if tp1_hit { half_size } else { pos_size };
    fees_paid += close_size * ep * fee_rate;
    let remaining_pnl = match direction {
        TradeDir::Bullish => close_size * (ep - entry),
        TradeDir::Bearish => close_size * (entry - ep),
    };
    let full_pnl = partial_pnl + remaining_pnl - fees_paid;
    let r = full_pnl / (capital * cfg.risk_pct);
    Some((full_pnl, r, last.open_time, ExitReason::Timeout, tp1_hit, be_triggered))
}

// ── Main entry point ───────────────────────────────────────────────────────────

pub struct IctStrategy {
    pub candles_5m:  Vec<crate::model::candle_stick::CandleStick>,
    pub candles_15m: Vec<crate::model::candle_stick::CandleStick>,
    pub candles_1h:  Vec<crate::model::candle_stick::CandleStick>,
    pub candles_4h:  Vec<crate::model::candle_stick::CandleStick>,
    pub config: IctConfig,
}

impl IctStrategy {
    pub fn run(&self) -> Vec<IctTrade> {
        let cfg = &self.config;

        let (biases_4h, periodic_map) = compute_4h_data(&self.candles_4h, cfg);
        let intraday_session = compute_intraday_session_levels(&self.candles_1h);

        let sweeps = detect_sweeps(
            &self.candles_5m,
            &self.candles_4h,
            &biases_4h,
            &periodic_map,
            &intraday_session,
            cfg,
        );

        let mut trades: Vec<IctTrade> = Vec::new();
        let mut capital = cfg.starting_capital;
        let mut daily_trade_count: HashMap<NaiveDate, u32> = HashMap::new();

        // Debug rejection counters
        let mut n_daily  = 0u32;
        let mut n_bias   = 0u32;
        let mut n_mss    = 0u32;
        let mut n_pd     = 0u32;
        let mut n_fill   = 0u32;
        let mut n_sanity = 0u32;

        for sweep in &sweeps {
            let td = trading_date(sweep.ts_5m);

            let day_count = daily_trade_count.entry(td).or_insert(0);
            if *day_count >= cfg.max_trades_per_day { n_daily += 1; continue; }

            // Bias filter
            if cfg.use_bias_filter {
                let ok = match sweep.direction {
                    TradeDir::Bullish => sweep.bias == HtfBias::Bullish,
                    TradeDir::Bearish => sweep.bias == HtfBias::Bearish,
                };
                if !ok { n_bias += 1; continue; }
            }

            // MSS filter
            let (mss_pivot, mss_ts) = if cfg.use_mss_filter {
                match detect_mss(&self.candles_15m, sweep.ts_5m, sweep.direction, cfg) {
                    Some(r) => r,
                    None => { n_mss += 1; continue; }
                }
            } else {
                // No MSS filter: synthetic — 24 bars forward from sweep
                let fwd = (sweep.idx_5m + 24).min(self.candles_5m.len() - 1);
                (sweep.level_price, self.candles_5m[fwd].open_time)
            };

            // PD array detection
            let arrays = find_pd_arrays(
                &self.candles_5m,
                sweep.idx_5m,
                mss_ts,
                sweep.direction,
                cfg,
            );

            if arrays.is_empty() { n_pd += 1; continue; }

            // Array selection
            let array = if cfg.use_ote_filter {
                match find_best_array_ote(&arrays, sweep.sweep_extreme, mss_pivot, sweep.direction, cfg) {
                    Some(a) => a,
                    None => { n_pd += 1; continue; }
                }
            } else {
                // Without OTE: last (most recent) array — Python's arrays[-1]
                arrays[arrays.len() - 1]
            };

            let entry = (array.bottom + array.top) / Decimal::from(2);

            // SL beyond sweep extreme
            let sl = match sweep.direction {
                TradeDir::Bullish => sweep.sweep_extreme * (Decimal::ONE - cfg.sl_buffer),
                TradeDir::Bearish => sweep.sweep_extreme * (Decimal::ONE + cfg.sl_buffer),
            };

            // Sanity: entry must be on correct side of SL
            let valid_sl = match sweep.direction {
                TradeDir::Bullish => entry > sl,
                TradeDir::Bearish => entry < sl,
            };
            if !valid_sl { n_sanity += 1; continue; }

            // TP targets — use session levels at sweep timestamp (intraday developing)
            let sweep_session = session_levels_at(&intraday_session, sweep.ts_5m);
            let periodic = match periodic_map.get(&td) {
                Some(l) => l,
                None => { n_sanity += 1; continue; }
            };

            let (tp1, tp2) = match find_tp_levels(entry, sweep.direction, periodic, sweep_session, sl, cfg.min_rr) {
                Some(t) => t,
                None => {
                    // Fallback: min_rr * risk — matches Python fallback
                    let risk = (entry - sl).abs();
                    match sweep.direction {
                        TradeDir::Bullish => (
                            entry + cfg.min_rr * risk,
                            entry + cfg.min_rr * Decimal::from(2) * risk,
                        ),
                        TradeDir::Bearish => (
                            entry - cfg.min_rr * risk,
                            entry - cfg.min_rr * Decimal::from(2) * risk,
                        ),
                    }
                }
            };

            // Entry fill starts at MSS bar (Python: sim_start = mss_ts; df_5m.loc[mss_ts:])
            let mss_5m_end = self.candles_5m.partition_point(|c| c.open_time <= mss_ts);
            let fill_start = mss_5m_end.saturating_sub(1);

            let result = simulate_trade(
                fill_start,
                sweep.direction,
                entry,
                sl,
                tp1,
                tp2,
                &self.candles_5m,
                capital,
                cfg,
            );

            if let Some((pnl, r, exit_ts, exit_reason, tp1_hit, be_triggered)) = result {
                capital += pnl;
                *day_count += 1;

                let exit_price = match &exit_reason {
                    ExitReason::Tp2     => tp2,
                    ExitReason::Sl      => sl,
                    ExitReason::Tp1Only => tp1,
                    ExitReason::Timeout => {
                        let idx = self.candles_5m
                            .partition_point(|c| c.open_time <= exit_ts)
                            .saturating_sub(1)
                            .min(self.candles_5m.len() - 1);
                        self.candles_5m[idx].close.0
                    }
                };

                trades.push(IctTrade {
                    direction:    sweep.direction,
                    level_type:   sweep.level_type,
                    kill_zone:    sweep.kill_zone,
                    pd_array_type: array.kind,
                    entry_ts:     sweep.ts_5m,
                    exit_ts,
                    entry,
                    sl,
                    tp1,
                    tp2,
                    exit_price,
                    exit_reason,
                    r_multiple:   r,
                    pnl_usd:      pnl,
                    tp1_hit,
                    be_triggered,
                    equity:       capital,
                });
            } else {
                n_fill += 1;
            }
        }

        if cfg.debug {
            eprintln!(
                "[ICT Debug] sweeps={} | daily={} bias={} mss={} pd={} fill={} sanity={} | trades={}",
                sweeps.len(), n_daily, n_bias, n_mss, n_pd, n_fill, n_sanity, trades.len()
            );
        }

        trades
    }
}

// ── Summary statistics ────────────────────────────────────────────────────────

pub struct IctSummary {
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: Decimal,
    pub avg_r: Decimal,
    pub profit_factor: Decimal,
    pub max_drawdown_pct: Decimal,
    pub final_equity: Decimal,
    pub total_pnl: Decimal,
}

pub fn summarize(trades: &[IctTrade], starting_capital: Decimal) -> IctSummary {
    let total = trades.len();
    if total == 0 {
        return IctSummary {
            total_trades: 0, wins: 0, losses: 0,
            win_rate: Decimal::ZERO, avg_r: Decimal::ZERO,
            profit_factor: Decimal::ZERO, max_drawdown_pct: Decimal::ZERO,
            final_equity: starting_capital, total_pnl: Decimal::ZERO,
        };
    }

    let wins:   Vec<&IctTrade> = trades.iter().filter(|t| t.pnl_usd > Decimal::ZERO).collect();
    let losses: Vec<&IctTrade> = trades.iter().filter(|t| t.pnl_usd < Decimal::ZERO).collect();

    let win_count  = wins.len();
    let loss_count = losses.len();

    let win_rate = Decimal::from(win_count as u32)
        / Decimal::from(total as u32)
        * Decimal::from(100);

    let total_r: Decimal = trades.iter().map(|t| t.r_multiple).sum();
    let avg_r = total_r / Decimal::from(total as u32);

    let gross_profit: Decimal = wins.iter().map(|t| t.pnl_usd).sum();
    let gross_loss: Decimal   = losses.iter().map(|t| t.pnl_usd.abs()).sum();
    let profit_factor = if gross_loss == Decimal::ZERO {
        Decimal::from(9999)
    } else {
        gross_profit / gross_loss
    };

    let mut peak = starting_capital;
    let mut max_dd = Decimal::ZERO;
    for t in trades {
        if t.equity > peak { peak = t.equity; }
        if peak > Decimal::ZERO {
            let dd = (peak - t.equity) / peak * Decimal::from(100);
            if dd > max_dd { max_dd = dd; }
        }
    }

    let final_equity = trades.last().map(|t| t.equity).unwrap_or(starting_capital);
    let total_pnl = final_equity - starting_capital;

    IctSummary {
        total_trades: total,
        wins: win_count,
        losses: loss_count,
        win_rate: win_rate.round_dp(1),
        avg_r: avg_r.round_dp(3),
        profit_factor: profit_factor.round_dp(2),
        max_drawdown_pct: max_dd.round_dp(2),
        final_equity: final_equity.round_dp(2),
        total_pnl: total_pnl.round_dp(2),
    }
}
