use backtest::candle_stick_loader::CandleStickLoader;
use backtest::model::candle_stick::CandleStick;
use backtest::model::position_direction::PositionDirection;
use backtest::to_new_york_time;
use chrono::{Datelike, NaiveDate, TimeZone, Timelike, Utc};
use rayon::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use std::fs;
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
    ObQ25Retest,
    ObEdgeRetest,
}

#[derive(Clone, Copy)]
struct RunCfg {
    rr: Decimal,
    max_hold_bars: usize,
    min_wick_ticks: Decimal,
    use_asymmetric_wick: bool,
    min_wick_long_ticks: Decimal,
    min_wick_short_ticks: Decimal,
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
    use_ema_distance_filter: bool,
    min_close_ema_dist_ticks: Decimal,
    use_candle_quality_filter: bool,
    min_body_pct: Decimal,
    min_range_ticks: Decimal,
    use_trend_structure_filter: bool,
    structure_lookback: usize,
    use_loss_streak_breaker: bool,
    max_losses_per_day: usize,
}

#[derive(Clone, Copy)]
enum Timeframe {
    OneMinute,
    ThreeMinute,
}

#[derive(Clone)]
struct Candidate {
    name: String,
    timeframe: Timeframe,
    ema_period: usize,
    cfg: RunCfg,
}

#[derive(Clone)]
struct StressRow {
    slip_rt: Decimal,
    trades: usize,
    win_rate: Decimal,
    net_usd: Decimal,
    avg_usd: Decimal,
    positive_months: usize,
    negative_months: usize,
    total_months: usize,
    max_drawdown_usd: Decimal,
}

#[derive(Clone)]
struct ValidationRow {
    candidate: Candidate,
    stress_rows: Vec<StressRow>,
    verdict: &'static str,
}

fn load_mnq_1m() -> Vec<CandleStick> {
    CandleStickLoader::load_parquet("assets/mnq_1m_cont.parquet").expect("load mnq parquet")
}

fn validate_data(data: &[CandleStick], expected_spacing_sec: i64) {
    assert!(!data.is_empty(), "empty dataset");
    for (i, b) in data.iter().enumerate() {
        assert!(b.high >= b.low, "OHLC invalid at {i}");
        assert!(b.high >= b.open && b.high >= b.close, "OHLC invalid at {i}");
        assert!(b.low <= b.open && b.low <= b.close, "OHLC invalid at {i}");
        if i > 0 {
            let prev = data[i - 1];
            assert!(b.open_time > prev.open_time, "timestamp not monotonic at {i}");
            let delta = b.open_time - prev.open_time;
            assert!(delta % expected_spacing_sec == 0, "unexpected spacing at {i}: {delta}");
        }
    }
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
            out.push(CandleStick { open_time: cur, close_time: cur + bucket, open: o, high: h, low: l, close: c });
            cur = b;
            o = x.open;
            h = x.high;
            l = x.low;
            c = x.close;
        } else {
            if x.high > h { h = x.high; }
            if x.low < l { l = x.low; }
            c = x.close;
        }
    }
    out.push(CandleStick { open_time: cur, close_time: cur + bucket, open: o, high: h, low: l, close: c });
    out
}

fn ema_series(candles: &[CandleStick], period: usize) -> Vec<EmaPoint> {
    let mut out = Vec::new();
    if candles.len() < period { return out; }
    let k = Decimal::from_i64(2).unwrap() / Decimal::from_usize(period + 1).unwrap();
    let mut seed = Decimal::ZERO;
    for c in candles.iter().take(period) { seed += c.close.0; }
    let mut ema = seed / Decimal::from_usize(period).unwrap();
    out.push(EmaPoint { ts: candles[period - 1].close_time, value: ema });
    for c in candles.iter().skip(period) {
        ema = c.close.0 * k + ema * (Decimal::ONE - k);
        out.push(EmaPoint { ts: c.close_time, value: ema });
    }
    out
}

fn cutoff(date: &str) -> i64 {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("date");
    Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).expect("midnight")).timestamp()
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
    if i + 1 < period { return Decimal::ZERO; }
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

