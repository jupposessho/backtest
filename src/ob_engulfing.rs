extern crate rust_decimal;

use backtest::candle_stick_loader::{CandleDataSource, CandleStickLoader};
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade_result::TradeResult;
use backtest::model::trading_model::TradingModel;
use backtest::strategies::mc::{
    EntryMode, ExecutionConfig, MaxSlMode, Mc, McConfig, McMode, SignalPattern,
    SignalQualityConfig, TimeWindow, TrailingStopConfig, TrailingStopMode, TrendFilter,
};
use chrono::NaiveTime;
use rust_decimal::Decimal;

#[derive(Clone)]
struct Variant {
    label: String,
    cfg: McConfig,
}

struct VariantResult {
    label: String,
    trades: usize,
    win_rate: Decimal,
    profit_r: Decimal,
    profit_factor_r: Decimal,
    pnl_pct: Decimal,
}

fn d(v: i64) -> Decimal {
    Decimal::from(v)
}

fn load_mnq_1m() -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
        .unwrap_or_else(|e| panic!("failed loading assets/mnq_1m_cont.parquet: {e}"))
}

fn load_binance_tf(symbol: &str, tf: &str) -> Vec<CandleStick> {
    let path = format!("assets/binance_{}USDT_{}.json", symbol, tf);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed loading {}: {}", path, e));
    CandleStickLoader::load_source(CandleDataSource::BinanceJsonStr(&raw))
        .unwrap_or_else(|e| panic!("failed parsing {}: {}", path, e))
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

fn slice_between(data: &[CandleStick], start_ts: i64, end_ts: i64) -> Vec<CandleStick> {
    data.iter()
        .copied()
        .filter(|c| c.open_time >= start_ts && c.open_time < end_ts)
        .collect()
}

fn print_stats(label: &str, result: &BacktestResult) {
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

    println!("\n=== {} ===", label);
    println!("trades      : {}", total);
    println!("winners     : {}", winners);
    println!("losers      : {}", losers);
    println!("break_evens : {}", break_evens);
    println!("win_rate%   : {}", win_rate.round_dp(2));
    println!("profit_r    : {}", result.profit_in_r().round_dp(2));
    println!("profit_factor_r: {}", profit_factor_r);
    println!("points      : {}", result.profit_in_points().round_dp(2));
    println!("costs_total : {}", result.costs_total().round_dp(2));
    println!("pnl%        : {}", result.pnl().round_dp(2));
}

fn summarize(label: &str, result: &BacktestResult) -> VariantResult {
    let winners = result.result(TradeResult::Winner);
    let losers = result.result(TradeResult::Expense);
    let total = result.number_of_trades();
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as u32) / Decimal::from(total as u32) * d(100)
    };

    let gross_loss_r = Decimal::from(losers as u32);
    let gross_profit_r = result.profit_in_r() + gross_loss_r;
    let profit_factor_r = if gross_loss_r > Decimal::ZERO {
        gross_profit_r / gross_loss_r
    } else if gross_profit_r > Decimal::ZERO {
        Decimal::from(9999)
    } else {
        Decimal::ZERO
    };

    VariantResult {
        label: label.to_string(),
        trades: total,
        win_rate,
        profit_r: result.profit_in_r(),
        profit_factor_r,
        pnl_pct: result.pnl(),
    }
}

fn run_config(data: Vec<CandleStick>, cfg: McConfig, label: &str) -> VariantResult {
    let result = Mc { data, config: cfg }.execute();
    summarize(label, &result)
}

fn sorted_top(mut rows: Vec<VariantResult>, n: usize) -> Vec<VariantResult> {
    rows.sort_by(|a, b| {
        b.profit_r
            .cmp(&a.profit_r)
            .then(b.profit_factor_r.cmp(&a.profit_factor_r))
            .then(b.trades.cmp(&a.trades))
    });
    rows.into_iter().take(n).collect()
}

fn split_equal_windows(data: &[CandleStick], windows: usize) -> Vec<Vec<CandleStick>> {
    if windows == 0 || data.is_empty() {
        return vec![];
    }
    let chunk = data.len() / windows;
    if chunk == 0 {
        return vec![data.to_vec()];
    }
    let mut out = Vec::new();
    for i in 0..windows {
        let start = i * chunk;
        let end = if i == windows - 1 { data.len() } else { (i + 1) * chunk };
        out.push(data[start..end].to_vec());
    }
    out
}

fn tw(start_h: u32, start_m: u32, end_h: u32, end_m: u32) -> TimeWindow {
    TimeWindow {
        start: NaiveTime::from_hms_opt(start_h, start_m, 0).unwrap(),
        end: NaiveTime::from_hms_opt(end_h, end_m, 0).unwrap(),
    }
}

