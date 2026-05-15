extern crate rust_decimal;

use std::collections::BTreeMap;
use std::sync::Arc;

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::execute;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade_result::TradeResult;
use backtest::strategies::ttrades_fractal_mtf::{
    EntryVariant, FractalMTFConfig, KillzoneMode, ReversalConfirmMode, TTradesFractalMTF,
};
use backtest::to_new_york_time;
use chrono::Datelike;
use rust_decimal::Decimal;

#[derive(Clone)]
struct Row {
    name: String,
    net_6m_usd: Decimal,
    positive_months: usize,
    trades: usize,
    wins: usize,
    losses: usize,
}

fn cap(mut data: Vec<CandleStick>, max_bars: usize) -> Vec<CandleStick> {
    if data.len() > max_bars {
        data.truncate(max_bars);
    }
    data
}

fn load_ltf() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_SOLUSDT_5m.json"))
}

fn load_htf() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_SOLUSDT_1h.json"))
}

fn label_entry(v: EntryVariant) -> &'static str {
    match v {
        EntryVariant::Close => "close",
        EntryVariant::ObLevel => "ob_level",
        EntryVariant::ObMidpoint => "ob_mid",
    }
}

fn label_confirm(v: ReversalConfirmMode) -> &'static str {
    match v {
        ReversalConfirmMode::CisdOnly => "cisd_only",
        ReversalConfirmMode::IfvgOnly => "ifvg_only",
        ReversalConfirmMode::CisdAndIfvg => "cisd_and_ifvg",
        ReversalConfirmMode::CisdOrIfvg => "cisd_or_ifvg",
    }
}

fn label_kz(v: KillzoneMode) -> &'static str {
    match v {
        KillzoneMode::Off => "all_day",
        KillzoneMode::NyOnly => "ny_only",
        KillzoneMode::LondonNy => "london_ny",
    }
}

fn main() {
    let capped = 40_000usize;
    let ltf_data = Arc::new(cap(load_ltf(), capped));
    let mut htf_vec = cap(load_htf(), capped);
    if let Some(last_ltf_open) = ltf_data.last().map(|c| c.open_time) {
        htf_vec.retain(|c| c.open_time <= last_ltf_open);
    }
    let htf_data = Arc::new(htf_vec);

    let entries = [
        EntryVariant::Close,
        EntryVariant::ObLevel,
        EntryVariant::ObMidpoint,
    ];
    let confirms = [
        ReversalConfirmMode::IfvgOnly,
        ReversalConfirmMode::CisdOrIfvg,
        ReversalConfirmMode::CisdOnly,
    ];
    let rrs = [
        Decimal::new(10, 1),
        Decimal::new(12, 1),
        Decimal::new(15, 1),
    ];
    let killzones = [
        KillzoneMode::Off,
        KillzoneMode::LondonNy,
        KillzoneMode::NyOnly,
    ];
    let poi_pads = [0, 5, 10];
    let ob_tols = [0, 5, 10];

    let mut all_rows: Vec<Row> = vec![];

    for entry in entries {
        for confirm in confirms {
            for rr in rrs {
                for kz in killzones {
                    for poi in poi_pads {
                        for ob in ob_tols {
                            let mut cfg = FractalMTFConfig::default();
                            cfg.tick_size = Decimal::new(1, 3);
                            cfg.slippage_ticks_per_side = 0;
                            cfg.log_progress = false;
                            cfg.entry_variant = entry;
                            cfg.reversal_confirm_mode = confirm;
                            cfg.weekday_mask = 0b0111_1111;
                            cfg.killzone_mode = kz;
                            cfg.rr_target = rr;
                            cfg.poi_padding_bps = poi;
                            cfg.ob_sweep_tolerance_bps = ob;

                            let result = execute(TTradesFractalMTF {
                                ltf_data: Arc::clone(&ltf_data),
                                htf_data: Arc::clone(&htf_data),
                                config: cfg,
                            });

                            let mut by_month: BTreeMap<String, Decimal> = BTreeMap::new();
                            let mut wins = 0usize;
                            let mut losses = 0usize;
                            for t in &result.trades {
                                let dt = to_new_york_time(t.close_time);
                                let key = format!("{:04}-{:02}", dt.year(), dt.month());
                                let pnl_usd = (t.points().0 - t.total_costs()) * Decimal::from(10);
                                let m = by_month.entry(key).or_insert(Decimal::ZERO);
                                *m += pnl_usd;
                                match t.result {
                                    TradeResult::Winner => wins += 1,
                                    TradeResult::Expense => losses += 1,
                                    TradeResult::BreakEven => {}
                                }
                            }

                            let months: Vec<_> = by_month.iter().collect();
                            let take = months.len().min(6);
                            let last_six = &months[months.len().saturating_sub(take)..];
                            let mut net_6m = Decimal::ZERO;
                            let mut positive = 0usize;
                            for (_, net) in last_six {
                                net_6m += **net;
                                if **net > Decimal::ZERO {
                                    positive += 1;
                                }
                            }

                            all_rows.push(Row {
                                name: format!(
                                    "{}_{}_rr{}_{}_poi{}_ob{}",
                                    label_entry(entry),
                                    label_confirm(confirm),
                                    rr,
                                    label_kz(kz),
                                    poi,
                                    ob
                                ),
                                net_6m_usd: net_6m.round_dp(2),
                                positive_months: positive,
                                trades: result.trades.len(),
                                wins,
                                losses,
                            });
                        }
                    }
                }
            }
        }
    }

    // Hard target: at least 10 trades/month over ~6 months => >= 60 trades.
    let mut rows: Vec<Row> = all_rows.into_iter().filter(|r| r.trades >= 60).collect();
    rows.sort_by(|a, b| {
        b.net_6m_usd
            .cmp(&a.net_6m_usd)
            .then(b.positive_months.cmp(&a.positive_months))
            .then(b.trades.cmp(&a.trades))
    });

    println!("SOL high-frequency target sweep (5m/1h, recent ~6m, 10 SOL)");
    println!("filter: trades >= 60 over last ~6 months");
    println!("qualified_configs: {}", rows.len());
    println!("variant,net_6m_usd,positive_months,trades,wins,losses");
    for r in rows.iter().take(20) {
        println!(
            "{},{:.2},{},{},{},{}",
            r.name, r.net_6m_usd, r.positive_months, r.trades, r.wins, r.losses
        );
    }
}
