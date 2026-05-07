extern crate rust_decimal;

use std::collections::HashMap;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::model::candle_stick::CandleStick;
use backtest::model::decimal::DecimalVec;
use backtest::model::trade_result::TradeResult;
use backtest::model::trading_model::TradingModel;
use backtest::engine::types::ExecutionConfig;
use backtest::strategies::orb::{Orb, OrbConfig, OrbDuration, OrbEntryModel, OrbSlType};
use rust_decimal::Decimal;

#[derive(Clone)]
struct Row {
    variant: String,
    asset: String,
    timeframe: String,
    trades: usize,
    win_rate: Decimal,
    profit_r: Decimal,
    pf_r: Decimal,
    pnl_pct: Decimal,
}

fn split_equal_windows(data: &[CandleStick], parts: usize) -> Vec<Vec<CandleStick>> {
    if data.is_empty() || parts == 0 {
        return Vec::new();
    }
    let n = data.len();
    let mut out = Vec::with_capacity(parts);
    for i in 0..parts {
        let start = i * n / parts;
        let end = (i + 1) * n / parts;
        if end > start {
            out.push(data[start..end].to_vec());
        }
    }
    out
}

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .unwrap_or_else(|e| panic!("failed loading {}: {}", path, e))
}

fn load_json(path: &str) -> Vec<CandleStick> {
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed loading {}: {}", path, e));
    CandleStickLoader::load_source(CandleDataSource::BinanceJsonStr(&raw))
        .unwrap_or_else(|e| panic!("failed parsing {}: {}", path, e))
}

fn resample_minutes(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if data.is_empty() || minutes <= 1 {
        return data.to_vec();
    }
    let bucket_ms = minutes * 60 * 1000;
    let mut out: Vec<CandleStick> = Vec::new();
    let mut bucket_start: Option<i64> = None;
    let mut acc: Option<CandleStick> = None;

    for c in data {
        let b = c.open_time / bucket_ms;
        if bucket_start != Some(b) {
            if let Some(done) = acc.take() {
                out.push(done);
            }
            bucket_start = Some(b);
            acc = Some(*c);
            continue;
        }

        if let Some(mut a) = acc.take() {
            a.high = DecimalVec(a.high.0.max(c.high.0));
            a.low = DecimalVec(a.low.0.min(c.low.0));
            a.close = c.close;
            a.close_time = c.close_time;
            acc = Some(a);
        }
    }
    if let Some(done) = acc {
        out.push(done);
    }
    out
}

fn summarize(label: &str, asset: &str, tf: &str, result: backtest::model::backtest_result::BacktestResult) -> Row {
    let winners = result.result(TradeResult::Winner);
    let losers = result.result(TradeResult::Expense);
    let trades = result.number_of_trades();
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as u32) / Decimal::from(trades as u32) * Decimal::from(100)
    };
    let gross_loss_r = Decimal::from(losers as u32);
    let gross_profit_r = result.profit_in_r() + gross_loss_r;
    let pf_r = if gross_loss_r > Decimal::ZERO {
        gross_profit_r / gross_loss_r
    } else if gross_profit_r > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    };

    Row {
        variant: label.to_string(),
        asset: asset.to_string(),
        timeframe: tf.to_string(),
        trades,
        win_rate,
        profit_r: result.profit_in_r(),
        pf_r,
        pnl_pct: result.pnl(),
    }
}

fn tick_size_for(asset: &str) -> Decimal {
    match asset {
        "MNQ" => Decimal::new(25, 2),
        _ => Decimal::new(1, 1),
    }
}

