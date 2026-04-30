extern crate rust_decimal;

use chrono::Datelike;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    strategies::ict::{summarize, ExitReason, IctConfig, IctStrategy, IctTrade, PdArrayFilter, TradeDir},
    to_new_york_time,
};

// ── Asset descriptor ───────────────────────────────────────────────────────────

struct AssetData {
    name: &'static str,
    data_5m_json: &'static str,
    data_15m_json: &'static str,
    data_1h_json: &'static str,
    data_4h_json: &'static str,
}

// ── Segment breakdown helpers ─────────────────────────────────────────────────

fn print_segment_breakdown<F, K>(
    header: &str,
    trades: &[IctTrade],
    key_fn: F,
    label_fn: impl Fn(K) -> String,
    starting_capital: Decimal,
) where
    F: Fn(&IctTrade) -> K,
    K: Eq + std::hash::Hash + Clone + Ord,
{
    let mut groups: std::collections::HashMap<K, Vec<&IctTrade>> = std::collections::HashMap::new();
    for t in trades {
        groups.entry(key_fn(t)).or_default().push(t);
    }
    let mut keys: Vec<K> = groups.keys().cloned().collect();
    keys.sort();

    if keys.is_empty() {
        return;
    }

    println!("\n  {}", header);
    println!("  {:16}  {:>6}  {:>6}  {:>6}  {:>6}  {:>8}  {:>7}",
        "", "Trades", "Win%", "AvgR", "PF", "PnL$", "MaxDD%");
    println!("  {}", "─".repeat(72));

    for k in &keys {
        let group: Vec<&IctTrade> = groups[k].clone();
        let n = group.len();
        if n == 0 { continue; }
        let wins: usize = group.iter().filter(|t| t.r_multiple > Decimal::ZERO).count();
        let _losses: usize = group.iter().filter(|t| t.r_multiple < Decimal::ZERO).count();
        let win_rate = Decimal::from(wins as u32) / Decimal::from(n as u32) * Decimal::from(100);
        let total_r: Decimal = group.iter().map(|t| t.r_multiple).sum();
        let avg_r = total_r / Decimal::from(n as u32);
        let gross_profit: Decimal = group.iter().filter(|t| t.pnl_usd > Decimal::ZERO).map(|t| t.pnl_usd).sum();
        let gross_loss: Decimal = group.iter().filter(|t| t.pnl_usd < Decimal::ZERO).map(|t| t.pnl_usd.abs()).sum();
        let pf = if gross_loss == Decimal::ZERO { Decimal::from(9999) } else { gross_profit / gross_loss };
        let total_pnl: Decimal = group.iter().map(|t| t.pnl_usd).sum();

        // Max DD within group using equity snapshots
        let mut peak = starting_capital;
        let mut max_dd = Decimal::ZERO;
        let mut running = starting_capital;
        for t in &group {
            running += t.pnl_usd;
            if running > peak { peak = running; }
            if peak > Decimal::ZERO {
                let dd = (peak - running) / peak * Decimal::from(100);
                if dd > max_dd { max_dd = dd; }
            }
        }

        println!("  {:16}  {:>6}  {:>5.1}%  {:>6.3}  {:>6.2}  {:>+8.0}  {:>6.2}%",
            label_fn(k.clone()), n,
            win_rate.to_f64_approx(),
            avg_r.to_f64_approx(),
            pf.to_f64_approx(),
            total_pnl.to_f64_approx(),
            max_dd.to_f64_approx(),
        );
    }
}

trait ToF64Approx {
    fn to_f64_approx(&self) -> f64;
}
impl ToF64Approx for Decimal {
    fn to_f64_approx(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.to_f64().unwrap_or(0.0)
    }
}

// ── Per-asset runner ──────────────────────────────────────────────────────────

