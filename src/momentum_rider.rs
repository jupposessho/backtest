extern crate rust_decimal;

use chrono::{Datelike, Timelike};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    model::{
        backtest_result::BacktestResult,
        position_direction::PositionDirection,
        trade::Trade,
        trade_result::TradeResult,
    },
    strategies::crypto_momentum_rider::{
        MomentumRider, MomentumRiderConfig, MomentumTimeWindow, StopMode,
    },
    to_new_york_time,
};
use backtest::model::trading_model::TradingModel;

// ── Asset descriptors ──────────────────────────────────────────────────────────

struct AssetData {
    name: &'static str,
    data_15m_json: &'static str,
    data_1h_json: &'static str,
}

// ── Dollar-based equity metrics (same approach as time_filtered_backtest.rs) ──
//
// Returns: (final_balance, max_drawdown_pct, gross_profit, gross_loss)
fn equity_metrics(trades: &[Trade], starting_capital: Decimal) -> (Decimal, Decimal, Decimal, Decimal) {
    let risk_pct = Decimal::from_f32(0.01).unwrap();
    let mut balance = starting_capital;
    let mut peak = starting_capital;
    let mut max_dd = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;

    for t in trades {
        let risk_dist = (t.sl.0 - t.entry.0).abs();
        if risk_dist <= Decimal::ZERO { continue; }
        let pos_size = (balance * risk_pct) / risk_dist;
        let exit_price = match t.result {
            TradeResult::Winner    => t.tp.0,
            TradeResult::Expense   => t.sl.0,
            TradeResult::BreakEven => t.entry.0,
        };
        let gross_pnl = match t.direction {
            PositionDirection::Long  => (exit_price - t.entry.0) * pos_size,
            PositionDirection::Short => (t.entry.0 - exit_price) * pos_size,
        };
        let net_pnl = gross_pnl - t.entry_commission.0 * pos_size - t.exit_commission.0 * pos_size;
        balance += net_pnl;
        if net_pnl > Decimal::ZERO { gross_profit += net_pnl; }
        else { gross_loss += net_pnl.abs(); }
        if balance > peak { peak = balance; }
        if peak > Decimal::ZERO {
            let dd = (peak - balance) / peak * Decimal::from(100);
            if dd > max_dd { max_dd = dd; }
        }
    }
    (balance, max_dd.round_dp(2), gross_profit, gross_loss)
}

// ── Slice metrics ──────────────────────────────────────────────────────────────

struct SliceMetrics {
    count: usize,
    win_rate: Decimal,
    profit_factor: Decimal,
    expectancy_r: Decimal,
    balance: Decimal,
    max_dd: Decimal,
    avg_winner_rr: Decimal,
}

fn slice_metrics(trades: &[Trade]) -> SliceMetrics {
    let starting = Decimal::from(1000);
    let count = trades.len();
    if count == 0 {
        return SliceMetrics {
            count: 0, win_rate: Decimal::ZERO, profit_factor: Decimal::ZERO,
            expectancy_r: Decimal::ZERO, balance: starting,
            max_dd: Decimal::ZERO, avg_winner_rr: Decimal::ZERO,
        };
    }
    let win_count  = trades.iter().filter(|t| t.result == TradeResult::Winner).count();
    let loss_count = trades.iter().filter(|t| t.result == TradeResult::Expense).count();
    let win_rate = (Decimal::from(win_count as u32) / Decimal::from(count as u32)
        * Decimal::from(100)).round_dp(1);
    let gross_r_win: Decimal = trades.iter()
        .filter(|t| t.result == TradeResult::Winner)
        .map(|t| t.rr().0).sum();
    let profit_factor = if loss_count == 0 { Decimal::from(9999) }
        else { (gross_r_win / Decimal::from(loss_count as u32)).round_dp(2) };
    let total_r: Decimal = trades.iter().map(|t| match t.result {
        TradeResult::Winner    => t.rr().0,
        TradeResult::Expense   => Decimal::from(-1),
        TradeResult::BreakEven => Decimal::ZERO,
    }).sum();
    let expectancy_r = (total_r / Decimal::from(count as u32)).round_dp(3);
    let avg_winner_rr = if win_count == 0 { Decimal::ZERO }
        else { (gross_r_win / Decimal::from(win_count as u32)).round_dp(2) };
    let (balance, max_dd, _, _) = equity_metrics(trades, starting);
    SliceMetrics { count, win_rate, profit_factor, expectancy_r,
        balance: balance.round_dp(0), max_dd, avg_winner_rr }
}