fn session_variants(base: &McConfig) -> Vec<Variant> {
    vec![
        Variant {
            label: "session PM 06:00-09:15".to_string(),
            cfg: McConfig {
                trade_window: Some(tw(6, 0, 9, 15)),
                ..base.clone()
            },
        },
        Variant {
            label: "session NYAM 09:15-12:15".to_string(),
            cfg: McConfig {
                trade_window: Some(tw(9, 15, 12, 15)),
                ..base.clone()
            },
        },
        Variant {
            label: "session NYPM 12:15-15:30".to_string(),
            cfg: McConfig {
                trade_window: Some(tw(12, 15, 15, 30)),
                ..base.clone()
            },
        },
    ]
}

fn quality_variants(base: &McConfig) -> Vec<Variant> {
    vec![
        Variant {
            label: "quality off (baseline)".to_string(),
            cfg: base.clone(),
        },
        Variant {
            label: "quality body>=35%".to_string(),
            cfg: McConfig {
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(35, 2),
                    ..SignalQualityConfig::default()
                },
                ..base.clone()
            },
        },
        Variant {
            label: "quality body>=40%, range>=1.1x prev".to_string(),
            cfg: McConfig {
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(40, 2),
                    min_range_to_prev_range: Decimal::new(11, 1),
                    ..SignalQualityConfig::default()
                },
                ..base.clone()
            },
        },
        Variant {
            label: "quality body>=45%, range>=1.15x avg20".to_string(),
            cfg: McConfig {
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(45, 2),
                    min_range_to_avg_range: Decimal::new(115, 2),
                    avg_range_lookback: 20,
                    ..SignalQualityConfig::default()
                },
                ..base.clone()
            },
        },
    ]
}

fn bounded_sweep_variants(base: &McConfig) -> Vec<Variant> {
    let mut out = Vec::new();
    let entries = [EntryMode::PrevOpen, EntryMode::PairMidpoint];
    let entry_labels = ["prev_open", "pair_mid"];
    let rrs = [Decimal::new(15, 1), Decimal::new(18, 1), d(2)];
    let rr_labels = ["1.5", "1.8", "2.0"];
    let fills = [4usize, 6usize, 8usize];
    let trails = [TrailingStopMode::None, TrailingStopMode::Progressive];
    let trail_labels = ["none", "prog"];
    let quality = [false, true];

    for (ei, entry_mode) in entries.iter().enumerate() {
        for (ri, rr_target) in rrs.iter().enumerate() {
            for fill in fills {
                for (ti, trail_mode) in trails.iter().enumerate() {
                    for q in quality {
                        let sq = if q {
                            SignalQualityConfig {
                                min_body_to_range: Decimal::new(40, 2),
                                min_range_to_prev_range: Decimal::new(11, 1),
                                ..SignalQualityConfig::default()
                            }
                        } else {
                            SignalQualityConfig::default()
                        };
                        let q_label = if q { "q1" } else { "q0" };
                        out.push(Variant {
                            label: format!(
                                "sweep {} rr{} fill{} trail{} {}",
                                entry_labels[ei], rr_labels[ri], fill, trail_labels[ti], q_label
                            ),
                            cfg: McConfig {
                                entry_mode: entry_mode.clone(),
                                rr_target: *rr_target,
                                prev_open_fill_window_candles: fill,
                                trailing_stop: TrailingStopConfig {
                                    mode: trail_mode.clone(),
                                },
                                signal_quality: sq,
                                ..base.clone()
                            },
                        });
                    }
                }
            }
        }
    }

    out
}

fn nyam_fine_sweep_variants(base: &McConfig) -> Vec<Variant> {
    let mut out = Vec::new();
    let entries = [EntryMode::PrevOpen, EntryMode::PairMidpoint];
    let entry_labels = ["prev_open", "pair_mid"];
    let rrs = [
        Decimal::new(17, 1),
        Decimal::new(18, 1),
        Decimal::new(19, 1),
        d(2),
        Decimal::new(21, 1),
        Decimal::new(22, 1),
    ];
    let rr_labels = ["1.7", "1.8", "1.9", "2.0", "2.1", "2.2"];
    let fills = [3usize, 4usize, 5usize, 6usize, 8usize];
    let trails = [
        TrailingStopMode::None,
        TrailingStopMode::Progressive,
        TrailingStopMode::BreakEven1R,
        TrailingStopMode::StepHalfR,
        TrailingStopMode::Trail05RAt15R,
    ];
    let trail_labels = ["none", "prog", "be1r", "step_half_r", "trail05_at15"];

    for (ei, entry_mode) in entries.iter().enumerate() {
        for (ri, rr_target) in rrs.iter().enumerate() {
            for fill in fills {
                for (ti, trail_mode) in trails.iter().enumerate() {
                    out.push(Variant {
                        label: format!(
                            "nyam_fine {} rr{} fill{} trail{}",
                            entry_labels[ei], rr_labels[ri], fill, trail_labels[ti]
                        ),
                        cfg: McConfig {
                            entry_mode: entry_mode.clone(),
                            rr_target: *rr_target,
                            prev_open_fill_window_candles: fill,
                            trailing_stop: TrailingStopConfig {
                                mode: trail_mode.clone(),
                            },
                            signal_quality: SignalQualityConfig {
                                min_body_to_range: Decimal::new(40, 2),
                                min_range_to_prev_range: Decimal::new(11, 1),
                                ..SignalQualityConfig::default()
                            },
                            trade_window: Some(tw(9, 15, 12, 15)),
                            ..base.clone()
                        },
                    });
                }
            }
        }
    }

    out
}

