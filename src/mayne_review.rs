use rust_decimal::{prelude::FromPrimitive, Decimal};

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::engine::types::ExecutionConfig;
use backtest::model::candle_stick::CandleStick;
use backtest::model::trade_result::TradeResult;
use backtest::model::trigger_type::TriggerType;
use backtest::strategies::mayne::{Mayne, ReversalPattern, SlVariant, TpVariant};

#[derive(Clone)]
struct ReviewRow {
    pnl: Decimal,
    trades: usize,
    winners: usize,
    expenses: usize,
    profit_r: Decimal,
    cfg: String,
    htf_sfp_hits: usize,
    reversal_pass: usize,
    ifvg_found: usize,
    ifvg_distance_reject: usize,
    ltf_trigger_hit: usize,
    rr_pass: usize,
}

fn cap_pair(htf_data: &[CandleStick], ltf_data: &[CandleStick], htf_cap: usize) -> (Vec<CandleStick>, Vec<CandleStick>) {
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

fn load_btc_12h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_12h.json"))
}

fn load_btc_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_4h.json"))
}

fn load_btc_15m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_15m.json"))
}

fn load_btc_1h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_1h.json"))
}

fn load_btc_1h_downloaded() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/BTCUSDT_1h.json"))
}

fn load_btc_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/binance_BTCUSDT_5m.json"))
}

fn load_btc_1m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../assets/BTCUSDT_1m.json"))
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

fn resample_1h_to_12h(candles_1h: &[CandleStick]) -> Vec<CandleStick> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 11 < candles_1h.len() {
        let chunk = &candles_1h[i..i + 12];
        let open = chunk[0].open;
        let close = chunk[11].close;
        let open_time = chunk[0].open_time;
        let close_time = chunk[11].close_time;
        let mut high = chunk[0].high;
        let mut low = chunk[0].low;
        for c in chunk.iter().skip(1) {
            if c.high > high {
                high = c.high;
            }
            if c.low < low {
                low = c.low;
            }
        }
        out.push(CandleStick {
            open_time,
            open,
            high,
            low,
            close,
            close_time,
        });
        i += 12;
    }
    out
}

fn resample_minutes(candles: &[CandleStick], chunk_size: usize) -> Vec<CandleStick> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + (chunk_size - 1) < candles.len() {
        let chunk = &candles[i..i + chunk_size];
        let open = chunk[0].open;
        let close = chunk[chunk_size - 1].close;
        let open_time = chunk[0].open_time;
        let close_time = chunk[chunk_size - 1].close_time;
        let mut high = chunk[0].high;
        let mut low = chunk[0].low;
        for c in chunk.iter().skip(1) {
            if c.high > high {
                high = c.high;
            }
            if c.low < low {
                low = c.low;
            }
        }
        out.push(CandleStick {
            open_time,
            open,
            high,
            low,
            close,
            close_time,
        });
        i += chunk_size;
    }
    out
}

fn execution() -> ExecutionConfig {
    ExecutionConfig {
        commission_rate_per_side: Decimal::new(1, 3),
        fee_rate_per_side: Decimal::ZERO,
        slippage_ticks_per_side: 1,
        tick_size: Decimal::from_f32(0.01).unwrap(),
    }
}

fn run_asset(name: &str, pair: &str, htf_data: Vec<CandleStick>, ltf_data: Vec<CandleStick>) {
    let (htf_data, ltf_data) = cap_pair(&htf_data, &ltf_data, 3000);
    if htf_data.is_empty() || ltf_data.is_empty() {
        println!("\n=== {} | {} ===", name, pair);
        println!("skipped: empty htf/ltf after alignment (htf={}, ltf={})", htf_data.len(), ltf_data.len());
        return;
    }
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

    println!("\n=== {} | {} ===", name, pair);
    println!("using capped dataset: htf={}, ltf={}", htf_data.len(), ltf_data.len());
    let mut rows: Vec<ReviewRow> = Vec::new();

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
                            rows.push(ReviewRow {
                                pnl: result.pnl(),
                                trades: result.number_of_trades(),
                                winners: result.result(TradeResult::Winner),
                                expenses: result.result(TradeResult::Expense),
                                profit_r: result.profit_in_r(),
                                cfg: format!(
                                    "pat={:?} sl={:?} tp={:?} trig={:?} rr={} ifvg_max={}",
                                    pattern, sl_variant, tp_variant, trigger_type, rr, ifvg_max
                                ),
                                htf_sfp_hits: diag.htf_sfp_hits,
                                reversal_pass: diag.reversal_pass,
                                ifvg_found: diag.ifvg_found,
                                ifvg_distance_reject: diag.ifvg_distance_reject,
                                ltf_trigger_hit: diag.ltf_trigger_hit,
                                rr_pass: diag.rr_pass,
                            });
                        }
                    }
                }
            }
        }
    }

    let active_configs = rows.iter().filter(|r| r.trades > 0).count();
    let profitable_configs = rows.iter().filter(|r| r.pnl > Decimal::ZERO).count();
    println!(
        "configs={}, active(trades>0)={}, profitable(pnl>0)={}",
        rows.len(),
        active_configs,
        profitable_configs
    );

    let mut by_pnl = rows.clone();
    by_pnl.sort_by(|a, b| b.pnl.cmp(&a.pnl));
    println!("\nTop by pnl:");
    for (rank, row) in by_pnl.iter().take(8).enumerate() {
        println!(
            "#{:02} pnl={}%, trades={}, w={}, l={}, r={} :: {}",
            rank + 1,
            row.pnl,
            row.trades,
            row.winners,
            row.expenses,
            row.profit_r,
            row.cfg
        );
    }

    let mut by_trades = rows.clone();
    by_trades.sort_by(|a, b| b.trades.cmp(&a.trades));
    println!("\nTop by trades:");
    for (rank, row) in by_trades.iter().take(8).enumerate() {
        println!(
            "#{:02} trades={}, pnl={}%, w={}, l={} :: {}",
            rank + 1,
            row.trades,
            row.pnl,
            row.winners,
            row.expenses,
            row.cfg
        );
    }

    let mut by_r = rows.clone();
    by_r.sort_by(|a, b| b.profit_r.cmp(&a.profit_r));
    println!("\nTop by profit_in_r:");
    for (rank, row) in by_r.iter().take(8).enumerate() {
        println!(
            "#{:02} r={}, pnl={}%, trades={} :: {}",
            rank + 1,
            row.profit_r,
            row.pnl,
            row.trades,
            row.cfg
        );
    }

    println!("\nFirst active configs (trades>0):");
    for (rank, row) in by_pnl.iter().filter(|r| r.trades > 0).take(12).enumerate() {
        println!(
            "#{:02} pnl={}%, trades={}, w={}, l={}, sfp={}, rev={}, ifvg_ok={}, ifvg_rej={}, trig={}, rr={} :: {}",
            rank + 1,
            row.pnl,
            row.trades,
            row.winners,
            row.expenses,
            row.htf_sfp_hits,
            row.reversal_pass,
            row.ifvg_found,
            row.ifvg_distance_reject,
            row.ltf_trigger_hit,
            row.rr_pass,
            row.cfg
        );
    }
}

fn main() {
    let eth_1h = load_eth_1h();
    run_asset("ETHUSDT", "HTF 4h / LTF 15m", load_eth_4h(), load_eth_15m());
    run_asset("ETHUSDT", "HTF 1h / LTF 5m", eth_1h, load_eth_5m());
}
