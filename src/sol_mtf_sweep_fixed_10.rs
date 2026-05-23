extern crate rust_decimal;

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::execute;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::fee_config::FeeConfig;
use backtest::model::trade_result::TradeResult;
use backtest::strategies::ttrades_fractal_mtf::{
    CisdVariant, EntryVariant, FractalMTFConfig, KillzoneMode, ReversalConfirmMode,
    TTradesFractalMTF,
};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

#[derive(Clone)]
struct Row {
    timeframe: &'static str,
    mode: &'static str,
    cisd: &'static str,
    time_profile: &'static str,
    opportunity: &'static str,
    slippage: i32,
    trades: usize,
    win_rate: Decimal,
    pf: Decimal,
    net_usd_10sol: Decimal,
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn summarize_fixed_10sol(res: &BacktestResult) -> (usize, Decimal, Decimal, Decimal) {
    let trades = res.trades.len();
    let wins = res
        .trades
        .iter()
        .filter(|t| matches!(t.result, TradeResult::Winner))
        .count();
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_i32(wins as i32).unwrap() / Decimal::from_i32(trades as i32).unwrap()
            * Decimal::from(100))
        .round_dp(2)
    };
    let mut gp = Decimal::ZERO;
    let mut gl = Decimal::ZERO;
    let mut net = Decimal::ZERO;
    for t in &res.trades {
        let pnl_10 = (t.points().0 - t.total_costs()) * Decimal::from(10);
        net += pnl_10;
        if pnl_10 > Decimal::ZERO {
            gp += pnl_10;
        } else if pnl_10 < Decimal::ZERO {
            gl += -pnl_10;
        }
    }
    let pf = if gl > Decimal::ZERO {
        (gp / gl).round_dp(2)
    } else {
        Decimal::ZERO
    };
    (trades, win_rate, pf, net.round_dp(2))
}