// ── Filter helpers ─────────────────────────────────────────────────────────────

fn trade_hour(t: &Trade) -> u32  { to_new_york_time(t.open_time).hour() }
fn trade_dow(t: &Trade)  -> u32  { to_new_york_time(t.open_time).weekday().num_days_from_monday() }
fn trade_month(t: &Trade) -> u32 { to_new_york_time(t.open_time).month() }

fn filter_hours(trades: &[Trade], hours: &[u32]) -> Vec<Trade> {
    trades.iter().copied().filter(|t| hours.contains(&trade_hour(t))).collect()
}
fn filter_days(trades: &[Trade], days: &[u32]) -> Vec<Trade> {
    trades.iter().copied().filter(|t| days.contains(&trade_dow(t))).collect()
}

// ── Top-N by profit factor ─────────────────────────────────────────────────────

fn top_n_by_pf<F>(trades: &[Trade], n: usize, key_fn: F, max_key: u32) -> Vec<u32>
where F: Fn(&Trade) -> u32,
{
    let mut ranked: Vec<(u32, Decimal)> = (0..max_key).filter_map(|k| {
        let subset: Vec<Trade> = trades.iter().copied().filter(|t| key_fn(t) == k).collect();
        if subset.len() < 15 { return None; }
        let m = slice_metrics(&subset);
        if m.profit_factor == Decimal::ZERO { return None; }
        Some((k, m.profit_factor))
    }).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    ranked.into_iter().take(n).map(|(k, _)| k).collect()
}

// ── Print helpers ──────────────────────────────────────────────────────────────

fn pass(ok: bool) -> &'static str { if ok { "PASS" } else { "FAIL" } }

fn print_summary(name: &str, result: &BacktestResult) {
    let trades = result.number_of_trades();
    let m = slice_metrics(&result.trades);
    let pnl = result.pnl();
    println!("\n  {} — Full Dataset", name);
    println!("  {}", "─".repeat(52));
    println!("  {:22} {:>10} {:>8} {:>6}", "Metric", "Target", "Actual", "");
    println!("  {:22} {:>10} {:>8} {:>6}", "Trade count",      "≥ 100",  trades,              pass(trades >= 100));
    println!("  {:22} {:>10} {:>8} {:>6}", "Win rate (%)",     "≥ 40%",  m.win_rate,          pass(m.win_rate >= Decimal::from(40)));
    println!("  {:22} {:>10} {:>8} {:>6}", "Profit factor",    "≥ 1.4",  m.profit_factor,     pass(m.profit_factor >= Decimal::from_f32(1.4).unwrap()));
    println!("  {:22} {:>10} {:>8} {:>6}", "Expectancy (R)",   "> 0",    m.expectancy_r,      pass(m.expectancy_r > Decimal::ZERO));
    println!("  {:22} {:>10} {:>8} {:>6}", "Max drawdown (%)", "< 15%",  m.max_dd,            pass(m.max_dd < Decimal::from(15)));
    println!("  {:22} {:>10} {:>8} {:>6}", "Avg winner R:R",   "≥ 1.5",  m.avg_winner_rr,     pass(m.avg_winner_rr >= Decimal::from_f32(1.5).unwrap()));
    println!("  {:22} {:>10} {:>8} {:>6}", "Final P&L (%)",    "—",      pnl,                 "");
    println!("  {:22} {:>18}", "Winners / Losses",
        format!("{} / {}", result.result(TradeResult::Winner), result.result(TradeResult::Expense)));
}

fn print_table_header() {
    println!("  {:>10}  {:>6}  {:>5}     {:>4}   {:>8}  {:>9}  {:>7}  {:>8}",
        "", "Trades", "Win%", "PF", "Exp(R)", "Bal($)", "MaxDD%", "AvgWRR");
    println!("  {}", "─".repeat(75));
}

fn print_row(label: &str, m: &SliceMetrics) {
    if m.count == 0 { return; }
    println!("  {:>10}  {:>6}  {:>5}  {:>6}  {:>8}  {:>9}  {:>7}  {:>8}",
        label, m.count,
        format!("{:.1}%", m.win_rate),
        m.profit_factor, m.expectancy_r,
        format!("${}", m.balance),
        format!("{:.2}%", m.max_dd),
        m.avg_winner_rr,
    );
}

