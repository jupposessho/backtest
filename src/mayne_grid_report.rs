use rust_decimal::{prelude::FromPrimitive, Decimal};

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::engine::types::ExecutionConfig;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade_result::TradeResult;
use backtest::model::trigger_type::TriggerType;
use backtest::strategies::mayne::{Mayne, ReversalPattern, SlVariant, TpVariant};
use std::fs;

#[derive(Clone)]
struct Row {
    pair: &'static str,
    reversal_pattern: ReversalPattern,
    sl_variant: SlVariant,
    tp_variant: TpVariant,
    trigger_type: TriggerType,
    rr_threshold: Decimal,
    ifvg_max_confirm_bars: usize,
    pnl_pct: Decimal,
    profit_r: Decimal,
    trades: usize,
    winners: usize,
    expenses: usize,
    win_rate_pct: Decimal,
    htf_sfp_hits: usize,
    reversal_pass: usize,
    ifvg_found: usize,
    ifvg_distance_reject: usize,
    ltf_trigger_hit: usize,
    rr_pass: usize,
    verdict: &'static str,
}

fn execution() -> ExecutionConfig {
    ExecutionConfig {
        commission_rate_per_side: Decimal::new(1, 3),
        fee_rate_per_side: Decimal::ZERO,
        slippage_ticks_per_side: 1,
        tick_size: Decimal::from_f32(0.01).unwrap(),
    }
}

fn load_eth_1h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_ETHUSDT_1h.json"))
}

fn load_eth_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_ETHUSDT_4h.json"))
}

fn load_eth_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_ETHUSDT_15m.json"))
}

fn load_eth_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_ETHUSDT_5m.json"))
}

fn cap_pair(
    htf_data: &[CandleStick],
    ltf_data: &[CandleStick],
    htf_cap: usize,
) -> (Vec<CandleStick>, Vec<CandleStick>) {
    let htf_start = htf_data.len().saturating_sub(htf_cap);
    let htf = htf_data[htf_start..].to_vec();
    let min_time = htf.first().map(|c| c.open_time).unwrap_or(0);
    let ltf = ltf_data
        .iter()
        .copied()
        .filter(|c| c.open_time >= min_time)
        .collect::<Vec<_>>();
    (htf, ltf)
}

fn sweep_pair(
    pair: &'static str,
    htf_data: Vec<CandleStick>,
    ltf_data: Vec<CandleStick>,
) -> Vec<Row> {
    let patterns = [
        ReversalPattern::Mss,
        ReversalPattern::Ob,
        ReversalPattern::CisdBodyFlip,
        ReversalPattern::CisdStrictWickBreak,
        ReversalPattern::CisdLastSeriesCloseBreak,
        ReversalPattern::IfvgOnly,
        ReversalPattern::CisdStrictWickBreakAndIfvg,
        ReversalPattern::CisdStrictWickBreakOrIfvg,
    ];
    let sl_variants = [SlVariant::SfpExtreme, SlVariant::LtfRecentSwing];
    let tp_variants = [TpVariant::OpposingHtfSwing, TpVariant::OpposingLtfSwing];
    let trigger_types = [TriggerType::Close, TriggerType::Wick];
    let rr_thresholds = [
        Decimal::from_f32(0.75).unwrap(),
        Decimal::from_f32(1.0).unwrap(),
        Decimal::from_f32(1.5).unwrap(),
        Decimal::from_f32(2.0).unwrap(),
        Decimal::from_f32(3.0).unwrap(),
    ];
    let ifvg_max_confirm_bars = [6usize, 12usize, 24usize, 48usize, 96usize];

    let mut rows = Vec::new();

    for pattern in patterns {
        for sl_variant in sl_variants {
            for tp_variant in tp_variants {
                for trigger_type in trigger_types {
                    for rr in rr_thresholds {
                        for ifvg_max in ifvg_max_confirm_bars {
                            let model = Mayne {
                                rr_threshold: rr,
                                trigger_type,
                                reversal_pattern: pattern,
                                sl_variant,
                                tp_variant,
                                ifvg_max_confirm_bars: ifvg_max,
                                htf_data: htf_data.clone(),
                                ltf_data: ltf_data.clone(),
                                execution: execution(),
                            };
                            let (result, diag) = model.execute_with_diagnostics();
                            let trades = result.number_of_trades();
                            let winners = result.result(TradeResult::Winner);
                            let expenses = result.result(TradeResult::Expense);
                            let win_rate_pct = if trades == 0 {
                                Decimal::ZERO
                            } else {
                                (Decimal::from(winners as i64) * Decimal::from(100)
                                    / Decimal::from(trades as i64))
                                .trunc_with_scale(2)
                            };
                            let pnl = result.pnl();
                            let verdict = if trades >= 5 && pnl > Decimal::ZERO {
                                "PROMOTE"
                            } else {
                                "REJECT"
                            };
                            rows.push(Row {
                                pair,
                                reversal_pattern: pattern,
                                sl_variant,
                                tp_variant,
                                trigger_type,
                                rr_threshold: rr,
                                ifvg_max_confirm_bars: ifvg_max,
                                pnl_pct: pnl,
                                profit_r: result.profit_in_r(),
                                trades,
                                winners,
                                expenses,
                                win_rate_pct,
                                htf_sfp_hits: diag.htf_sfp_hits,
                                reversal_pass: diag.reversal_pass,
                                ifvg_found: diag.ifvg_found,
                                ifvg_distance_reject: diag.ifvg_distance_reject,
                                ltf_trigger_hit: diag.ltf_trigger_hit,
                                rr_pass: diag.rr_pass,
                                verdict,
                            });
                        }
                    }
                }
            }
        }
    }

    rows
}

