use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{TimeZone, Timelike};
use chrono_tz::America::New_York;

#[derive(Clone, Copy)]
enum Side {
    Top,
    Bottom,
}

#[derive(Clone)]
struct DaySample {
    range_size: f64,
    first_break: Side,
    overshoot_pct: f64,
    reclaim_bars: Option<usize>,
    two_sided: bool,
    entry_idx: Option<usize>,
    range_high: f64,
    range_low: f64,
    break_extreme: f64,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
}

#[derive(Clone)]
struct RuleResult {
    rule: String,
    trades: usize,
    win_rate: f64,
    expectancy_r: f64,
}

#[derive(Clone)]
struct PnlResult {
    rule: String,
    slip_ticks: i32,
    rt_commission_points: f64,
    trades: usize,
    win_rate: f64,
    net_expectancy_r: f64,
    avg_rr_before_costs: f64,
    target_mode: String,
}

fn dec_to_f64(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn pctile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn percentile_rank(sorted: &[f64], value: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let mut lo = 0usize;
    let mut hi = sorted.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if sorted[mid] <= value {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo as f64 / sorted.len() as f64
}

fn collect_samples(candles: &[CandleStick]) -> Vec<DaySample> {
    let mut samples = Vec::new();
    let mut i = 0usize;

    while i < candles.len() {
        let dt = New_York
            .timestamp_opt(candles[i].open_time, 0)
            .single()
            .expect("valid timestamp");
        let current_date = dt.date_naive();

        let mut j = i;
        while j < candles.len() {
            let d2 = New_York
                .timestamp_opt(candles[j].open_time, 0)
                .single()
                .expect("valid timestamp")
                .date_naive();
            if d2 != current_date {
                break;
            }
            j += 1;
        }

        let day = &candles[i..j];
        let mut range_high = f64::NEG_INFINITY;
        let mut range_low = f64::INFINITY;
        let mut range_exists = false;
        let mut post_ix = Vec::new();

        for (k, c) in day.iter().enumerate() {
            let t = New_York
                .timestamp_opt(c.open_time, 0)
                .single()
                .expect("valid timestamp");
            let hour = t.hour();
            if (6..9).contains(&hour) {
                range_exists = true;
                let h = dec_to_f64(c.high.0);
                let l = dec_to_f64(c.low.0);
                if h > range_high {
                    range_high = h;
                }
                if l < range_low {
                    range_low = l;
                }
            }
            if hour >= 9 {
                post_ix.push(k);
            }
        }

        if !range_exists || post_ix.is_empty() || range_high <= range_low {
            i = j;
            continue;
        }

        let range_size = range_high - range_low;
        let mut break_idx = None;
        let mut break_side = None;

        for k in &post_ix {
            let c = day[*k];
            let h = dec_to_f64(c.high.0);
            let l = dec_to_f64(c.low.0);
            let hit_top = h >= range_high;
            let hit_bottom = l <= range_low;
            if hit_top && hit_bottom {
                break_idx = Some(*k);
                break_side = Some(Side::Top);
                break;
            }
            if hit_top {
                break_idx = Some(*k);
                break_side = Some(Side::Top);
                break;
            }
            if hit_bottom {
                break_idx = Some(*k);
                break_side = Some(Side::Bottom);
                break;
            }
        }

        if break_idx.is_none() || break_side.is_none() {
            i = j;
            continue;
        }

        let break_idx = break_idx.expect("break index present");
        let break_side = break_side.expect("break side present");
        let mut overshoot: f64 = 0.0;
        let mut break_extreme = match break_side {
            Side::Top => range_high,
            Side::Bottom => range_low,
        };
        let mut reclaim_bars = None;

        for (off, c) in day[break_idx..].iter().enumerate() {
            let h = dec_to_f64(c.high.0);
            let l = dec_to_f64(c.low.0);
            let cl = dec_to_f64(c.close.0);

            match break_side {
                Side::Top => {
                    if h > range_high {
                        overshoot = overshoot.max(h - range_high);
                        if h > break_extreme {
                            break_extreme = h;
                        }
                    }
                    if cl <= range_high {
                        reclaim_bars = Some(off);
                        break;
                    }
                }
                Side::Bottom => {
                    if l < range_low {
                        overshoot = overshoot.max(range_low - l);
                        if l < break_extreme {
                            break_extreme = l;
                        }
                    }
                    if cl >= range_low {
                        reclaim_bars = Some(off);
                        break;
                    }
                }
            }
        }

        let mut two_sided = false;
        if let Some(rb) = reclaim_bars {
            let entry_idx = break_idx + rb;
            for c in &day[entry_idx..] {
                let h = dec_to_f64(c.high.0);
                let l = dec_to_f64(c.low.0);
                match break_side {
                    Side::Top => {
                        if l <= range_low {
                            two_sided = true;
                            break;
                        }
                    }
                    Side::Bottom => {
                        if h >= range_high {
                            two_sided = true;
                            break;
                        }
                    }
                }
            }
        }

        let overshoot_pct = if range_size > 0.0 {
            overshoot / range_size * 100.0
        } else {
            0.0
        };

        samples.push(DaySample {
            range_size,
            first_break: break_side,
            overshoot_pct,
            reclaim_bars,
            two_sided,
            entry_idx: reclaim_bars.map(|rb| break_idx + rb + 1),
            range_high,
            range_low,
            break_extreme,
            opens: day.iter().map(|c| dec_to_f64(c.open.0)).collect(),
            highs: day.iter().map(|c| dec_to_f64(c.high.0)).collect(),
            lows: day.iter().map(|c| dec_to_f64(c.low.0)).collect(),
        });

        i = j;
    }

    samples
}

fn simulate_trade_r(
    sample: &DaySample,
    slip_ticks: i32,
    rt_commission_points: f64,
    tick_size: f64,
    target_mode: &str,
) -> Option<(f64, bool, f64)> {
    let entry_idx = sample.entry_idx?;
    if entry_idx >= sample.opens.len() {
        return None;
    }

    let entry_raw = sample.opens[entry_idx];
    let slip = tick_size * slip_ticks as f64;
    let (entry, stop, target) = match sample.first_break {
        Side::Top => {
            let e = entry_raw - slip;
            let s = sample.break_extreme + tick_size + slip;
            let t = sample.range_low + slip;
            (e, s, t)
        }
        Side::Bottom => {
            let e = entry_raw + slip;
            let s = sample.break_extreme - tick_size - slip;
            let t = sample.range_high - slip;
            (e, s, t)
        }
    };

    let risk = (entry - stop).abs();
    if risk < tick_size {
        return None;
    }

    let target_adj = if target_mode == "fixed_1r" {
        match sample.first_break {
            Side::Top => entry - risk,
            Side::Bottom => entry + risk,
        }
    } else if target_mode == "fixed_1p5r" {
        match sample.first_break {
            Side::Top => entry - 1.5 * risk,
            Side::Bottom => entry + 1.5 * risk,
        }
    } else if target_mode == "fixed_2r" {
        match sample.first_break {
            Side::Top => entry - 2.0 * risk,
            Side::Bottom => entry + 2.0 * risk,
        }
    } else {
        target
    };

    let reward = (target_adj - entry).abs();
    let rr_before_costs = reward / risk;
    let cost_r = rt_commission_points / risk;

    for i in entry_idx..sample.highs.len() {
        let h = sample.highs[i];
        let l = sample.lows[i];
        match sample.first_break {
            Side::Top => {
                let stop_hit = h >= stop;
                let target_hit = l <= target_adj;
                if stop_hit && target_hit {
                    return Some((-1.0 - cost_r, false, rr_before_costs));
                }
                if stop_hit {
                    return Some((-1.0 - cost_r, false, rr_before_costs));
                }
                if target_hit {
                    return Some((rr_before_costs - cost_r, true, rr_before_costs));
                }
            }
            Side::Bottom => {
                let stop_hit = l <= stop;
                let target_hit = h >= target_adj;
                if stop_hit && target_hit {
                    return Some((-1.0 - cost_r, false, rr_before_costs));
                }
                if stop_hit {
                    return Some((-1.0 - cost_r, false, rr_before_costs));
                }
                if target_hit {
                    return Some((rr_before_costs - cost_r, true, rr_before_costs));
                }
            }
        }
    }

    None
}

fn evaluate_pnl_rules(samples: &[DaySample], top_rules: &[RuleResult]) -> Vec<PnlResult> {
    let mut sorted_ranges: Vec<f64> = samples.iter().map(|s| s.range_size).collect();
    sorted_ranges.sort_by(f64::total_cmp);
    let slip_ticks = [0_i32, 1, 2];
    let commissions = [0.0_f64, 0.25, 0.5];
    let target_modes = ["opp_range", "fixed_1r", "fixed_1p5r", "fixed_2r"];
    let tick_size = 0.25_f64;
    let mut out = Vec::new();

    for r in top_rules.iter().take(8) {
        let parts: Vec<&str> = r.rule.split(',').collect();
        if parts.len() != 3 {
            continue;
        }
        let reclaim_cap = parts[0]
            .split("<=")
            .nth(1)
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);
        let max_ov = parts[1]
            .split("<=")
            .nth(1)
            .and_then(|v| v.split('%').next())
            .and_then(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(35.0);
        let range_pct_cap = parts[2]
            .split("<=")
            .nth(1)
            .and_then(|v| v.split('%').next())
            .and_then(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(30.0)
            / 100.0;

        for slip in slip_ticks {
            for commission in commissions {
                for target_mode in target_modes {
                    let mut vals = Vec::new();
                    let mut wins = 0usize;
                    let mut sum_rr = 0.0;

                    for s in samples {
                        let rp = percentile_rank(&sorted_ranges, s.range_size);
                        if rp > range_pct_cap {
                            continue;
                        }
                        if s.overshoot_pct > max_ov {
                            continue;
                        }
                        if s.reclaim_bars.is_none() || s.reclaim_bars.unwrap_or(999) > reclaim_cap {
                            continue;
                        }

                        if let Some((net_r, win, rr_before_costs)) =
                            simulate_trade_r(s, slip, commission, tick_size, target_mode)
                        {
                            vals.push(net_r);
                            sum_rr += rr_before_costs;
                            if win {
                                wins += 1;
                            }
                        }
                    }

                    if vals.len() < 40 {
                        continue;
                    }
                    let expectancy = vals.iter().sum::<f64>() / vals.len() as f64;
                    let win_rate = wins as f64 / vals.len() as f64 * 100.0;
                    out.push(PnlResult {
                        rule: r.rule.clone(),
                        slip_ticks: slip,
                        rt_commission_points: commission,
                        trades: vals.len(),
                        win_rate,
                        net_expectancy_r: expectancy,
                        avg_rr_before_costs: sum_rr / vals.len() as f64,
                        target_mode: target_mode.to_string(),
                    });
                }
            }
        }
    }

    out.sort_by(|a, b| {
        b.net_expectancy_r
            .total_cmp(&a.net_expectancy_r)
            .then(b.win_rate.total_cmp(&a.win_rate))
            .then(b.trades.cmp(&a.trades))
    });
    out
}

fn evaluate_rules(samples: &[DaySample]) -> Vec<RuleResult> {
    let mut sorted_ranges: Vec<f64> = samples.iter().map(|s| s.range_size).collect();
    sorted_ranges.sort_by(f64::total_cmp);

    let reclaim_caps = [1usize, 2, 3, 4, 5];
    let max_overshoots = [15.0, 25.0, 35.0, 50.0, 75.0, 100.0];
    let range_pct_caps = [0.3, 0.4, 0.5, 0.6, 0.7];

    let mut out = Vec::new();

    for reclaim_cap in reclaim_caps {
        for max_ov in max_overshoots {
            for range_pct_cap in range_pct_caps {
                let mut selected = Vec::new();
                for s in samples {
                    let rp = percentile_rank(&sorted_ranges, s.range_size);
                    if rp > range_pct_cap {
                        continue;
                    }
                    if s.overshoot_pct > max_ov {
                        continue;
                    }
                    if let Some(rb) = s.reclaim_bars {
                        if rb <= reclaim_cap {
                            selected.push(s);
                        }
                    }
                }

                if selected.len() < 40 {
                    continue;
                }

                let wins = selected.iter().filter(|s| s.two_sided).count();
                let losses = selected.len() - wins;
                let win_rate = wins as f64 / selected.len() as f64 * 100.0;
                let expectancy_r = (wins as f64 - losses as f64) / selected.len() as f64;

                out.push(RuleResult {
                    rule: format!(
                        "reclaim<= {} bars, overshoot<= {:.0}% range, range_pct<= {:.0}%",
                        reclaim_cap,
                        max_ov,
                        range_pct_cap * 100.0
                    ),
                    trades: selected.len(),
                    win_rate,
                    expectancy_r,
                });
            }
        }
    }

    out.sort_by(|a, b| {
        b.expectancy_r
            .total_cmp(&a.expectancy_r)
            .then(b.win_rate.total_cmp(&a.win_rate))
            .then(b.trades.cmp(&a.trades))
    });
    out
}

fn main() {
    let candles =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");

    let samples = collect_samples(&candles);
    if samples.is_empty() {
        println!("No valid MNQ day samples found.");
        return;
    }

    let total = samples.len();
    let two_sided = samples.iter().filter(|s| s.two_sided).count();
    let baseline_win = two_sided as f64 / total as f64 * 100.0;
    let baseline_exp = (two_sided as f64 - (total - two_sided) as f64) / total as f64;

    let top_breaks = samples
        .iter()
        .filter(|s| matches!(s.first_break, Side::Top))
        .count();
    let bottom_breaks = total - top_breaks;

    let mut ov = samples.iter().map(|s| s.overshoot_pct).collect::<Vec<_>>();
    ov.sort_by(f64::total_cmp);

    println!("MNQ 6-9 NY first-break reversal scan");
    println!("Days: {}", total);
    println!("First break top: {} bottom: {}", top_breaks, bottom_breaks);
    println!(
        "Baseline fade-first-break -> win_rate={:.2}% expectancy={:.3}R",
        baseline_win, baseline_exp
    );
    println!(
        "Overshoot%% of range: p50={:.1}% p75={:.1}% p90={:.1}%",
        pctile(&ov, 0.5),
        pctile(&ov, 0.75),
        pctile(&ov, 0.9)
    );

    let rules = evaluate_rules(&samples);
    println!("\nTop filter sets (min 40 trades):");
    for r in rules.iter().take(15) {
        println!(
            "- {} | trades={} win_rate={:.2}% expectancy={:.3}R",
            r.rule, r.trades, r.win_rate, r.expectancy_r
        );
    }

    let pnl_rules = evaluate_pnl_rules(&samples, &rules);
    println!("\nNet PnL-style ranking (entry next open, stop-first tie-break):");
    for r in pnl_rules.iter().take(12) {
        println!(
            "- {} | target={} slip={}t comm_rt={:.2}pts | trades={} win_rate={:.2}% net_exp={:.3}R avg_rr={:.2}",
            r.rule,
            r.target_mode,
            r.slip_ticks,
            r.rt_commission_points,
            r.trades,
            r.win_rate,
            r.net_expectancy_r,
            r.avg_rr_before_costs
        );
    }
}
