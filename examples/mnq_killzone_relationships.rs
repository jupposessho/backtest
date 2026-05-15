use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    model::candle_stick::CandleStick,
};
use chrono::{Days, NaiveDate, TimeZone, Timelike};
use chrono_tz::America::New_York;
use rayon::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

const ZONE_COUNT: usize = 5;

#[derive(Clone, Copy)]
struct ZoneDef {
    name: &'static str,
    start_minute: u32,
    end_minute: u32,
    wraps_midnight: bool,
}

const ZONES: [ZoneDef; ZONE_COUNT] = [
    ZoneDef {
        name: "ASIA",
        start_minute: 20 * 60,
        end_minute: 0,
        wraps_midnight: true,
    },
    ZoneDef {
        name: "LONDON",
        start_minute: 2 * 60 + 20,
        end_minute: 5 * 60,
        wraps_midnight: false,
    },
    ZoneDef {
        name: "NYAM",
        start_minute: 9 * 60 + 30,
        end_minute: 11 * 60,
        wraps_midnight: false,
    },
    ZoneDef {
        name: "LUNCH",
        start_minute: 12 * 60,
        end_minute: 13 * 60,
        wraps_midnight: false,
    },
    ZoneDef {
        name: "NYPM",
        start_minute: 13 * 60 + 30,
        end_minute: 16 * 60,
        wraps_midnight: false,
    },
];

#[derive(Clone, Copy)]
struct Bar {
    o: f64,
    hi: f64,
    lo: f64,
    c: f64,
    minute_of_day: u32,
}

#[derive(Clone, Copy)]
struct ZoneSummary {
    start_idx: usize,
    end_idx: usize,
    high: f64,
    low: f64,
    mid: f64,
}

#[derive(Clone)]
struct DaySummary {
    session_day: NaiveDate,
    bars: Vec<Bar>,
    zones: [Option<ZoneSummary>; ZONE_COUNT],
}

#[derive(Default, Clone, Copy)]
struct SideStats {
    touches: usize,
    rejections: usize,
    same_bar_rejections: usize,
    opposite_by_target_end: usize,
    opposite_by_eod: usize,
    reclaim_bars_total: usize,
}

#[derive(Default, Clone, Copy)]
struct PairStats {
    eligible_days: usize,
    high: SideStats,
    low: SideStats,
    mid_from_above: SideStats,
    mid_from_below: SideStats,
}

#[derive(Clone, Copy)]
struct SideOutcome {
    touched: bool,
    rejected: bool,
    same_bar_rejection: bool,
    opposite_by_target_end: bool,
    opposite_by_eod: bool,
    reclaim_bars: usize,
}

