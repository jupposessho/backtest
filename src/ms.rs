extern crate rust_decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    engine::types::ExecutionConfig,
    model::{
        backtest_result::BacktestResult, candle_stick::CandleStick, decimal::DecimalVec,
        session::Session, sl_trategy::SlStrategy, trading_model::TradingModel,
    },
    parse_datetime,
    strategies::macro_soup::MacroSoup,
};
use rust_decimal::Decimal;

#[derive(Clone)]
struct Row {
    start: String,
    end: String,
    slippage_ticks: i32,
    trades: usize,
    win_rate: Decimal,
    profit_r: Decimal,
    pf_r: Decimal,
    pnl_pct: Decimal,
    points: Decimal,
    costs: Decimal,
}

fn load_binance() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_15m.json"))
}

fn summarize(start: String, end: String, slippage_ticks: i32, result: BacktestResult) -> Row {
    let winners = result.result(backtest::model::trade_result::TradeResult::Winner);
    let losers = result.result(backtest::model::trade_result::TradeResult::Expense);
    let trades = result.number_of_trades();
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(winners as u32) / Decimal::from(trades as u32) * Decimal::from(100)
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

    Row {
        start,
        end,
        slippage_ticks,
        trades,
        win_rate,
        profit_r: result.profit_in_r(),
        pf_r,
        pnl_pct: result.pnl(),
        points: result.profit_in_points(),
        costs: result.costs_total(),
    }
}

fn main() {
    let candlesticks = load_binance();
    let mut all_rows: Vec<Row> = Vec::new();

    let base_start = parse_datetime("2022-09-30 05:00:00").unwrap().time();
    let last = parse_datetime("2022-09-30 18:00:00").unwrap().time();

    for slippage_ticks in [1_i32, 2_i32, 3_i32] {
        let mut start = base_start;
        while start < last {
            let session_end = start + chrono::Duration::minutes(60);
            let trading_model = MacroSoup {
                candles: candlesticks.clone(),
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
                    slippage_ticks_per_side: slippage_ticks,
                    tick_size: Decimal::new(1, 2),
                },
            };
            let result = trading_model.execute();
            all_rows.push(summarize(
                start.format("%H:%M").to_string(),
                session_end.format("%H:%M").to_string(),
                slippage_ticks,
                result,
            ));
            start += chrono::Duration::minutes(5);
        }
    }

    let mut md = String::new();
    md.push_str("# MacroSoup Realism Validation\n\n");
    md.push_str("Dataset: BTCUSDT 15m (`assets/binance_BTCUSDT_15m.json`).\n\n");
    md.push_str("Defined time ranges: session start from 05:00 to 17:55 NY, 60-minute session length, 5-minute step.\n\n");
    md.push_str("Realism settings: next-candle execution path in strategy loop, stop-first intrabar checks, commission 0.1% per side, slippage stress at 1/2/3 ticks (tick=0.1).\n\n");

    for slip in [1_i32, 2_i32, 3_i32] {
        let mut rows: Vec<Row> = all_rows
            .iter()
            .filter(|r| r.slippage_ticks == slip)
            .cloned()
            .collect();
        rows.sort_by(|a, b| b.profit_r.cmp(&a.profit_r).then(b.pf_r.cmp(&a.pf_r)));

        let tested = rows.len();
        let profitable = rows.iter().filter(|r| r.profit_r > Decimal::ZERO).count();
        let pf_ge_12 = rows
            .iter()
            .filter(|r| r.pf_r >= Decimal::new(12, 1))
            .count();

        md.push_str(&format!("## Slippage {} Tick(s)\n\n", slip));
        md.push_str(&format!(
            "- Tested ranges: {}\n- Profit_r > 0: {}\n- PF >= 1.20: {}\n\n",
            tested, profitable, pf_ge_12
        ));
        md.push_str(
            "| start | end | trades | win_rate_% | profit_r | pf_r | pnl_% | points | costs |\n",
        );
        md.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|\n");
        for r in rows.iter().take(20) {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                r.start,
                r.end,
                r.trades,
                r.win_rate.round_dp(2),
                r.profit_r.round_dp(2),
                r.pf_r.round_dp(2),
                r.pnl_pct.round_dp(2),
                r.points.round_dp(2),
                r.costs.round_dp(2)
            ));
        }
        md.push_str("\n");
    }

    md.push_str("## Full Tested Ranges\n\n");
    md.push_str("| slippage_ticks | start | end | trades | win_rate_% | profit_r | pf_r | pnl_% | points | costs |\n");
    md.push_str("|---:|---|---|---:|---:|---:|---:|---:|---:|---:|\n");
    let mut all_sorted = all_rows.clone();
    all_sorted.sort_by(|a, b| {
        a.slippage_ticks
            .cmp(&b.slippage_ticks)
            .then(a.start.cmp(&b.start))
    });
    for r in &all_sorted {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.slippage_ticks,
            r.start,
            r.end,
            r.trades,
            r.win_rate.round_dp(2),
            r.profit_r.round_dp(2),
            r.pf_r.round_dp(2),
            r.pnl_pct.round_dp(2),
            r.points.round_dp(2),
            r.costs.round_dp(2)
        ));
    }

    std::fs::write(
        "reports/strategy_overviews/MACRO_SOUP_REALISM_REPORT.md",
        md,
    )
    .unwrap_or_else(|e| panic!("failed writing report: {}", e));

    println!("Wrote reports/strategy_overviews/MACRO_SOUP_REALISM_REPORT.md");
}
