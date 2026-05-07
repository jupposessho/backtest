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

#[derive(Clone, Copy)]
struct Variant {
    entry: EntryVariant,
    confirm: ReversalConfirmMode,
    rr: Decimal,
    killzone: KillzoneMode,
}

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

fn load_sol_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_SOLUSDT_15m.json"))
}

fn load_sol_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_SOLUSDT_4h.json"))
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
    let ltf_data = Arc::new(cap(load_sol_15m(), capped));
    let mut htf_vec = cap(load_sol_4h(), capped);
    if let Some(last_ltf_open) = ltf_data.last().map(|c| c.open_time) {
        htf_vec.retain(|c| c.open_time <= last_ltf_open);
    }
    let htf_data = Arc::new(htf_vec);

    let entries = [EntryVariant::Close, EntryVariant::ObLevel, EntryVariant::ObMidpoint];
    let confirms = [
        ReversalConfirmMode::CisdOnly,
        ReversalConfirmMode::IfvgOnly,
        ReversalConfirmMode::CisdAndIfvg,
        ReversalConfirmMode::CisdOrIfvg,
    ];
    let rrs = [Decimal::new(15, 1), Decimal::from(2)];
    let killzones = [KillzoneMode::Off, KillzoneMode::NyOnly];

    let mut rows: Vec<Row> = vec![];
    for entry in entries {
        for confirm in confirms {
            for rr in rrs {
                for killzone in killzones {
                    let mut cfg = FractalMTFConfig::default();
                    cfg.tick_size = Decimal::new(1, 3);
                    cfg.slippage_ticks_per_side = 0;
                    cfg.log_progress = false;
                    cfg.entry_variant = entry;
                    cfg.reversal_confirm_mode = confirm;
                    cfg.weekday_mask = 0b0111_1111;
                    cfg.killzone_mode = killzone;
                    cfg.rr_target = rr;
                    cfg.poi_padding_bps = 0;
                    cfg.ob_sweep_tolerance_bps = 0;

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
                        let entry_m = by_month.entry(key).or_insert(Decimal::ZERO);
                        *entry_m += pnl_usd;
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

                    let name = format!(
                        "{}_{}_rr{}_{}",
                        label_entry(entry),
                        label_confirm(confirm),
                        rr,
                        label_kz(killzone)
                    );
                    rows.push(Row {
                        name,
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

    rows.sort_by(|a, b| {
        b.net_6m_usd
            .cmp(&a.net_6m_usd)
            .then(b.positive_months.cmp(&a.positive_months))
    });

    println!("SOL 15m/4h rescue sweep (recent ~6m, 10 SOL position)");
    println!("variant,net_6m_usd,positive_months,trades,wins,losses");
    for r in rows.iter().take(12) {
        println!(
            "{},{:.2},{},{},{},{}",
            r.name,
            r.net_6m_usd,
            r.positive_months,
            r.trades,
            r.wins,
            r.losses
        );
    }
}