fn d2f(d: rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

fn in_zone(minute_of_day: u32, zone: ZoneDef) -> bool {
    if zone.wraps_midnight {
        minute_of_day >= zone.start_minute || minute_of_day < zone.end_minute
    } else {
        minute_of_day >= zone.start_minute && minute_of_day < zone.end_minute
    }
}

fn session_day_for(minute_of_day: u32, date: NaiveDate) -> NaiveDate {
    if minute_of_day >= 20 * 60 {
        date.checked_add_days(Days::new(1)).unwrap_or(date)
    } else {
        date
    }
}

fn summarize_zone(bars: &[Bar], zone: ZoneDef) -> Option<ZoneSummary> {
    let mut start_idx = None;
    let mut end_idx = 0usize;
    let mut high = f64::NEG_INFINITY;
    let mut low = f64::INFINITY;

    for (idx, bar) in bars.iter().enumerate() {
        if !in_zone(bar.minute_of_day, zone) {
            continue;
        }
        if start_idx.is_none() {
            start_idx = Some(idx);
        }
        end_idx = idx;
        high = high.max(bar.hi);
        low = low.min(bar.lo);
    }

    start_idx.map(|start_idx| ZoneSummary {
        start_idx,
        end_idx,
        high,
        low,
        mid: (high + low) * 0.5,
    })
}

fn load_days() -> Arc<Vec<DaySummary>> {
    let candles: Vec<CandleStick> =
        CandleStickLoader::load_source(CandleDataSource::ParquetPath("assets/mnq_1m_cont.parquet"))
            .expect("failed loading MNQ parquet");

    let mut grouped: BTreeMap<NaiveDate, Vec<Bar>> = BTreeMap::new();
    for candle in candles {
        let dt = New_York
            .timestamp_opt(candle.open_time, 0)
            .single()
            .expect("valid timestamp");
        let minute_of_day = dt.hour() * 60 + dt.minute();
        let session_day = session_day_for(minute_of_day, dt.date_naive());
        grouped.entry(session_day).or_default().push(Bar {
            o: d2f(candle.open.0),
            hi: d2f(candle.high.0),
            lo: d2f(candle.low.0),
            c: d2f(candle.close.0),
            minute_of_day,
        });
    }

    let days = grouped
        .into_iter()
        .filter_map(|(session_day, bars)| {
            let zones = ZONES.map(|zone| summarize_zone(&bars, zone));
            if zones.iter().all(Option::is_none) {
                None
            } else {
                Some(DaySummary {
                    session_day,
                    bars,
                    zones,
                })
            }
        })
        .collect();

    Arc::new(days)
}

fn analyze_side(
    bars: &[Bar],
    source: ZoneSummary,
    target: ZoneSummary,
    reject_from_high: bool,
) -> SideOutcome {
    let source_level = if reject_from_high {
        source.high
    } else {
        source.low
    };
    let opposite_level = if reject_from_high {
        source.low
    } else {
        source.high
    };

    let touch_idx = (target.start_idx..=target.end_idx).find(|&idx| {
        if reject_from_high {
            bars[idx].hi >= source_level
        } else {
            bars[idx].lo <= source_level
        }
    });

    let Some(touch_idx) = touch_idx else {
        return SideOutcome {
            touched: false,
            rejected: false,
            same_bar_rejection: false,
            opposite_by_target_end: false,
            opposite_by_eod: false,
            reclaim_bars: 0,
        };
    };

    let reject_idx = (touch_idx..=target.end_idx).find(|&idx| {
        if reject_from_high {
            bars[idx].c < source_level
        } else {
            bars[idx].c > source_level
        }
    });

    let Some(reject_idx) = reject_idx else {
        return SideOutcome {
            touched: true,
            rejected: false,
            same_bar_rejection: false,
            opposite_by_target_end: false,
            opposite_by_eod: false,
            reclaim_bars: 0,
        };
    };

    let opposite_by_target_end = (reject_idx..=target.end_idx).any(|idx| {
        if reject_from_high {
            bars[idx].lo <= opposite_level
        } else {
            bars[idx].hi >= opposite_level
        }
    });
    let opposite_by_eod = (reject_idx..bars.len()).any(|idx| {
        if reject_from_high {
            bars[idx].lo <= opposite_level
        } else {
            bars[idx].hi >= opposite_level
        }
    });

    SideOutcome {
        touched: true,
        rejected: true,
        same_bar_rejection: reject_idx == touch_idx,
        opposite_by_target_end,
        opposite_by_eod,
        reclaim_bars: reject_idx - touch_idx,
    }
}

fn analyze_midpoint(
    bars: &[Bar],
    source: ZoneSummary,
    target: ZoneSummary,
    from_above: bool,
) -> SideOutcome {
    let touch_idx = (target.start_idx..=target.end_idx).find(|&idx| {
        let bar = bars[idx];
        if from_above {
            bar.o > source.mid && bar.lo <= source.mid
        } else {
            bar.o < source.mid && bar.hi >= source.mid
        }
    });

    let Some(touch_idx) = touch_idx else {
        return SideOutcome {
            touched: false,
            rejected: false,
            same_bar_rejection: false,
            opposite_by_target_end: false,
            opposite_by_eod: false,
            reclaim_bars: 0,
        };
    };

    let reject_idx = (touch_idx..=target.end_idx).find(|&idx| {
        if from_above {
            bars[idx].c > source.mid
        } else {
            bars[idx].c < source.mid
        }
    });

    let Some(reject_idx) = reject_idx else {
        return SideOutcome {
            touched: true,
            rejected: false,
            same_bar_rejection: false,
            opposite_by_target_end: false,
            opposite_by_eod: false,
            reclaim_bars: 0,
        };
    };

    let opposite_level = if from_above { source.high } else { source.low };
    let opposite_by_target_end = (reject_idx..=target.end_idx).any(|idx| {
        if from_above {
            bars[idx].hi >= opposite_level
        } else {
            bars[idx].lo <= opposite_level
        }
    });
    let opposite_by_eod = (reject_idx..bars.len()).any(|idx| {
        if from_above {
            bars[idx].hi >= opposite_level
        } else {
            bars[idx].lo <= opposite_level
        }
    });

    SideOutcome {
        touched: true,
        rejected: true,
        same_bar_rejection: reject_idx == touch_idx,
        opposite_by_target_end,
        opposite_by_eod,
        reclaim_bars: reject_idx - touch_idx,
    }
}

fn merge_side(stats: &mut SideStats, outcome: SideOutcome) {
    if !outcome.touched {
        return;
    }
    stats.touches += 1;
    if !outcome.rejected {
        return;
    }
    stats.rejections += 1;
    if outcome.same_bar_rejection {
        stats.same_bar_rejections += 1;
    }
    if outcome.opposite_by_target_end {
        stats.opposite_by_target_end += 1;
    }
    if outcome.opposite_by_eod {
        stats.opposite_by_eod += 1;
    }
    stats.reclaim_bars_total += outcome.reclaim_bars;
}

fn analyze_pair(days: Arc<Vec<DaySummary>>, source_idx: usize, target_idx: usize) -> PairStats {
    let mut stats = PairStats::default();
    for day in days.iter() {
        let Some(source) = day.zones[source_idx] else {
            continue;
        };
        let Some(target) = day.zones[target_idx] else {
            continue;
        };
        stats.eligible_days += 1;
        merge_side(
            &mut stats.high,
            analyze_side(&day.bars, source, target, true),
        );
        merge_side(
            &mut stats.low,
            analyze_side(&day.bars, source, target, false),
        );
        merge_side(
            &mut stats.mid_from_above,
            analyze_midpoint(&day.bars, source, target, true),
        );
        merge_side(
            &mut stats.mid_from_below,
            analyze_midpoint(&day.bars, source, target, false),
        );
    }
    stats
}

fn pct(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 * 100.0 / den as f64
    }
}

