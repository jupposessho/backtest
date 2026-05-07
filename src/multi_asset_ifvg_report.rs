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
    asset: &'static str,
    variant: String,
    net_6m_usd_at_10: Decimal,
    net_6m_usd_norm: Decimal,
    positive_months: usize,
    trades: usize,
    wins: usize,
    losses: usize,
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn label_kz(v: KillzoneMode) -> &'static str {
    match v {
        KillzoneMode::Off => "all_day",
        KillzoneMode::NyOnly => "ny_only",
        KillzoneMode::LondonNy => "london_ny",
    }
}

fn norm_mult(asset: &str) -> Decimal {
    match asset {
        "BTC" => Decimal::new(1, 2), // 0.1 vs 10 baseline
        "ETH" => Decimal::new(1, 1), // 1 vs 10 baseline
        "SOL" => Decimal::ONE,       // 10 vs 10 baseline
        _ => Decimal::ONE,
    }
}

fn eval_asset(
    asset: &'static str,
    ltf: Arc<Vec<CandleStick>>,
    mut htf: Vec<CandleStick>,
    tick: Decimal,
) -> Vec<Row> {
    if let Some(last) = ltf.last().map(|c| c.open_time) {
        htf.retain(|c| c.open_time <= last);
    }

    let rrs = [
        Decimal::new(12, 1),
        Decimal::new(15, 1),
        Decimal::new(18, 1),
        Decimal::from(2),
    ];
    let poi_pads = [0, 5, 10];
    let ob_tols = [0, 5, 10];
    let killzones = [KillzoneMode::Off, KillzoneMode::NyOnly];
    let nm = norm_mult(asset);

    let mut rows = Vec::new();
    for rr in rrs {
        for poi in poi_pads {
            for ob in ob_tols {
                for kz in killzones {
                    let mut cfg = FractalMTFConfig::default();
                    cfg.tick_size = tick;
                    cfg.slippage_ticks_per_side = 0;
                    cfg.log_progress = false;
                    cfg.entry_variant = EntryVariant::Close;
                    cfg.reversal_confirm_mode = ReversalConfirmMode::IfvgOnly;
                    cfg.weekday_mask = 0b0111_1111;
                    cfg.killzone_mode = kz;
                    cfg.rr_target = rr;
                    cfg.poi_padding_bps = poi;
                    cfg.ob_sweep_tolerance_bps = ob;

                    let result = execute(TTradesFractalMTF {
                        ltf_data: Arc::clone(&ltf),
                        htf_data: Arc::new(htf.clone()),
                        config: cfg,
                    });

                    let mut by_month: BTreeMap<String, Decimal> = BTreeMap::new();
                    let mut wins = 0usize;
                    let mut losses = 0usize;
                    for t in &result.trades {
                        let dt = to_new_york_time(t.close_time);
                        let key = format!("{:04}-{:02}", dt.year(), dt.month());
                        let pnl_usd_10 = (t.points().0 - t.total_costs()) * Decimal::from(10);
                        let m = by_month.entry(key).or_insert(Decimal::ZERO);
                        *m += pnl_usd_10;
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

                    rows.push(Row {
                        asset,
                        variant: format!(
                            "close_ifvg_rr{}_poi{}_ob{}_{}",
                            rr,
                            poi,
                            ob,
                            label_kz(kz)
                        ),
                        net_6m_usd_at_10: net_6m.round_dp(2),
                        net_6m_usd_norm: (net_6m * nm).round_dp(2),
                        positive_months: positive,
                        trades: result.trades.len(),
                        wins,
                        losses,
                    });
                }
            }
        }
    }
    rows
}

fn main() {
    let mut all = Vec::new();
    all.extend(eval_asset(
        "BTC",
        Arc::new(load("assets/binance_BTCUSDT_15m.json")),
        load("assets/binance_BTCUSDT_4h.json"),
        Decimal::new(2, 2),
    ));
    all.extend(eval_asset(
        "ETH",
        Arc::new(load("assets/binance_ETHUSDT_15m.json")),
        load("assets/binance_ETHUSDT_4h.json"),
        Decimal::new(2, 2),
    ));
    all.extend(eval_asset(
        "SOL",
        Arc::new(load("assets/binance_SOLUSDT_15m.json")),
        load("assets/binance_SOLUSDT_4h.json"),
        Decimal::new(1, 3),
    ));

    let mut md = String::new();
    md.push_str("# Multi-Asset iFVG Tune (Recent Window)\n\n");
    md.push_str("Scope: `ttrades_fractal_mtf` close-entry + ifvg-only, on BTC/ETH/SOL 15m/4h.\n");
    md.push_str("Window: latest ~6 months in current asset JSON files.\n");
    md.push_str("Costs: strategy defaults with slippage ticks = 0.\n");
    md.push_str("Normalization: BTC shown at 0.1 coin, ETH at 1 coin, SOL at 10 coins.\n\n");

    for asset in ["BTC", "ETH", "SOL"] {
        let mut rows: Vec<Row> = all.iter().filter(|r| r.asset == asset).cloned().collect();
        rows.sort_by(|a, b| {
            b.net_6m_usd_norm
                .cmp(&a.net_6m_usd_norm)
                .then(b.positive_months.cmp(&a.positive_months))
                .then(b.trades.cmp(&a.trades))
        });

        md.push_str(&format!("## {}\n\n", asset));
        md.push_str("| variant | net_6m_usd_at_10 | net_6m_usd_normalized | positive_months | trades | wins | losses |\n");
        md.push_str("|---|---:|---:|---:|---:|---:|---:|\n");
        for r in rows {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                r.variant,
                r.net_6m_usd_at_10.round_dp(2),
                r.net_6m_usd_norm.round_dp(2),
                r.positive_months,
                r.trades,
                r.wins,
                r.losses
            ));
        }
        md.push_str("\n");
    }

    std::fs::write(
        "reports/strategy_overviews/MULTI_ASSET_IFVG_TUNE_REPORT.md",
        md,
    )
    .unwrap_or_else(|e| panic!("failed writing report: {}", e));

    println!("Wrote reports/strategy_overviews/MULTI_ASSET_IFVG_TUNE_REPORT.md");
}
