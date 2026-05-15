extern crate rust_decimal;

use clap::{Arg, ArgAction, Command};
use rayon::prelude::*;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::fs;
use std::sync::Arc;

use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    execute,
    model::{
        backtest_result::BacktestResult, candle_stick::CandleStick, fee_config::FeeConfig,
        trade_result::TradeResult,
    },
    strategies::{
        ttrades_fractal::{FractalConfig, TTradesFractal},
        ttrades_fractal_mtf::{
            CisdVariant, EntryVariant, FractalMTFConfig, KillzoneMode, ReversalConfirmMode,
            TTradesFractalMTF,
        },
    },
};

#[derive(Clone)]
struct Row {
    strategy: &'static str,
    market: &'static str,
    asset: &'static str,
    timeframe: &'static str,
    net_profit_pct: Decimal,
    net_unit: &'static str,
    profit_factor: Decimal,
    win_rate: Decimal,
    trades: usize,
    max_drawdown_pct: Decimal,
    total_costs: Decimal,
    wf_train_net: String,
    wf_test_net: String,
    wf_test_pf: String,
    validated_reality_check: &'static str,
    setup_coverage: String,
    final_verdict: &'static str,
}

fn point_value_usd(asset: &str) -> Option<Decimal> {
    match asset {
        "MNQ" => Some(Decimal::from(2)),
        "MES" => Some(Decimal::from(5)),
        "GOLD" => Some(Decimal::from(10)),
        _ => None,
    }
}

fn market(asset: &str) -> &'static str {
    if point_value_usd(asset).is_some() {
        "futures"
    } else {
        "crypto"
    }
}

fn net_usd_1_micro(result: &BacktestResult, asset: &str) -> Decimal {
    let Some(mult) = point_value_usd(asset) else {
        return Decimal::ZERO;
    };
    let gross_points = result.trades.iter().map(|t| t.points().0).sum::<Decimal>();
    let costs_points = result
        .trades
        .iter()
        .map(|t| t.total_costs())
        .sum::<Decimal>();
    ((gross_points - costs_points) * mult).round_dp(2)
}

fn load_binance(json: &'static str) -> Vec<CandleStick> {
    CandleStickLoader::load_binance(json)
}

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .expect("failed loading parquet")
}

fn cap(mut data: Vec<CandleStick>, max_bars: usize) -> Vec<CandleStick> {
    if data.len() > max_bars {
        data.truncate(max_bars);
    }
    data
}

fn summarize(result: &BacktestResult) -> (Decimal, Decimal, Decimal, Decimal, Decimal, usize) {
    let total = result.number_of_trades();
    let wins = result.result(TradeResult::Winner);
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_i32(wins as i32).unwrap() / Decimal::from_i32(total as i32).unwrap()
            * Decimal::from(100))
        .round_dp(2)
    };

    let mut capital = Decimal::from(1000);
    let mut peak = capital;
    let mut max_dd = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;
    let r = Decimal::from_f32(0.01).unwrap();

    for t in &result.trades {
        let risk = match t.direction {
            backtest::model::position_direction::PositionDirection::Long => t.entry.0 - t.sl.0,
            backtest::model::position_direction::PositionDirection::Short => t.sl.0 - t.entry.0,
        };
        let gross_r = if risk <= Decimal::ZERO {
            Decimal::ZERO
        } else {
            match t.result {
                TradeResult::Winner => t.rr().0,
                TradeResult::Expense => Decimal::from(-1),
                TradeResult::BreakEven => Decimal::ZERO,
            }
        };

        let change = capital * r * gross_r.trunc_with_scale(4) - t.total_costs();
        if change > Decimal::ZERO {
            gross_profit += change;
        } else if change < Decimal::ZERO {
            gross_loss += -change;
        }
        capital += change;
        if capital > peak {
            peak = capital;
        }
        if peak > Decimal::ZERO {
            let dd = ((peak - capital) / peak * Decimal::from(100)).round_dp(2);
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }

    let pf = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).round_dp(2)
    } else {
        Decimal::ZERO
    };
    let net =
        ((capital - Decimal::from(1000)) / Decimal::from(1000) * Decimal::from(100)).round_dp(2);
    let total_costs = result
        .trades
        .iter()
        .map(|t| t.total_costs())
        .sum::<Decimal>()
        .round_dp(2);
    (net, pf, win_rate, max_dd, total_costs, total)
}

