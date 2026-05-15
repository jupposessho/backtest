use std::collections::BTreeMap;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    engine::types::ExecutionConfig,
    model::{
        candle_stick::CandleStick, decimal::DecimalVec, session::Session, sl_trategy::SlStrategy,
        trade_result::TradeResult, trading_model::TradingModel,
    },
    parse_datetime,
    strategies::macro_soup::MacroSoup,
    to_new_york_time,
};
use chrono::Datelike;
use rust_decimal::Decimal;

#[derive(Clone)]
struct Candidate {
    start: String,
    end: String,
    slippage_ticks: i32,
    fee_mult: Decimal,
    trades: usize,
    win_rate: Decimal,
    profit_r: Decimal,
    pf_r: Decimal,
    total_usd_0p1: Decimal,
    pos_months: usize,
    months: usize,
    max_monthly_dd: Decimal,
    status: String,
    fail_reason: String,
}

fn load_binance() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_15m.json"))
}

fn month_stats(
    trades: &[backtest::model::trade::Trade],
    qty_btc: Decimal,
) -> (Decimal, usize, usize, Decimal) {
    let mut by_month: BTreeMap<String, Decimal> = BTreeMap::new();
    for t in trades {
        let dt = to_new_york_time(t.close_time);
        let key = format!("{:04}-{:02}", dt.year(), dt.month());
        let gross = t.points().0 * qty_btc;
        let costs = t.total_costs() * qty_btc;
        *by_month.entry(key).or_insert(Decimal::ZERO) += gross - costs;
    }

    let total: Decimal = by_month.values().copied().sum();
    let months = by_month.len();
    let pos_months = by_month.values().filter(|x| **x > Decimal::ZERO).count();

    let mut eq = Decimal::ZERO;
    let mut peak = Decimal::ZERO;
    let mut max_dd = Decimal::ZERO;
    for v in by_month.values() {
        eq += *v;
        if eq > peak {
            peak = eq;
        }
        let dd = peak - eq;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    (total, pos_months, months, max_dd)
}

fn main() {
    let candles = load_binance();
    let qty_btc = Decimal::new(1, 1); // 0.1 BTC

    let min_avg_trades_per_month = Decimal::new(8, 0);
    let min_pos_month_rate = Decimal::new(55, 2); // 55%
    let max_dd_usd = Decimal::new(2500, 0); // 0.1 BTC sizing drawdown cap

    let base_start = parse_datetime("2022-09-30 05:00:00").unwrap().time();
    let last = parse_datetime("2022-09-30 18:00:00").unwrap().time();

    let fee_mults = [Decimal::new(100, 2)];
    let slippages = [1_i32, 2_i32, 3_i32];

    // Use top windows already identified in MACRO_SOUP_REALISM_REPORT.md to keep runtime bounded.
    let top_windows = ["16:00", "15:50", "15:55"];

    let mut rows: Vec<Candidate> = Vec::new();

    for slip in slippages {
        for fee_mult in fee_mults {
            for start_hm in top_windows {
                let start = parse_datetime(&format!("2022-09-30 {}:00", start_hm))
                    .unwrap()
                    .time();
                let session_end = start + chrono::Duration::minutes(60);
                let model = MacroSoup {
                    candles: candles.clone(),
                    rr_threshold: Decimal::from(3),
                    be_threshold: Some(DecimalVec::new(2)),
                    session: Session {
                        start,
                        end: session_end,
                    },
                    sl_strategy: SlStrategy::None,
                    max_duration_min: 120,
                    execution: ExecutionConfig {
                        commission_rate_per_side: Decimal::new(1, 3) * fee_mult,
                        fee_rate_per_side: Decimal::ZERO,
                        slippage_ticks_per_side: slip,
                        tick_size: Decimal::new(1, 2),
                    },
                };
                let result = model.execute();

                let winners = result.result(TradeResult::Winner);
                let losers = result.result(TradeResult::Expense);
                let trades = result.number_of_trades();
                let win_rate = if trades == 0 {
                    Decimal::ZERO
                } else {
                    Decimal::from(winners as u32) / Decimal::from(trades as u32)
                        * Decimal::new(100, 0)
                };

                let gross_loss_r = Decimal::from(losers as u32);
                let gross_profit_r = result.profit_in_r() + gross_loss_r;
                let pf_r = if gross_loss_r > Decimal::ZERO {
                    gross_profit_r / gross_loss_r
                } else if gross_profit_r > Decimal::ZERO {
                    Decimal::from(9999)
                } else {
                    Decimal::ZERO
                };

                let (total_usd, pos_months, months, max_monthly_dd) =
                    month_stats(&result.trades, qty_btc);

                let avg_trades_per_month = if months == 0 {
                    Decimal::ZERO
                } else {
                    Decimal::from(trades as u32) / Decimal::from(months as u32)
                };
                let pos_rate = if months == 0 {
                    Decimal::ZERO
                } else {
                    Decimal::from(pos_months as u32) / Decimal::from(months as u32)
                };

                let mut fail_reasons: Vec<&str> = Vec::new();
                if avg_trades_per_month < min_avg_trades_per_month {
                    fail_reasons.push("low_participation");
                }
                if pos_rate < min_pos_month_rate {
                    fail_reasons.push("low_pos_month_rate");
                }
                if max_monthly_dd > max_dd_usd {
                    fail_reasons.push("high_monthly_dd");
                }
                if total_usd <= Decimal::ZERO {
                    fail_reasons.push("non_positive_total");
                }
                if pf_r < Decimal::new(12, 1) {
                    fail_reasons.push("pf_below_1p2");
                }

                let (status, fail_reason) = if fail_reasons.is_empty() {
                    ("PASS".to_string(), String::new())
                } else {
                    ("FAIL".to_string(), fail_reasons.join(","))
                };

                rows.push(Candidate {
                    start: start.format("%H:%M").to_string(),
                    end: session_end.format("%H:%M").to_string(),
                    slippage_ticks: slip,
                    fee_mult,
                    trades,
                    win_rate,
                    profit_r: result.profit_in_r(),
                    pf_r,
                    total_usd_0p1: total_usd,
                    pos_months,
                    months,
                    max_monthly_dd,
                    status,
                    fail_reason,
                });
            }
        }
    }

    rows.sort_by(|a, b| {
        b.total_usd_0p1
            .cmp(&a.total_usd_0p1)
            .then(b.pf_r.cmp(&a.pf_r))
            .then(b.profit_r.cmp(&a.profit_r))
    });

    let mut pass_rows: Vec<Candidate> = rows
        .iter()
        .filter(|x| x.status == "PASS")
        .cloned()
        .collect();
    pass_rows.sort_by(|a, b| b.total_usd_0p1.cmp(&a.total_usd_0p1));

    let mut deduped: Vec<Candidate> = Vec::new();
    for r in pass_rows {
        let keep = deduped
            .iter()
            .all(|d| d.start != r.start && d.end != r.end && d.slippage_ticks != r.slippage_ticks);
        if keep {
            deduped.push(r);
        }
        if deduped.len() >= 5 {
            break;
        }
    }

    let mut md = String::new();
    md.push_str("# MacroSoup Robustness Gate\n\n");
    md.push_str("Gates:\n");
    md.push_str("- min average trades/month >= 8\n");
    md.push_str("- positive month rate >= 55%\n");
    md.push_str("- max monthly equity drawdown (USD, 0.1 BTC) <= 2500\n");
    md.push_str("- total USD (0.1 BTC) > 0\n");
    md.push_str("- PF >= 1.20\n\n");

    let total = rows.len();
    let pass_count = rows.iter().filter(|x| x.status == "PASS").count();
    md.push_str("- Stress shortlist: top 3 windows from MACRO_SOUP_REALISM_REPORT.md\n");
    md.push_str(&format!(
        "- Tested stress rows: {}\n- Pass rows: {}\n\n",
        total, pass_count
    ));

    md.push_str("## Top PASS (deduped)\n\n");
    md.push_str("| rank | start | end | slip | fee_mult | trades | win_rate_% | pf_r | profit_r | total_usd_0p1 | pos_months | months | max_monthly_dd |\n");
    md.push_str("|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for (i, r) in deduped.iter().enumerate() {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            r.start,
            r.end,
            r.slippage_ticks,
            (r.fee_mult * Decimal::new(100, 0)).round_dp(0),
            r.trades,
            r.win_rate.round_dp(2),
            r.pf_r.round_dp(2),
            r.profit_r.round_dp(2),
            r.total_usd_0p1.round_dp(2),
            r.pos_months,
            r.months,
            r.max_monthly_dd.round_dp(2)
        ));
    }

    md.push_str("\n## Top FAIL (why)\n\n");
    md.push_str("| start | end | slip | fee_mult | total_usd_0p1 | pf_r | fail_reason |\n");
    md.push_str("|---|---|---:|---:|---:|---:|---|\n");
    for r in rows.iter().filter(|x| x.status == "FAIL").take(20) {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            r.start,
            r.end,
            r.slippage_ticks,
            (r.fee_mult * Decimal::new(100, 0)).round_dp(0),
            r.total_usd_0p1.round_dp(2),
            r.pf_r.round_dp(2),
            r.fail_reason
        ));
    }

    std::fs::write("reports/strategy_overviews/MACRO_SOUP_ROBUSTNESS.md", md)
        .unwrap_or_else(|e| panic!("failed writing report: {}", e));

    println!("Wrote reports/strategy_overviews/MACRO_SOUP_ROBUSTNESS.md");
}
