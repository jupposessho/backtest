use backtest::candle_stick_loader::CandleStickLoader;
use backtest::execute;
use backtest::model::candle_stick::CandleStick;
use backtest::strategies::ce::{resample_to_5m, score, CeConfig, CeStrategy};
use std::fs;

struct Args {
    parquet: Option<String>,
    csv: Option<String>,
    top: usize,
    max_bars: usize,
    single_best: bool,
    research_london: bool,
    folds: usize,
    min_avg_test_trades: usize,
    commission_mult: i64,
    slippage_mult: i64,
}

fn parse_args() -> Args {
    let mut parquet = None;
    let mut csv = None;
    let mut top = 20usize;
    let mut max_bars = 120_000usize;
    let mut single_best = false;
    let mut research_london = false;
    let mut folds = 5usize;
    let mut min_avg_test_trades = 0usize;
    let mut commission_mult = 100i64;
    let mut slippage_mult = 100i64;
    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1usize;
    while i < argv.len() {
        match argv[i].as_str() {
            "--parquet" if i + 1 < argv.len() => {
                parquet = Some(argv[i + 1].clone());
                i += 2;
            }
            "--csv" if i + 1 < argv.len() => {
                csv = Some(argv[i + 1].clone());
                i += 2;
            }
            "--top" if i + 1 < argv.len() => {
                top = argv[i + 1].parse::<usize>().unwrap_or(20);
                i += 2;
            }
            "--max-bars" if i + 1 < argv.len() => {
                max_bars = argv[i + 1].parse::<usize>().unwrap_or(120_000);
                i += 2;
            }
            "--single-best" => {
                single_best = true;
                i += 1;
            }
            "--research-london" => {
                research_london = true;
                i += 1;
            }
            "--folds" if i + 1 < argv.len() => {
                folds = argv[i + 1].parse::<usize>().unwrap_or(5);
                i += 2;
            }
            "--min-avg-test-trades" if i + 1 < argv.len() => {
                min_avg_test_trades = argv[i + 1].parse::<usize>().unwrap_or(0);
                i += 2;
            }
            "--commission-mult" if i + 1 < argv.len() => {
                commission_mult = argv[i + 1].parse::<i64>().unwrap_or(100);
                i += 2;
            }
            "--slippage-mult" if i + 1 < argv.len() => {
                slippage_mult = argv[i + 1].parse::<i64>().unwrap_or(100);
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    Args {
        parquet,
        csv,
        top,
        max_bars,
        single_best,
        research_london,
        folds,
        min_avg_test_trades,
        commission_mult,
        slippage_mult,
    }
}

fn rolling_fold_slices(data: &[CandleStick], folds: usize) -> Vec<(Vec<CandleStick>, Vec<CandleStick>)> {
    let f = folds.max(3);
    let n = data.len();
    let seg = (n / f).max(1);
    let mut out = Vec::new();
    for i in 1..f {
        let test_start = i * seg;
        let test_end = if i == f - 1 { n } else { ((i + 1) * seg).min(n) };
        if test_start >= n || test_end <= test_start {
            continue;
        }
        let train = data[..test_start].to_vec();
        let test = data[test_start..test_end].to_vec();
        if train.len() > 50 && test.len() > 20 {
            out.push((train, test));
        }
    }
    out
}

fn load(args: &Args) -> Vec<CandleStick> {
    if let Some(path) = &args.parquet {
        return CandleStickLoader::load_parquet(path).expect("load parquet");
    }
    if let Some(path) = &args.csv {
        return CandleStickLoader::load_csv(path).expect("load csv");
    }
    panic!("pass --parquet <path> or --csv <path>");
}

fn main() {
    let args = parse_args();
    let mut raw = load(&args);
    if raw.len() > args.max_bars {
        let start = raw.len() - args.max_bars;
        raw = raw[start..].to_vec();
    }
    let data_5m = resample_to_5m(&raw);

    let mut cases: Vec<(String, CeConfig)> = Vec::new();

    if args.single_best {
        let mut cfg = CeConfig::default();
        cfg.min_swing_points = 8.into();
        cfg.rr_trend_aligned = rust_decimal::Decimal::new(25, 1);
        cfg.rr_counter_trend = rust_decimal::Decimal::new(12, 1);
        cfg.max_wait_bars = 3;
        cfg.max_hold_bars = 48;
        cfg.use_trend_filter = true;
        cfg.use_vol_filter = true;
        cfg.require_impulse_move = false;
        cfg.trade_london_open = true;
        cfg.trade_ny_am = false;
        cfg.trade_ny_mid = false;
        cfg.trade_power_hour = false;
        cases.push(("london_only_s8_rr2.5/1.2_w3_h48_t1_v1_q0".to_string(), cfg));
    }
    let swing_points = if args.research_london { vec![6, 8, 10] } else { vec![8, 10] };
    let rr_aligned = if args.research_london { vec![18, 20, 22, 25] } else { vec![20, 22, 25] };
    let rr_counter = if args.research_london { vec![10, 12, 15] } else { vec![12, 15] };
    let max_waits = if args.research_london { vec![2usize, 3usize, 5usize] } else { vec![3usize, 5usize] };
    let max_holds = if args.research_london {
        vec![12usize, 18usize, 24usize, 36usize, 48usize, 60usize]
    } else {
        vec![12usize, 18usize, 24usize, 36usize, 48usize]
    };
    let trend_filters = vec![true];
    let vol_filters = vec![true];
    let impulse_filters = if args.research_london { vec![true, false] } else { vec![true, false] };
    let rejection_filters = if args.research_london { vec![true, false] } else { vec![false] };
    let session_profiles = if args.research_london {
        vec![
            (true, false, false, false, "london_only"),
            (true, true, false, false, "london_nyam"),
        ]
    } else {
        vec![
            (false, true, true, false, "ny_am_mid"),
            (false, true, false, false, "ny_am_only"),
            (false, false, true, false, "ny_mid_only"),
            (false, true, true, true, "ny_full"),
            (true, false, false, false, "london_only"),
            (true, true, true, false, "london_ny"),
        ]
    };

    for min_swing in swing_points {
        for ra in &rr_aligned {
            for rc in &rr_counter {
                for mw in &max_waits {
                    for mh in &max_holds {
                        for tf in &trend_filters {
                            for vf in &vol_filters {
                                for qf in &impulse_filters {
                                    for rf in &rejection_filters {
                                        for (lon, nyam, nymid, ph, sname) in &session_profiles {
                                        let mut cfg = CeConfig::default();
                                        cfg.min_swing_points = min_swing.into();
                                        cfg.rr_trend_aligned = rust_decimal::Decimal::new(*ra, 1);
                                        cfg.rr_counter_trend = rust_decimal::Decimal::new(*rc, 1);
                                        cfg.max_wait_bars = *mw;
                                        cfg.max_hold_bars = *mh;
                                        cfg.use_trend_filter = *tf;
                                        cfg.use_vol_filter = *vf;
                                        cfg.require_impulse_move = *qf;
                                        cfg.require_rejection_confirm = *rf;
                                        cfg.trade_london_open = *lon;
                                        cfg.trade_ny_am = *nyam;
                                        cfg.trade_ny_mid = *nymid;
                                        cfg.trade_power_hour = *ph;
                                        cfg.commission_round_trip_usd =
                                            cfg.commission_round_trip_usd * rust_decimal::Decimal::new(args.commission_mult, 2);
                                        cfg.slippage_round_trip_usd =
                                            cfg.slippage_round_trip_usd * rust_decimal::Decimal::new(args.slippage_mult, 2);
                                        let name = format!(
                                            "{}_s{}_rr{:.1}/{:.1}_w{}_h{}_t{}_v{}_q{}_r{}",
                                            sname,
                                            min_swing,
                                            (*ra as f64) / 10.0,
                                            (*rc as f64) / 10.0,
                                            *mw,
                                            *mh,
                                            if *tf { 1 } else { 0 },
                                            if *vf { 1 } else { 0 },
                                            if *qf { 1 } else { 0 },
                                            if *rf { 1 } else { 0 }
                                        );
                                        if !args.single_best {
                                            cases.push((name, cfg));
                                        }
                                    }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let folds = if args.research_london {
        rolling_fold_slices(&data_5m, args.folds)
    } else {
        let split = ((data_5m.len() as f64) * 0.7).floor() as usize;
        let split = split.clamp(1, data_5m.len().saturating_sub(1));
        vec![(data_5m[..split].to_vec(), data_5m[split..].to_vec())]
    };

    let mut rows = Vec::new();
    for (name, cfg) in cases {
        let mut train_net = rust_decimal::Decimal::ZERO;
        let mut train_pf = rust_decimal::Decimal::ZERO;
        let mut train_win = rust_decimal::Decimal::ZERO;
        let mut train_n = 0usize;
        let mut test_net = rust_decimal::Decimal::ZERO;
        let mut test_pf = rust_decimal::Decimal::ZERO;
        let mut test_win = rust_decimal::Decimal::ZERO;
        let mut test_n = 0usize;
        let mut fold_count = 0usize;

        for (train, test) in &folds {
            let model_train = CeStrategy {
                data_5m: train.clone(),
                config: cfg.clone(),
            };
            let model_test = CeStrategy {
                data_5m: test.clone(),
                config: cfg.clone(),
            };
            let train_result = execute(model_train);
            let test_result = execute(model_test);
            let (trn, trpf, trw, trn_n) = score(&train_result);
            let (ten, tepf, tew, ten_n) = score(&test_result);
            train_net += trn;
            train_pf += trpf;
            train_win += trw;
            train_n += trn_n;
            test_net += ten;
            test_pf += tepf;
            test_win += tew;
            test_n += ten_n;
            fold_count += 1;
        }

        if fold_count > 0 {
            let d = rust_decimal::Decimal::from(fold_count as i64);
            train_net /= d;
            train_pf /= d;
            train_win /= d;
            test_net /= d;
            test_pf /= d;
            test_win /= d;
            train_n /= fold_count;
            test_n /= fold_count;
        }
        rows.push((
            name, train_net, train_pf, train_win, train_n, test_net, test_pf, test_win, test_n,
        ));
    }

    if args.min_avg_test_trades > 0 {
        rows.retain(|r| r.8 >= args.min_avg_test_trades);
    }

    rows.sort_by(|a, b| b.5.cmp(&a.5));

    println!("CE sweep results (walk-forward sorted by avg test net USD):");
    println!("name,train_net_usd,train_pf,train_win_rate,train_trades,test_net_usd,test_pf,test_win_rate,test_trades");
    for (name, trn, trpf, trw, trn_n, ten, tepf, tew, ten_n) in rows.iter().take(args.top) {
        println!(
            "{},{},{},{},{},{},{},{},{}",
            name,
            trn.round_dp(2),
            trpf.round_dp(2),
            trw.round_dp(2),
            trn_n,
            ten.round_dp(2),
            tepf.round_dp(2),
            tew.round_dp(2),
            ten_n
        );
    }

    let mut md = String::new();
    md.push_str("# CE Sweep (Walk-Forward)\n\n");
    md.push_str("Sorted by average `test_net_usd` descending.\n\n");
    md.push_str("| name | train_net_usd | train_pf | train_win_% | train_trades | test_net_usd | test_pf | test_win_% | test_trades |\n");
    md.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for (name, trn, trpf, trw, trn_n, ten, tepf, tew, ten_n) in &rows {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            name,
            trn.round_dp(2),
            trpf.round_dp(2),
            trw.round_dp(2),
            trn_n,
            ten.round_dp(2),
            tepf.round_dp(2),
            tew.round_dp(2),
            ten_n
        ));
    }
    fs::create_dir_all("reports/strategy_overviews").expect("create reports dir");
    fs::write("reports/strategy_overviews/CE_SWEEP.md", md).expect("write CE sweep report");
    println!("Wrote reports/strategy_overviews/CE_SWEEP.md");
}