fn split_train_test(
    data: &Arc<Vec<CandleStick>>,
    train_ratio: f64,
) -> (Arc<Vec<CandleStick>>, Arc<Vec<CandleStick>>) {
    let n = data.len();
    let split = ((n as f64) * train_ratio).floor() as usize;
    let split = split.clamp(1, n.saturating_sub(1));
    let train = data[..split].to_vec();
    let test = data[split..].to_vec();
    (Arc::new(train), Arc::new(test))
}

fn resample_from_1m(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if data.is_empty() {
        return vec![];
    }
    let bucket = minutes * 60;
    let mut out: Vec<CandleStick> = Vec::new();

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

fn verdict(net: Decimal, pf: Decimal, trades: usize) -> &'static str {
    if trades < 100 {
        return "PARTIALLY_TESTED";
    }
    if net > Decimal::ZERO && pf >= Decimal::from_f32(1.2).unwrap() {
        "FULLY_TESTED"
    } else {
        "PARTIALLY_TESTED"
    }
}

fn run_fractal_case(
    asset: &'static str,
    timeframe: &'static str,
    data: Arc<Vec<CandleStick>>,
    tick_size: Decimal,
    slippage_levels: &[i32],
) -> Row {
    let mkt = market(asset);
    let net_unit = if mkt == "futures" { "$" } else { "%" };
    let mut naive_cfg = FractalConfig::default();
    naive_cfg.tick_size = tick_size;
    naive_cfg.slippage_ticks_per_side = 0;
    naive_cfg.fee_config = FeeConfig::zero();
    let naive_result = execute(TTradesFractal {
        data: Arc::clone(&data),
        config: naive_cfg,
    });
    let naive = summarize(&naive_result);
    let naive_net = if mkt == "futures" {
        net_usd_1_micro(&naive_result, asset)
    } else {
        naive.0
    };

    let naive_bad = naive.0 <= Decimal::ZERO || naive.1 < Decimal::ONE;
    if naive_bad {
        return Row {
            strategy: "ttrades_fractal",
            market: mkt,
            asset,
            timeframe,
            net_profit_pct: naive_net,
            net_unit,
            profit_factor: naive.1,
            win_rate: naive.2,
            trades: naive.5,
            max_drawdown_pct: naive.3,
            total_costs: Decimal::ZERO,
            wf_train_net: "SKIPPED_BAD_NAIVE".to_string(),
            wf_test_net: "SKIPPED_BAD_NAIVE".to_string(),
            wf_test_pf: "SKIPPED_BAD_NAIVE".to_string(),
            validated_reality_check: "SKIPPED_BAD_NAIVE",
            setup_coverage: format!(
                "naive_only(rr=2,use_fvg=true,lookback=20,require_cisd=true);slippage={:?}",
                slippage_levels
            ),
            final_verdict: "PARTIALLY_TESTED",
        };
    }

    let mut outcomes: Vec<(
        i32,
        (Decimal, Decimal, Decimal, Decimal, Decimal, usize),
        Decimal,
    )> = Vec::new();
    for &ticks in slippage_levels {
        let mut cfg = FractalConfig::default();
        cfg.tick_size = tick_size;
        cfg.slippage_ticks_per_side = ticks;
        let result = execute(TTradesFractal {
            data: Arc::clone(&data),
            config: cfg,
        });
        let s = summarize(&result);
        let net_disp = if mkt == "futures" {
            net_usd_1_micro(&result, asset)
        } else {
            s.0
        };
        outcomes.push((ticks, s, net_disp));
    }

    let (_, base, base_net) = outcomes[0].clone();
    let (train_data, test_data) = split_train_test(&data, 0.7);
    let mut wf_cfg = FractalConfig::default();
    wf_cfg.tick_size = tick_size;
    wf_cfg.slippage_ticks_per_side = 0;
    let wf_train_result = execute(TTradesFractal {
        data: train_data,
        config: wf_cfg.clone(),
    });
    let wf_train = summarize(&wf_train_result);
    let wf_train_net = if mkt == "futures" {
        net_usd_1_micro(&wf_train_result, asset).to_string()
    } else {
        wf_train.0.to_string()
    };
    let wf_test_result = execute(TTradesFractal {
        data: test_data,
        config: wf_cfg,
    });
    let wf_test = summarize(&wf_test_result);
    let wf_test_net = if mkt == "futures" {
        net_usd_1_micro(&wf_test_result, asset).to_string()
    } else {
        wf_test.0.to_string()
    };

    Row {
        strategy: "ttrades_fractal",
        market: mkt,
        asset,
        timeframe,
        net_profit_pct: base_net,
        net_unit,
        profit_factor: base.1,
        win_rate: base.2,
        trades: base.5,
        max_drawdown_pct: base.3,
        total_costs: base.4,
        wf_train_net,
        wf_test_net,
        wf_test_pf: wf_test.1.to_string(),
        validated_reality_check: "PARTIAL",
        setup_coverage: format!(
            "rr=[1,2,3] planned; current=rr=2,use_fvg=true,lookback=20,require_cisd=true;slippage={:?}",
            slippage_levels
        ),
        final_verdict: verdict(base.0, base.1, base.5),
    }
}

