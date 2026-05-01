extern crate rust_decimal;

use chrono::NaiveTime;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, trade_result::TradeResult},
    strategies::orb_london_reversal::{OrbLondonReversal, OrbLondonReversalConfig},
};

#[derive(Clone)]
struct Candidate {
    asset: &'static str,
    orb_mins: u32,
    close_h: u32,
    min_exc: Decimal,
    max_reenter: Option<usize>,
    be_r: Option<Decimal>,
    time_stop: Option<usize>,
    trades: usize,
    win_rate: Decimal,
    pf: Decimal,
    pnl_pct: Decimal,
    net_usd: Decimal,
    maxdd_usd: Decimal,
}

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path)).expect("failed loading parquet")
}

fn summarize(result: &BacktestResult) -> (usize, Decimal, Decimal, Decimal, Decimal, Decimal) {
    let total = result.number_of_trades();
    let wins = result.result(TradeResult::Winner);
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_i32(wins as i32).unwrap() / Decimal::from_i32(total as i32).unwrap() * Decimal::from(100))
            .round_dp(2)
    };

    let mut capital = Decimal::from(1000);
    let mut peak = capital;
    let mut max_dd_abs = Decimal::ZERO;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;
    let r = Decimal::from_f32(0.01).unwrap();

    for t in &result.trades {
        let gross_r = match t.result {
            TradeResult::Winner => t.rr().0,
            TradeResult::Expense => Decimal::from(-1),
            TradeResult::BreakEven => Decimal::ZERO,
        };
        let change = capital * r * gross_r.trunc_with_scale(4) - t.total_costs();
        if change > Decimal::ZERO {
            gross_profit += change;
        } else if change < Decimal::ZERO {
            gross_loss += -change;
        }
        capital += change;
        if capital > peak {
            peak = capital;
        }
        let dd_abs = peak - capital;
        if dd_abs > max_dd_abs {
            max_dd_abs = dd_abs;
        }
    }

    let pf = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).round_dp(2)
    } else {
        Decimal::ZERO
    };
    let pnl_pct = ((capital - Decimal::from(1000)) / Decimal::from(1000) * Decimal::from(100)).round_dp(2);
    let net_usd = (capital - Decimal::from(1000)).round_dp(2);
    (total, win_rate, pf, pnl_pct, net_usd, max_dd_abs.round_dp(2))
}

fn eval(
    asset: &'static str,
    data: &[CandleStick],
    orb_mins: u32,
    close_h: u32,
    min_exc: Decimal,
    max_reenter: Option<usize>,
    be_r: Option<Decimal>,
    time_stop: Option<usize>,
) -> Candidate {
    let cfg = OrbLondonReversalConfig {
        orb_start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        orb_end: NaiveTime::from_hms_opt(8, orb_mins, 0).unwrap(),
        session_end: NaiveTime::from_hms_opt(close_h, 0, 0).unwrap(),
        eod_close: true,
        min_first_break_excursion_pct_of_orb: min_exc,
        max_bars_to_reenter: max_reenter,
        breakeven_at_r: be_r,
        time_stop_bars: time_stop,
    };
    let result = execute(OrbLondonReversal {
        data: data.to_vec(),
        config: cfg,
    });
    let (trades, win_rate, pf, pnl_pct, net_usd, maxdd_usd) = summarize(&result);
    Candidate {
        asset,
        orb_mins,
        close_h,
        min_exc,
        max_reenter,
        be_r,
        time_stop,
        trades,
        win_rate,
        pf,
        pnl_pct,
        net_usd,
        maxdd_usd,
    }
}

