extern crate rust_decimal;

use std::sync::Arc;
use std::{fs::File, io::Write};

use chrono::NaiveTime;
use rust_decimal::Decimal;

use backtest::candle_stick_loader::CandleStickLoader;
use backtest::model::backtest_result::BacktestResult;
use backtest::model::position_direction::PositionDirection;
use backtest::model::trade_result::TradeResult;
use backtest::strategies::weekday_engulfing::{
    optimize_configs, resample_from_1m, DayDirection, DayParams, WeekdayEngulfingConfig,
};

fn load_mnq_1m() -> Vec<backtest::model::candle_stick::CandleStick> {
    CandleStickLoader::load_parquet("assets/mnq_1m_cont.parquet")
        .unwrap_or_else(|_| panic!("failed loading assets/mnq_1m_cont.parquet"))
}

fn print_result(label: &str, result: &BacktestResult) {
    let trades = result.number_of_trades();
    let wins = result.result(backtest::model::trade_result::TradeResult::Winner);
    let win_rate = if trades == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(wins as u32) / Decimal::from(trades as u32) * Decimal::from(100)
    };
    println!(
        "{label:28} trades={trades:4} win_rate={:6}% pf_r={} pnl%={} points={}",
        win_rate.round_dp(2),
        {
            let losses = result.result(backtest::model::trade_result::TradeResult::Expense);
            let gross_loss_r = Decimal::from(losses as u32);
            let gross_profit_r = result.profit_in_r() + gross_loss_r;
            if gross_loss_r > Decimal::ZERO {
                (gross_profit_r / gross_loss_r).round_dp(2)
            } else {
                Decimal::from(9999)
            }
        },
        result.pnl().round_dp(2),
        result.profit_in_points().round_dp(2),
    );
}

fn print_diagnostics(label: &str, result: &BacktestResult, usd_per_point: Decimal) {
    let trades = result.number_of_trades();
    let wins = result.result(TradeResult::Winner);
    let losses = result.result(TradeResult::Expense);
    let be = result.result(TradeResult::BreakEven);

    let mut gross_win_points = Decimal::ZERO;
    let mut gross_loss_points = Decimal::ZERO;
    let mut long_count = 0usize;
    let mut short_count = 0usize;

    for t in &result.trades {
        match t.direction {
            PositionDirection::Long => long_count += 1,
            PositionDirection::Short => short_count += 1,
        }
        let p = t.points().0;
        if p > Decimal::ZERO {
            gross_win_points += p;
        } else if p < Decimal::ZERO {
            gross_loss_points += -p;
        }
    }

    let avg_win = if wins > 0 {
        gross_win_points / Decimal::from(wins as u32)
    } else {
        Decimal::ZERO
    };
    let avg_loss = if losses > 0 {
        gross_loss_points / Decimal::from(losses as u32)
    } else {
        Decimal::ZERO
    };
    let payoff = if avg_loss > Decimal::ZERO {
        (avg_win / avg_loss).round_dp(3)
    } else {
        Decimal::ZERO
    };
    let wr = if trades > 0 {
        Decimal::from(wins as u32) / Decimal::from(trades as u32)
    } else {
        Decimal::ZERO
    };
    let lr = if trades > 0 {
        Decimal::from(losses as u32) / Decimal::from(trades as u32)
    } else {
        Decimal::ZERO
    };
    let expectancy_points = (wr * avg_win) - (lr * avg_loss);

    let gross_win_usd = (gross_win_points * usd_per_point).round_dp(2);
    let gross_loss_usd = (gross_loss_points * usd_per_point).round_dp(2);
    let net_points = result.profit_in_points().round_dp(2);
    let net_usd = (net_points * usd_per_point).round_dp(2);

    println!("\nDiagnostics: {label}");
    println!(
        "trades={} wins={} losses={} be={} longs={} shorts={}",
        trades, wins, losses, be, long_count, short_count
    );
    println!(
        "gross_win_pts={} gross_loss_pts={} avg_win_pts={} avg_loss_pts={}",
        gross_win_points.round_dp(2),
        gross_loss_points.round_dp(2),
        avg_win.round_dp(3),
        avg_loss.round_dp(3)
    );
    println!(
        "payoff={} expectancy_pts/trade={} net_pts={} net_usd={} gross_win_usd={} gross_loss_usd={}",
        payoff,
        expectancy_points.round_dp(4),
        net_points,
        net_usd,
        gross_win_usd,
        gross_loss_usd
    );
}

