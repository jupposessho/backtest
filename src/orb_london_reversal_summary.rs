extern crate rust_decimal;

use chrono::NaiveTime;
use clap::{Arg, Command};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    execute,
    model::{backtest_result::BacktestResult, candle_stick::CandleStick, trade_result::TradeResult},
    strategies::orb_london_reversal::{OrbLondonReversal, OrbLondonReversalConfig},
};

#[derive(Clone)]
struct Row {
    profile: &'static str,
    asset: &'static str,
    orb_window_mins: u32,
    session_close: String,
    trades: usize,
    win_rate: Decimal,
    profit_factor: Decimal,
    max_dd_pct: Decimal,
    profit_r: Decimal,
    pnl_pct: Decimal,
    net_profit_usd: Decimal,
    max_dd_usd: Decimal,
}

#[derive(Clone, Copy)]
struct Preset {
    profile: &'static str,
    orb_window_mins: u32,
    session_close: (u32, u32),
    min_excursion_pct: Decimal,
    max_reenter_bars: Option<usize>,
    breakeven_at_r: Option<Decimal>,
    time_stop_bars: Option<usize>,
}

fn load_parquet(path: &str) -> Vec<CandleStick> {
    CandleStickLoader::load_source(CandleDataSource::ParquetPath(path)).expect("failed loading parquet")
}

fn summarize(result: &BacktestResult) -> (usize, Decimal, Decimal, Decimal, Decimal, Decimal, Decimal, Decimal) {
    let total = result.number_of_trades();
    let wins = result.result(TradeResult::Winner);
    let win_rate = if total == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from_i32(wins as i32).unwrap() / Decimal::from_i32(total as i32).unwrap()
            * Decimal::from(100))
        .round_dp(2)
    };

    let mut capital = Decimal::from(1000);
    let mut peak = capital;
    let mut max_dd = Decimal::ZERO;
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
        if peak > Decimal::ZERO {
            let dd_abs = peak - capital;
            if dd_abs > max_dd_abs {
                max_dd_abs = dd_abs;
            }
            let dd = ((peak - capital) / peak * Decimal::from(100)).round_dp(2);
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }

    let pf = if gross_loss > Decimal::ZERO {
        (gross_profit / gross_loss).round_dp(2)
    } else {
        Decimal::ZERO
    };
    let net_profit_usd = (capital - Decimal::from(1000)).round_dp(2);
    (
        total,
        win_rate,
        pf,
        max_dd,
        result.profit_in_r(),
        result.pnl(),
        net_profit_usd,
        max_dd_abs.round_dp(2),
    )
}

fn run_case(
    profile: &'static str,
    asset: &'static str,
    data: &[CandleStick],
    orb_window_mins: u32,
    session_close: (u32, u32),
    min_excursion_pct: Decimal,
    max_reenter_bars: Option<usize>,
    breakeven_at_r: Option<Decimal>,
    time_stop_bars: Option<usize>,
) -> Row {
    let cfg = OrbLondonReversalConfig {
        orb_start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        orb_end: NaiveTime::from_hms_opt(8, orb_window_mins, 0).unwrap(),
        session_end: NaiveTime::from_hms_opt(session_close.0, session_close.1, 0).unwrap(),
        eod_close: true,
        min_first_break_excursion_pct_of_orb: min_excursion_pct,
        max_bars_to_reenter: max_reenter_bars,
        breakeven_at_r,
        time_stop_bars,
    };

    let result = execute(OrbLondonReversal {
        data: data.to_vec(),
        config: cfg,
    });

    let (trades, win_rate, pf, max_dd, profit_r, pnl_pct, net_profit_usd, max_dd_usd) = summarize(&result);

    Row {
        profile,
        asset,
        orb_window_mins,
        session_close: format!("{:02}:{:02}", session_close.0, session_close.1),
        trades,
        win_rate,
        profit_factor: pf,
        max_dd_pct: max_dd,
        profit_r,
        pnl_pct,
        net_profit_usd,
        max_dd_usd,
    }
}