// ── Time breakdown (per-asset) ─────────────────────────────────────────────────

fn print_hour_breakdown(trades: &[Trade]) {
    println!("\n  HOUR OF ENTRY (NY timezone)");
    print_table_header();
    for h in 0u32..24 {
        let s: Vec<Trade> = trades.iter().copied().filter(|t| trade_hour(t) == h).collect();
        if s.len() < 5 { continue; }
        print_row(&format!("{:02}:00", h), &slice_metrics(&s));
    }
}

fn print_dow_breakdown(trades: &[Trade]) {
    println!("\n  DAY OF WEEK");
    print_table_header();
    let names = ["Mon","Tue","Wed","Thu","Fri","Sat","Sun"];
    for d in 0u32..7 {
        let s: Vec<Trade> = trades.iter().copied().filter(|t| trade_dow(t) == d).collect();
        if s.len() < 5 { continue; }
        print_row(names[d as usize], &slice_metrics(&s));
    }
}

fn print_month_breakdown(trades: &[Trade]) {
    println!("\n  MONTH");
    print_table_header();
    let names = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    for mo in 1u32..=12 {
        let s: Vec<Trade> = trades.iter().copied().filter(|t| trade_month(t) == mo).collect();
        if s.len() < 5 { continue; }
        print_row(names[(mo-1) as usize], &slice_metrics(&s));
    }
}

/// Prints the best-combo analysis and returns (best_hours, best_days) for reuse in the sweep.
fn print_best_combo(trades: &[Trade]) -> (Vec<u32>, Vec<u32>) {
    let best_hours = top_n_by_pf(trades, 4, trade_hour, 24);
    let best_days  = top_n_by_pf(trades, 3, trade_dow,  7);

    let day_names = ["Mon","Tue","Wed","Thu","Fri","Sat","Sun"];
    let hour_str: Vec<String> = best_hours.iter().map(|&h| format!("{:02}:00", h)).collect();
    let day_str:  Vec<String> = best_days.iter().map(|&d| day_names[d as usize].to_string()).collect();

    println!("\n  BEST COMBINATION (post-hoc filter on current trades)");
    println!("  Hours : {}", if hour_str.is_empty() { "—".into() } else { hour_str.join(", ") });
    println!("  Days  : {}", if day_str.is_empty()  { "—".into() } else { day_str.join(", ") });

    if !best_hours.is_empty() {
        let s = filter_hours(trades, &best_hours);
        println!("\n  → Hours only"); print_table_header(); print_row("filtered", &slice_metrics(&s));
    }
    if !best_days.is_empty() {
        let s = filter_days(trades, &best_days);
        println!("\n  → Days only");  print_table_header(); print_row("filtered", &slice_metrics(&s));
    }
    if !best_hours.is_empty() && !best_days.is_empty() {
        let s = filter_days(&filter_hours(trades, &best_hours), &best_days);
        println!("\n  → Hours + Days combined"); print_table_header(); print_row("filtered", &slice_metrics(&s));
    }

    (best_hours, best_days)
}

fn print_time_analysis(name: &str, trades: &[Trade]) -> (Vec<u32>, Vec<u32>) {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  TIME ANALYSIS — {:^48} ║", name);
    println!("╚══════════════════════════════════════════════════════════════════╝");
    print_hour_breakdown(trades);
    print_dow_breakdown(trades);
    print_month_breakdown(trades);
    print_best_combo(trades)
}

// ── Parameter sweep ────────────────────────────────────────────────────────────
//
// Explanation of the stop-placement problem
// -----------------------------------------
// The default stop is  candle_low − 1×ATR.
// On a breakout close, (close − low) ≈ 0.5–1.5 × ATR, so
//   risk = (close − low) + ATR  ≈ 1.5–2.5 × ATR.
// TP1 = close + 1.5×ATR, which in R terms ≈ 1.5 / 1.5–2.5 ≈ 0.6–1.0 R.
// With blending (50% TP1 + 50% trail from entry), the avg winner lands at ~0.55 R.
// A 1 R loss cancels ~1.8 wins at 0.55 R → even at 65% win rate expectancy is negative.
//
// Fix: EntryBased stop = close − 1×ATR  →  risk = exactly 1×ATR.
// TP1 = close + 1.5×ATR  = exactly 1.5 R.  Now ~1.1 wins needed per loss.