fn run_asset(asset: &AssetData, cfg: IctConfig) {
    let candles_5m  = CandleStickLoader::load_binance(asset.data_5m_json);
    let candles_15m = CandleStickLoader::load_binance(asset.data_15m_json);
    let candles_1h  = CandleStickLoader::load_binance(asset.data_1h_json);
    let candles_4h  = CandleStickLoader::load_binance(asset.data_4h_json);

    let strategy = IctStrategy { candles_5m, candles_15m, candles_1h, candles_4h, config: cfg.clone() };
    let trades = strategy.run();
    let summary = summarize(&trades, cfg.starting_capital);

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ICT 2022 — {:^49} ║", asset.name);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!("  Total trades   : {}", summary.total_trades);
    println!("  Win / Loss     : {} / {}", summary.wins, summary.losses);
    println!("  Win rate       : {:.1}%", summary.win_rate.to_f64_approx());
    println!("  Avg R          : {:.3}", summary.avg_r.to_f64_approx());
    println!("  Profit factor  : {:.2}", summary.profit_factor.to_f64_approx());
    println!("  Max drawdown   : {:.2}%", summary.max_drawdown_pct.to_f64_approx());
    println!("  Final equity   : ${:.2}", summary.final_equity.to_f64_approx());
    println!("  Total P&L      : ${:+.2}", summary.total_pnl.to_f64_approx());

    if trades.is_empty() { return; }

    // ── Segment breakdowns ────────────────────────────────────────────────────

    print_segment_breakdown(
        "BY DIRECTION",
        &trades,
        |t| match t.direction { TradeDir::Bullish => "Bullish", TradeDir::Bearish => "Bearish" },
        |k: &str| k.to_string(),
        cfg.starting_capital,
    );

    print_segment_breakdown(
        "BY KILL ZONE",
        &trades,
        |t| t.kill_zone,
        |k: &str| k.to_string(),
        cfg.starting_capital,
    );

    print_segment_breakdown(
        "BY LEVEL TYPE",
        &trades,
        |t| t.level_type,
        |k: &str| k.to_string(),
        cfg.starting_capital,
    );

    print_segment_breakdown(
        "BY PD ARRAY",
        &trades,
        |t| t.pd_array_type,
        |k: &str| k.to_string(),
        cfg.starting_capital,
    );

    print_segment_breakdown(
        "BY EXIT REASON",
        &trades,
        |t| match t.exit_reason {
            ExitReason::Sl => "SL",
            ExitReason::Tp1Only => "TP1+SL",
            ExitReason::Tp2 => "TP2",
            ExitReason::Timeout => "Timeout",
        },
        |k: &str| k.to_string(),
        cfg.starting_capital,
    );

    // Monthly breakdown
    print_segment_breakdown(
        "BY MONTH (year-month)",
        &trades,
        |t| {
            let dt = to_new_york_time(t.entry_ts);
            (dt.year(), dt.month())
        },
        |(y, m)| format!("{y}-{m:02}"),
        cfg.starting_capital,
    );

    // Equity curve (yearly)
    print_segment_breakdown(
        "BY YEAR",
        &trades,
        |t| to_new_york_time(t.entry_ts).year(),
        |y: i32| y.to_string(),
        cfg.starting_capital,
    );

    println!();
}

// ── Config sweep ──────────────────────────────────────────────────────────────