fn run_mtf_case(
    asset: &'static str,
    timeframe: &'static str,
    mode_name: &'static str,
    time_profile_name: &'static str,
    opportunity_name: &'static str,
    ltf_data: Arc<Vec<CandleStick>>,
    htf_data: Arc<Vec<CandleStick>>,
    tick_size: Decimal,
    slippage_levels: &[i32],
    cisd_name: &'static str,
    cisd_variant: CisdVariant,
    reversal_mode: ReversalConfirmMode,
    weekday_mask: u8,
    killzone_mode: KillzoneMode,
    rr_target: Decimal,
    entry_variant: EntryVariant,
    poi_padding_bps: i32,
    ob_sweep_tolerance_bps: i32,
) -> Row {
    let mkt = market(asset);
    let net_unit = if mkt == "futures" { "$" } else { "%" };
    let mut htf_vec = (*htf_data).clone();
    if let Some(last) = ltf_data.last().map(|c| c.open_time) {
        htf_vec.retain(|c| c.open_time <= last);
    }
    let htf = Arc::new(htf_vec);

    let mut naive_cfg = FractalMTFConfig::default();
    naive_cfg.tick_size = tick_size;
    naive_cfg.slippage_ticks_per_side = 0;
    naive_cfg.fee_config = FeeConfig::zero();
    naive_cfg.log_progress = false;
    naive_cfg.entry_variant = entry_variant;
    naive_cfg.cisd_variant = cisd_variant;
    naive_cfg.reversal_confirm_mode = reversal_mode;
    naive_cfg.weekday_mask = weekday_mask;
    naive_cfg.killzone_mode = killzone_mode;
    naive_cfg.rr_target = rr_target;
    naive_cfg.poi_padding_bps = poi_padding_bps;
    naive_cfg.ob_sweep_tolerance_bps = ob_sweep_tolerance_bps;
    let naive_result = execute(TTradesFractalMTF {
        ltf_data: Arc::clone(&ltf_data),
        htf_data: Arc::clone(&htf),
        config: naive_cfg,
    });
    let naive = summarize(&naive_result);
    let naive_net = if mkt == "futures" {
        net_usd_1_micro(&naive_result, asset)
    } else {
        naive.0
    };

    let naive_bad = naive.0 <= Decimal::ZERO || naive.1 < Decimal::ONE;
    if naive_bad {
        return Row {
            strategy: "ttrades_fractal_mtf",
            market: mkt,
            asset,
            timeframe,
            net_profit_pct: naive_net,
            net_unit,
            profit_factor: naive.1,
            win_rate: naive.2,
            trades: naive.5,
            max_drawdown_pct: naive.3,
            total_costs: Decimal::ZERO,
            wf_train_net: "SKIPPED_BAD_NAIVE".to_string(),
            wf_test_net: "SKIPPED_BAD_NAIVE".to_string(),
            wf_test_pf: "SKIPPED_BAD_NAIVE".to_string(),
            validated_reality_check: "SKIPPED_BAD_NAIVE",
            setup_coverage: format!("naive_only(rr={});confirm_mode={};time_filter={};opportunity={};poi_pad_bps={};ob_tol_bps={};slippage={:?}", rr_target, mode_name, time_profile_name, opportunity_name, poi_padding_bps, ob_sweep_tolerance_bps, slippage_levels),
            final_verdict: "PARTIALLY_TESTED",
        };
    }

    let mut outcomes: Vec<(
        i32,
        (Decimal, Decimal, Decimal, Decimal, Decimal, usize),
        Decimal,
    )> = Vec::new();
    for &ticks in slippage_levels {
        let mut cfg = FractalMTFConfig::default();
        cfg.tick_size = tick_size;
        cfg.slippage_ticks_per_side = ticks;
        cfg.log_progress = false;
        cfg.entry_variant = entry_variant;
        cfg.cisd_variant = cisd_variant;
        cfg.reversal_confirm_mode = reversal_mode;
        cfg.weekday_mask = weekday_mask;
        cfg.killzone_mode = killzone_mode;
        cfg.rr_target = rr_target;
        cfg.poi_padding_bps = poi_padding_bps;
        cfg.ob_sweep_tolerance_bps = ob_sweep_tolerance_bps;
        let result = execute(TTradesFractalMTF {
            ltf_data: Arc::clone(&ltf_data),
            htf_data: Arc::clone(&htf),
            config: cfg,
        });
        let s = summarize(&result);
        let net_disp = if mkt == "futures" {
            net_usd_1_micro(&result, asset)
        } else {
            s.0
        };
        outcomes.push((ticks, s, net_disp));
    }

    let (_, base, base_net) = outcomes[0].clone();
    let (ltf_train, ltf_test) = split_train_test(&ltf_data, 0.7);
    let mut htf_train = (*htf).clone();
    let mut htf_test = (*htf).clone();
    if let Some(last_train) = ltf_train.last().map(|c| c.open_time) {
        htf_train.retain(|c| c.open_time <= last_train);
    }
    if let Some(first_test) = ltf_test.first().map(|c| c.open_time) {
        htf_test.retain(|c| c.open_time >= first_test);
    }
    let mut wf_cfg = FractalMTFConfig::default();
    wf_cfg.tick_size = tick_size;
    wf_cfg.slippage_ticks_per_side = 0;
    wf_cfg.log_progress = false;
    wf_cfg.entry_variant = entry_variant;
    wf_cfg.cisd_variant = cisd_variant;
    wf_cfg.reversal_confirm_mode = reversal_mode;
    wf_cfg.weekday_mask = weekday_mask;
    wf_cfg.killzone_mode = killzone_mode;
    wf_cfg.rr_target = rr_target;
    wf_cfg.poi_padding_bps = poi_padding_bps;
    wf_cfg.ob_sweep_tolerance_bps = ob_sweep_tolerance_bps;
    let wf_train_result = execute(TTradesFractalMTF {
        ltf_data: ltf_train,
        htf_data: Arc::new(htf_train),
        config: wf_cfg.clone(),
    });
    let wf_train = summarize(&wf_train_result);
    let wf_train_net = if mkt == "futures" {
        net_usd_1_micro(&wf_train_result, asset).to_string()
    } else {
        wf_train.0.to_string()
    };
    let wf_test_result = execute(TTradesFractalMTF {
        ltf_data: ltf_test,
        htf_data: Arc::new(htf_test),
        config: wf_cfg,
    });
    let wf_test = summarize(&wf_test_result);
    let wf_test_net = if mkt == "futures" {
        net_usd_1_micro(&wf_test_result, asset).to_string()
    } else {
        wf_test.0.to_string()
    };

    Row {
        strategy: "ttrades_fractal_mtf",
        market: mkt,
        asset,
        timeframe,
        net_profit_pct: base_net,
        net_unit,
        profit_factor: base.1,
        win_rate: base.2,
        trades: base.5,
        max_drawdown_pct: base.3,
        total_costs: base.4,
        wf_train_net,
        wf_test_net,
        wf_test_pf: wf_test.1.to_string(),
        validated_reality_check: "PARTIAL",
        setup_coverage: format!("entry={:?};cisd_variant={};confirm_mode={};time_filter={};opportunity={};rr={};poi_pad_bps={};ob_tol_bps={};htf_bias+poi+cisd/ifvg+ob;slippage={:?}", entry_variant, cisd_name, mode_name, time_profile_name, opportunity_name, rr_target, poi_padding_bps, ob_sweep_tolerance_bps, slippage_levels),
        final_verdict: verdict(base.0, base.1, base.5),
    }
}

