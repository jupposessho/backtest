use std::collections::BTreeMap;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    engine::types::ExecutionConfig,
    model::{candle_stick::CandleStick, decimal::DecimalVec, session::Session, sl_trategy::SlStrategy},
    parse_datetime, to_new_york_time,
    strategies::macro_soup::MacroSoup,
};
use chrono::Datelike;
use rust_decimal::Decimal;
use backtest::model::trading_model::TradingModel;

#[derive(Clone)]
struct SetupRow {
    start: String,
    end: String,
    trades: usize,
    profit_r: Decimal,
    pf_r: Decimal,
}

fn load_binance() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_15m.json"))
}

fn main() {
    let candles = load_binance();
    let slip = 1_i32;
    let qty_btc = Decimal::new(1, 1); // 0.1 BTC

    let base_start = parse_datetime("2022-09-30 05:00:00").unwrap().time();
    let last = parse_datetime("2022-09-30 18:00:00").unwrap().time();

    let mut setups: Vec<SetupRow> = Vec::new();
    let mut start = base_start;

    while start < last {
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
                commission_rate_per_side: Decimal::new(1, 3),
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: slip,
                tick_size: Decimal::new(1, 2),
            },
        };

        let result = model.execute();
        let losers = result.result(backtest::model::trade_result::TradeResult::Expense);
        let gross_loss_r = Decimal::from(losers as u32);
        let gross_profit_r = result.profit_in_r() + gross_loss_r;
        let pf_r = if gross_loss_r > Decimal::ZERO {
            gross_profit_r / gross_loss_r
        } else if gross_profit_r > Decimal::ZERO {
            Decimal::from(9999)
        } else {
            Decimal::ZERO
        };

        setups.push(SetupRow {
            start: start.format("%H:%M").to_string(),
            end: session_end.format("%H:%M").to_string(),
            trades: result.number_of_trades(),
            profit_r: result.profit_in_r(),
            pf_r,
        });

        start += chrono::Duration::minutes(5);
    }

    setups.sort_by(|a, b| b.profit_r.cmp(&a.profit_r).then(b.pf_r.cmp(&a.pf_r)));
    let top3: Vec<_> = setups.into_iter().take(3).collect();

    println!("macro_soup_top3_monthly_breakdown slip_ticks={} qty_btc={}", slip, qty_btc);
    for (idx, s) in top3.iter().enumerate() {
        let s_start = parse_datetime(&format!("2022-09-30 {}:00", s.start)).unwrap().time();
        let s_end = parse_datetime(&format!("2022-09-30 {}:00", s.end)).unwrap().time();
        let model = MacroSoup {
            candles: candles.clone(),
            rr_threshold: Decimal::from(3),
            be_threshold: Some(DecimalVec::new(2)),
            session: Session {
                start: s_start,
                end: s_end,
            },
            sl_strategy: SlStrategy::None,
            max_duration_min: 120,
            execution: ExecutionConfig {
                commission_rate_per_side: Decimal::new(1, 3),
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: slip,
                tick_size: Decimal::new(1, 2),
            },
        };

        let result = model.execute();
        let mut by_month: BTreeMap<String, Decimal> = BTreeMap::new();
        for t in result.trades {
            let dt = to_new_york_time(t.close_time);
            let key = format!("{:04}-{:02}", dt.year(), dt.month());
            let gross = t.points().0 * qty_btc;
            let costs = t.total_costs() * qty_btc;
            *by_month.entry(key).or_insert(Decimal::ZERO) += gross - costs;
        }

        let total: Decimal = by_month.values().copied().sum();

        println!(
            "setup_{} {}-{} trades={} profit_r={} pf_r={} total_usd_0p1btc={}",
            idx + 1,
            s.start,
            s.end,
            s.trades,
            s.profit_r.round_dp(2),
            s.pf_r.round_dp(2),
            total.round_dp(2)
        );
        println!("month,net_usd_0p1btc");
        for (m, v) in &by_month {
            println!("{},{}", m, v.round_dp(2));
        }
    }
}