fn avg_bars(total: usize, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn print_side(label: &str, stats: SideStats) {
    println!(
        "  - {label}: touches={} | rejections={} ({:.2}% of touches) | same-bar={} ({:.2}% of rejections) | opp by target end={} ({:.2}% of rejections) | opp by EOD={} ({:.2}% of rejections) | avg reclaim bars={:.2}",
        stats.touches,
        stats.rejections,
        pct(stats.rejections, stats.touches),
        stats.same_bar_rejections,
        pct(stats.same_bar_rejections, stats.rejections),
        stats.opposite_by_target_end,
        pct(stats.opposite_by_target_end, stats.rejections),
        stats.opposite_by_eod,
        pct(stats.opposite_by_eod, stats.rejections),
        avg_bars(stats.reclaim_bars_total, stats.rejections),
    );
}

fn print_pair(source_idx: usize, target_idx: usize, stats: PairStats) {
    let combined = SideStats {
        touches: stats.high.touches + stats.low.touches,
        rejections: stats.high.rejections + stats.low.rejections,
        same_bar_rejections: stats.high.same_bar_rejections + stats.low.same_bar_rejections,
        opposite_by_target_end: stats.high.opposite_by_target_end
            + stats.low.opposite_by_target_end,
        opposite_by_eod: stats.high.opposite_by_eod + stats.low.opposite_by_eod,
        reclaim_bars_total: stats.high.reclaim_bars_total + stats.low.reclaim_bars_total,
    };
    let midpoint_combined = SideStats {
        touches: stats.mid_from_above.touches + stats.mid_from_below.touches,
        rejections: stats.mid_from_above.rejections + stats.mid_from_below.rejections,
        same_bar_rejections: stats.mid_from_above.same_bar_rejections
            + stats.mid_from_below.same_bar_rejections,
        opposite_by_target_end: stats.mid_from_above.opposite_by_target_end
            + stats.mid_from_below.opposite_by_target_end,
        opposite_by_eod: stats.mid_from_above.opposite_by_eod
            + stats.mid_from_below.opposite_by_eod,
        reclaim_bars_total: stats.mid_from_above.reclaim_bars_total
            + stats.mid_from_below.reclaim_bars_total,
    };

    println!(
        "\n## {} -> {}\n- eligible days: {}",
        ZONES[source_idx].name, ZONES[target_idx].name, stats.eligible_days
    );
    print_side("source high rejection", stats.high);
    print_side("source low rejection", stats.low);
    print_side("combined", combined);
    print_side("midpoint reject from above", stats.mid_from_above);
    print_side("midpoint reject from below", stats.mid_from_below);
    print_side("midpoint combined", midpoint_combined);
}

fn main() {
    let worker_cap = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);
    rayon::ThreadPoolBuilder::new()
        .num_threads(worker_cap)
        .build_global()
        .ok();

    let days = load_days();
    let pairs: Vec<(usize, usize)> = (0..ZONE_COUNT)
        .flat_map(|source_idx| {
            ((source_idx + 1)..ZONE_COUNT).map(move |target_idx| (source_idx, target_idx))
        })
        .collect();

    let mut results: Vec<(usize, usize, PairStats)> = pairs
        .par_iter()
        .map(|&(source_idx, target_idx)| {
            (
                source_idx,
                target_idx,
                analyze_pair(Arc::clone(&days), source_idx, target_idx),
            )
        })
        .collect();
    results.sort_by_key(|(source_idx, target_idx, _)| (*source_idx, *target_idx));

    let first_day = days
        .first()
        .map(|d| d.session_day.to_string())
        .unwrap_or_default();
    let last_day = days
        .last()
        .map(|d| d.session_day.to_string())
        .unwrap_or_default();

    println!("# MNQ Killzone Relationship Scan");
    println!("- Dataset: `assets/mnq_1m_cont.parquet`");
    println!(
        "- Session days: {} (`{}` -> `{}`)",
        days.len(),
        first_day,
        last_day
    );
    println!("- Session-day roll: `20:00` NY. Asia is attached to the following London/NY date.");
    println!(
        "- Window convention: start inclusive, end exclusive. `20:00-00:00` means `20:00-23:59`."
    );
    println!("- Rejection definition: later box first touches the earlier box high/low, then closes back inside that boundary within the same later box.");
    println!("- Midpoint definition: later box first tags the earlier box midpoint from one side, then reclaims back to that side before the target box ends.");
    println!("- Outcome tracking: after rejection, does price reach the opposite side of the source box by target-box end and by session-day end?");
    println!("- Runtime: data loaded once, pairs scanned in parallel with `Arc` + `rayon` (threads capped at {}).", worker_cap);

    for (source_idx, target_idx, stats) in results {
        print_pair(source_idx, target_idx, stats);
    }
}