fn run(tf: &[CandleStick], ema_5m: &[EmaPoint], from_ts: i64, cfg: RunCfg) -> Vec<Trade> {
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
        if cfg.use_loss_streak_breaker && loss_streak_today >= cfg.max_losses_per_day { i += 1; continue; }
        if c.open_time < from_ts { i += 1; continue; }
        if !in_session(c.open_time, cfg.session) { i += 1; continue; }
        while eidx + 1 < ema_5m.len() && ema_5m[eidx + 1].ts <= c.close_time { eidx += 1; }
        if eidx >= ema_5m.len() || ema_5m[eidx].ts > c.close_time { i += 1; continue; }
        let ema = ema_5m[eidx].value;
        if cfg.use_ema_distance_filter {
            let dist_ticks = (c.close.0 - ema).abs() / cfg.tick_size;
            if dist_ticks < cfg.min_close_ema_dist_ticks { i += 1; continue; }
        }
        if cfg.use_candle_quality_filter {
            let range = c.high.0 - c.low.0;
            if range <= Decimal::ZERO { i += 1; continue; }
            let body = (c.close.0 - c.open.0).abs();
            let body_pct = body / range * Decimal::from(100);
            let range_ticks = range / cfg.tick_size;
            if body_pct < cfg.min_body_pct || range_ticks < cfg.min_range_ticks { i += 1; continue; }
        }
        let long_signal = c.low.0 < ema && c.close.0 > ema;
        let short_signal = c.high.0 > ema && c.close.0 < ema;
        if !long_signal && !short_signal { i += 1; continue; }
        let direction = if long_signal { PositionDirection::Long } else { PositionDirection::Short };
        if cfg.use_trend_structure_filter {
            if eidx == 0 || i < cfg.structure_lookback.max(2) { i += 1; continue; }
            let ema_slope_up = ema_5m[eidx].value > ema_5m[eidx - 1].value;
            let ema_slope_down = ema_5m[eidx].value < ema_5m[eidx - 1].value;
            let mut hh = true; let mut hl = true; let mut lh = true; let mut ll = true;
            let start = i + 1 - cfg.structure_lookback;
            for k in (start + 1)..=i {
                if tf[k].high.0 <= tf[k - 1].high.0 { hh = false; }
                if tf[k].low.0 <= tf[k - 1].low.0 { hl = false; }
                if tf[k].high.0 >= tf[k - 1].high.0 { lh = false; }
                if tf[k].low.0 >= tf[k - 1].low.0 { ll = false; }
            }
            let trend_ok = match direction { PositionDirection::Long => ema_slope_up && hh && hl, PositionDirection::Short => ema_slope_down && lh && ll };
            if !trend_ok { i += 1; continue; }
        }
        let wick_pen = if direction == PositionDirection::Long { ema - c.low.0 } else { c.high.0 - ema };
        let min_wick_ticks_eff = if cfg.use_asymmetric_wick {
            if direction == PositionDirection::Long { cfg.min_wick_long_ticks } else { cfg.min_wick_short_ticks }
        } else { cfg.min_wick_ticks };
        if wick_pen < min_wick_ticks_eff * cfg.tick_size { i += 1; continue; }
        let entry = c.close.0;
        let wick_risk = if direction == PositionDirection::Long { entry - c.low.0 } else { c.high.0 - entry };
        let mut risk = wick_risk;
        let atr_v = atr(tf, i, cfg.atr_period);
        if cfg.use_regime_filter {
            let atr_ticks = if cfg.tick_size > Decimal::ZERO { atr_v / cfg.tick_size } else { Decimal::ZERO };
            if atr_ticks < cfg.regime_min_atr_ticks || atr_ticks > cfg.regime_max_atr_ticks { i += 1; continue; }
        }
        let atr_floor = atr_v * cfg.atr_floor_mult;
        let min_stop = cfg.min_stop_ticks * cfg.tick_size;
        let atr_stop = if atr_floor > min_stop { atr_floor } else { min_stop };
        match cfg.stop_mode {
            StopMode::Wick => { if min_stop > risk { risk = min_stop; } }
            StopMode::Atr => risk = atr_stop,
            StopMode::Hybrid => { if atr_stop > risk { risk = atr_stop; } }
        }
        if risk <= Decimal::ZERO { i += 1; continue; }
        let cost_r = (cfg.fee_rt + cfg.cost_filter_slippage_rt) / (risk * point_value);
        if cost_r > cfg.max_cost_r { i += 1; continue; }
        let signal_open = c.open.0;
        let signal_close = c.close.0;
        let ob_top = if signal_open > signal_close { signal_open } else { signal_close };
        let ob_bottom = if signal_open < signal_close { signal_open } else { signal_close };
        let mut actual_entry = entry;
        let mut entry_idx = i;
        if !matches!(cfg.entry_mode, EntryMode::Immediate) {
            let retest_level = match cfg.entry_mode {
                EntryMode::ObMidRetest => (ob_top + ob_bottom) / Decimal::from(2),
                EntryMode::ObQ25Retest => ob_bottom + (ob_top - ob_bottom) * Decimal::new(25, 2),
                EntryMode::ObEdgeRetest => if direction == PositionDirection::Long { ob_top } else { ob_bottom },
                EntryMode::Immediate => entry,
            };
            let mut found = false;
            let max_j = (i + cfg.ob_wait_bars).min(tf.len().saturating_sub(1));
            let mut j2 = i + 1;
            while j2 <= max_j {
                let nx = tf[j2];
                let touch = if direction == PositionDirection::Long { nx.low.0 <= retest_level } else { nx.high.0 >= retest_level };
                let confirm = if direction == PositionDirection::Long { nx.close.0 > nx.open.0 } else { nx.close.0 < nx.open.0 };
                if touch && confirm {
                    let next_idx = j2 + 1;
                    if next_idx >= tf.len() { break; }
                    actual_entry = tf[next_idx].open.0;
                    entry_idx = next_idx;
                    found = true;
                    break;
                }
                j2 += 1;
            }
            if !found { i += 1; continue; }
        }
        let sl = if direction == PositionDirection::Long { actual_entry - risk } else { actual_entry + risk };
        let rr_eff = if cfg.use_dynamic_rr {
            let atr_ticks = if cfg.tick_size > Decimal::ZERO { atr_v / cfg.tick_size } else { Decimal::ZERO };
            if atr_ticks < Decimal::from(6) { cfg.rr_low_vol } else if atr_ticks < Decimal::from(14) { cfg.rr_mid_vol } else { cfg.rr_high_vol }
        } else { cfg.rr };
        let tp = if direction == PositionDirection::Long { actual_entry + risk * rr_eff } else { actual_entry - risk * rr_eff };
        let mut pnl_points = Decimal::ZERO;
        let mut j = entry_idx + 1;
        let mut exit_ts = tf[entry_idx].close_time;
        let mut held = 0usize;
        while j < tf.len() {
            let nx = tf[j];
            let hit_sl = if direction == PositionDirection::Long { nx.low.0 <= sl } else { nx.high.0 >= sl };
            let hit_tp = if direction == PositionDirection::Long { nx.high.0 >= tp } else { nx.low.0 <= tp };
            let sl_fill = if direction == PositionDirection::Long { if nx.open.0 < sl { nx.open.0 } else { sl } } else if nx.open.0 > sl { nx.open.0 } else { sl };
            if hit_sl && hit_tp || hit_sl {
                pnl_points = if direction == PositionDirection::Long { sl_fill - actual_entry } else { actual_entry - sl_fill };
                exit_ts = nx.close_time;
                break;
            }
            if hit_tp {
                pnl_points = risk * rr_eff;
                exit_ts = nx.close_time;
                break;
            }
            if cfg.require_micro_confirm && held == 0 {
                let bad_confirm = if direction == PositionDirection::Long { nx.close.0 < nx.open.0 } else { nx.close.0 > nx.open.0 };
                if bad_confirm {
                    pnl_points = (nx.close.0 - actual_entry) * if direction == PositionDirection::Long { Decimal::ONE } else { -Decimal::ONE };
                    exit_ts = nx.close_time;
                    break;
                }
            }
            held += 1;
            if held >= cfg.max_hold_bars {
                pnl_points = (nx.close.0 - actual_entry) * if direction == PositionDirection::Long { Decimal::ONE } else { -Decimal::ONE };
                exit_ts = nx.close_time;
                break;
            }
            j += 1;
        }
        let pnl_usd = pnl_points * point_value - cfg.fee_rt - cfg.slippage_rt;
        if cfg.use_loss_streak_breaker {
            if pnl_usd < Decimal::ZERO { loss_streak_today += 1; } else if pnl_usd > Decimal::ZERO { loss_streak_today = 0; }
        }
        out.push(Trade { close_time: exit_ts, pnl_usd });
        i = if j > i { j } else { i + 1 };
    }
    out
}