fn md(rows: &[Row], capped_bars: usize, fast: bool) -> String {
    let mut out = String::new();
    out.push_str("# Strategy Validation Matrix\n\n");
    out.push_str("Scope: all implemented TTrades strategies, all available assets and reasonable timeframes.\n\n");
    out.push_str(&format!(
        "Data cap per dataset for this pass: {} bars.\n\n",
        capped_bars
    ));
    out.push_str(&format!("Mode: {}\n\n", if fast { "FAST" } else { "FULL" }));
    out.push_str(
        "Optimization: if naive (no fee/slippage) is not tradable, realism sweeps are skipped.\n\n",
    );
    out.push_str("Units: crypto net is `%`; futures net is `$` per 1 micro contract (MNQ=$2/pt, MES=$5/pt, GOLD=$10/pt).\n\n");
    out.push_str("| strategy | market | asset | timeframe | net_result | net_unit | profit_factor | win_rate_% | trades | max_dd_% | total_costs | wf_train_net | wf_test_net | wf_test_pf | validated_reality_check | setup_coverage | final_verdict |\n");
    out.push_str(
        "|---|---|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---|\n",
    );
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.strategy,
            r.market,
            r.asset,
            r.timeframe,
            r.net_profit_pct,
            r.net_unit,
            r.profit_factor,
            r.win_rate,
            r.trades,
            r.max_drawdown_pct,
            r.total_costs,
            r.wf_train_net,
            r.wf_test_net,
            r.wf_test_pf,
            r.validated_reality_check,
            r.setup_coverage,
            r.final_verdict,
        ));
    }
    out
}