fn run_config_sweep(asset: &AssetData) {
    println!("\n  CONFIG SWEEP — {}", asset.name);
    println!("  {:42}  {:>6}  {:>5}  {:>6}  {:>6}  {:>8}",
        "Config", "Trades", "Win%", "AvgR", "PF", "PnL$");
    println!("  {}", "─".repeat(85));

    let base = IctConfig::default();

    struct Variant {
        label: &'static str,
        bias: bool,
        kz: bool,
        mss: bool,
        ote: bool,
        pd: PdArrayFilter,
    }

    let variants = vec![
        Variant { label: "All filters ON (default)",       bias: true,  kz: true,  mss: true,  ote: true,  pd: PdArrayFilter::Both },
        Variant { label: "No bias filter",                  bias: false, kz: true,  mss: true,  ote: true,  pd: PdArrayFilter::Both },
        Variant { label: "No kill zone filter",             bias: true,  kz: false, mss: true,  ote: true,  pd: PdArrayFilter::Both },
        Variant { label: "No MSS filter",                   bias: true,  kz: true,  mss: false, ote: true,  pd: PdArrayFilter::Both },
        Variant { label: "No OTE filter",                   bias: true,  kz: true,  mss: true,  ote: false, pd: PdArrayFilter::Both },
        Variant { label: "FVG only",                        bias: true,  kz: true,  mss: true,  ote: true,  pd: PdArrayFilter::FvgOnly },
        Variant { label: "OB only",                         bias: true,  kz: true,  mss: true,  ote: true,  pd: PdArrayFilter::ObOnly },
        Variant { label: "All filters OFF (max trades)",    bias: false, kz: false, mss: false, ote: false, pd: PdArrayFilter::Both },
    ];

    let candles_5m  = CandleStickLoader::load_binance(asset.data_5m_json);
    let candles_15m = CandleStickLoader::load_binance(asset.data_15m_json);
    let candles_1h  = CandleStickLoader::load_binance(asset.data_1h_json);
    let candles_4h  = CandleStickLoader::load_binance(asset.data_4h_json);

    for v in &variants {
        let cfg = IctConfig {
            use_bias_filter: v.bias,
            use_kill_zone_filter: v.kz,
            use_mss_filter: v.mss,
            use_ote_filter: v.ote,
            pd_array_filter: v.pd,
            ..base.clone()
        };
        let strategy = IctStrategy {
            candles_5m: candles_5m.clone(),
            candles_15m: candles_15m.clone(),
            candles_1h: candles_1h.clone(),
            candles_4h: candles_4h.clone(),
            config: cfg.clone(),
        };
        let trades = strategy.run();
        let s = summarize(&trades, cfg.starting_capital);
        let _pnl_sign = if s.total_pnl >= Decimal::ZERO { "+" } else { "" };
        println!("  {:42}  {:>6}  {:>4.1}%  {:>6.3}  {:>6.2}  {:>+8.0}",
            v.label,
            s.total_trades,
            s.win_rate.to_f64_approx(),
            s.avg_r.to_f64_approx(),
            s.profit_factor.to_f64_approx(),
            s.total_pnl.to_f64_approx(),
        );
    }
}

// ── Main ───────────────────────────────────────────────────────────────────────

fn main() {
    let btc = AssetData {
        name: "BTC/USDT",
        data_5m_json:  include_str!("../assets/binance_BTCUSDT_5m.json"),
        data_15m_json: include_str!("../assets/binance_BTCUSDT_15m.json"),
        data_1h_json:  include_str!("../assets/binance_BTCUSDT_1h.json"),
        data_4h_json:  include_str!("../assets/binance_BTCUSDT_4h.json"),
    };

    println!("\n╔═════════════════════════════════════════════════════════════╗");
    println!("║   ICT 2022 Mentorship Strategy — Backtest                  ║");
    println!("╚═════════════════════════════════════════════════════════════╝");

    // ── Python-equivalent config (run_ict_tests.py) ───────────────────────────
    // cfg = {**CONFIG, use_bias_filter: False, use_ote_filter: False,
    //         pd_array_filter: 'ob_only', entry_fill_bars: 24}
    // Expected: ~397 trades, 35.3% WR, 1.40 PF, $38,499 equity
    println!("\n  [Python-equivalent config: bias=off, OTE=off, OB-only, fill=24]");
    let py_cfg = IctConfig {
        use_bias_filter: false,
        use_ote_filter: false,
        pd_array_filter: PdArrayFilter::ObOnly,
        entry_fill_bars: 24,
        debug: true,
        ..IctConfig::default()
    };
    run_asset(&btc, py_cfg);

    // ── Default config ────────────────────────────────────────────────────────
    println!("\n  [Default config: all filters ON, OB+FVG, fill=96]");
    run_asset(&btc, IctConfig::default());

    // Config sweep
    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("  CONFIG SWEEP — filter impact analysis");
    println!("═══════════════════════════════════════════════════════════════════");
    run_config_sweep(&btc);

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("  Analysis complete.");
    println!("═══════════════════════════════════════════════════════════════════\n");
}
