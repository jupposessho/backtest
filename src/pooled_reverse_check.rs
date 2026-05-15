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
) -> (BTreeMap<String, Decimal>, BTreeMap<String, Decimal>) {
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

    let mut orig = BTreeMap::<String, Decimal>::new();
    let mut rev = BTreeMap::<String, Decimal>::new();

    for t in &res.trades {
        let d = to_new_york_time(t.close_time);
        let k = format!("{:04}-{:02}", d.year(), d.month());
        // 10 units each asset as in prior pooled run
        let gross = t.points().0 * Decimal::from(10);
        let costs = t.total_costs() * Decimal::from(10);
        let orig_pnl = gross - costs;
        let rev_pnl = -gross - costs;
        *orig.entry(k.clone()).or_insert(Decimal::ZERO) += orig_pnl;
        *rev.entry(k).or_insert(Decimal::ZERO) += rev_pnl;
    }

    (orig, rev)
}

fn main() {
    let btc5 = Arc::new(load("assets/binance_BTCUSDT_5m.json"));
    let btc1 = load("assets/binance_BTCUSDT_1h.json");
    let eth5 = Arc::new(load("assets/binance_ETHUSDT_5m.json"));
    let eth1 = load("assets/binance_ETHUSDT_1h.json");
    let sol5 = Arc::new(load("assets/binance_SOLUSDT_5m.json"));
    let sol1 = load("assets/binance_SOLUSDT_1h.json");

    let cfgs = [
        Cfg {
            name: "close_cisd_or_ifvg_rr1.5_london_ny_poi0_ob10",
            entry: EntryVariant::Close,
            confirm: ReversalConfirmMode::CisdOrIfvg,
            rr: Decimal::new(15, 1),
            kz: KillzoneMode::LondonNy,
            poi: 0,
            ob: 10,
        },
        Cfg {
            name: "close_cisd_only_rr1.5_london_ny_poi0_ob10",
            entry: EntryVariant::Close,
            confirm: ReversalConfirmMode::CisdOnly,
            rr: Decimal::new(15, 1),
            kz: KillzoneMode::LondonNy,
            poi: 0,
            ob: 10,
        },
        Cfg {
            name: "close_cisd_or_ifvg_rr1.2_london_ny_poi0_ob10",
            entry: EntryVariant::Close,
            confirm: ReversalConfirmMode::CisdOrIfvg,
            rr: Decimal::new(12, 1),
            kz: KillzoneMode::LondonNy,
            poi: 0,
            ob: 10,
        },
    ];

    for c in cfgs {
        let (o_btc, r_btc) = eval_asset(Arc::clone(&btc5), btc1.clone(), Decimal::new(2, 2), c);
        let (o_eth, r_eth) = eval_asset(Arc::clone(&eth5), eth1.clone(), Decimal::new(2, 2), c);
        let (o_sol, r_sol) = eval_asset(Arc::clone(&sol5), sol1.clone(), Decimal::new(1, 3), c);

        let mut o_pool = BTreeMap::<String, Decimal>::new();
        let mut r_pool = BTreeMap::<String, Decimal>::new();
        for (k, v) in o_btc.iter().chain(o_eth.iter()).chain(o_sol.iter()) {
            *o_pool.entry(k.clone()).or_insert(Decimal::ZERO) += *v;
        }
        for (k, v) in r_btc.iter().chain(r_eth.iter()).chain(r_sol.iter()) {
            *r_pool.entry(k.clone()).or_insert(Decimal::ZERO) += *v;
        }

        let o_months: Vec<_> = o_pool.iter().collect();
        let r_months: Vec<_> = r_pool.iter().collect();
        let n = o_months.len().min(6);
        let ol = &o_months[o_months.len().saturating_sub(n)..];
        let rl = &r_months[r_months.len().saturating_sub(n)..];

        let mut o_sum = Decimal::ZERO;
        let mut r_sum = Decimal::ZERO;
        let mut o_pos = 0usize;
        let mut r_pos = 0usize;
        for (_, v) in ol {
            o_sum += **v;
            if **v > Decimal::ZERO {
                o_pos += 1;
            }
        }
        for (_, v) in rl {
            r_sum += **v;
            if **v > Decimal::ZERO {
                r_pos += 1;
            }
        }

        println!("config: {}", c.name);
        println!(
            "original_6m_usd={:.2} pos_months={}/{} | reversed_6m_usd={:.2} pos_months={}/{}",
            o_sum.round_dp(2),
            o_pos,
            n,
            r_sum.round_dp(2),
            r_pos,
            n
        );
        println!("month,orig_usd,reversed_usd");
        for i in 0..n {
            let (mo, vo) = ol[i];
            let (_, vr) = rl[i];
            println!("{},{:.2},{:.2}", mo, vo.round_dp(2), vr.round_dp(2));
        }
        println!();
    }
}
