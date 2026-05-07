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

fn main() {
    // Match ttrades_matrix FULL mode data cap.
    let capped = 40_000usize;
    let ltf_data = Arc::new(cap(load_sol_15m(), capped));
    let mut htf_vec = cap(load_sol_4h(), capped);
    if let Some(last_ltf_open) = ltf_data.last().map(|c| c.open_time) {
        htf_vec.retain(|c| c.open_time <= last_ltf_open);
    }
    let htf_data = Arc::new(htf_vec);

    // Variant from matrix top row:
    // entry=ObMidpoint;confirm_mode=cisd_only;time_filter=all_day_all_week;
    // opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;slippage=[0,1,2]
    // Base row uses first slippage level => 0 ticks.
    let mut cfg = FractalMTFConfig::default();
    cfg.tick_size = Decimal::new(1, 3); // SOL tick from matrix
    cfg.slippage_ticks_per_side = 0;
    cfg.log_progress = false;
    cfg.entry_variant = EntryVariant::ObMidpoint;
    cfg.reversal_confirm_mode = ReversalConfirmMode::CisdOnly;
    cfg.weekday_mask = 0b0111_1111;
    cfg.killzone_mode = KillzoneMode::Off;
    cfg.rr_target = Decimal::from(2);
    cfg.poi_padding_bps = 0;
    cfg.ob_sweep_tolerance_bps = 0;

    let result = execute(TTradesFractalMTF {
        ltf_data,
        htf_data,
        config: cfg,
    });

    let qty = Decimal::from(10);
    let mut by_month: BTreeMap<String, (Decimal, usize, usize, usize)> = BTreeMap::new();

    for t in &result.trades {
        let dt = to_new_york_time(t.close_time);
        let key = format!("{:04}-{:02}", dt.year(), dt.month());
        let pnl_per_sol = t.points().0 - t.total_costs();
        let pnl_usd = pnl_per_sol * qty;
        let entry = by_month
            .entry(key)
            .or_insert((Decimal::ZERO, 0usize, 0usize, 0usize));
        entry.0 += pnl_usd;
        entry.1 += 1;
        match t.result {
            TradeResult::Winner => entry.2 += 1,
            TradeResult::Expense => entry.3 += 1,
            TradeResult::BreakEven => {}
        }
    }

    let months: Vec<_> = by_month.iter().collect();
    let take = months.len().min(6);
    let last_six = &months[months.len().saturating_sub(take)..];

    println!("SOL 15m/4h TTrades Fractal MTF (ObMidpoint, CISD-only, RR=2, slippage=0)");
    println!("Fixed position size: 10 SOL per trade");
    println!("Total trades: {}", result.trades.len());
    println!();
    println!("month,net_usd,trades,wins,losses");

    let mut total = Decimal::ZERO;
    for (m, (net, trades, wins, losses)) in last_six {
        total += *net;
        println!(
            "{},{:.2},{},{},{}",
            m,
            net.round_dp(2),
            trades,
            wins,
            losses
        );
    }
    println!("last_6m_total,{:.2}", total.round_dp(2));
}