fn monthly_summary(trades: &[Trade]) -> (usize, usize, usize, BTreeMap<String, Decimal>) {
    let mut m = BTreeMap::new();
    for t in trades {
        let d = to_new_york_time(t.close_time);
        let k = format!("{:04}-{:02}", d.year(), d.month());
        *m.entry(k).or_insert(Decimal::ZERO) += t.pnl_usd;
    }
    let mut pos = 0usize;
    let mut neg = 0usize;
    for v in m.values() {
        if *v > Decimal::ZERO { pos += 1; }
        else if *v < Decimal::ZERO { neg += 1; }
    }
    (pos, neg, m.len(), m)
}

fn max_drawdown_usd(trades: &[Trade]) -> Decimal {
    let mut equity = Decimal::ZERO;
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    for t in trades {
        equity += t.pnl_usd;
        if equity > peak { peak = equity; }
        let dd = peak - equity;
        if dd > max_dd { max_dd = dd; }
    }
    max_dd.round_dp(2)
}

fn summarize_stress(slip_rt: Decimal, trades: &[Trade]) -> StressRow {
    let trades_n = trades.len();
    let wins = trades.iter().filter(|t| t.pnl_usd > Decimal::ZERO).count();
    let net_usd: Decimal = trades.iter().map(|t| t.pnl_usd).sum();
    let avg_usd = if trades_n > 0 { net_usd / Decimal::from_usize(trades_n).unwrap() } else { Decimal::ZERO };
    let win_rate = if trades_n > 0 {
        Decimal::from_usize(wins).unwrap() * Decimal::from(100) / Decimal::from_usize(trades_n).unwrap()
    } else { Decimal::ZERO };
    let (positive_months, negative_months, total_months, _) = monthly_summary(trades);
    StressRow { slip_rt, trades: trades_n, win_rate: win_rate.round_dp(2), net_usd: net_usd.round_dp(2), avg_usd: avg_usd.round_dp(2), positive_months, negative_months, total_months, max_drawdown_usd: max_drawdown_usd(trades) }
}