fn main() {
    let matches = Command::new("ttrades_matrix")
        .arg(Arg::new("fast").long("fast").action(ArgAction::SetTrue))
        .get_matches();
    let fast = matches.get_flag("fast");

    let workers = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);
    rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build_global()
        .ok();

    let capped = if fast { 20_000usize } else { 40_000usize };
    let slippage_levels: Vec<i32> = if fast { vec![0, 1] } else { vec![0, 1, 2] };

    let btc_5m = Arc::new(cap(
        load_binance(include_str!("../assets/binance_BTCUSDT_5m.json")),
        capped,
    ));
    let btc_15m = Arc::new(cap(
        load_binance(include_str!("../assets/binance_BTCUSDT_15m.json")),
        capped,
    ));
    let btc_1h = Arc::new(cap(
        load_binance(include_str!("../assets/binance_BTCUSDT_1h.json")),
        capped,
    ));
    let btc_4h = Arc::new(cap(
        load_binance(include_str!("../assets/binance_BTCUSDT_4h.json")),
        capped,
    ));

    let eth_5m = Arc::new(cap(
        load_binance(include_str!("../assets/binance_ETHUSDT_5m.json")),
        capped,
    ));
    let eth_15m = Arc::new(cap(
        load_binance(include_str!("../assets/binance_ETHUSDT_15m.json")),
        capped,
    ));
    let eth_1h = Arc::new(cap(
        load_binance(include_str!("../assets/binance_ETHUSDT_1h.json")),
        capped,
    ));
    let eth_4h = Arc::new(cap(
        load_binance(include_str!("../assets/binance_ETHUSDT_4h.json")),
        capped,
    ));

    let sol_5m = Arc::new(cap(
        load_binance(include_str!("../assets/binance_SOLUSDT_5m.json")),
        capped,
    ));
    let sol_15m = Arc::new(cap(
        load_binance(include_str!("../assets/binance_SOLUSDT_15m.json")),
        capped,
    ));
    let sol_1h = Arc::new(cap(
        load_binance(include_str!("../assets/binance_SOLUSDT_1h.json")),
        capped,
    ));
    let sol_4h = Arc::new(cap(
        load_binance(include_str!("../assets/binance_SOLUSDT_4h.json")),
        capped,
    ));

    let gold_1m = Arc::new(cap(
        load_parquet("/Users/waff/develop/play/nq/gold_1m_cont_clean.parquet"),
        capped,
    ));
    let mnq_1m = Arc::new(cap(load_parquet("assets/mnq_1m_cont.parquet"), capped));
    let mes_1m = Arc::new(cap(load_parquet("assets/mes_1m_cont.parquet"), capped));

    let gold_15m = Arc::new(resample_from_1m(&gold_1m, 15));
    let mnq_15m = Arc::new(resample_from_1m(&mnq_1m, 15));
    let mes_15m = Arc::new(resample_from_1m(&mes_1m, 15));
    let gold_5m = Arc::new(resample_from_1m(&gold_1m, 5));
    let mnq_5m = Arc::new(resample_from_1m(&mnq_1m, 5));
    let mes_5m = Arc::new(resample_from_1m(&mes_1m, 5));
    let gold_1h = Arc::new(resample_from_1m(&gold_1m, 60));
    let mnq_1h = Arc::new(resample_from_1m(&mnq_1m, 60));
    let mes_1h = Arc::new(resample_from_1m(&mes_1m, 60));
    let gold_4h = Arc::new(resample_from_1m(&gold_1m, 240));
    let mnq_4h = Arc::new(resample_from_1m(&mnq_1m, 240));
    let mes_4h = Arc::new(resample_from_1m(&mes_1m, 240));

    let fractal_jobs: Vec<(&'static str, &'static str, Arc<Vec<CandleStick>>, Decimal)> = vec![
        (
            "BTC",
            "5m",
            Arc::clone(&btc_5m),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "BTC",
            "15m",
            Arc::clone(&btc_15m),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "BTC",
            "1h",
            Arc::clone(&btc_1h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "BTC",
            "4h",
            Arc::clone(&btc_4h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "ETH",
            "5m",
            Arc::clone(&eth_5m),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "ETH",
            "15m",
            Arc::clone(&eth_15m),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "ETH",
            "1h",
            Arc::clone(&eth_1h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "ETH",
            "4h",
            Arc::clone(&eth_4h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "SOL",
            "5m",
            Arc::clone(&sol_5m),
            Decimal::from_f32(0.001).unwrap(),
        ),
        (
            "SOL",
            "15m",
            Arc::clone(&sol_15m),
            Decimal::from_f32(0.001).unwrap(),
        ),
        (
            "SOL",
            "1h",
            Arc::clone(&sol_1h),
            Decimal::from_f32(0.001).unwrap(),
        ),
        (
            "SOL",
            "4h",
            Arc::clone(&sol_4h),
            Decimal::from_f32(0.001).unwrap(),
        ),
        (
            "GOLD",
            "1m",
            Arc::clone(&gold_1m),
            Decimal::from_f32(0.1).unwrap(),
        ),
        (
            "MNQ",
            "1m",
            Arc::clone(&mnq_1m),
            Decimal::from_f32(0.25).unwrap(),
        ),
        (
            "MES",
            "1m",
            Arc::clone(&mes_1m),
            Decimal::from_f32(0.25).unwrap(),
        ),
    ];

    let base_mtf_jobs: Vec<(
        &'static str,
        &'static str,
        Arc<Vec<CandleStick>>,
        Arc<Vec<CandleStick>>,
        Decimal,
    )> = vec![
        (
            "BTC",
            "5m/1h",
            Arc::clone(&btc_5m),
            Arc::clone(&btc_1h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "BTC",
            "15m/4h",
            Arc::clone(&btc_15m),
            Arc::clone(&btc_4h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "ETH",
            "5m/1h",
            Arc::clone(&eth_5m),
            Arc::clone(&eth_1h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "ETH",
            "15m/4h",
            Arc::clone(&eth_15m),
            Arc::clone(&eth_4h),
            Decimal::from_f32(0.01).unwrap(),
        ),
        (
            "SOL",
            "5m/1h",
            Arc::clone(&sol_5m),
            Arc::clone(&sol_1h),
            Decimal::from_f32(0.001).unwrap(),
        ),
        (
            "SOL",
            "15m/4h",
            Arc::clone(&sol_15m),
            Arc::clone(&sol_4h),
            Decimal::from_f32(0.001).unwrap(),
        ),
        (
            "GOLD",
            "5m/1h",
            Arc::clone(&gold_5m),
            Arc::clone(&gold_1h),
            Decimal::from_f32(0.1).unwrap(),
        ),
        (
            "MNQ",
            "5m/1h",
            Arc::clone(&mnq_5m),
            Arc::clone(&mnq_1h),
            Decimal::from_f32(0.25).unwrap(),
        ),
        (
            "MES",
            "5m/1h",
            Arc::clone(&mes_5m),
            Arc::clone(&mes_1h),
            Decimal::from_f32(0.25).unwrap(),
        ),
        (
            "GOLD",
            "15m/4h",
            Arc::clone(&gold_15m),
            Arc::clone(&gold_4h),
            Decimal::from_f32(0.1).unwrap(),
        ),
        (
            "MNQ",
            "15m/4h",
            Arc::clone(&mnq_15m),
            Arc::clone(&mnq_4h),
            Decimal::from_f32(0.25).unwrap(),
        ),
        (
            "MES",
            "15m/4h",
            Arc::clone(&mes_15m),
            Arc::clone(&mes_4h),
            Decimal::from_f32(0.25).unwrap(),
        ),
        (
            "GOLD",
            "1m/15m",
            Arc::clone(&gold_1m),
            Arc::clone(&gold_15m),
            Decimal::from_f32(0.1).unwrap(),
        ),
        (
            "MNQ",
            "1m/15m",
            Arc::clone(&mnq_1m),
            Arc::clone(&mnq_15m),
            Decimal::from_f32(0.25).unwrap(),
        ),
        (
            "MES",
            "1m/15m",
            Arc::clone(&mes_1m),
            Arc::clone(&mes_15m),
            Decimal::from_f32(0.25).unwrap(),
        ),
    ];

    let reversal_modes: Vec<(&'static str, ReversalConfirmMode)> = vec![
        ("cisd_only", ReversalConfirmMode::CisdOnly),
        ("ifvg_only", ReversalConfirmMode::IfvgOnly),
        ("cisd_and_ifvg", ReversalConfirmMode::CisdAndIfvg),
        ("cisd_or_ifvg", ReversalConfirmMode::CisdOrIfvg),
    ];
    let cisd_variants: Vec<(&'static str, CisdVariant)> = vec![
        ("body_flip", CisdVariant::BodyFlip),
        ("strict_wick_break", CisdVariant::StrictWickBreak),
        ("last_series_close_break", CisdVariant::LastSeriesCloseBreak),
        ("failure_swing", CisdVariant::FailureSwing),
    ];

    let time_profiles: Vec<(&'static str, u8, KillzoneMode)> = vec![
        ("all_day_all_week", 0b0111_1111, KillzoneMode::Off),
        ("ny_weekdays", 0b0001_1111, KillzoneMode::NyOnly),
        ("london_ny_weekdays", 0b0001_1111, KillzoneMode::LondonNy),
    ];

    let baseline_opportunity = (
        "baseline",
        Decimal::from(2),
        EntryVariant::ObMidpoint,
        0i32,
        0i32,
    );
    let selective_opportunities: Vec<(&'static str, Decimal, EntryVariant, i32, i32)> = vec![
        (
            "more_hits_close_rr15",
            Decimal::new(15, 1),
            EntryVariant::Close,
            5,
            5,
        ),
        (
            "more_hits_ob_level_rr15",
            Decimal::new(15, 1),
            EntryVariant::ObLevel,
            10,
            8,
        ),
        (
            "more_hits_close_rr12",
            Decimal::new(12, 1),
            EntryVariant::Close,
            10,
            10,
        ),
        (
            "more_hits_ob_mid_rr15",
            Decimal::new(15, 1),
            EntryVariant::ObMidpoint,
            10,
            8,
        ),
    ];

    let mut mtf_jobs: Vec<(
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Arc<Vec<CandleStick>>,
        Arc<Vec<CandleStick>>,
        Decimal,
        CisdVariant,
        ReversalConfirmMode,
        u8,
        KillzoneMode,
        Decimal,
        EntryVariant,
        i32,
        i32,
    )> = vec![];
    for (asset, tf, ltf, htf, tick) in base_mtf_jobs {
        for (cisd_name, cisd_variant) in &cisd_variants {
            for (mode_name, mode) in &reversal_modes {
                for (time_profile_name, weekday_mask, killzone_mode) in &time_profiles {
                    mtf_jobs.push((
                        asset,
                        tf,
                        *mode_name,
                        *time_profile_name,
                        baseline_opportunity.0,
                        *cisd_name,
                        Arc::clone(&ltf),
                        Arc::clone(&htf),
                        tick,
                        *cisd_variant,
                        *mode,
                        *weekday_mask,
                        *killzone_mode,
                        baseline_opportunity.1,
                        baseline_opportunity.2,
                        baseline_opportunity.3,
                        baseline_opportunity.4,
                    ));

                    let selective_target = matches!(
                        (asset, tf),
                        ("ETH", "5m/1h") | ("MES", "1m/15m") | ("GOLD", "1m/15m")
                    );
                    let selective_mode = matches!(*mode_name, "ifvg_only" | "cisd_and_ifvg");
                    let selective_time = matches!(
                        *time_profile_name,
                        "all_day_all_week" | "london_ny_weekdays"
                    );

                    if selective_target && selective_mode && selective_time {
                        for (
                            opportunity_name,
                            rr_target,
                            entry_variant,
                            poi_padding_bps,
                            ob_sweep_tolerance_bps,
                        ) in &selective_opportunities
                        {
                            mtf_jobs.push((
                                asset,
                                tf,
                                *mode_name,
                                *time_profile_name,
                                *opportunity_name,
                                *cisd_name,
                                Arc::clone(&ltf),
                                Arc::clone(&htf),
                                tick,
                                *cisd_variant,
                                *mode,
                                *weekday_mask,
                                *killzone_mode,
                                *rr_target,
                                *entry_variant,
                                *poi_padding_bps,
                                *ob_sweep_tolerance_bps,
                            ));
                        }
                    }
                }
            }
        }
    }

    let mut rows: Vec<Row> = fractal_jobs
        .par_iter()
        .map(|(asset, tf, data, tick)| {
            run_fractal_case(asset, tf, Arc::clone(data), *tick, &slippage_levels)
        })
        .collect();

    let mtf_rows: Vec<Row> = mtf_jobs
        .par_iter()
        .map(
            |(
                asset,
                tf,
                mode_name,
                time_profile_name,
                opportunity_name,
                cisd_name,
                ltf,
                htf,
                tick,
                cisd_variant,
                mode,
                weekday_mask,
                killzone_mode,
                rr_target,
                entry_variant,
                poi_padding_bps,
                ob_sweep_tolerance_bps,
            )| {
                run_mtf_case(
                    asset,
                    tf,
                    mode_name,
                    time_profile_name,
                    opportunity_name,
                    Arc::clone(ltf),
                    Arc::clone(htf),
                    *tick,
                    &slippage_levels,
                    *cisd_name,
                    *cisd_variant,
                    *mode,
                    *weekday_mask,
                    *killzone_mode,
                    *rr_target,
                    *entry_variant,
                    *poi_padding_bps,
                    *ob_sweep_tolerance_bps,
                )
            },
        )
        .collect();
    rows.extend(mtf_rows);

    rows.sort_by(|a, b| {
        a.strategy
            .cmp(b.strategy)
            .then_with(|| a.asset.cmp(b.asset))
            .then_with(|| a.timeframe.cmp(b.timeframe))
    });

    let report = md(&rows, capped, fast);
    let out_dir = "reports/strategy_overviews";
    fs::create_dir_all(out_dir).expect("failed creating report directory");
    let out_path = format!("{}/STRATEGY_VALIDATION_MATRIX.md", out_dir);
    fs::write(&out_path, report).expect("failed writing report");
    println!("Wrote {} with {} rows", out_path, rows.len());
}