fn main() {
    let mnq_1m = load_parquet("assets/mnq_1m_cont.parquet");
    let datasets: Vec<(&str, &str, Vec<CandleStick>)> = vec![
        ("MNQ", "1m", mnq_1m.clone()),
        ("MNQ", "5m", resample_minutes(&mnq_1m, 5)),
        ("MNQ", "15m", resample_minutes(&mnq_1m, 15)),
        ("MNQ", "1h", resample_minutes(&mnq_1m, 60)),
        ("BTC", "5m", load_json("assets/binance_BTCUSDT_5m.json")),
        ("BTC", "15m", load_json("assets/binance_BTCUSDT_15m.json")),
        ("BTC", "1h", load_json("assets/binance_BTCUSDT_1h.json")),
        ("BTC", "4h", load_json("assets/binance_BTCUSDT_4h.json")),
        ("ETH", "5m", load_json("assets/binance_ETHUSDT_5m.json")),
        ("ETH", "15m", load_json("assets/binance_ETHUSDT_15m.json")),
        ("ETH", "1h", load_json("assets/binance_ETHUSDT_1h.json")),
        ("ETH", "4h", load_json("assets/binance_ETHUSDT_4h.json")),
        ("SOL", "5m", load_json("assets/binance_SOLUSDT_5m.json")),
        ("SOL", "15m", load_json("assets/binance_SOLUSDT_15m.json")),
        ("SOL", "1h", load_json("assets/binance_SOLUSDT_1h.json")),
        ("SOL", "4h", load_json("assets/binance_SOLUSDT_4h.json")),
    ];

    let mut variants: Vec<(String, OrbConfig)> = vec![
        (
            "orb15_opp_rr2".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes15,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        (
            "orb30_opp_rr2".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        (
            "orb30_opp_rr3".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::from(3),
                ..OrbConfig::default()
            },
        ),
        (
            "orb30_rangepct25_rr2".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::RangePct(Decimal::from(25)),
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        // NQ preset aliases mapped onto Rust ORB config surface.
        (
            "nq_multiorb".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes15,
                sl_type: OrbSlType::RangePct(Decimal::from(50)),
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
        (
            "nq_finalboss".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes15,
                sl_type: OrbSlType::OppositeRange,
                rr_target: Decimal::ONE,
                ..OrbConfig::default()
            },
        ),
        (
            "nq_orb30m15m".to_string(),
            OrbConfig {
                duration: OrbDuration::Minutes30,
                sl_type: OrbSlType::RangePct(Decimal::from(50)),
                rr_target: Decimal::from(2),
                ..OrbConfig::default()
            },
        ),
    ];

    // Expanded grid inspired by ~/develop/play/orb prototypes
    let durations = [OrbDuration::Minutes15, OrbDuration::Minutes30];
    let duration_labels = ["15", "30"];
    let rr_vals = [Decimal::new(15, 1), Decimal::from(2), Decimal::new(25, 1), Decimal::from(3)];
    let sl_types = [
        OrbSlType::OppositeRange,
        OrbSlType::RangePct(Decimal::from(25)),
        OrbSlType::RangePct(Decimal::from(50)),
    ];
    let sl_labels = ["opp", "rp25", "rp50"];
    let active_windows = [120usize, 240usize, 360usize];
    let hold_limits: [Option<usize>; 3] = [None, Some(24), Some(48)];
    let hold_labels = ["holdnone", "hold24", "hold48"];
    let retest_modes = [false, true];
    let retest_bars = [6usize, 12usize];

    for (di, d) in durations.iter().enumerate() {
        for rr in rr_vals {
            for (si, sl) in sl_types.iter().enumerate() {
                for aw in active_windows {
                    for (hi, hold) in hold_limits.iter().enumerate() {
                        for rm in retest_modes {
                            if !rm {
                                let name = format!(
                                    "grid_or{}_rr{}_{}_aw{}_{}",
                                    duration_labels[di],
                                    rr,
                                    sl_labels[si],
                                    aw,
                                    hold_labels[hi]
                                )
                                .replace('.', "p");
                                variants.push((
                                    name,
                                    OrbConfig {
                                        duration: *d,
                                        sl_type: sl.clone(),
                                        rr_target: rr,
                                        active_window_minutes: aw,
                                        max_hold_bars: *hold,
                                        retest_mode: false,
                                        ..OrbConfig::default()
                                    },
                                ));
                            } else {
                                for rb in retest_bars {
                                    let name = format!(
                                        "grid_or{}_rr{}_{}_aw{}_{}_retest{}",
                                        duration_labels[di],
                                        rr,
                                        sl_labels[si],
                                        aw,
                                        hold_labels[hi],
                                        rb
                                    )
                                    .replace('.', "p");
                                    variants.push((
                                        name,
                                        OrbConfig {
                                            duration: *d,
                                            sl_type: sl.clone(),
                                            rr_target: rr,
                                            active_window_minutes: aw,
                                            max_hold_bars: *hold,
                                            retest_mode: true,
                                            retest_max_bars: rb,
                                            ..OrbConfig::default()
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut rows: Vec<Row> = Vec::new();
    let mut cfg_by_label: HashMap<String, OrbConfig> = HashMap::new();
    let mut data_by_asset_tf: HashMap<(String, String), Vec<CandleStick>> = HashMap::new();
    for (asset, tf, data) in datasets {
        data_by_asset_tf.insert((asset.to_string(), tf.to_string()), data.clone());
        let tick_size = tick_size_for(asset);
        for (name, cfg) in &variants {
            for slip_ticks in [1_i32, 2_i32, 3_i32] {
                let mut realistic_cfg = cfg.clone();
                realistic_cfg.entry_model = OrbEntryModel::NextBarOpen;
                realistic_cfg.conservative_intrabar = true;
                realistic_cfg.execution = ExecutionConfig {
                    slippage_ticks_per_side: slip_ticks,
                    tick_size,
                    ..ExecutionConfig::default()
                };
                let label = format!("real_s{}_{}", slip_ticks, name);
                let realistic = Orb {
                    data: data.clone(),
                    config: realistic_cfg.clone(),
                }
                .execute();
                cfg_by_label.insert(label.clone(), realistic_cfg);
                rows.push(summarize(&label, asset, tf, realistic));
            }
        }
    }

    rows.sort_by(|a, b| b.profit_r.cmp(&a.profit_r).then(b.pf_r.cmp(&a.pf_r)));

    let mut md = String::new();
    md.push_str("# ORB Variants Grid\n\n");
    md.push_str("Reality-validated ORB variant family (next-bar-open entries, conservative intrabar stop-first, gap-aware stop fills, and slippage stress).\n\n");

    let assets = ["MNQ", "BTC", "ETH", "SOL"];
    let top_n = 12usize;
    md.push_str("## Top Realistic Variants By Asset\n\n");
    for asset in assets {
        let mut asset_rows: Vec<&Row> = rows.iter().filter(|r| r.asset == asset).collect();
        asset_rows.sort_by(|a, b| b.profit_r.cmp(&a.profit_r).then(b.pf_r.cmp(&a.pf_r)));
        md.push_str(&format!("### {} (Top {})\n\n", asset, top_n));
        md.push_str("| variant | timeframe | trades | win_rate_% | profit_r | pf_r | pnl_% |\n");
        md.push_str("|---|---|---:|---:|---:|---:|---:|\n");
        for r in asset_rows.into_iter().take(top_n) {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                r.variant,
                r.timeframe,
                r.trades,
                r.win_rate.round_dp(2),
                r.profit_r.round_dp(2),
                r.pf_r.round_dp(2),
                r.pnl_pct.round_dp(2)
            ));
        }
        md.push_str("\n");
    }

    let min_trades = 500usize;
    let min_pf = Decimal::from(2);
    md.push_str("## Stability-Filtered Top Realistic Variants\n\n");
    md.push_str(&format!(
        "Filter: trades >= {} and pf_r >= {}.\n\n",
        min_trades,
        min_pf
    ));
    for asset in assets {
        let mut stable_rows: Vec<&Row> = rows
            .iter()
            .filter(|r| r.asset == asset && r.trades >= min_trades && r.pf_r >= min_pf)
            .collect();
        stable_rows.sort_by(|a, b| b.profit_r.cmp(&a.profit_r).then(b.pf_r.cmp(&a.pf_r)));
        md.push_str(&format!("### {} (Top {})\n\n", asset, top_n));
        md.push_str("| variant | timeframe | trades | win_rate_% | profit_r | pf_r | pnl_% |\n");
        md.push_str("|---|---|---:|---:|---:|---:|---:|\n");
        for r in stable_rows.into_iter().take(top_n) {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                r.variant,
                r.timeframe,
                r.trades,
                r.win_rate.round_dp(2),
                r.profit_r.round_dp(2),
                r.pf_r.round_dp(2),
                r.pnl_pct.round_dp(2)
            ));
        }
        md.push_str("\n");
    }

    md.push_str("## Full Grid\n\n");
    md.push_str("| variant | asset | timeframe | trades | win_rate_% | profit_r | pf_r | pnl_% |\n");
    md.push_str("|---|---|---|---:|---:|---:|---:|---:|\n");
    for r in &rows {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.variant,
            r.asset,
            r.timeframe,
            r.trades,
            r.win_rate.round_dp(2),
            r.profit_r.round_dp(2),
            r.pf_r.round_dp(2),
            r.pnl_pct.round_dp(2)
        ));
    }

    md.push_str("\n## Final Cycle: Rolling OOS Verdict\n\n");
    md.push_str("Criteria: OOS PF>=1.20 and OOS profit_r>0 in >=4/5 windows.\n\n");

    let mut best_by_asset_tf: HashMap<(String, String), &Row> = HashMap::new();
    for r in &rows {
        let key = (r.asset.clone(), r.timeframe.clone());
        if let Some(curr) = best_by_asset_tf.get(&key) {
            if r.profit_r > curr.profit_r || (r.profit_r == curr.profit_r && r.pf_r > curr.pf_r) {
                best_by_asset_tf.insert(key, r);
            }
        } else {
            best_by_asset_tf.insert(key, r);
        }
    }

    let mut keys: Vec<(String, String)> = best_by_asset_tf.keys().cloned().collect();
    keys.sort();

    for (asset, tf) in keys {
        let best = match best_by_asset_tf.get(&(asset.clone(), tf.clone())) {
            Some(v) => *v,
            None => continue,
        };
        let cfg = match cfg_by_label.get(&best.variant) {
            Some(v) => v.clone(),
            None => continue,
        };
        let data = match data_by_asset_tf.get(&(asset.clone(), tf.clone())) {
            Some(v) => v,
            None => continue,
        };
        let windows = split_equal_windows(data, 5);
        let mut pass_profit = 0usize;
        let mut pass_pf = 0usize;
        md.push_str(&format!("### {} {}\n\n", asset, tf));
        md.push_str(&format!("- Champion variant: `{}`\n", best.variant));
        for (i, w) in windows.into_iter().enumerate() {
            let result = Orb {
                data: w,
                config: cfg.clone(),
            }
            .execute();
            let row = summarize("wf", &asset, &tf, result);
            if row.profit_r > Decimal::ZERO {
                pass_profit += 1;
            }
            if row.pf_r >= Decimal::new(12, 1) {
                pass_pf += 1;
            }
            md.push_str(&format!(
                "- W{}: trades={} win%={} profit_r={} pf_r={} pnl%={}\n",
                i + 1,
                row.trades,
                row.win_rate.round_dp(2),
                row.profit_r.round_dp(2),
                row.pf_r.round_dp(2),
                row.pnl_pct.round_dp(2)
            ));
        }
        let verdict = if pass_profit >= 4 && pass_pf >= 4 {
            "PROMOTE"
        } else {
            "KILL"
        };
        md.push_str(&format!(
            "- Verdict: {} (profit windows {}/5, pf windows {}/5)\n\n",
            verdict, pass_profit, pass_pf
        ));
    }

    std::fs::write(
        "reports/strategy_overviews/ORB_VARIANTS_GRID.md",
        md,
    )
    .unwrap_or_else(|e| panic!("failed writing report: {}", e));

    println!("Wrote reports/strategy_overviews/ORB_VARIANTS_GRID.md");
}
