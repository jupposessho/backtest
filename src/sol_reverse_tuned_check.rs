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
    rr: Decimal,
    poi: i32,
    ob: i32,
    kz: KillzoneMode,
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn eval(
    ltf: Arc<Vec<CandleStick>>,
    mut htf: Vec<CandleStick>,
    c: Cfg,
) -> (BTreeMap<String, Decimal>, BTreeMap<String, Decimal>) {
    if let Some(last) = ltf.last().map(|x| x.open_time) {
        htf.retain(|x| x.open_time <= last);
    }
    let mut cfg = FractalMTFConfig::default();
    cfg.tick_size = Decimal::new(1, 3);
    cfg.slippage_ticks_per_side = 0;
    cfg.log_progress = false;
    cfg.entry_variant = EntryVariant::Close;
    cfg.reversal_confirm_mode = ReversalConfirmMode::IfvgOnly;
    cfg.weekday_mask = 0b0111_1111;
    cfg.killzone_mode = c.kz;
    cfg.rr_target = c.rr;
    cfg.poi_padding_bps = c.poi;
    cfg.ob_sweep_tolerance_bps = c.ob;

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
        let gross = t.points().0 * Decimal::from(10);
        let costs = t.total_costs() * Decimal::from(10);
        *orig.entry(k.clone()).or_insert(Decimal::ZERO) += gross - costs;
        *rev.entry(k).or_insert(Decimal::ZERO) += -gross - costs;
    }
    (orig, rev)
}

fn main() {
    let ltf = Arc::new(load("assets/binance_SOLUSDT_15m.json"));
    let htf = load("assets/binance_SOLUSDT_4h.json");

    let cfgs = [
        Cfg {
            name: "close_ifvg_rr2_poi10_ob10_ny_only",
            rr: Decimal::from(2),
            poi: 10,
            ob: 10,
            kz: KillzoneMode::NyOnly,
        },
        Cfg {
            name: "close_ifvg_rr1.8_poi10_ob10_ny_only",
            rr: Decimal::new(18, 1),
            poi: 10,
            ob: 10,
            kz: KillzoneMode::NyOnly,
        },
        Cfg {
            name: "close_ifvg_rr1.5_poi10_ob10_ny_only",
            rr: Decimal::new(15, 1),
            poi: 10,
            ob: 10,
            kz: KillzoneMode::NyOnly,
        },
        Cfg {
            name: "close_ifvg_rr2_poi0_ob10_all_day",
            rr: Decimal::from(2),
            poi: 0,
            ob: 10,
            kz: KillzoneMode::Off,
        },
    ];

    for c in cfgs {
        let (orig, rev) = eval(Arc::clone(&ltf), htf.clone(), c);
        let om: Vec<_> = orig.iter().collect();
        let rm: Vec<_> = rev.iter().collect();
        let n = om.len().min(6);
        let ol = &om[om.len().saturating_sub(n)..];
        let rl = &rm[rm.len().saturating_sub(n)..];
        let mut os = Decimal::ZERO;
        let mut rs = Decimal::ZERO;
        let mut op = 0usize;
        let mut rp = 0usize;
        for (_, v) in ol {
            os += **v;
            if **v > Decimal::ZERO {
                op += 1;
            }
        }
        for (_, v) in rl {
            rs += **v;
            if **v > Decimal::ZERO {
                rp += 1;
            }
        }

        println!("config: {}", c.name);
        println!(
            "original_6m_usd={:.2} pos_months={}/{} | reversed_6m_usd={:.2} pos_months={}/{}",
            os.round_dp(2),
            op,
            n,
            rs.round_dp(2),
            rp,
            n
        );
        println!("month,orig_usd,reversed_usd");
        for i in 0..n {
            let (m, vo) = ol[i];
            let (_, vr) = rl[i];
            println!("{},{:.2},{:.2}", m, vo.round_dp(2), vr.round_dp(2));
        }
        println!();
    }
}
