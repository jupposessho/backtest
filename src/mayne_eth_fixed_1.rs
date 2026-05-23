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
    trades: usize,
    winners: usize,
    win_rate_pct: Decimal,
    pf_usd: Decimal,
    net_usd_1eth: Decimal,
    max_dd_usd: Decimal,
    max_dd_pct: Decimal,
    score_net_over_dd: Decimal,
}

fn execution() -> ExecutionConfig {
    ExecutionConfig {
        commission_rate_per_side: Decimal::new(1, 3),
        fee_rate_per_side: Decimal::ZERO,
        slippage_ticks_per_side: 1,
        tick_size: Decimal::from_f32(0.01).unwrap(),
    }
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
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

fn fixed_stats(result: &backtest::model::backtest_result::BacktestResult) -> (Decimal, Decimal, Decimal, Decimal) {
    let mut equity = Decimal::from(10_000);
    let mut peak = equity;
    let mut max_dd_usd = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;
    for t in &result.trades {
        let pnl = t.points().0 - t.total_costs();
        equity += pnl;
        if pnl > Decimal::ZERO {
            gross_profit += pnl;
        } else if pnl < Decimal::ZERO {
            gross_loss += -pnl;
        }
        if equity > peak {
            peak = equity;
        }
        let dd = peak - equity;
        if dd > max_dd_usd {
            max_dd_usd = dd;
        }
    }
    let net = (equity - Decimal::from(10_000)).round_dp(2);
    let pf = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).round_dp(2)
    } else {
        Decimal::ZERO
    };
    let max_dd_pct = if peak > Decimal::ZERO {
        (max_dd_usd / peak * Decimal::from(100)).round_dp(2)
    } else {
        Decimal::ZERO
    };
    (net, max_dd_usd.round_dp(2), max_dd_pct, pf)
}

fn sweep_pair(pair: &'static str, htf_data: Vec<CandleStick>, ltf_data: Vec<CandleStick>) -> Vec<Row> {
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
        Decimal::from_f32(1.25).unwrap(),
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
                            let (result, _) = model.execute_with_diagnostics();
                            let trades = result.number_of_trades();
                            let winners = result.result(TradeResult::Winner);
                            let win_rate_pct = if trades == 0 {
                                Decimal::ZERO
                            } else {
                                (Decimal::from(winners as i64) * Decimal::from(100)
                                    / Decimal::from(trades as i64))
                                .round_dp(2)
                            };
                            let (net, dd_usd, dd_pct, pf) = fixed_stats(&result);
                            let score = (net / (dd_usd + Decimal::ONE)).round_dp(4);
                            rows.push(Row {
                                pair,
                                reversal_pattern: pattern,
                                sl_variant,
                                tp_variant,
                                trigger_type,
                                rr_threshold: rr,
                                ifvg_max_confirm_bars: ifvg_max,
                                trades,
                                winners,
                                win_rate_pct,
                                pf_usd: pf,
                                net_usd_1eth: net,
                                max_dd_usd: dd_usd,
                                max_dd_pct: dd_pct,
                                score_net_over_dd: score,
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
    out.push_str("# Mayne ETH Fixed 1 Contract Sweep\n\n");
    out.push_str("- Sizing: fixed 1 ETH per trade\n");
    out.push_str("- Costs: commission 0.1% per side, slippage 1 tick per side\n\n");

    out.push_str("## Top 20 by Net USD\n\n");
    out.push_str("| rank | pair | net_usd_1eth | max_dd_usd | max_dd_% | pf_usd | trades | win_rate_% | cfg |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---|\n");
    let mut by_net = rows.to_vec();
    by_net.sort_by(|a, b| b.net_usd_1eth.cmp(&a.net_usd_1eth).then(b.trades.cmp(&a.trades)));
    for (i, r) in by_net.iter().take(20).enumerate() {
        let cfg = format!(
            "pat={:?};sl={:?};tp={:?};trig={:?};rr={};ifvg_max={}",
            r.reversal_pattern,
            r.sl_variant,
            r.tp_variant,
            r.trigger_type,
            r.rr_threshold,
            r.ifvg_max_confirm_bars
        );
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            r.pair,
            r.net_usd_1eth,
            r.max_dd_usd,
            r.max_dd_pct,
            r.pf_usd,
            r.trades,
            r.win_rate_pct,
            cfg
        ));
    }
    out.push('\n');

    out.push_str("## Top 20 by Net/DD Score (net / (dd+1))\n\n");
    out.push_str("| rank | pair | score | net_usd_1eth | max_dd_usd | max_dd_% | pf_usd | trades | win_rate_% | cfg |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---:|---|\n");
    let mut by_score = rows.to_vec();
    by_score.sort_by(|a, b| b.score_net_over_dd.cmp(&a.score_net_over_dd).then(b.net_usd_1eth.cmp(&a.net_usd_1eth)));
    for (i, r) in by_score
        .iter()
        .filter(|r| r.net_usd_1eth > Decimal::ZERO && r.trades >= 5)
        .take(20)
        .enumerate()
    {
        let cfg = format!(
            "pat={:?};sl={:?};tp={:?};trig={:?};rr={};ifvg_max={}",
            r.reversal_pattern,
            r.sl_variant,
            r.tp_variant,
            r.trigger_type,
            r.rr_threshold,
            r.ifvg_max_confirm_bars
        );
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            r.pair,
            r.score_net_over_dd,
            r.net_usd_1eth,
            r.max_dd_usd,
            r.max_dd_pct,
            r.pf_usd,
            r.trades,
            r.win_rate_pct,
            cfg
        ));
    }
    out.push('\n');

    out
}

fn main() {
    let (eth_4h, eth_15m) = cap_pair(
        &load("assets/binance_ETHUSDT_4h.json"),
        &load("assets/binance_ETHUSDT_15m.json"),
        3000,
    );
    let (eth_1h, eth_5m) = cap_pair(
        &load("assets/binance_ETHUSDT_1h.json"),
        &load("assets/binance_ETHUSDT_5m.json"),
        3000,
    );

    let mut rows = Vec::new();
    rows.extend(sweep_pair("ETH 4h/15m", eth_4h, eth_15m));
    rows.extend(sweep_pair("ETH 1h/5m", eth_1h, eth_5m));

    let report = build_report(&rows);
    let path = "reports/strategy_overviews/MAYNE_ETH_FIXED_1_SWEEP.md";
    fs::write(path, report).expect("failed to write report");
    println!("wrote {} ({} rows)", path, rows.len());
}