fn payoff_ratio(result: &BacktestResult) -> Decimal {
    let wins = result.result(TradeResult::Winner);
    let losses = result.result(TradeResult::Expense);
    if wins == 0 || losses == 0 {
        return Decimal::ZERO;
    }
    let mut gross_win_points = Decimal::ZERO;
    let mut gross_loss_points = Decimal::ZERO;
    for t in &result.trades {
        let p = t.points().0;
        if p > Decimal::ZERO {
            gross_win_points += p;
        } else if p < Decimal::ZERO {
            gross_loss_points += -p;
        }
    }
    let avg_win = gross_win_points / Decimal::from(wins as u32);
    let avg_loss = gross_loss_points / Decimal::from(losses as u32);
    if avg_loss > Decimal::ZERO {
        avg_win / avg_loss
    } else {
        Decimal::ZERO
    }
}

fn main() {
    let data_1m = load_mnq_1m();
    let data_15m = Arc::new(resample_from_1m(&data_1m, 15));

    let mut base = WeekdayEngulfingConfig::default();
    base.max_loss_usd_per_trade = Decimal::from(250);
    base.contracts = 5;
    base.monday = DayParams {
        tp_pct: Decimal::from(25),
        sl_pct: Decimal::from(50),
        min_engulf_pct: Decimal::ZERO,
        max_engulf_pct: Decimal::from(300),
        direction: DayDirection::LongOnly,
        entry_cutoff: NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
    };
    base.tuesday = DayParams {
        tp_pct: Decimal::from(50),
        sl_pct: Decimal::from(125),
        min_engulf_pct: Decimal::from(200),
        max_engulf_pct: Decimal::from(300),
        direction: DayDirection::LongOnly,
        entry_cutoff: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
    };
    base.wednesday = DayParams {
        tp_pct: Decimal::from(75),
        sl_pct: Decimal::from(125),
        min_engulf_pct: Decimal::ZERO,
        max_engulf_pct: Decimal::from(300),
        direction: DayDirection::LongOnly,
        entry_cutoff: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
    };
    base.thursday = DayParams {
        tp_pct: Decimal::from(50),
        sl_pct: Decimal::from(75),
        min_engulf_pct: Decimal::ZERO,
        max_engulf_pct: Decimal::from(50),
        direction: DayDirection::LongOnly,
        entry_cutoff: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
    };
    base.friday = DayParams {
        tp_pct: Decimal::from(25),
        sl_pct: Decimal::from(40),
        min_engulf_pct: Decimal::ZERO,
        max_engulf_pct: Decimal::from(50),
        direction: DayDirection::LongShort,
        entry_cutoff: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
    };

    let tp_nudges = [Decimal::new(90, 2), Decimal::ONE, Decimal::new(110, 2)];
    let sl_nudges = [Decimal::new(90, 2), Decimal::ONE, Decimal::new(110, 2)];
    let engulf_min_add = [Decimal::new(-25, 0), Decimal::ZERO, Decimal::new(25, 0)];
    let engulf_max_nudges = [Decimal::new(90, 2), Decimal::ONE, Decimal::new(110, 2)];
    let cutoff_shift_minutes = [-30_i64, 0_i64, 30_i64];

    let min_tp = Decimal::new(5, 0);
    let min_sl = Decimal::new(10, 0);
    let min_max_engulf = Decimal::new(25, 0);

    let mut variants = Vec::new();
    variants.push(("edgeful_table_exact".to_string(), base.clone()));

    for tp_n in tp_nudges {
        for sl_n in sl_nudges {
            for min_add in engulf_min_add {
                for max_n in engulf_max_nudges {
                    for cutoff_shift in cutoff_shift_minutes {
                        let mut cfg = base.clone();

                        for day in [
                            &mut cfg.monday,
                            &mut cfg.tuesday,
                            &mut cfg.wednesday,
                            &mut cfg.thursday,
                            &mut cfg.friday,
                        ] {
                            day.tp_pct = (day.tp_pct * tp_n).max(min_tp).round_dp(2);
                            day.sl_pct = (day.sl_pct * sl_n).max(min_sl).round_dp(2);
                            day.min_engulf_pct = (day.min_engulf_pct + min_add).max(Decimal::ZERO);
                            day.max_engulf_pct =
                                (day.max_engulf_pct * max_n).max(min_max_engulf).round_dp(2);

                            let shifted = day
                                .entry_cutoff
                                .overflowing_add_signed(chrono::Duration::minutes(cutoff_shift))
                                .0;
                            let floor = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
                            let cap = NaiveTime::from_hms_opt(15, 45, 0).unwrap();
                            day.entry_cutoff = shifted.max(floor).min(cap);
                        }

                        variants.push((
                            format!(
                                "tp={} sl={} min_add={} max={} cutoff={}m",
                                tp_n, sl_n, min_add, max_n, cutoff_shift
                            ),
                            cfg,
                        ));
                    }
                }
            }
        }
    }

    let mut rows = optimize_configs(Arc::clone(&data_15m), variants);
    rows.sort_by(|a, b| {
        b.result
            .profit_in_r()
            .cmp(&a.result.profit_in_r())
            .then(b.result.number_of_trades().cmp(&a.result.number_of_trades()))
    });

    println!("MNQ 15m weekday engulfing sweep (Arc+rayon)");
    for row in rows.iter().take(10) {
        print_result(&row.label, &row.result);
    }

    let usd_per_point = base.point_value_usd * Decimal::from(base.contracts);

    let min_payoff = Decimal::new(7, 1);
    let min_trades = 120usize;
    let mut robust: Vec<_> = rows
        .iter()
        .filter(|r| r.result.number_of_trades() >= min_trades)
        .filter(|r| payoff_ratio(&r.result) >= min_payoff)
        .collect();
    robust.sort_by(|a, b| {
        let a_usd = (a.result.profit_in_points() * usd_per_point).round_dp(2);
        let b_usd = (b.result.profit_in_points() * usd_per_point).round_dp(2);
        b_usd
            .cmp(&a_usd)
            .then(payoff_ratio(&b.result).cmp(&payoff_ratio(&a.result)))
            .then(b.result.number_of_trades().cmp(&a.result.number_of_trades()))
    });

    println!(
        "\nRobust shortlist (payoff >= {}, trades >= {}): {} rows",
        min_payoff, min_trades, robust.len()
    );
    for row in robust.iter().take(10) {
        let pnl_usd = (row.result.profit_in_points() * usd_per_point).round_dp(2);
        println!(
            "{} | pnl_usd={} | payoff={} | trades={} | wr={}%%",
            row.label,
            pnl_usd,
            payoff_ratio(&row.result).round_dp(3),
            row.result.number_of_trades(),
            {
                let t = row.result.number_of_trades();
                if t == 0 {
                    Decimal::ZERO
                } else {
                    Decimal::from(row.result.result(TradeResult::Winner) as u32)
                        / Decimal::from(t as u32)
                        * Decimal::from(100)
                }
            }
            .round_dp(2)
        );
    }

    if let Some(top) = rows.first() {
        print_diagnostics(&top.label, &top.result, usd_per_point);
    }
    if let Some(edgeful) = rows.iter().find(|r| r.label == "edgeful_table_exact") {
        print_diagnostics("edgeful_table_exact", &edgeful.result, usd_per_point);
    }

    let csv_path = "reports/strategy_overviews/MNQ_WEEKDAY_ENGULFING_BOUNDED_SWEEP.csv";
    let mut f = File::create(csv_path).expect("create CSV");
    writeln!(
        f,
        "label,trades,win_rate,pf_r,pnl_pct,profit_r,points,pnl_usd,gross_win_usd,gross_loss_usd,avg_win_pts,avg_loss_pts,payoff"
    )
    .expect("write CSV header");

    for row in &rows {
        let trades = row.result.number_of_trades();
        let wins = row.result.result(backtest::model::trade_result::TradeResult::Winner);
        let losses = row.result.result(backtest::model::trade_result::TradeResult::Expense);
        let win_rate = if trades == 0 {
            Decimal::ZERO
        } else {
            (Decimal::from(wins as u32) / Decimal::from(trades as u32) * Decimal::from(100))
                .round_dp(2)
        };
        let gross_loss_r = Decimal::from(losses as u32);
        let gross_profit_r = row.result.profit_in_r() + gross_loss_r;
        let pf_r = if gross_loss_r > Decimal::ZERO {
            (gross_profit_r / gross_loss_r).round_dp(2)
        } else {
            Decimal::from(9999)
        };

        let mut gross_win_points = Decimal::ZERO;
        let mut gross_loss_points = Decimal::ZERO;
        for t in &row.result.trades {
            let p = t.points().0;
            if p > Decimal::ZERO {
                gross_win_points += p;
            } else if p < Decimal::ZERO {
                gross_loss_points += -p;
            }
        }
        let avg_win_pts = if wins > 0 {
            gross_win_points / Decimal::from(wins as u32)
        } else {
            Decimal::ZERO
        };
        let avg_loss_pts = if losses > 0 {
            gross_loss_points / Decimal::from(losses as u32)
        } else {
            Decimal::ZERO
        };
        let payoff = if avg_loss_pts > Decimal::ZERO {
            avg_win_pts / avg_loss_pts
        } else {
            Decimal::ZERO
        };
        let pnl_usd = (row.result.profit_in_points() * usd_per_point).round_dp(2);
        let gross_win_usd = (gross_win_points * usd_per_point).round_dp(2);
        let gross_loss_usd = (gross_loss_points * usd_per_point).round_dp(2);

        writeln!(
            f,
            "\"{}\",{},{},{},{},{},{},{},{},{},{},{},{}",
            row.label.replace('"', ""),
            trades,
            win_rate,
            pf_r,
            row.result.pnl().round_dp(2),
            row.result.profit_in_r().round_dp(2),
            row.result.profit_in_points().round_dp(2),
            pnl_usd,
            gross_win_usd,
            gross_loss_usd,
            avg_win_pts.round_dp(4),
            avg_loss_pts.round_dp(4),
            payoff.round_dp(4)
        )
        .expect("write CSV row");
    }
    println!("wrote {} rows to {}", rows.len(), csv_path);

    let robust_csv = "reports/strategy_overviews/MNQ_WEEKDAY_ENGULFING_BOUNDED_SWEEP_ROBUST.csv";
    let mut rf = File::create(robust_csv).expect("create robust CSV");
    writeln!(
        rf,
        "label,trades,win_rate,pnl_usd,payoff,pnl_pct,points"
    )
    .expect("write robust CSV header");
    for row in robust {
        let trades = row.result.number_of_trades();
        let wins = row.result.result(TradeResult::Winner);
        let win_rate = if trades == 0 {
            Decimal::ZERO
        } else {
            (Decimal::from(wins as u32) / Decimal::from(trades as u32) * Decimal::from(100))
                .round_dp(2)
        };
        let pnl_usd = (row.result.profit_in_points() * usd_per_point).round_dp(2);
        writeln!(
            rf,
            "\"{}\",{},{},{},{},{},{}",
            row.label.replace('"', ""),
            trades,
            win_rate,
            pnl_usd,
            payoff_ratio(&row.result).round_dp(4),
            row.result.pnl().round_dp(2),
            row.result.profit_in_points().round_dp(2)
        )
        .expect("write robust CSV row");
    }
    println!("wrote robust shortlist to {}", robust_csv);
}
