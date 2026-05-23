extern crate rust_decimal;

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::execute;
use backtest::model::candle_stick::CandleStick;
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
    wins: usize,
    win_rate: Decimal,
    pf: Decimal,
    net_usd_1eth: Decimal,
}

#[derive(Clone)]
struct RobustRow {
    timeframe: &'static str,
    mode: &'static str,
    cisd: &'static str,
    time_profile: &'static str,
    opportunity: &'static str,
    trades: usize,
    wins: usize,
    win_rate: Decimal,
    pf: Decimal,
    net_min_usd: Decimal,
    net_avg_usd: Decimal,
    net_max_usd: Decimal,
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn summarize_fixed_1eth(res: &backtest::model::backtest_result::BacktestResult) -> (usize, usize, Decimal, Decimal, Decimal) {
    let trades = res.trades.len();
    let wins = res
        .trades
        .iter()
        .filter(|t| matches!(t.result, backtest::model::trade_result::TradeResult::Winner))
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
        let pnl = t.points().0 - t.total_costs();
        net += pnl;
        if pnl > Decimal::ZERO {
            gp += pnl;
        } else if pnl < Decimal::ZERO {
            gl += -pnl;
        }
    }
    let pf = if gl > Decimal::ZERO {
        (gp / gl).round_dp(2)
    } else {
        Decimal::ZERO
    };

    (trades, wins, win_rate, pf, net.round_dp(2))
}