struct SweepConfig {
    label: &'static str,
    stop_mode: StopMode,
    tp1_atr_mult: Option<Decimal>,   // None = use default (1.5)
    trail_atr_mult: Option<Decimal>, // None = use default (1.0)
    time_window: Option<MomentumTimeWindow>,
}

fn run_sweep(asset: &AssetData, cfgs: &[SweepConfig]) {
    println!("\n  {}", asset.name);
    println!("  {:32} {:>6}  {:>5}     {:>4}   {:>8}  {:>9}  {:>7}  {:>8}",
        "Config", "Trades", "Win%", "PF", "Exp(R)", "Bal($)", "MaxDD%", "AvgWRR");
    println!("  {}", "─".repeat(90));

    let data_15m   = CandleStickLoader::load_binance(asset.data_15m_json);
    let volume_15m = CandleStickLoader::load_binance_volumes(asset.data_15m_json);
    let data_1h    = CandleStickLoader::load_binance(asset.data_1h_json);

    for sc in cfgs {
        let config = MomentumRiderConfig {
            stop_mode: sc.stop_mode,
            tp1_atr_mult:   sc.tp1_atr_mult.unwrap_or(Decimal::from_f32(1.5).unwrap()),
            trail_atr_mult: sc.trail_atr_mult.unwrap_or(Decimal::ONE),
            time_window: sc.time_window.clone(),
            ..MomentumRiderConfig::default()
        };
        let result = MomentumRider {
            data_15m: data_15m.clone(),
            volume_15m: volume_15m.clone(),
            data_1h: data_1h.clone(),
            config,
        }.execute();
        let m = slice_metrics(&result.trades);
        let profit_icon = if m.balance > Decimal::from(1000) { "+" } else { " " };
        println!("  {:32} {:>6}  {:>5}  {:>6}  {:>8}  {:>9}  {:>7}  {:>8}  {}",
            sc.label, m.count,
            format!("{:.1}%", m.win_rate),
            m.profit_factor, m.expectancy_r,
            format!("${}", m.balance),
            format!("{:.2}%", m.max_dd),
            m.avg_winner_rr,
            profit_icon,
        );
    }
}

// ── Strategy runner ────────────────────────────────────────────────────────────

fn run_asset(asset: &AssetData) -> BacktestResult {
    let data_15m   = CandleStickLoader::load_binance(asset.data_15m_json);
    let volume_15m = CandleStickLoader::load_binance_volumes(asset.data_15m_json);
    let data_1h    = CandleStickLoader::load_binance(asset.data_1h_json);
    MomentumRider { data_15m, volume_15m, data_1h, config: MomentumRiderConfig::default() }.execute()
}

// ── Main ───────────────────────────────────────────────────────────────────────