fn main() {
    let mnq = load_parquet("assets/mnq_1m_cont.parquet");
    let mes = load_parquet("assets/mes_1m_cont.parquet");
    let gold = load_parquet("assets/gold_1m_cont.parquet");

    let assets: [(&str, &Vec<CandleStick>); 3] = [("MNQ", &mnq), ("MES", &mes), ("GOLD", &gold)];
    let orb = [10u32, 15u32, 20u32, 30u32];
    let close = [14u32, 16u32, 17u32, 18u32];
    let min_exc = [0, 5, 10, 15, 20].map(Decimal::from);
    let max_reenter = [None, Some(12usize), Some(24usize), Some(36usize)];
    let be_r = [None, Some(Decimal::new(6, 1)), Some(Decimal::ONE)];
    let time_stop = [None, Some(24usize), Some(36usize), Some(48usize)];

    let mut all: Vec<Candidate> = Vec::new();
    for (asset, data) in assets {
        for o in orb {
            for c in close {
                for e in min_exc {
                    for r in max_reenter {
                        for b in be_r {
                            for t in time_stop {
                                all.push(eval(asset, data, o, c, e, r, b, t));
                            }
                        }
                    }
                }
            }
        }
    }

    all.sort_by(|a, b| {
        let score_a = a.net_usd - a.maxdd_usd * Decimal::new(3, 1);
        let score_b = b.net_usd - b.maxdd_usd * Decimal::new(3, 1);
        score_b.cmp(&score_a)
    });

    let min_pf = Decimal::new(115, 2); // 1.15
    let min_net_usd = Decimal::from(400);
    let max_dd_usd = Decimal::from(250);
    let min_trades = 200usize;

    let tradable: Vec<&Candidate> = all
        .iter()
        .filter(|c| {
            c.pf >= min_pf
                && c.net_usd >= min_net_usd
                && c.maxdd_usd <= max_dd_usd
                && c.trades >= min_trades
        })
        .collect();

    println!("Top 10 by score = net_usd - 0.3*maxdd_usd");
    println!("{:<5} {:<4} {:<5} {:<5} {:<8} {:<5} {:<8} {:>6} {:>6} {:>6} {:>8} {:>9} {:>9}",
        "asset", "orb", "close", "exc%", "reenter", "be_r", "tstop", "trades", "win%", "pf", "pnl%", "net_usd", "maxdd$");
    println!("{}", "-".repeat(120));
    for c in all.iter().take(10) {
        println!(
            "{:<5} {:<4} {:<5} {:<5} {:<8} {:<5} {:<8} {:>6} {:>6} {:>6} {:>8} {:>9} {:>9}",
            c.asset,
            format!("{}m", c.orb_mins),
            format!("{}:00", c.close_h),
            c.min_exc,
            c.max_reenter.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
            c.be_r.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
            c.time_stop.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
            c.trades,
            c.win_rate,
            c.pf,
            c.pnl_pct,
            c.net_usd,
            c.maxdd_usd,
        );
    }

    println!();
    println!(
        "Tradable filter: pf>={} net_usd>={} maxdd_usd<={} trades>={}",
        min_pf, min_net_usd, max_dd_usd, min_trades
    );
    println!("{:<5} {:<4} {:<5} {:<5} {:<8} {:<5} {:<8} {:>6} {:>6} {:>6} {:>8} {:>9} {:>9}",
        "asset", "orb", "close", "exc%", "reenter", "be_r", "tstop", "trades", "win%", "pf", "pnl%", "net_usd", "maxdd$");
    println!("{}", "-".repeat(120));
    for c in tradable.iter().take(20) {
        println!(
            "{:<5} {:<4} {:<5} {:<5} {:<8} {:<5} {:<8} {:>6} {:>6} {:>6} {:>8} {:>9} {:>9}",
            c.asset,
            format!("{}m", c.orb_mins),
            format!("{}:00", c.close_h),
            c.min_exc,
            c.max_reenter.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
            c.be_r.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
            c.time_stop.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()),
            c.trades,
            c.win_rate,
            c.pf,
            c.pnl_pct,
            c.net_usd,
            c.maxdd_usd,
        );
    }
    if tradable.is_empty() {
        println!("No configs passed tradable filter.");
    }

    println!();
    for asset in ["MNQ", "MES", "GOLD"] {
        if let Some(best_net) = all
            .iter()
            .filter(|x| x.asset == asset)
            .max_by(|a, b| a.net_usd.cmp(&b.net_usd))
        {
            println!(
                "Best {} by net_usd: orb={}m close={}:00 exc%={} reenter={:?} be_r={:?} tstop={:?} trades={} pf={} pnl%={} net_usd={} maxdd$={}",
                asset,
                best_net.orb_mins,
                best_net.close_h,
                best_net.min_exc,
                best_net.max_reenter,
                best_net.be_r,
                best_net.time_stop,
                best_net.trades,
                best_net.pf,
                best_net.pnl_pct,
                best_net.net_usd,
                best_net.maxdd_usd,
            );
        }

        let best_tradable = tradable
            .iter()
            .filter(|x| x.asset == asset)
            .max_by(|a, b| a.net_usd.cmp(&b.net_usd));
        match best_tradable {
            Some(c) => println!(
                "Best {} tradable: orb={}m close={}:00 exc%={} reenter={:?} be_r={:?} tstop={:?} trades={} win%={} pf={} pnl%={} net_usd={} maxdd$={}",
                asset,
                c.orb_mins,
                c.close_h,
                c.min_exc,
                c.max_reenter,
                c.be_r,
                c.time_stop,
                c.trades,
                c.win_rate,
                c.pf,
                c.pnl_pct,
                c.net_usd,
                c.maxdd_usd,
            ),
            None => println!("Best {} tradable: none", asset),
        }
    }
}
