use rust_decimal::{prelude::FromPrimitive, Decimal};

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::engine::types::ExecutionConfig;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade_result::TradeResult;
use backtest::model::trigger_type::TriggerType;
use backtest::strategies::mayne::{Mayne, ReversalPattern, SlVariant, TpVariant};
use std::fs;

#[derive(Clone)]
struct Row {
    pair: &'static str,
    cfg: String,
    trades: usize,
    win_rate_pct: Decimal,
    pf_usd: Decimal,
    net_usd_10sol: Decimal,
    max_dd_usd: Decimal,
    score: Decimal,
}

fn execution() -> ExecutionConfig {
    ExecutionConfig {
        commission_rate_per_side: Decimal::new(1, 3),
        fee_rate_per_side: Decimal::ZERO,
        slippage_ticks_per_side: 1,
        tick_size: Decimal::from_f32(0.001).unwrap(),
    }
}

fn load(path: &str) -> Vec<CandleStick> {
    let s = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {}", path, e));
    CandleStickLoader::load_binance(&s)
}

fn cap_pair(htf_data: &[CandleStick], ltf_data: &[CandleStick], htf_cap: usize) -> (Vec<CandleStick>, Vec<CandleStick>) {
    let htf_start = htf_data.len().saturating_sub(htf_cap);
    let htf = htf_data[htf_start..].to_vec();
    let min_time = htf.first().map(|c| c.open_time).unwrap_or(0);
    let ltf = ltf_data.iter().copied().filter(|c| c.open_time >= min_time).collect::<Vec<_>>();
    (htf, ltf)
}

fn fixed_stats_10sol(result: &BacktestResult) -> (Decimal, Decimal, Decimal) {
    let mut eq = Decimal::from(10_000);
    let mut peak = eq;
    let mut max_dd = Decimal::ZERO;
    let mut gp = Decimal::ZERO;
    let mut gl = Decimal::ZERO;
    for t in &result.trades {
        let pnl = (t.points().0 - t.total_costs()) * Decimal::from(10);
        eq += pnl;
        if pnl > Decimal::ZERO {
            gp += pnl;
        } else if pnl < Decimal::ZERO {
            gl += -pnl;
        }
        if eq > peak {
            peak = eq;
        }
        let dd = peak - eq;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    let net = (eq - Decimal::from(10_000)).round_dp(2);
    let pf = if gl > Decimal::ZERO { (gp / gl).round_dp(2) } else { Decimal::ZERO };
    (net, max_dd.round_dp(2), pf)
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
                            let wins = result.result(TradeResult::Winner);
                            let win_rate = if trades == 0 {
                                Decimal::ZERO
                            } else {
                                (Decimal::from(wins as i64) * Decimal::from(100) / Decimal::from(trades as i64)).round_dp(2)
                            };
                            let (net, dd, pf) = fixed_stats_10sol(&result);
                            let score = (net / (dd + Decimal::ONE)).round_dp(4);
                            rows.push(Row {
                                pair,
                                cfg: format!(
                                    "pat={:?};sl={:?};tp={:?};trig={:?};rr={};ifvg_max={}",
                                    pattern, sl_variant, tp_variant, trigger_type, rr, ifvg_max
                                ),
                                trades,
                                win_rate_pct: win_rate,
                                pf_usd: pf,
                                net_usd_10sol: net,
                                max_dd_usd: dd,
                                score,
                            });
                        }
                    }
                }
            }
        }
    }
    rows
}

fn main() {
    let (sol_4h, sol_15m) = cap_pair(&load("assets/binance_SOLUSDT_4h.json"), &load("assets/binance_SOLUSDT_15m.json"), 3000);
    let (sol_1h, sol_5m) = cap_pair(&load("assets/binance_SOLUSDT_1h.json"), &load("assets/binance_SOLUSDT_5m.json"), 3000);

    let mut rows = Vec::new();
    rows.extend(sweep_pair("SOL 4h/15m", sol_4h, sol_15m));
    rows.extend(sweep_pair("SOL 1h/5m", sol_1h, sol_5m));

    rows.sort_by(|a, b| b.net_usd_10sol.cmp(&a.net_usd_10sol).then(b.trades.cmp(&a.trades)));

    let mut out = String::new();
    out.push_str("# Mayne SOL Fixed 10 Contract Sweep\n\n");
    out.push_str("| rank | pair | net_usd_10sol | max_dd_usd | pf_usd | trades | win_rate_% | score | cfg |\n");
    out.push_str("|---:|---|---:|---:|---:|---:|---:|---:|---|\n");
    for (i, r) in rows.iter().take(30).enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            r.pair,
            r.net_usd_10sol,
            r.max_dd_usd,
            r.pf_usd,
            r.trades,
            r.win_rate_pct,
            r.score,
            r.cfg
        ));
    }

    let path = "reports/strategy_overviews/MAYNE_SOL_FIXED_10_SWEEP.md";
    fs::write(path, out).expect("write report");
    println!("wrote {} ({} rows)", path, rows.len());
}