fn candidate_verdict(rows: &[StressRow]) -> &'static str {
    let all_positive = rows.iter().all(|r| r.net_usd > Decimal::ZERO);
    let robust_months = rows.iter().all(|r| r.positive_months >= r.negative_months && r.positive_months >= 8);
    let dd_ok = rows.iter().all(|r| r.max_drawdown_usd <= Decimal::from(6000));
    let trades_ok = rows.first().map(|r| r.trades >= 300).unwrap_or(false);
    if all_positive && robust_months && dd_ok && trades_ok {
        "PROMOTABLE"
    } else if rows.first().map(|r| r.net_usd > Decimal::ZERO).unwrap_or(false) {
        "PARTIAL"
    } else {
        "REJECT"
    }
}

fn timeframe_name(tf: Timeframe) -> &'static str {
    match tf { Timeframe::OneMinute => "1m", Timeframe::ThreeMinute => "3m" }
}

fn session_name(s: SessionFilter) -> &'static str {
    match s {
        SessionFilter::All => "all",
        SessionFilter::London => "london",
        SessionFilter::NyAm => "ny_am",
        SessionFilter::NyOpen => "ny_open",
        SessionFilter::NyLate => "ny_late",
    }
}

fn stop_name(s: StopMode) -> &'static str {
    match s { StopMode::Wick => "wick", StopMode::Atr => "atr", StopMode::Hybrid => "hybrid" }
}

fn entry_mode_name(e: EntryMode) -> &'static str {
    match e { EntryMode::Immediate => "immediate", EntryMode::ObMidRetest => "ob_mid", EntryMode::ObQ25Retest => "ob_q25", EntryMode::ObEdgeRetest => "ob_edge" }
}

fn candidate_label(c: &Candidate) -> String {
    format!(
        "{} ema{} rr{} wick{} atr{} cap{} {} {} {}",
        timeframe_name(c.timeframe),
        c.ema_period,
        c.cfg.rr.round_dp(2),
        c.cfg.min_wick_ticks.round_dp(2),
        c.cfg.atr_floor_mult.round_dp(2),
        c.cfg.max_cost_r.round_dp(2),
        session_name(c.cfg.session),
        stop_name(c.cfg.stop_mode),
        entry_mode_name(c.cfg.entry_mode),
    )
}