fn main() {
    let eth_5m = Arc::new(load("assets/binance_ETHUSDT_5m.json"));
    let eth_1h = Arc::new(load("assets/binance_ETHUSDT_1h.json"));
    let eth_15m = Arc::new(load("assets/binance_ETHUSDT_15m.json"));
    let eth_4h = Arc::new(load("assets/binance_ETHUSDT_4h.json"));

    let base_jobs: Vec<(&'static str, Arc<Vec<CandleStick>>, Arc<Vec<CandleStick>>)> = vec![
        ("5m/1h", Arc::clone(&eth_5m), Arc::clone(&eth_1h)),
        ("15m/4h", Arc::clone(&eth_15m), Arc::clone(&eth_4h)),
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
    let opportunities: Vec<(&'static str, Decimal, EntryVariant, i32, i32)> = vec![
        ("baseline", Decimal::from(2), EntryVariant::ObMidpoint, 0, 0),
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
        (
            "ultra_hits_close_rr10",
            Decimal::new(10, 1),
            EntryVariant::Close,
            15,
            20,
        ),
        (
            "ultra_hits_close_rr08",
            Decimal::new(8, 1),
            EntryVariant::Close,
            20,
            25,
        ),
        (
            "ultra_hits_ob_level_rr10",
            Decimal::new(10, 1),
            EntryVariant::ObLevel,
            15,
            20,
        ),
        (
            "max_hits_close_rr06",
            Decimal::new(6, 1),
            EntryVariant::Close,
            30,
            40,
        ),
        (
            "max_hits_close_rr05",
            Decimal::new(5, 1),
            EntryVariant::Close,
            35,
            50,
        ),
        (
            "max_hits_ob_level_rr06",
            Decimal::new(6, 1),
            EntryVariant::ObLevel,
            30,
            40,
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
                            cfg.tick_size = Decimal::from_f32(0.01).unwrap();
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
                            cfg.fee_config = backtest::model::fee_config::FeeConfig::binance_standard();

                            let result = execute(TTradesFractalMTF {
                                ltf_data: Arc::clone(&ltf),
                                htf_data: Arc::clone(&htf),
                                config: cfg,
                            });
                            let (trades, wins, win_rate, pf, net) = summarize_fixed_1eth(&result);
                            rows.push(Row {
                                timeframe: tf,
                                mode: *mode_name,
                                cisd: *cisd_name,
                                time_profile: *tp_name,
                                opportunity: *opp_name,
                                slippage: slip,
                                trades,
                                wins,
                                win_rate,
                                pf,
                                net_usd_1eth: net,
                            });
                        }
                    }
                }
            }
        }
    }

    rows.sort_by(|a, b| {
        b.net_usd_1eth
            .cmp(&a.net_usd_1eth)
            .then(b.pf.cmp(&a.pf))
            .then(b.trades.cmp(&a.trades))
    });

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

    let mut robust: Vec<RobustRow> = Vec::new();
    for ((timeframe, mode, cisd, time_profile, opportunity), rs) in grouped {
        if rs.len() != 3 {
            continue;
        }
        let mut nets: Vec<Decimal> = rs.iter().map(|r| r.net_usd_1eth).collect();
        nets.sort();
        let net_min = nets[0];
        let net_max = nets[nets.len() - 1];
        let net_avg = (nets.iter().copied().sum::<Decimal>() / Decimal::from(3)).round_dp(2);

        // slips are modeled as only cost deltas here, so trade count/winrate/pf are stable.
        let ref_row = rs
            .iter()
            .find(|r| r.slippage == 1)
            .cloned()
            .unwrap_or_else(|| rs[0].clone());

        robust.push(RobustRow {
            timeframe,
            mode,
            cisd,
            time_profile,
            opportunity,
            trades: ref_row.trades,
            wins: ref_row.wins,
            win_rate: ref_row.win_rate,
            pf: ref_row.pf,
            net_min_usd: net_min,
            net_avg_usd: net_avg,
            net_max_usd: net_max,
        });
    }

    robust.sort_by(|a, b| {
        b.net_min_usd
            .cmp(&a.net_min_usd)
            .then(b.net_avg_usd.cmp(&a.net_avg_usd))
            .then(b.pf.cmp(&a.pf))
            .then(b.trades.cmp(&a.trades))
    });

    let mut dense = robust.clone();
    dense.sort_by(|a, b| {
        b.trades
            .cmp(&a.trades)
            .then(b.net_min_usd.cmp(&a.net_min_usd))
            .then(b.pf.cmp(&a.pf))
    });

    let mut out = String::new();
    out.push_str("# ETH-only TTrades MTF Sweeps (Fixed 1 ETH)\n\n");
    out.push_str("- Strategy: ttrades_fractal_mtf\n");
    out.push_str("- Sizing: fixed 1 ETH per trade\n");
    out.push_str("- Costs: Binance standard fee config + slippage 1/2/3 ticks per side\n");
    out.push_str("- Timeframes: 5m/1h and 15m/4h\n\n");

    for tf in ["5m/1h", "15m/4h"] {
        out.push_str(&format!("## Top 15 - {}\n\n", tf));
        out.push_str("| rank | net_usd_1eth | pf | win_rate_% | trades | wins | mode | cisd | time_profile | opportunity | slip |\n");
        out.push_str("|---:|---:|---:|---:|---:|---:|---|---|---|---|---:|\n");
        for (idx, r) in rows.iter().filter(|r| r.timeframe == tf).take(15).enumerate() {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                idx + 1,
                r.net_usd_1eth,
                r.pf,
                r.win_rate,
                r.trades,
                r.wins,
                r.mode,
                r.cisd,
                r.time_profile,
                r.opportunity,
                r.slippage
            ));
        }
        out.push('\n');
    }

    out.push_str("## Robust Top 20 (ranked by worst-case slip net)\n\n");
    out.push_str("| rank | timeframe | net_min_usd | net_avg_usd | net_max_usd | pf | win_rate_% | trades | wins | mode | cisd | time_profile | opportunity |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|\n");
    for (idx, r) in robust.iter().take(20).enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            idx + 1,
            r.timeframe,
            r.net_min_usd,
            r.net_avg_usd,
            r.net_max_usd,
            r.pf,
            r.win_rate,
            r.trades,
            r.wins,
            r.mode,
            r.cisd,
            r.time_profile,
            r.opportunity
        ));
    }
    out.push('\n');

    out.push_str("## Trade Density Top 20 (positive worst-case slip)\n\n");
    out.push_str("| rank | timeframe | trades | wins | win_rate_% | pf | net_min_usd | net_avg_usd | net_max_usd | mode | cisd | time_profile | opportunity |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|\n");
    let mut rank = 1usize;
    for r in dense.iter().filter(|r| r.net_min_usd > Decimal::ZERO).take(20) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            rank,
            r.timeframe,
            r.trades,
            r.wins,
            r.win_rate,
            r.pf,
            r.net_min_usd,
            r.net_avg_usd,
            r.net_max_usd,
            r.mode,
            r.cisd,
            r.time_profile,
            r.opportunity
        ));
        rank += 1;
    }
    out.push('\n');

    out.push_str("## Objective: High Trade Count (trades>=200, PF>=1.0)\n\n");
    out.push_str("| rank | timeframe | trades | wins | win_rate_% | pf | net_min_usd | net_avg_usd | net_max_usd | mode | cisd | time_profile | opportunity |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|\n");
    let mut objective_rank = 1usize;
    for r in dense
        .iter()
        .filter(|r| r.trades >= 200 && r.pf >= Decimal::ONE)
        .take(20)
    {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            objective_rank,
            r.timeframe,
            r.trades,
            r.wins,
            r.win_rate,
            r.pf,
            r.net_min_usd,
            r.net_avg_usd,
            r.net_max_usd,
            r.mode,
            r.cisd,
            r.time_profile,
            r.opportunity
        ));
        objective_rank += 1;
    }
    if objective_rank == 1 {
        out.push_str("| - | - | - | - | - | - | - | - | - | - | - | - | no qualifying preset in current grid |\n");
    }
    out.push('\n');

    let out_path = "reports/strategy_overviews/ETH_MTF_SWEEP_FIXED_1ETH.md";
    fs::write(out_path, out).expect("write ETH sweep report");
    println!("Wrote {} ({} rows)", out_path, rows.len());
}