fn main() {
    let sol_5m = Arc::new(load("assets/binance_SOLUSDT_5m.json"));
    let sol_1h = Arc::new(load("assets/binance_SOLUSDT_1h.json"));
    let sol_15m = Arc::new(load("assets/binance_SOLUSDT_15m.json"));
    let sol_4h = Arc::new(load("assets/binance_SOLUSDT_4h.json"));

    let base_jobs: Vec<(&'static str, Arc<Vec<CandleStick>>, Arc<Vec<CandleStick>>)> = vec![
        ("5m/1h", Arc::clone(&sol_5m), Arc::clone(&sol_1h)),
        ("15m/4h", Arc::clone(&sol_15m), Arc::clone(&sol_4h)),
    ];

    let reversal_modes = vec![
        ("cisd_only", ReversalConfirmMode::CisdOnly),
        ("ifvg_only", ReversalConfirmMode::IfvgOnly),
        ("cisd_and_ifvg", ReversalConfirmMode::CisdAndIfvg),
        ("cisd_or_ifvg", ReversalConfirmMode::CisdOrIfvg),
    ];
    let cisd_variants = vec![
        ("body_flip", CisdVariant::BodyFlip),
        ("strict_wick_break", CisdVariant::StrictWickBreak),
        ("last_series_close_break", CisdVariant::LastSeriesCloseBreak),
        ("failure_swing", CisdVariant::FailureSwing),
    ];
    let time_profiles = vec![
        ("all_day_all_week", 0b0111_1111, KillzoneMode::Off),
        ("ny_weekdays", 0b0001_1111, KillzoneMode::NyOnly),
        ("london_ny_weekdays", 0b0001_1111, KillzoneMode::LondonNy),
    ];
    let opportunities = vec![
        ("baseline", Decimal::from(2), EntryVariant::ObMidpoint, 0, 0),
        (
            "more_hits_close_rr15",
            Decimal::new(15, 1),
            EntryVariant::Close,
            5,
            5,
        ),
        (
            "more_hits_close_rr12",
            Decimal::new(12, 1),
            EntryVariant::Close,
            10,
            10,
        ),
    ];
    let slips = [1, 2, 3];

    let mut rows: Vec<Row> = Vec::new();
    for (tf, ltf, htf) in base_jobs {
        for (mode_name, mode) in &reversal_modes {
            for (cisd_name, cisd_variant) in &cisd_variants {
                for (tp_name, weekday_mask, kz_mode) in &time_profiles {
                    for (opp_name, rr_target, entry_variant, poi_bps, ob_bps) in &opportunities {
                        for slip in slips {
                            let mut cfg = FractalMTFConfig::default();
                            cfg.tick_size = Decimal::from_f32(0.001).unwrap();
                            cfg.slippage_ticks_per_side = slip;
                            cfg.log_progress = false;
                            cfg.reversal_confirm_mode = *mode;
                            cfg.cisd_variant = *cisd_variant;
                            cfg.weekday_mask = *weekday_mask;
                            cfg.killzone_mode = *kz_mode;
                            cfg.rr_target = *rr_target;
                            cfg.entry_variant = *entry_variant;
                            cfg.poi_padding_bps = *poi_bps;
                            cfg.ob_sweep_tolerance_bps = *ob_bps;
                            cfg.fee_config = FeeConfig::binance_standard();

                            let result = execute(TTradesFractalMTF {
                                ltf_data: Arc::clone(&ltf),
                                htf_data: Arc::clone(&htf),
                                config: cfg,
                            });
                            let (trades, win_rate, pf, net) = summarize_fixed_10sol(&result);
                            rows.push(Row {
                                timeframe: tf,
                                mode: *mode_name,
                                cisd: *cisd_name,
                                time_profile: *tp_name,
                                opportunity: *opp_name,
                                slippage: slip,
                                trades,
                                win_rate,
                                pf,
                                net_usd_10sol: net,
                            });
                        }
                    }
                }
            }
        }
    }

    let mut grouped: BTreeMap<
        (&'static str, &'static str, &'static str, &'static str, &'static str),
        Vec<Row>,
    > = BTreeMap::new();
    for r in rows.iter().cloned() {
        grouped
            .entry((r.timeframe, r.mode, r.cisd, r.time_profile, r.opportunity))
            .or_default()
            .push(r);
    }

    let mut robust: Vec<(String, usize, Decimal, Decimal, Decimal, Decimal, Decimal)> = Vec::new();
    for ((tf, mode, cisd, tp, opp), rs) in grouped {
        if rs.len() != 3 {
            continue;
        }
        let mut nets: Vec<Decimal> = rs.iter().map(|r| r.net_usd_10sol).collect();
        nets.sort();
        let net_min = nets[0];
        let net_avg = (nets.iter().copied().sum::<Decimal>() / Decimal::from(3)).round_dp(2);
        let net_max = nets[2];
        let ref_row = rs.iter().find(|r| r.slippage == 1).cloned().unwrap_or(rs[0].clone());
        robust.push((
            format!("{} | {} | {} | {} | {}", tf, mode, cisd, tp, opp),
            ref_row.trades,
            ref_row.win_rate,
            ref_row.pf,
            net_min,
            net_avg,
            net_max,
        ));
    }
    robust.sort_by(|a, b| b.4.cmp(&a.4).then(b.5.cmp(&a.5)).then(b.3.cmp(&a.3)));

    let mut out = String::new();
    out.push_str("# SOL TTrades MTF Sweep (Fixed 10 Contracts)\n\n");
    out.push_str("| rank | setup | trades | win_rate_% | pf | net_min_usd | net_avg_usd | net_max_usd |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|---:|\n");
    for (i, r) in robust.iter().take(30).enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            r.0,
            r.1,
            r.2,
            r.3,
            r.4,
            r.5,
            r.6
        ));
    }

    let out_path = "reports/strategy_overviews/SOL_TTRADES_FIXED_10_SWEEP.md";
    fs::write(out_path, out).expect("write report");
    println!("Wrote {} ({} rows)", out_path, rows.len());
}