fn main() {
    let one_min = load_mnq_1m();
    validate_data(&one_min, 60);
    let three_min = resample(&one_min, 3);
    validate_data(&three_min, 180);
    let five_min = resample(&one_min, 5);
    validate_data(&five_min, 300);

    let workers = std::thread::available_parallelism().map(|n| n.get().min(8)).unwrap_or(4);
    rayon::ThreadPoolBuilder::new().num_threads(workers).build_global().ok();

    let one_min = Arc::new(one_min);
    let three_min = Arc::new(three_min);
    let mut ema_map: HashMap<usize, Arc<Vec<EmaPoint>>> = HashMap::new();
    for p in [200usize, 300usize] {
        ema_map.insert(p, Arc::new(ema_series(&five_min, p)));
    }

    let from_ts = cutoff("2025-01-01");
    let base = RunCfg {
        rr: Decimal::from(4), max_hold_bars: 120, min_wick_ticks: Decimal::from(4),
        use_asymmetric_wick: false, min_wick_long_ticks: Decimal::from(4), min_wick_short_ticks: Decimal::from(4),
        min_stop_ticks: Decimal::from(8), atr_floor_mult: Decimal::new(5,1), atr_period: 14,
        max_cost_r: Decimal::new(15,2), tick_size: Decimal::new(25,2), fee_rt: Decimal::new(124,2),
        slippage_rt: Decimal::ONE, cost_filter_slippage_rt: Decimal::ONE, session: SessionFilter::All,
        stop_mode: StopMode::Hybrid, entry_mode: EntryMode::Immediate, ob_wait_bars: 8,
        use_regime_filter: false, regime_min_atr_ticks: Decimal::from(4), regime_max_atr_ticks: Decimal::from(30),
        require_micro_confirm: false, use_dynamic_rr: false, rr_low_vol: Decimal::from(3), rr_mid_vol: Decimal::from(4), rr_high_vol: Decimal::from(5),
        use_ema_distance_filter: false, min_close_ema_dist_ticks: Decimal::from(2), use_candle_quality_filter: false,
        min_body_pct: Decimal::from(30), min_range_ticks: Decimal::from(6), use_trend_structure_filter: false, structure_lookback: 3,
        use_loss_streak_breaker: false, max_losses_per_day: 2,
    };

    let candidates = vec![
        Candidate {
            name: "3m_ema300_baseline".to_string(),
            timeframe: Timeframe::ThreeMinute,
            ema_period: 300,
            cfg: RunCfg { rr: Decimal::from(3), min_wick_ticks: Decimal::from(2), atr_floor_mult: Decimal::ONE, max_cost_r: Decimal::new(10,2), session: SessionFilter::NyAm, ..base },
        },
        Candidate {
            name: "3m_ema200_baseline".to_string(),
            timeframe: Timeframe::ThreeMinute,
            ema_period: 200,
            cfg: RunCfg { rr: Decimal::from(3), min_wick_ticks: Decimal::from(2), atr_floor_mult: Decimal::ONE, max_cost_r: Decimal::new(10,2), session: SessionFilter::NyAm, ..base },
        },
        Candidate {
            name: "3m_top_sweep_net".to_string(),
            timeframe: Timeframe::ThreeMinute,
            ema_period: 200,
            cfg: RunCfg { rr: Decimal::from(5), min_wick_ticks: Decimal::from(2), atr_floor_mult: Decimal::ONE, max_cost_r: Decimal::new(10,2), session: SessionFilter::NyAm, stop_mode: StopMode::Hybrid, entry_mode: EntryMode::Immediate, ..base },
        },
        Candidate {
            name: "1m_top_sweep_net".to_string(),
            timeframe: Timeframe::OneMinute,
            ema_period: 200,
            cfg: RunCfg { rr: Decimal::from(3), min_wick_ticks: Decimal::from(8), atr_floor_mult: Decimal::new(5,1), max_cost_r: Decimal::new(10,2), session: SessionFilter::NyOpen, stop_mode: StopMode::Wick, entry_mode: EntryMode::ObEdgeRetest, ..base },
        },
        Candidate {
            name: "3m_current_best_post_fix".to_string(),
            timeframe: Timeframe::ThreeMinute,
            ema_period: 200,
            cfg: RunCfg { rr: Decimal::from(2), min_wick_ticks: Decimal::from(8), atr_floor_mult: Decimal::new(5,1), max_cost_r: Decimal::new(10,2), session: SessionFilter::All, stop_mode: StopMode::Atr, entry_mode: EntryMode::ObMidRetest, ob_wait_bars: 8, max_hold_bars: 120, ..base },
        },
    ];

    let base_ema300 = RunCfg { rr: Decimal::from(3), min_wick_ticks: Decimal::from(2), atr_floor_mult: Decimal::ONE, max_cost_r: Decimal::new(10,2), session: SessionFilter::NyAm, ..base };
    let base_top3m = RunCfg { rr: Decimal::from(5), min_wick_ticks: Decimal::from(2), atr_floor_mult: Decimal::ONE, max_cost_r: Decimal::new(10,2), session: SessionFilter::NyAm, stop_mode: StopMode::Hybrid, entry_mode: EntryMode::Immediate, ..base };

    let mut candidates = candidates;
    for ema_period in [200usize, 300usize] {
        for rr in [Decimal::from(3), Decimal::from(4), Decimal::from(5)] {
            for wick in [Decimal::from(2), Decimal::from(4), Decimal::from(6), Decimal::from(8)] {
                for atr_mult in [Decimal::new(75,2), Decimal::ONE, Decimal::new(125,2)] {
                    for max_cost_r in [Decimal::new(10,2), Decimal::new(15,2)] {
                        for session in [SessionFilter::NyAm, SessionFilter::NyOpen] {
                            for stop_mode in [StopMode::Hybrid, StopMode::Wick, StopMode::Atr] {
                                let template = if ema_period == 300 { base_ema300 } else { base_top3m };
                                let cfg = RunCfg {
                                    rr,
                                    min_wick_ticks: wick,
                                    atr_floor_mult: atr_mult,
                                    max_cost_r,
                                    session,
                                    stop_mode,
                                    ..template
                                };
                                candidates.push(Candidate {
                                    name: format!(
                                        "neighbor_{}",
                                        candidate_label(&Candidate {
                                            name: String::new(),
                                            timeframe: Timeframe::ThreeMinute,
                                            ema_period,
                                            cfg,
                                        })
                                        .replace(' ', "_")
                                    ),
                                    timeframe: Timeframe::ThreeMinute,
                                    ema_period,
                                    cfg,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let primary_base = RunCfg {
        rr: Decimal::from(5),
        min_wick_ticks: Decimal::from(6),
        atr_floor_mult: Decimal::new(75, 2),
        max_cost_r: Decimal::new(10, 2),
        session: SessionFilter::NyAm,
        stop_mode: StopMode::Wick,
        entry_mode: EntryMode::Immediate,
        ..base
    };

    for use_regime_filter in [false, true] {
        for require_micro_confirm in [false, true] {
            for use_ema_distance_filter in [false, true] {
                for use_candle_quality_filter in [false, true] {
                    for use_trend_structure_filter in [false, true] {
                        for use_loss_streak_breaker in [false, true] {
                            let cfg = RunCfg {
                                use_regime_filter,
                                regime_min_atr_ticks: Decimal::from(2),
                                regime_max_atr_ticks: Decimal::from(200),
                                require_micro_confirm,
                                use_ema_distance_filter,
                                min_close_ema_dist_ticks: Decimal::from(1),
                                use_candle_quality_filter,
                                min_body_pct: Decimal::from(25),
                                min_range_ticks: Decimal::from(5),
                                use_trend_structure_filter,
                                structure_lookback: 3,
                                use_loss_streak_breaker,
                                max_losses_per_day: 2,
                                ..primary_base
                            };
                            candidates.push(Candidate {
                                name: format!(
                                    "quality_{}{}{}{}{}{}",
                                    if use_regime_filter { "reg" } else { "noreg" },
                                    if require_micro_confirm { "_micro" } else { "_nomicro" },
                                    if use_ema_distance_filter { "_ema" } else { "_noema" },
                                    if use_candle_quality_filter { "_candle" } else { "_nocandle" },
                                    if use_trend_structure_filter { "_trend" } else { "_notrend" },
                                    if use_loss_streak_breaker { "_streak" } else { "_nostreak" },
                                ),
                                timeframe: Timeframe::ThreeMinute,
                                ema_period: 300,
                                cfg,
                            });
                        }
                    }
                }
            }
        }
    }

    let slips = [Decimal::ONE, Decimal::new(15,1), Decimal::from(2)];

    let rows: Vec<ValidationRow> = candidates
        .par_iter()
        .map(|candidate| {
            let tf = match candidate.timeframe { Timeframe::OneMinute => Arc::clone(&one_min), Timeframe::ThreeMinute => Arc::clone(&three_min) };
            let ema = Arc::clone(ema_map.get(&candidate.ema_period).expect("ema period"));
            let stress_rows: Vec<StressRow> = slips
                .iter()
                .map(|slip| {
                    let mut cfg = candidate.cfg;
                    cfg.slippage_rt = *slip;
                    let trades = run(tf.as_slice(), ema.as_slice(), from_ts, cfg);
                    summarize_stress(*slip, &trades)
                })
                .collect();
            let verdict = candidate_verdict(&stress_rows);
            ValidationRow { candidate: candidate.clone(), stress_rows, verdict }
        })
        .collect();

    let mut sorted = rows;
    sorted.sort_by(|a, b| {
        b.stress_rows[0]
            .net_usd
            .cmp(&a.stress_rows[0].net_usd)
            .then_with(|| b.stress_rows[1].net_usd.cmp(&a.stress_rows[1].net_usd))
    });

    let mut quality_sorted = sorted.clone();
    quality_sorted.sort_by(|a, b| {
        let a0 = &a.stress_rows[0];
        let b0 = &b.stress_rows[0];
        let a_all_positive = a.stress_rows.iter().all(|r| r.net_usd > Decimal::ZERO);
        let b_all_positive = b.stress_rows.iter().all(|r| r.net_usd > Decimal::ZERO);
        b_all_positive
            .cmp(&a_all_positive)
            .then_with(|| b0.win_rate.cmp(&a0.win_rate))
            .then_with(|| a0.max_drawdown_usd.cmp(&b0.max_drawdown_usd))
            .then_with(|| b0.net_usd.cmp(&a0.net_usd))
    });

    let mut md = String::new();
    md.push_str("# MNQ EMA Wick Reclaim 2025 Validation\n\n");
    md.push_str("- Symbol: MNQ\n");
    md.push_str("- Date filter: >= 2025-01-01\n");
    md.push_str("- Costs: $1.24 round-trip fee + slippage stress $1.00 / $1.50 / $2.00\n");
    md.push_str("- Runtime optimization: shared datasets via Arc, candidate validation via rayon\n\n");
    md.push_str("## Candidate Summary\n\n");
    md.push_str("| candidate | config | verdict | slip$ | trades | win_rate_% | net_usd | avg_usd | +months | -months | max_dd_usd |\n");
    md.push_str("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for row in &sorted {
        for stress in &row.stress_rows {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                row.candidate.name,
                candidate_label(&row.candidate),
                row.verdict,
                stress.slip_rt.round_dp(2),
                stress.trades,
                stress.win_rate,
                stress.net_usd,
                stress.avg_usd,
                stress.positive_months,
                stress.negative_months,
                stress.max_drawdown_usd,
            ));
        }
    }

    md.push_str("\n## Quality-Ranked Candidates\n\n");
    md.push_str("| candidate | config | verdict | slip1 win_rate_% | slip1 net_usd | slip1 max_dd_usd | all_slips_positive |\n");
    md.push_str("|---|---|---|---:|---:|---:|---|\n");
    for row in quality_sorted.iter().take(20) {
        let s1 = &row.stress_rows[0];
        let all_positive = row.stress_rows.iter().all(|r| r.net_usd > Decimal::ZERO);
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            row.candidate.name,
            candidate_label(&row.candidate),
            row.verdict,
            s1.win_rate,
            s1.net_usd,
            s1.max_drawdown_usd,
            all_positive,
        ));
    }

    fs::create_dir_all("reports/strategy_overviews").expect("create reports dir");
    let report_path = "reports/strategy_overviews/MNQ_EMA_WICK_RECLAIM_2025_VALIDATION.md";
    fs::write(report_path, md).expect("write report");

    println!("Wrote {}", report_path);
    for row in &sorted {
        println!("{} | {}", row.candidate.name, row.verdict);
        for stress in &row.stress_rows {
            println!(
                "  slip=${}: trades={} win={}%, net=${}, +m/-m={}/{}, max_dd=${}",
                stress.slip_rt.round_dp(2),
                stress.trades,
                stress.win_rate,
                stress.net_usd,
                stress.positive_months,
                stress.negative_months,
                stress.max_drawdown_usd,
            );
        }
    }
}
