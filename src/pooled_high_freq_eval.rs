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
struct Cfg {
    name: &'static str,
    entry: EntryVariant,
    confirm: ReversalConfirmMode,
    rr: Decimal,
    kz: KillzoneMode,
    poi: i32,
    ob: i32,
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn eval_asset(
    ltf: Arc<Vec<CandleStick>>,
    mut htf: Vec<CandleStick>,
    tick: Decimal,
    cfgv: Cfg,
) -> (BTreeMap<String, Decimal>, usize) {
    if let Some(last) = ltf.last().map(|c| c.open_time) {
        htf.retain(|c| c.open_time <= last);
    }
    let mut cfg = FractalMTFConfig::default();
    cfg.tick_size = tick;
    cfg.slippage_ticks_per_side = 0;
    cfg.log_progress = false;
    cfg.entry_variant = cfgv.entry;
    cfg.reversal_confirm_mode = cfgv.confirm;
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
    let mut bym = BTreeMap::<String, Decimal>::new();
    for t in &res.trades {
        let d = to_new_york_time(t.close_time);
        let k = format!("{:04}-{:02}", d.year(), d.month());
        let pnl = (t.points().0 - t.total_costs()) * Decimal::from(10);
        *bym.entry(k).or_insert(Decimal::ZERO) += pnl;
    }
    (bym, res.trades.len())
}

fn main() {
    let btc5 = Arc::new(load("assets/binance_BTCUSDT_5m.json"));
    let btc1 = load("assets/binance_BTCUSDT_1h.json");
    let eth5 = Arc::new(load("assets/binance_ETHUSDT_5m.json"));
    let eth1 = load("assets/binance_ETHUSDT_1h.json");
    let sol5 = Arc::new(load("assets/binance_SOLUSDT_5m.json"));
    let sol1 = load("assets/binance_SOLUSDT_1h.json");

    let cfgs = [
        Cfg { name: "close_cisd_or_ifvg_rr1.5_london_ny_poi0_ob10", entry: EntryVariant::Close, confirm: ReversalConfirmMode::CisdOrIfvg, rr: Decimal::new(15,1), kz: KillzoneMode::LondonNy, poi:0, ob:10 },
        Cfg { name: "close_cisd_only_rr1.5_london_ny_poi0_ob10", entry: EntryVariant::Close, confirm: ReversalConfirmMode::CisdOnly, rr: Decimal::new(15,1), kz: KillzoneMode::LondonNy, poi:0, ob:10 },
        Cfg { name: "close_cisd_or_ifvg_rr1.2_london_ny_poi0_ob10", entry: EntryVariant::Close, confirm: ReversalConfirmMode::CisdOrIfvg, rr: Decimal::new(12,1), kz: KillzoneMode::LondonNy, poi:0, ob:10 },
    ];

    for c in cfgs {
        let (b_btc, t_btc) = eval_asset(Arc::clone(&btc5), btc1.clone(), Decimal::new(2,2), c);
        let (b_eth, t_eth) = eval_asset(Arc::clone(&eth5), eth1.clone(), Decimal::new(2,2), c);
        let (b_sol, t_sol) = eval_asset(Arc::clone(&sol5), sol1.clone(), Decimal::new(1,3), c);
        let mut pooled: BTreeMap<String, Decimal> = BTreeMap::new();
        for (k,v) in b_btc.iter().chain(b_eth.iter()).chain(b_sol.iter()) {
            *pooled.entry(k.clone()).or_insert(Decimal::ZERO) += *v;
        }
        let months: Vec<_> = pooled.iter().collect();
        let n = months.len().min(6);
        let last = &months[months.len().saturating_sub(n)..];
        let mut total = Decimal::ZERO;
        let mut pos = 0;
        for (_,v) in last { total += **v; if **v > Decimal::ZERO { pos += 1; } }
        println!("config: {}", c.name);
        println!("trades: pooled={} (btc={}, eth={}, sol={})", t_btc+t_eth+t_sol, t_btc, t_eth, t_sol);
        println!("last6m_total_usd: {:.2} positive_months: {}/{}", total.round_dp(2), pos, n);
        println!("month,pooled_usd");
        for (m,v) in last { println!("{},{:.2}", m, v.round_dp(2)); }
        println!();
    }
}