fn main() {
    let data_15m = resample_from_1m(&load_mnq_1m(), 15);

    let base = McConfig {
        mode: McMode::ReversalDaily,
        pattern: SignalPattern::Engulfing,
        rr_target: Decimal::from(2),
        entry_mode: EntryMode::PrevOpen,
        prev_open_fill_window_candles: 6,
        trend_filter: TrendFilter::None,
        trade_window: Some(tw(6, 0, 15, 30)),
        trailing_stop: TrailingStopConfig {
            mode: TrailingStopMode::None,
        },
        execution: ExecutionConfig {
            market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
            commission_rate_per_side: Decimal::ZERO,
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 1,
            tick_size: Decimal::new(25, 2),
        },
        ..McConfig::default()
    };

    println!("MNQ 15m OB(engulfing) backtest variants");
    println!("bars: {}", data_15m.len());

    let quick = vec![
        Variant {
            label: "quick prev_open".to_string(),
            cfg: McConfig {
                entry_mode: EntryMode::PrevOpen,
                ..base.clone()
            },
        },
        Variant {
            label: "quick pair_mid".to_string(),
            cfg: McConfig {
                entry_mode: EntryMode::PairMidpoint,
                ..base.clone()
            },
        },
        Variant {
            label: "quick close".to_string(),
            cfg: McConfig {
                entry_mode: EntryMode::Close,
                ..base.clone()
            },
        },
    ];

    let mut all_results = Vec::new();

    println!("\n--- Quick Entry Pass ---");
    for v in quick {
        let result = Mc {
            data: data_15m.clone(),
            config: v.cfg,
        }
        .execute();
        print_stats(&v.label, &result);
        all_results.push(summarize(&v.label, &result));
    }

    println!("\n--- Session Variants (prev_open) ---");
    for v in session_variants(&McConfig {
        entry_mode: EntryMode::PrevOpen,
        ..base.clone()
    }) {
        let result = Mc {
            data: data_15m.clone(),
            config: v.cfg,
        }
        .execute();
        print_stats(&v.label, &result);
        all_results.push(summarize(&v.label, &result));
    }

    println!("\n--- Quality Variants (prev_open, full session) ---");
    for v in quality_variants(&McConfig {
        entry_mode: EntryMode::PrevOpen,
        ..base.clone()
    }) {
        let result = Mc {
            data: data_15m.clone(),
            config: v.cfg,
        }
        .execute();
        print_stats(&v.label, &result);
        all_results.push(summarize(&v.label, &result));
    }

    println!("\n--- Bounded Sweep (72 variants) ---");
    let sweep_variants = bounded_sweep_variants(&base);
    for v in sweep_variants {
        let result = Mc {
            data: data_15m.clone(),
            config: v.cfg,
        }
        .execute();
        all_results.push(summarize(&v.label, &result));
    }

    all_results.sort_by(|a, b| {
        b.profit_r
            .cmp(&a.profit_r)
            .then(b.profit_factor_r.cmp(&a.profit_factor_r))
            .then(b.trades.cmp(&a.trades))
    });

    println!("\nTop 12 by profit_r:");
    for row in all_results.iter().take(12) {
        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            row.label,
            row.trades,
            row.win_rate.round_dp(2),
            row.profit_r.round_dp(2),
            row.profit_factor_r.round_dp(2),
            row.pnl_pct.round_dp(2),
        );
    }

    println!("\n--- NYAM Fine Sweep (quality-on) ---");
    let mut fine_results = Vec::new();
    for v in nyam_fine_sweep_variants(&base) {
        let result = Mc {
            data: data_15m.clone(),
            config: v.cfg,
        }
        .execute();
        fine_results.push(summarize(&v.label, &result));
    }
    fine_results.sort_by(|a, b| {
        b.profit_r
            .cmp(&a.profit_r)
            .then(b.profit_factor_r.cmp(&a.profit_factor_r))
            .then(b.trades.cmp(&a.trades))
    });
    println!("Top 15 NYAM fine variants by profit_r:");
    for row in fine_results.iter().take(15) {
        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            row.label,
            row.trades,
            row.win_rate.round_dp(2),
            row.profit_r.round_dp(2),
            row.profit_factor_r.round_dp(2),
            row.pnl_pct.round_dp(2),
        );
    }

    println!("\n--- Fixed SL 25pts (NYAM + quality-on + pair_mid rr2.0) ---");
    let fixed_sl_base = McConfig {
        entry_mode: EntryMode::PairMidpoint,
        rr_target: d(2),
        prev_open_fill_window_candles: 4,
        trailing_stop: TrailingStopConfig {
            mode: TrailingStopMode::None,
        },
        signal_quality: SignalQualityConfig {
            min_body_to_range: Decimal::new(40, 2),
            min_range_to_prev_range: Decimal::new(11, 1),
            ..SignalQualityConfig::default()
        },
        trade_window: Some(tw(9, 15, 12, 15)),
        max_sl_points: Some(d(25)),
        ..base.clone()
    };

    for v in [
        Variant {
            label: "fixed_sl25 keep_entry_move_stop".to_string(),
            cfg: McConfig {
                max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                ..fixed_sl_base.clone()
            },
        },
        Variant {
            label: "fixed_sl25 keep_stop_move_entry".to_string(),
            cfg: McConfig {
                max_sl_mode: MaxSlMode::KeepStopMoveEntry,
                ..fixed_sl_base.clone()
            },
        },
    ] {
        let result = Mc {
            data: data_15m.clone(),
            config: v.cfg,
        }
        .execute();
        print_stats(&v.label, &result);
    }

    println!("\n--- Fixed SL Grid (NYAM + quality-on + pair_mid rr2.0) ---");
    let sl_caps = [
        Decimal::from(20),
        Decimal::new(225, 1),
        Decimal::from(25),
        Decimal::new(275, 1),
        Decimal::from(30),
    ];
    let mut sl_grid_results = Vec::new();
    for sl_cap in sl_caps {
        for (mode, mode_label) in [
            (MaxSlMode::KeepEntryMoveStop, "keep_entry_move_stop"),
            (MaxSlMode::KeepStopMoveEntry, "keep_stop_move_entry"),
        ] {
            let label = format!("sl_cap={} {}", sl_cap, mode_label);
            let cfg = McConfig {
                max_sl_points: Some(sl_cap),
                max_sl_mode: mode,
                ..fixed_sl_base.clone()
            };
            let result = Mc {
                data: data_15m.clone(),
                config: cfg,
            }
            .execute();
            sl_grid_results.push(summarize(&label, &result));
        }
    }

    sl_grid_results.sort_by(|a, b| {
        b.profit_r
            .cmp(&a.profit_r)
            .then(b.profit_factor_r.cmp(&a.profit_factor_r))
            .then(b.trades.cmp(&a.trades))
    });

    println!("Top SL grid variants by profit_r:");
    for row in sl_grid_results.iter() {
        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            row.label,
            row.trades,
            row.win_rate.round_dp(2),
            row.profit_r.round_dp(2),
            row.profit_factor_r.round_dp(2),
            row.pnl_pct.round_dp(2),
        );
    }

    println!("\n--- Narrow Confirmation + Date Split ---");
    let caps = [Decimal::new(275, 1), Decimal::from(30), Decimal::new(325, 1)];
    let rrs = [Decimal::new(19, 1), Decimal::from(2), Decimal::new(21, 1)];
    let modes = [
        (MaxSlMode::KeepEntryMoveStop, "keep_entry_move_stop"),
        (MaxSlMode::KeepStopMoveEntry, "keep_stop_move_entry"),
    ];

    let min_ts = data_15m.first().map(|c| c.open_time).unwrap_or(0);
    let max_ts = data_15m.last().map(|c| c.open_time).unwrap_or(0);
    let split_ts = min_ts + ((max_ts - min_ts) / 2);
    let train_data = slice_between(&data_15m, min_ts, split_ts);
    let test_data = slice_between(&data_15m, split_ts, max_ts + 1);

    println!(
        "split_ts={} train_bars={} test_bars={}",
        split_ts,
        train_data.len(),
        test_data.len()
    );

    let mut train_rows: Vec<(McConfig, VariantResult)> = Vec::new();
    for cap in caps {
        for rr in rrs {
            for (mode, mode_label) in &modes {
                let label = format!("narrow cap={} rr={} {}", cap, rr, mode_label);
                let cfg = McConfig {
                    max_sl_points: Some(cap),
                    max_sl_mode: mode.clone(),
                    rr_target: rr,
                    ..fixed_sl_base.clone()
                };
                let train_result = Mc {
                    data: train_data.clone(),
                    config: cfg.clone(),
                }
                .execute();
                train_rows.push((cfg, summarize(&label, &train_result)));
            }
        }
    }

    train_rows.sort_by(|a, b| {
        b.1.profit_r
            .cmp(&a.1.profit_r)
            .then(b.1.profit_factor_r.cmp(&a.1.profit_factor_r))
            .then(b.1.trades.cmp(&a.1.trades))
    });

    println!("Top 6 train variants:");
    for (_, row) in train_rows.iter().take(6) {
        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            row.label,
            row.trades,
            row.win_rate.round_dp(2),
            row.profit_r.round_dp(2),
            row.profit_factor_r.round_dp(2),
            row.pnl_pct.round_dp(2),
        );
    }

    if let Some((best_cfg, best_train)) = train_rows.first() {
        let test_result = Mc {
            data: test_data,
            config: best_cfg.clone(),
        }
        .execute();
        let test_row = summarize(&format!("OOS of [{}]", best_train.label), &test_result);

        println!("Best train candidate:");
        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            best_train.label,
            best_train.trades,
            best_train.win_rate.round_dp(2),
            best_train.profit_r.round_dp(2),
            best_train.profit_factor_r.round_dp(2),
            best_train.pnl_pct.round_dp(2),
        );
        println!("Out-of-sample performance:");
        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            test_row.label,
            test_row.trades,
            test_row.win_rate.round_dp(2),
            test_row.profit_r.round_dp(2),
            test_row.profit_factor_r.round_dp(2),
            test_row.pnl_pct.round_dp(2),
        );
    }

    println!("\n--- Production Candidate Detailed Stats (full data) ---");
    let prod_cfg = McConfig {
        max_sl_points: Some(Decimal::new(325, 1)),
        max_sl_mode: MaxSlMode::KeepEntryMoveStop,
        rr_target: Decimal::from(2),
        ..fixed_sl_base
    };
    let prod_result = Mc {
        data: data_15m,
        config: prod_cfg,
    }
    .execute();
    print_stats("prod_candidate cap32.5 rr2 keep_entry_move_stop", &prod_result);

    println!("\n--- ETH Transfer Test (5m,15m,1h,4h) ---");
    let eth_tfs = ["5m", "15m", "1h", "4h"];
    let mut eth_rows: Vec<VariantResult> = Vec::new();
    for tf in eth_tfs {
        let data = load_binance_tf("ETH", tf);

        let full_day_cfg = McConfig {
            mode: McMode::ReversalDaily,
            pattern: SignalPattern::Engulfing,
            entry_mode: EntryMode::PairMidpoint,
            rr_target: Decimal::from(2),
            prev_open_fill_window_candles: 4,
            trailing_stop: TrailingStopConfig {
                mode: TrailingStopMode::None,
            },
            signal_quality: SignalQualityConfig {
                min_body_to_range: Decimal::new(40, 2),
                min_range_to_prev_range: Decimal::new(11, 1),
                ..SignalQualityConfig::default()
            },
            max_sl_points: Some(Decimal::new(325, 1)),
            max_sl_mode: MaxSlMode::KeepEntryMoveStop,
            trend_filter: TrendFilter::None,
            trade_window: None,
            execution: ExecutionConfig {
                market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                commission_rate_per_side: Decimal::ZERO,
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: 1,
                tick_size: Decimal::new(1, 2),
            },
            ..McConfig::default()
        };

        let nyam_cfg = McConfig {
            trade_window: Some(tw(9, 15, 12, 15)),
            ..full_day_cfg.clone()
        };

        let full_label = format!("ETH {} full_day", tf);
        let nyam_label = format!("ETH {} nyam", tf);
        let full_row = run_config(data.clone(), full_day_cfg, &full_label);
        let nyam_row = run_config(data, nyam_cfg, &nyam_label);

        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            full_row.label,
            full_row.trades,
            full_row.win_rate.round_dp(2),
            full_row.profit_r.round_dp(2),
            full_row.profit_factor_r.round_dp(2),
            full_row.pnl_pct.round_dp(2),
        );
        println!(
            "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
            nyam_row.label,
            nyam_row.trades,
            nyam_row.win_rate.round_dp(2),
            nyam_row.profit_r.round_dp(2),
            nyam_row.profit_factor_r.round_dp(2),
            nyam_row.pnl_pct.round_dp(2),
        );

        eth_rows.push(full_row);
        eth_rows.push(nyam_row);
    }

    let mut report = String::new();
    report.push_str("# OB Engulfing Progress Report\n\n");
    report.push_str("## MNQ Production Candidate\n");
    report.push_str("- Session: NYAM (09:15-12:15)\n");
    report.push_str("- Pattern: Engulfing (OB style), Entry: PairMidpoint\n");
    report.push_str("- Quality: body>=40%, range>=1.1x prev\n");
    report.push_str("- RR: 2.0\n");
    report.push_str("- Max SL: 32.5 points, mode: KeepEntryMoveStop\n");
    report.push_str("- Full-data stats: trades=120, win%=43.33, profit_r=35.70, pf_r=1.52, pnl%=33.92\n");
    report.push_str("- Net points: 2264.87\n");
    report.push_str("- Net profit (MNQ $2/point, 1 contract): $4529.74\n\n");

    report.push_str("## ETH Transfer Test\n");
    report.push_str("Transferred the same setup to ETH (5m/15m/1h/4h). For crypto, both full-day and NYAM windows were tested.\n\n");
    report.push_str("| Variant | Trades | Win% | Profit R | PF R | PnL% |\n");
    report.push_str("|---|---:|---:|---:|---:|---:|\n");
    for row in &eth_rows {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            row.label,
            row.trades,
            row.win_rate.round_dp(2),
            row.profit_r.round_dp(2),
            row.profit_factor_r.round_dp(2),
            row.pnl_pct.round_dp(2)
        ));
    }

    println!("\n--- ETH Focused Refinement Sweep (15m,4h) ---");
    let rr_grid = [Decimal::new(18, 1), Decimal::new(20, 1), Decimal::new(22, 1)];
    let sl_grid = [Decimal::new(275, 1), Decimal::from(30), Decimal::new(325, 1)];
    let body_grid = [Decimal::new(35, 2), Decimal::new(40, 2), Decimal::new(45, 2)];
    let range_prev_grid = [Decimal::ZERO, Decimal::new(11, 1), Decimal::new(12, 1)];
    let tfs = ["15m", "4h"];

    for tf in tfs {
        let data = load_binance_tf("ETH", tf);
        let mut rows = Vec::new();
        for rr in rr_grid {
            for sl in sl_grid {
                for body in body_grid {
                    for rprev in range_prev_grid {
                        let label = format!(
                            "ETH {} rr={} sl={} body={} rprev={}",
                            tf, rr, sl, body, rprev
                        );
                        let cfg = McConfig {
                            mode: McMode::ReversalDaily,
                            pattern: SignalPattern::Engulfing,
                            entry_mode: EntryMode::PairMidpoint,
                            rr_target: rr,
                            prev_open_fill_window_candles: 4,
                            trailing_stop: TrailingStopConfig {
                                mode: TrailingStopMode::None,
                            },
                            signal_quality: SignalQualityConfig {
                                min_body_to_range: body,
                                min_range_to_prev_range: rprev,
                                ..SignalQualityConfig::default()
                            },
                            max_sl_points: Some(sl),
                            max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                            trend_filter: TrendFilter::None,
                            trade_window: None,
                            execution: ExecutionConfig {
                                market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                                commission_rate_per_side: Decimal::ZERO,
                                fee_rate_per_side: Decimal::ZERO,
                                slippage_ticks_per_side: 1,
                                tick_size: Decimal::new(1, 2),
                            },
                            ..McConfig::default()
                        };
                        rows.push(run_config(data.clone(), cfg, &label));
                    }
                }
            }
        }

        let top = sorted_top(rows, 5);
        println!("Top 5 ETH {} refined variants:", tf);
        for row in &top {
            println!(
                "{} | trades={} win%={} profit_r={} pf_r={} pnl%={}",
                row.label,
                row.trades,
                row.win_rate.round_dp(2),
                row.profit_r.round_dp(2),
                row.profit_factor_r.round_dp(2),
                row.pnl_pct.round_dp(2),
            );
        }

        report.push_str("\n");
        report.push_str(&format!("## ETH {} Refinement Top 5\n", tf));
        report.push_str("| Variant | Trades | Win% | Profit R | PF R | PnL% |\n");
        report.push_str("|---|---:|---:|---:|---:|---:|\n");
        for row in &top {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                row.label,
                row.trades,
                row.win_rate.round_dp(2),
                row.profit_r.round_dp(2),
                row.profit_factor_r.round_dp(2),
                row.pnl_pct.round_dp(2)
            ));
        }

        report.push_str("\n");
        report.push_str(&format!("## ETH {} Robustness Split (winner)\n", tf));

        let winner_cfg = if tf == "15m" {
            McConfig {
                mode: McMode::ReversalDaily,
                pattern: SignalPattern::Engulfing,
                entry_mode: EntryMode::PairMidpoint,
                rr_target: Decimal::from(2),
                prev_open_fill_window_candles: 4,
                trailing_stop: TrailingStopConfig {
                    mode: TrailingStopMode::None,
                },
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(45, 2),
                    min_range_to_prev_range: Decimal::ZERO,
                    ..SignalQualityConfig::default()
                },
                max_sl_points: Some(Decimal::from(30)),
                max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                trend_filter: TrendFilter::None,
                trade_window: None,
                execution: ExecutionConfig {
                    market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                    commission_rate_per_side: Decimal::ZERO,
                    fee_rate_per_side: Decimal::ZERO,
                    slippage_ticks_per_side: 1,
                    tick_size: Decimal::new(1, 2),
                },
                ..McConfig::default()
            }
        } else {
            McConfig {
                mode: McMode::ReversalDaily,
                pattern: SignalPattern::Engulfing,
                entry_mode: EntryMode::PairMidpoint,
                rr_target: Decimal::new(22, 1),
                prev_open_fill_window_candles: 4,
                trailing_stop: TrailingStopConfig {
                    mode: TrailingStopMode::None,
                },
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(45, 2),
                    min_range_to_prev_range: Decimal::ZERO,
                    ..SignalQualityConfig::default()
                },
                max_sl_points: Some(Decimal::from(30)),
                max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                trend_filter: TrendFilter::None,
                trade_window: None,
                execution: ExecutionConfig {
                    market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                    commission_rate_per_side: Decimal::ZERO,
                    fee_rate_per_side: Decimal::ZERO,
                    slippage_ticks_per_side: 1,
                    tick_size: Decimal::new(1, 2),
                },
                ..McConfig::default()
            }
        };

        let min_ts = data.first().map(|c| c.open_time).unwrap_or(0);
        let max_ts = data.last().map(|c| c.open_time).unwrap_or(0);
        let split_ts = min_ts + ((max_ts - min_ts) / 2);
        let train = slice_between(&data, min_ts, split_ts);
        let test = slice_between(&data, split_ts, max_ts + 1);

        let train_row = run_config(train, winner_cfg.clone(), &format!("ETH {} winner train", tf));
        let test_row = run_config(test, winner_cfg, &format!("ETH {} winner test", tf));

        println!(
            "ETH {} winner split | train: trades={} win%={} profit_r={} pf_r={} pnl%={} | test: trades={} win%={} profit_r={} pf_r={} pnl%={}",
            tf,
            train_row.trades,
            train_row.win_rate.round_dp(2),
            train_row.profit_r.round_dp(2),
            train_row.profit_factor_r.round_dp(2),
            train_row.pnl_pct.round_dp(2),
            test_row.trades,
            test_row.win_rate.round_dp(2),
            test_row.profit_r.round_dp(2),
            test_row.profit_factor_r.round_dp(2),
            test_row.pnl_pct.round_dp(2),
        );

        report.push_str(&format!("- Split timestamp: {}\n", split_ts));
        report.push_str(&format!(
            "- Train: trades={}, win%={}, profit_r={}, pf_r={}, pnl%={}\n",
            train_row.trades,
            train_row.win_rate.round_dp(2),
            train_row.profit_r.round_dp(2),
            train_row.profit_factor_r.round_dp(2),
            train_row.pnl_pct.round_dp(2)
        ));
        report.push_str(&format!(
            "- Test: trades={}, win%={}, profit_r={}, pf_r={}, pnl%={}\n",
            test_row.trades,
            test_row.win_rate.round_dp(2),
            test_row.profit_r.round_dp(2),
            test_row.profit_factor_r.round_dp(2),
            test_row.pnl_pct.round_dp(2)
        ));
    }

    println!("\n--- Final Cycle: Rolling OOS Verdict (ETH 15m / 4h) ---");
    report.push_str("\n## Final Cycle: Rolling OOS Verdict\n");
    report.push_str("Criteria: OOS PF>=1.20 and OOS profit_r>0 in >=4/5 windows.\n\n");

    let final_candidates = vec![
        (
            "ETH 15m base",
            load_binance_tf("ETH", "15m"),
            McConfig {
                mode: McMode::ReversalDaily,
                pattern: SignalPattern::Engulfing,
                entry_mode: EntryMode::PairMidpoint,
                rr_target: Decimal::from(2),
                prev_open_fill_window_candles: 4,
                trailing_stop: TrailingStopConfig {
                    mode: TrailingStopMode::None,
                },
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(45, 2),
                    min_range_to_prev_range: Decimal::ZERO,
                    ..SignalQualityConfig::default()
                },
                max_sl_points: Some(Decimal::from(30)),
                max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                trend_filter: TrendFilter::None,
                trade_window: None,
                execution: ExecutionConfig {
                    market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                    commission_rate_per_side: Decimal::ZERO,
                    fee_rate_per_side: Decimal::ZERO,
                    slippage_ticks_per_side: 1,
                    tick_size: Decimal::new(1, 2),
                },
                ..McConfig::default()
            },
        ),
        (
            "ETH 15m ema50/200",
            load_binance_tf("ETH", "15m"),
            McConfig {
                mode: McMode::ReversalDaily,
                pattern: SignalPattern::Engulfing,
                entry_mode: EntryMode::PairMidpoint,
                rr_target: Decimal::from(2),
                prev_open_fill_window_candles: 4,
                trailing_stop: TrailingStopConfig {
                    mode: TrailingStopMode::None,
                },
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(45, 2),
                    min_range_to_prev_range: Decimal::ZERO,
                    ..SignalQualityConfig::default()
                },
                max_sl_points: Some(Decimal::from(30)),
                max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                trend_filter: TrendFilter::Ema { fast: 50, slow: 200 },
                trade_window: None,
                execution: ExecutionConfig {
                    market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                    commission_rate_per_side: Decimal::ZERO,
                    fee_rate_per_side: Decimal::ZERO,
                    slippage_ticks_per_side: 1,
                    tick_size: Decimal::new(1, 2),
                },
                ..McConfig::default()
            },
        ),
        (
            "ETH 4h base",
            load_binance_tf("ETH", "4h"),
            McConfig {
                mode: McMode::ReversalDaily,
                pattern: SignalPattern::Engulfing,
                entry_mode: EntryMode::PairMidpoint,
                rr_target: Decimal::new(22, 1),
                prev_open_fill_window_candles: 4,
                trailing_stop: TrailingStopConfig {
                    mode: TrailingStopMode::None,
                },
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(45, 2),
                    min_range_to_prev_range: Decimal::ZERO,
                    ..SignalQualityConfig::default()
                },
                max_sl_points: Some(Decimal::from(30)),
                max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                trend_filter: TrendFilter::None,
                trade_window: None,
                execution: ExecutionConfig {
                    market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                    commission_rate_per_side: Decimal::ZERO,
                    fee_rate_per_side: Decimal::ZERO,
                    slippage_ticks_per_side: 1,
                    tick_size: Decimal::new(1, 2),
                },
                ..McConfig::default()
            },
        ),
        (
            "ETH 4h ema50/200",
            load_binance_tf("ETH", "4h"),
            McConfig {
                mode: McMode::ReversalDaily,
                pattern: SignalPattern::Engulfing,
                entry_mode: EntryMode::PairMidpoint,
                rr_target: Decimal::new(22, 1),
                prev_open_fill_window_candles: 4,
                trailing_stop: TrailingStopConfig {
                    mode: TrailingStopMode::None,
                },
                signal_quality: SignalQualityConfig {
                    min_body_to_range: Decimal::new(45, 2),
                    min_range_to_prev_range: Decimal::ZERO,
                    ..SignalQualityConfig::default()
                },
                max_sl_points: Some(Decimal::from(30)),
                max_sl_mode: MaxSlMode::KeepEntryMoveStop,
                trend_filter: TrendFilter::Ema { fast: 50, slow: 200 },
                trade_window: None,
                execution: ExecutionConfig {
                    market_entry: backtest::strategies::mc::MarketEntryMode::NextBarOpen,
                    commission_rate_per_side: Decimal::ZERO,
                    fee_rate_per_side: Decimal::ZERO,
                    slippage_ticks_per_side: 1,
                    tick_size: Decimal::new(1, 2),
                },
                ..McConfig::default()
            },
        ),
    ];

    for (name, data, cfg) in final_candidates {
        let windows = split_equal_windows(&data, 5);
        let mut pass_profit = 0usize;
        let mut pass_pf = 0usize;
        let mut lines = Vec::new();
        for (i, w) in windows.into_iter().enumerate() {
            let r = run_config(w, cfg.clone(), "wf");
            if r.profit_r > Decimal::ZERO {
                pass_profit += 1;
            }
            if r.profit_factor_r >= Decimal::new(12, 1) {
                pass_pf += 1;
            }
            lines.push(format!(
                "W{}: trades={} profit_r={} pf_r={} pnl%={}",
                i + 1,
                r.trades,
                r.profit_r.round_dp(2),
                r.profit_factor_r.round_dp(2),
                r.pnl_pct.round_dp(2)
            ));
        }
        let verdict = if pass_profit >= 4 && pass_pf >= 4 {
            "PROMOTE"
        } else {
            "KILL"
        };
        println!(
            "{} => {} (profit windows {}/5, pf windows {}/5)",
            name, verdict, pass_profit, pass_pf
        );
        report.push_str(&format!("### {}\n", name));
        report.push_str(&format!(
            "- Verdict: {} (profit windows {}/5, pf windows {}/5)\n",
            verdict, pass_profit, pass_pf
        ));
        for l in lines {
            report.push_str(&format!("- {}\n", l));
        }
        report.push_str("\n");
    }

    std::fs::write(
        "reports/strategy_overviews/OB_ENGULFING_MNQ_ETH_REPORT.md",
        report,
    )
    .unwrap_or_else(|e| panic!("failed writing report: {}", e));
}