fn main() {
    let matches = Command::new("ORB London Reversal Summary")
        .arg(
            Arg::new("max-bars")
                .long("max-bars")
                .value_parser(clap::value_parser!(usize))
                .required(false),
        )
        .get_matches();

    let max_bars = matches.get_one::<usize>("max-bars").copied();

    let mut mnq = load_parquet("assets/mnq_1m_cont.parquet");
    let mut mes = load_parquet("assets/mes_1m_cont.parquet");
    let mut gold = load_parquet("assets/gold_1m_cont.parquet");

    if let Some(limit) = max_bars {
        if mnq.len() > limit {
            mnq.truncate(limit);
        }
        if mes.len() > limit {
            mes.truncate(limit);
        }
        if gold.len() > limit {
            gold.truncate(limit);
        }
    }

    let baseline_presets = [
        Preset {
            profile: "baseline_15_12",
            orb_window_mins: 15,
            session_close: (12, 0),
            min_excursion_pct: Decimal::ZERO,
            max_reenter_bars: None,
            breakeven_at_r: None,
            time_stop_bars: None,
        },
        Preset {
            profile: "baseline_15_14",
            orb_window_mins: 15,
            session_close: (14, 0),
            min_excursion_pct: Decimal::ZERO,
            max_reenter_bars: None,
            breakeven_at_r: None,
            time_stop_bars: None,
        },
        Preset {
            profile: "baseline_15_17",
            orb_window_mins: 15,
            session_close: (17, 0),
            min_excursion_pct: Decimal::ZERO,
            max_reenter_bars: None,
            breakeven_at_r: None,
            time_stop_bars: None,
        },
    ];

    let optimized_presets = [
        Preset {
            profile: "mnq_optimized",
            orb_window_mins: 30,
            session_close: (14, 0),
            min_excursion_pct: Decimal::from(20),
            max_reenter_bars: Some(12),
            breakeven_at_r: None,
            time_stop_bars: None,
        },
        Preset {
            profile: "mes_optimized",
            orb_window_mins: 15,
            session_close: (18, 0),
            min_excursion_pct: Decimal::from(10),
            max_reenter_bars: Some(12),
            breakeven_at_r: None,
            time_stop_bars: None,
        },
        Preset {
            profile: "gold_optimized",
            orb_window_mins: 15,
            session_close: (18, 0),
            min_excursion_pct: Decimal::from(10),
            max_reenter_bars: None,
            breakeven_at_r: None,
            time_stop_bars: None,
        },
    ];

    let datasets: [(&str, &Vec<CandleStick>); 3] = [
        ("MNQ", &mnq),
        ("MES", &mes),
        ("GOLD", &gold),
    ];

    let mut rows: Vec<Row> = Vec::new();
    for (asset, data) in &datasets {
        for p in baseline_presets {
            rows.push(run_case(
                p.profile,
                asset,
                data,
                p.orb_window_mins,
                p.session_close,
                p.min_excursion_pct,
                p.max_reenter_bars,
                p.breakeven_at_r,
                p.time_stop_bars,
            ));
        }

        let asset_optimized = match *asset {
            "MNQ" => Some(optimized_presets[0]),
            "MES" => Some(optimized_presets[1]),
            "GOLD" => Some(optimized_presets[2]),
            _ => None,
        };
        if let Some(p) = asset_optimized {
            rows.push(run_case(
                p.profile,
                asset,
                data,
                p.orb_window_mins,
                p.session_close,
                p.min_excursion_pct,
                p.max_reenter_bars,
                p.breakeven_at_r,
                p.time_stop_bars,
            ));
        }
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                      ORB LONDON REVERSAL — SUMMARY MATRIX                       ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("{:<10} {:<6} {:<5} {:<6} {:>7} {:>7} {:>6} {:>8} {:>10} {:>8} {:>11} {:>10}",
        "profile", "asset", "orb", "close", "trades", "win%", "pf", "maxdd%", "profit_r", "pnl%", "net_usd", "maxdd_usd"
    );
    println!("{}", "-".repeat(120));

    for r in &rows {
        println!(
            "{:<10} {:<6} {:<5} {:<6} {:>7} {:>7} {:>6} {:>8} {:>10} {:>8} {:>11} {:>10}",
            r.profile,
            r.asset,
            format!("{}m", r.orb_window_mins),
            r.session_close,
            r.trades,
            r.win_rate,
            r.profit_factor,
            r.max_dd_pct,
            r.profit_r,
            r.pnl_pct,
            r.net_profit_usd,
            r.max_dd_usd,
        );
    }

    println!();
    println!("Best case per asset (by pnl%):");
    for asset in ["MNQ", "MES", "GOLD"] {
        if let Some(best) = rows
            .iter()
            .filter(|r| r.asset == asset)
            .max_by(|a, b| a.pnl_pct.cmp(&b.pnl_pct))
        {
            println!(
                "- {} {} -> orb={}m, close={}, trades={}, win%={}, pf={}, pnl%={}, net_usd={}, maxdd_usd={}",
                asset,
                best.profile,
                best.orb_window_mins,
                best.session_close,
                best.trades,
                best.win_rate,
                best.profit_factor,
                best.pnl_pct,
                best.net_profit_usd,
                best.max_dd_usd
            );
        }
    }
}
