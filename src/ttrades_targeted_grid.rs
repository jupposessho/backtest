extern crate rust_decimal;

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::fs;
use std::sync::Arc;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, fee_config::FeeConfig, trade_result::TradeResult},
    strategies::ttrades_fractal_mtf::{
        CisdVariant, EntryVariant, FractalMTFConfig, ReversalConfirmMode, TTradesFractalMTF,
    },
};

fn summarize(result: &BacktestResult) -> (Decimal, Decimal, Decimal, Decimal, usize) {
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
    let mut gp = Decimal::ZERO;
    let mut gl = Decimal::ZERO;
    let r = Decimal::from_f32(0.01).unwrap();

    for t in &result.trades {
        let change = capital * r * t.gross_r().trunc_with_scale(4) - t.total_costs();
        if change > Decimal::ZERO {
            gp += change;
        } else if change < Decimal::ZERO {
            gl += -change;
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

    let pf = if gl > Decimal::ZERO { (gp / gl).round_dp(2) } else { Decimal::ZERO };
    let net = ((capital - Decimal::from(1000)) / Decimal::from(1000) * Decimal::from(100)).round_dp(2);
    (net, pf, win_rate, max_dd, total)
}

fn split_train_test(data: &Arc<Vec<CandleStick>>) -> (Arc<Vec<CandleStick>>, Arc<Vec<CandleStick>>) {
    let n = data.len();
    let split = ((n as f64) * 0.7).floor() as usize;
    let split = split.clamp(1, n.saturating_sub(1));
    (Arc::new(data[..split].to_vec()), Arc::new(data[split..].to_vec()))
}

fn main() {
    let capped = 60_000usize;
    let mut ltf = CandleStickLoader::load_binance(include_str!("../assets/binance_SOLUSDT_15m.json"));
    let mut htf = CandleStickLoader::load_binance(include_str!("../assets/binance_SOLUSDT_4h.json"));
    if ltf.len() > capped { ltf.truncate(capped); }
    if let Some(last) = ltf.last().map(|c| c.open_time) {
        htf.retain(|c| c.open_time <= last);
    }

    let ltf = Arc::new(ltf);
    let htf = Arc::new(htf);
    let (ltf_train, ltf_test) = split_train_test(&ltf);
    let mut htf_train = (*htf).clone();
    let mut htf_test = (*htf).clone();
    if let Some(last_train) = ltf_train.last().map(|c| c.open_time) { htf_train.retain(|c| c.open_time <= last_train); }
    if let Some(first_test) = ltf_test.first().map(|c| c.open_time) { htf_test.retain(|c| c.open_time >= first_test); }
    let htf_train = Arc::new(htf_train);
    let htf_test = Arc::new(htf_test);

    let rrs = [Decimal::from(15) / Decimal::from(10), Decimal::from(2), Decimal::from(25) / Decimal::from(10), Decimal::from(3)];
    let fees = [
        ("zero", FeeConfig::zero()),
        ("binance_std", FeeConfig::binance_standard()),
        ("conservative", FeeConfig::conservative()),
    ];
    let slips = [0, 1, 2, 3];

    let mut lines = vec![
        "# TTrades Targeted Grid (SOL 15m/4h MTF)".to_string(),
        "".to_string(),
        "| cisd_variant | reversal_mode | entry_variant | rr | fee_profile | slippage_ticks | train_net_% | test_net_% | test_pf | test_win_% | trades_test | verdict |".to_string(),
        "|---|---|---|---:|---|---:|---:|---:|---:|---:|---:|---|".to_string(),
    ];

    let entry_variants = [
        ("close", EntryVariant::Close),
        ("ob_level", EntryVariant::ObLevel),
        ("ob_midpoint", EntryVariant::ObMidpoint),
    ];
    let cisd_variants = [
        ("body_flip", CisdVariant::BodyFlip),
        ("strict_wick_break", CisdVariant::StrictWickBreak),
        ("last_series_close_break", CisdVariant::LastSeriesCloseBreak),
    ];
    let reversal_modes = [
        ("cisd_only", ReversalConfirmMode::CisdOnly),
        ("ifvg_only", ReversalConfirmMode::IfvgOnly),
        ("cisd_and_ifvg", ReversalConfirmMode::CisdAndIfvg),
        ("cisd_or_ifvg", ReversalConfirmMode::CisdOrIfvg),
    ];

    for (cisd_name, cisd_variant) in cisd_variants {
        for (mode_name, mode) in reversal_modes {
            for (entry_name, entry_variant) in entry_variants {
            for rr in rrs {
                for (fee_name, fee_cfg) in fees {
                    for slip in slips {
                let mut cfg = FractalMTFConfig::default();
                cfg.rr_target = rr;
                cfg.fee_config = fee_cfg;
                cfg.slippage_ticks_per_side = slip;
                cfg.tick_size = Decimal::from_f32(0.001).unwrap();
                cfg.log_progress = false;
                cfg.entry_variant = entry_variant;
                cfg.cisd_variant = cisd_variant;
                cfg.reversal_confirm_mode = mode;

                let train = execute(TTradesFractalMTF { ltf_data: Arc::clone(&ltf_train), htf_data: Arc::clone(&htf_train), config: cfg.clone() });
                let test = execute(TTradesFractalMTF { ltf_data: Arc::clone(&ltf_test), htf_data: Arc::clone(&htf_test), config: cfg });
                let tr = summarize(&train);
                let te = summarize(&test);
                let verdict = if te.0 > Decimal::ZERO && te.1 >= Decimal::from_f32(1.2).unwrap() { "PROMOTE" } else { "REJECT" };
                lines.push(format!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                    cisd_name, mode_name, entry_name, rr.round_dp(2), fee_name, slip, tr.0, te.0, te.1, te.2, te.4, verdict
                ));
                    }
                }
            }
        }
        }
    }

    let out_dir = "reports/strategy_overviews";
    fs::create_dir_all(out_dir).expect("create report directory");
    let out_path = format!("{}/TTRADES_TARGETED_GRID_SOL_MTF.md", out_dir);
    fs::write(&out_path, lines.join("\n") + "\n").expect("write report");
    println!("Wrote {}", out_path);
}