fn build_report(rows: &[Row]) -> String {
    let mut out = String::new();
    out.push_str("# Mayne ETH Detailed Sweep Grid\n\n");
    out.push_str("Pairs: ETH 4h/15m and ETH 1h/5m.\n");
    out.push_str("Grid: reversal_pattern x sl_variant x tp_variant x trigger_type x rr_threshold x ifvg_max_confirm_bars.\n");
    out.push_str(
        "Verdict rule for this sheet: `PROMOTE` if trades >= 5 and pnl_pct > 0, else `REJECT`.\n\n",
    );

    out.push_str("## Validation Matrix\n\n");
    out.push_str("| strategy | asset | pair | configs | active_configs | profitable_configs | best_pnl_% | best_profit_r | best_trades | best_config |\n");
    out.push_str("|---|---|---|---:|---:|---:|---:|---:|---:|---|\n");

    for pair in ["ETH 4h/15m", "ETH 1h/5m"] {
        let subset = rows.iter().filter(|r| r.pair == pair).collect::<Vec<_>>();
        let configs = subset.len();
        let active = subset.iter().filter(|r| r.trades > 0).count();
        let profitable = subset.iter().filter(|r| r.pnl_pct > Decimal::ZERO).count();
        let best = subset
            .iter()
            .max_by(|a, b| a.pnl_pct.cmp(&b.pnl_pct))
            .copied();

        if let Some(b) = best {
            let cfg = format!(
                "pat={:?};sl={:?};tp={:?};trig={:?};rr={};ifvg_max={}",
                b.reversal_pattern,
                b.sl_variant,
                b.tp_variant,
                b.trigger_type,
                b.rr_threshold,
                b.ifvg_max_confirm_bars
            );
            out.push_str(&format!(
                "| mayne | ETH | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                pair, configs, active, profitable, b.pnl_pct, b.profit_r, b.trades, cfg
            ));
        }
    }

    out.push_str("\n## Full Grid\n\n");
    out.push_str("| pair | reversal_pattern | sl_variant | tp_variant | trigger_type | rr_threshold | ifvg_max_confirm_bars | pnl_% | profit_r | trades | winners | expenses | win_rate_% | htf_sfp_hits | reversal_pass | ifvg_found | ifvg_distance_reject | ltf_trigger_hit | rr_pass | verdict |\n");
    out.push_str("|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");

    let mut sorted = rows.to_vec();
    sorted.sort_by(|a, b| {
        a.pair
            .cmp(b.pair)
            .then_with(|| b.pnl_pct.cmp(&a.pnl_pct))
            .then_with(|| b.trades.cmp(&a.trades))
    });

    for r in sorted {
        out.push_str(&format!(
            "| {} | {:?} | {:?} | {:?} | {:?} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.pair,
            r.reversal_pattern,
            r.sl_variant,
            r.tp_variant,
            r.trigger_type,
            r.rr_threshold,
            r.ifvg_max_confirm_bars,
            r.pnl_pct,
            r.profit_r,
            r.trades,
            r.winners,
            r.expenses,
            r.win_rate_pct,
            r.htf_sfp_hits,
            r.reversal_pass,
            r.ifvg_found,
            r.ifvg_distance_reject,
            r.ltf_trigger_hit,
            r.rr_pass,
            r.verdict,
        ));
    }

    out
}

fn main() {
    let (eth_4h, eth_15m) = cap_pair(&load_eth_4h(), &load_eth_15m(), 3000);
    let (eth_1h, eth_5m) = cap_pair(&load_eth_1h(), &load_eth_5m(), 3000);

    let mut rows = Vec::new();
    rows.extend(sweep_pair("ETH 4h/15m", eth_4h, eth_15m));
    rows.extend(sweep_pair("ETH 1h/5m", eth_1h, eth_5m));

    let report = build_report(&rows);
    let path = "reports/strategy_overviews/MAYNE_ETH_DETAILED_GRID.md";
    fs::write(path, report).expect("failed to write report");
    println!("wrote {}", path);
}
