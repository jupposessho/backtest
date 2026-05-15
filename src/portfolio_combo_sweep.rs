extern crate rust_decimal;

use std::collections::BTreeMap;
use std::sync::Arc;

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::execute;
use backtest::model::candle_stick::CandleStick;
use backtest::strategies::ttrades_fractal_mtf::{
    EntryVariant, FractalMTFConfig, KillzoneMode, ReversalConfirmMode, TTradesFractalMTF,
};
use backtest::to_new_york_time;
use chrono::Datelike;
use rust_decimal::Decimal;

#[derive(Clone, Copy)]
struct LegCfg {
    name: &'static str,
    rr: Decimal,
    poi: i32,
    ob: i32,
    kz: KillzoneMode,
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn eval_leg(
    ltf: Arc<Vec<CandleStick>>,
    mut htf: Vec<CandleStick>,
    tick: Decimal,
    cfgv: LegCfg,
    size_mult_vs_10: Decimal,
) -> BTreeMap<String, Decimal> {
    if let Some(last) = ltf.last().map(|c| c.open_time) {
        htf.retain(|c| c.open_time <= last);
    }

    let mut cfg = FractalMTFConfig::default();
    cfg.tick_size = tick;
    cfg.slippage_ticks_per_side = 0;
    cfg.log_progress = false;
    cfg.entry_variant = EntryVariant::Close;
    cfg.reversal_confirm_mode = ReversalConfirmMode::IfvgOnly;
    cfg.weekday_mask = 0b0111_1111;
    cfg.killzone_mode = cfgv.kz;
    cfg.rr_target = cfgv.rr;
    cfg.poi_padding_bps = cfgv.poi;
    cfg.ob_sweep_tolerance_bps = cfgv.ob;

    let res = execute(TTradesFractalMTF {
        ltf_data: ltf,
        htf_data: Arc::new(htf),
        config: cfg,
    });

    let mut by_month = BTreeMap::<String, Decimal>::new();
    for t in &res.trades {
        let dt = to_new_york_time(t.close_time);
        let k = format!("{:04}-{:02}", dt.year(), dt.month());
        let pnl_10 = (t.points().0 - t.total_costs()) * Decimal::from(10);
        *by_month.entry(k).or_insert(Decimal::ZERO) += pnl_10 * size_mult_vs_10;
    }
    by_month
}

fn score_last6(m: &BTreeMap<String, Decimal>) -> (Decimal, usize, Vec<(String, Decimal)>) {
    let v: Vec<_> = m.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let n = v.len().min(6);
    let last = v[v.len().saturating_sub(n)..].to_vec();
    let mut total = Decimal::ZERO;
    let mut pos = 0usize;
    for (_, x) in &last {
        total += *x;
        if *x > Decimal::ZERO {
            pos += 1;
        }
    }
    (total.round_dp(2), pos, last)
}

fn main() {
    let btc5 = Arc::new(load("assets/binance_BTCUSDT_15m.json"));
    let btc4 = load("assets/binance_BTCUSDT_4h.json");
    let eth5 = Arc::new(load("assets/binance_ETHUSDT_15m.json"));
    let eth4 = load("assets/binance_ETHUSDT_4h.json");
    let sol5 = Arc::new(load("assets/binance_SOLUSDT_15m.json"));
    let sol4 = load("assets/binance_SOLUSDT_4h.json");

    // top-3 per asset from recent tune output
    let btc_cfgs = [
        LegCfg {
            name: "btc_rr2_poi0_ob5_all_day",
            rr: Decimal::from(2),
            poi: 0,
            ob: 5,
            kz: KillzoneMode::Off,
        },
        LegCfg {
            name: "btc_rr1.8_poi0_ob5_all_day",
            rr: Decimal::new(18, 1),
            poi: 0,
            ob: 5,
            kz: KillzoneMode::Off,
        },
        LegCfg {
            name: "btc_rr2_poi5_ob5_all_day",
            rr: Decimal::from(2),
            poi: 5,
            ob: 5,
            kz: KillzoneMode::Off,
        },
    ];
    let eth_cfgs = [
        LegCfg {
            name: "eth_rr2_poi0_ob0_ny_only",
            rr: Decimal::from(2),
            poi: 0,
            ob: 0,
            kz: KillzoneMode::NyOnly,
        },
        LegCfg {
            name: "eth_rr2_poi0_ob0_all_day",
            rr: Decimal::from(2),
            poi: 0,
            ob: 0,
            kz: KillzoneMode::Off,
        },
        LegCfg {
            name: "eth_rr1.8_poi0_ob0_ny_only",
            rr: Decimal::new(18, 1),
            poi: 0,
            ob: 0,
            kz: KillzoneMode::NyOnly,
        },
    ];
    let sol_cfgs = [
        LegCfg {
            name: "sol_rr2_poi10_ob10_ny_only",
            rr: Decimal::from(2),
            poi: 10,
            ob: 10,
            kz: KillzoneMode::NyOnly,
        },
        LegCfg {
            name: "sol_rr1.8_poi10_ob10_ny_only",
            rr: Decimal::new(18, 1),
            poi: 10,
            ob: 10,
            kz: KillzoneMode::NyOnly,
        },
        LegCfg {
            name: "sol_rr1.5_poi10_ob10_ny_only",
            rr: Decimal::new(15, 1),
            poi: 10,
            ob: 10,
            kz: KillzoneMode::NyOnly,
        },
    ];

    // sizing from user: BTC 0.1, ETH 1, SOL 10 vs 10-unit baseline
    let btc_mult = Decimal::new(1, 2);
    let eth_mult = Decimal::new(1, 1);
    let sol_mult = Decimal::ONE;

    let mut best_name = String::new();
    let mut best_total = Decimal::from(-9_999_999);
    let mut best_pos = 0usize;
    let mut best_curve: Vec<(String, Decimal)> = vec![];

    println!("top_combos (by last6m_total then positive_months)");
    let mut rows: Vec<(String, Decimal, usize, Vec<(String, Decimal)>)> = vec![];

    for b in btc_cfgs {
        let bm = eval_leg(
            Arc::clone(&btc5),
            btc4.clone(),
            Decimal::new(2, 2),
            b,
            btc_mult,
        );
        for e in eth_cfgs {
            let em = eval_leg(
                Arc::clone(&eth5),
                eth4.clone(),
                Decimal::new(2, 2),
                e,
                eth_mult,
            );
            for s in sol_cfgs {
                let sm = eval_leg(
                    Arc::clone(&sol5),
                    sol4.clone(),
                    Decimal::new(1, 3),
                    s,
                    sol_mult,
                );
                let mut pooled = BTreeMap::<String, Decimal>::new();
                for (k, v) in bm.iter().chain(em.iter()).chain(sm.iter()) {
                    *pooled.entry(k.clone()).or_insert(Decimal::ZERO) += *v;
                }
                let (total, pos, curve) = score_last6(&pooled);
                let name = format!("{} | {} | {}", b.name, e.name, s.name);
                rows.push((name.clone(), total, pos, curve.clone()));
                if total > best_total || (total == best_total && pos > best_pos) {
                    best_total = total;
                    best_pos = pos;
                    best_name = name;
                    best_curve = curve;
                }
            }
        }
    }

    rows.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    for (name, total, pos, _) in rows.iter().take(10) {
        println!(
            "{},total={:.2},pos_months={}/6",
            name,
            total.round_dp(2),
            pos
        );
    }

    println!();
    println!("BEST_COMBO");
    println!("{}", best_name);
    println!("best_total_last6m_usd={:.2}", best_total.round_dp(2));
    println!("best_positive_months={}/6", best_pos);
    println!("month,pooled_usd");
    for (m, v) in best_curve {
        println!("{},{:.2}", m, v.round_dp(2));
    }
}
