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

struct Leg {
    asset: &'static str,
    ltf_path: &'static str,
    htf_path: &'static str,
    tick: Decimal,
    size_multiplier_vs_10: Decimal,
    rr: Decimal,
    poi: i32,
    ob: i32,
    kz: KillzoneMode,
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn eval_leg(leg: &Leg) -> (BTreeMap<String, Decimal>, usize) {
    let ltf = Arc::new(load(leg.ltf_path));
    let mut htf = load(leg.htf_path);
    if let Some(last) = ltf.last().map(|c| c.open_time) {
        htf.retain(|c| c.open_time <= last);
    }

    let mut cfg = FractalMTFConfig::default();
    cfg.tick_size = leg.tick;
    cfg.slippage_ticks_per_side = 0;
    cfg.log_progress = false;
    cfg.entry_variant = EntryVariant::Close;
    cfg.reversal_confirm_mode = ReversalConfirmMode::IfvgOnly;
    cfg.weekday_mask = 0b0111_1111;
    cfg.killzone_mode = leg.kz;
    cfg.rr_target = leg.rr;
    cfg.poi_padding_bps = leg.poi;
    cfg.ob_sweep_tolerance_bps = leg.ob;

    let res = execute(TTradesFractalMTF {
        ltf_data: ltf,
        htf_data: Arc::new(htf),
        config: cfg,
    });

    let mut by_month = BTreeMap::<String, Decimal>::new();
    for t in &res.trades {
        let dt = to_new_york_time(t.close_time);
        let key = format!("{:04}-{:02}", dt.year(), dt.month());
        let base_pnl_10_units = (t.points().0 - t.total_costs()) * Decimal::from(10);
        let scaled = base_pnl_10_units * leg.size_multiplier_vs_10;
        *by_month.entry(key).or_insert(Decimal::ZERO) += scaled;
    }

    (by_month, res.trades.len())
}

fn main() {
    let legs = vec![
        // BTC: best from sweep, scaled to 0.1 BTC (vs baseline 10 BTC => x0.01)
        Leg {
            asset: "BTC",
            ltf_path: "assets/binance_BTCUSDT_15m.json",
            htf_path: "assets/binance_BTCUSDT_4h.json",
            tick: Decimal::new(2, 2),
            size_multiplier_vs_10: Decimal::new(1, 2),
            rr: Decimal::from(2),
            poi: 0,
            ob: 5,
            kz: KillzoneMode::Off,
        },
        // ETH: best from sweep, scaled to 1 ETH (vs baseline 10 ETH => x0.1)
        Leg {
            asset: "ETH",
            ltf_path: "assets/binance_ETHUSDT_15m.json",
            htf_path: "assets/binance_ETHUSDT_4h.json",
            tick: Decimal::new(2, 2),
            size_multiplier_vs_10: Decimal::new(1, 1),
            rr: Decimal::from(2),
            poi: 0,
            ob: 0,
            kz: KillzoneMode::NyOnly,
        },
        // SOL: best from sweep, keep 10 SOL baseline (x1.0)
        Leg {
            asset: "SOL",
            ltf_path: "assets/binance_SOLUSDT_15m.json",
            htf_path: "assets/binance_SOLUSDT_4h.json",
            tick: Decimal::new(1, 3),
            size_multiplier_vs_10: Decimal::ONE,
            rr: Decimal::from(2),
            poi: 10,
            ob: 10,
            kz: KillzoneMode::NyOnly,
        },
    ];

    let mut pooled = BTreeMap::<String, Decimal>::new();
    for leg in &legs {
        let (m, trades) = eval_leg(leg);
        let mut total = Decimal::ZERO;
        for (k, v) in &m {
            total += *v;
            *pooled.entry(k.clone()).or_insert(Decimal::ZERO) += *v;
        }
        println!(
            "asset={} trades={} six_month_like_total={:.2}",
            leg.asset,
            trades,
            total.round_dp(2)
        );
    }

    let months: Vec<_> = pooled.iter().collect();
    let n = months.len().min(6);
    let last = &months[months.len().saturating_sub(n)..];
    let mut total = Decimal::ZERO;
    let mut pos = 0usize;
    println!("month,pooled_usd");
    for (m, v) in last {
        total += **v;
        if **v > Decimal::ZERO {
            pos += 1;
        }
        println!("{},{:.2}", m, v.round_dp(2));
    }
    println!("pooled_last_6m_total,{:.2}", total.round_dp(2));
    println!("pooled_positive_months,{}/{}", pos, n);
}