fn main() {
    let assets = [
        AssetData {
            name: "BTC/USDT",
            data_15m_json: include_str!("../assets/binance_BTCUSDT_15m.json"),
            data_1h_json:  include_str!("../assets/binance_BTCUSDT_1h.json"),
        },
        AssetData {
            name: "ETH/USDT",
            data_15m_json: include_str!("../assets/binance_ETHUSDT_15m.json"),
            data_1h_json:  include_str!("../assets/binance_ETHUSDT_1h.json"),
        },
        AssetData {
            name: "SOL/USDT",
            data_15m_json: include_str!("../assets/binance_SOLUSDT_15m.json"),
            data_1h_json:  include_str!("../assets/binance_SOLUSDT_1h.json"),
        },
    ];

    println!("\n╔═══════════════════════════════════════════════════╗");
    println!("║   Crypto Momentum Rider — Backtest Analysis      ║");
    println!("╚═══════════════════════════════════════════════════╝");
    println!("  Note: Macro event & BTC dominance filters not applied.\n");

    // ── 1: Full-dataset summary ────────────────────────────────────────────────
    println!("═══════════════════════════════════════════════════════════════════");
    println!("  SECTION 1 — FULL DATASET SUMMARY (default config)");
    println!("═══════════════════════════════════════════════════════════════════");
    let results: Vec<(&AssetData, BacktestResult)> = assets.iter()
        .map(|a| (a, run_asset(a)))
        .collect();
    for (asset, result) in &results { print_summary(asset.name, result); }

    // ── 2: Time analysis (derive best hours/days per asset) ───────────────────
    println!("\n\n═══════════════════════════════════════════════════════════════════");
    println!("  SECTION 2 — TIME FILTERING ANALYSIS");
    println!("  (Post-hoc slicing of default-config trades)");
    println!("═══════════════════════════════════════════════════════════════════");

    // Collect per-asset best windows for use in sweep
    let mut best_windows: Vec<(Vec<u32>, Vec<u32>)> = Vec::new();
    for (asset, result) in &results {
        let combo = print_time_analysis(asset.name, &result.trades);
        best_windows.push(combo);
    }

    // ── 3: Parameter sweep ────────────────────────────────────────────────────
    //
    // Why avg winner RR ≈ 0.55 (the root cause)
    // ------------------------------------------
    // stop = candle_low − ATR  →  risk = (close − low) + ATR ≈ 2× ATR.
    // TP1 = close + 1.5×ATR   →  TP1 in R ≈ 0.75 R (not 1.5 R!).
    // Blended (50% TP1 + 50% trail at ~entry) ≈ 0.55 R per winner.
    // Even at 65% win rate:  0.65×0.55 − 0.35×1 = −0.008 R/trade  →  slight loss.
    //
    // Fix: EntryBased stop = close − ATR  →  risk = 1×ATR exactly.
    // TP1 = 1.5×ATR = 1.5 R.  Blended ≈ 0.75–1.1 R.  Now positive.
    println!("\n\n═══════════════════════════════════════════════════════════════════");
    println!("  SECTION 3 — PARAMETER SWEEP");
    println!("  Root cause: stop at candle_low−ATR makes risk ≈ 2×ATR,");
    println!("  so TP1 (1.5×ATR from entry) only reaches ~0.75 R, not 1.5 R.");
    println!("  Two fixes tested:");
    println!("    EntryBased stop  → risk = 1×ATR, TP1 = 1.5 R (tighter stop, fewer wins)");
    println!("    Calibrated TP1   → keep candle-low stop, raise TP1 to 3×ATR for ~1.5 R");
    println!("  '+' = profitable (balance > $1000 starting capital)");
    println!("═══════════════════════════════════════════════════════════════════");

    for (i, (asset, _)) in results.iter().enumerate() {
        let (ref best_hours, ref best_days) = best_windows[i];

        let t3 = Decimal::from(3);
        let t2 = Decimal::from(2);
        let sweep_cfgs: Vec<SweepConfig> = vec![
            SweepConfig {
                label: "A: Default (candle-low, TP1=1.5×ATR)",
                stop_mode: StopMode::CandleLow,
                tp1_atr_mult: None, trail_atr_mult: None,
                time_window: None,
            },
            SweepConfig {
                label: "B: Entry-based stop",
                stop_mode: StopMode::EntryBased,
                tp1_atr_mult: None, trail_atr_mult: None,
                time_window: None,
            },
            // Candle-low stop but TP1 calibrated to match actual R.
            // risk ≈ (close−low) + ATR ≈ 2×ATR on breakout candles,
            // so TP1=3×ATR targets genuine ~1.5 R.
            SweepConfig {
                label: "C: Candle-low, TP1=3×ATR (calibrated)",
                stop_mode: StopMode::CandleLow,
                tp1_atr_mult: Some(t3), trail_atr_mult: None,
                time_window: None,
            },
            SweepConfig {
                label: "D: Candle-low, TP1=3×ATR + time win",
                stop_mode: StopMode::CandleLow,
                tp1_atr_mult: Some(t3), trail_atr_mult: None,
                time_window: Some(MomentumTimeWindow::new(
                    best_hours.clone(), best_days.clone(),
                )),
            },
            // Entry-based stop with a looser trail (2×ATR) lets winners run further.
            SweepConfig {
                label: "E: Entry-stop, trail=2×ATR + time win",
                stop_mode: StopMode::EntryBased,
                tp1_atr_mult: None, trail_atr_mult: Some(t2),
                time_window: Some(MomentumTimeWindow::new(
                    best_hours.clone(), best_days.clone(),
                )),
            },
        ];
        run_sweep(asset, &sweep_cfgs);
    }

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("  Analysis complete.");
    println!("═══════════════════════════════════════════════════════════════════\n");
}
