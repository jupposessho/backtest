use std::collections::BTreeMap;

use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    engine::types::ExecutionConfig,
    model::{
        candle_stick::CandleStick, decimal::DecimalVec, session::Session, sl_trategy::SlStrategy,
        trading_model::TradingModel,
    },
    parse_datetime,
    strategies::macro_soup::MacroSoup,
    to_new_york_time,
};
use chrono::Datelike;
use rust_decimal::Decimal;

fn resample_minutes(data: &[CandleStick], minutes: i64) -> Vec<CandleStick> {
    if minutes <= 1 {
        return data.to_vec();
    }
    let sec = minutes * 60;
    let mut out: Vec<CandleStick> = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let start_ts = data[i].open_time;
        let bucket = (start_ts / sec) * sec;
        let open = data[i].open;
        let mut high = data[i].high;
        let mut low = data[i].low;
        let mut close = data[i].close;
        let mut close_time = data[i].close_time;
        let mut j = i + 1;
        while j < data.len() {
            let b = (data[j].open_time / sec) * sec;
            if b != bucket {
                break;
            }
            if data[j].high > high {
                high = data[j].high;
            }
            if data[j].low < low {
                low = data[j].low;
            }
            close = data[j].close;
            close_time = data[j].close_time;
            j += 1;
        }
        out.push(CandleStick {
            open_time: bucket,
            open,
            high,
            low,
            close,
            close_time,
        });
        i = j;
    }
    out
}

fn monthly_for_window(
    candles: &[CandleStick],
    start_hm: &str,
    from_ts: i64,
) -> (usize, Decimal, BTreeMap<String, Decimal>) {
    let start = parse_datetime(&format!("2022-09-30 {}:00", start_hm))
        .unwrap()
        .time();
    let end = start + chrono::Duration::minutes(60);

    let model = MacroSoup {
        candles: candles.to_vec(),
        rr_threshold: Decimal::from(3),
        be_threshold: Some(DecimalVec::new(2)),
        session: Session { start, end },
        sl_strategy: SlStrategy::None,
        max_duration_min: 120,
        execution: ExecutionConfig {
            commission_rate_per_side: Decimal::ZERO,
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 1,
            tick_size: Decimal::new(25, 2),
        },
    };

    let result = model.execute();
    let mut by_month: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut trades = 0usize;

    for t in result.trades {
        if t.close_time < from_ts {
            continue;
        }
        trades += 1;
        let dt = to_new_york_time(t.close_time);
        let key = format!("{:04}-{:02}", dt.year(), dt.month());
        let usd = t.points().0 * Decimal::from(2); // MNQ $2/point per 1 micro
        *by_month.entry(key).or_insert(Decimal::ZERO) += usd;
    }

    let total: Decimal = by_month.values().copied().sum();
    (trades, total, by_month)
}

fn main() {
    let top_windows = ["15:50", "15:55", "16:05", "16:00", "16:10"];
    let from_ts = parse_datetime("2025-01-01 00:00:00").unwrap().timestamp();

    let path = "/Users/waff/develop/play/nq/mnq_1m_cont.parquet";
    let mut mnq_1m = CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .unwrap_or_else(|e| panic!("failed to load MNQ parquet at {}: {}", path, e));

    let keep_last = 120_000usize;
    if mnq_1m.len() > keep_last {
        let drop_n = mnq_1m.len() - keep_last;
        mnq_1m.drain(0..drop_n);
    }
    let mnq_3m = resample_minutes(&mnq_1m, 3);

    let mut md = String::new();
    md.push_str("# MacroSoup MNQ Monthly Breakdown (From 2025-01-01)\n\n");
    md.push_str("Top windows: 15:50, 15:55, 16:05, 16:00, 16:10\n");
    md.push_str("Execution: slippage=1 tick, no fees, no commissions\n");
    md.push_str("Contract: MNQ 1 micro ($2/point)\n\n");

    md.push_str("## 1m\n\n");
    for w in top_windows {
        let (trades, total, by_month) = monthly_for_window(&mnq_1m, w, from_ts);
        md.push_str(&format!(
            "### Window {}-{}\n\n",
            w,
            parse_datetime(&format!("2022-09-30 {}:00", w))
                .unwrap()
                .time()
                + chrono::Duration::minutes(60)
        ));
        md.push_str(&format!("- trades since 2025-01-01: {}\n", trades));
        md.push_str(&format!("- total usd: {}\n\n", total.round_dp(2)));
        md.push_str("| month | usd |\n|---|---:|\n");
        for (m, v) in by_month {
            md.push_str(&format!("| {} | {} |\n", m, v.round_dp(2)));
        }
        md.push_str("\n");
    }

    md.push_str("## 3m\n\n");
    for w in top_windows {
        let (trades, total, by_month) = monthly_for_window(&mnq_3m, w, from_ts);
        md.push_str(&format!(
            "### Window {}-{}\n\n",
            w,
            parse_datetime(&format!("2022-09-30 {}:00", w))
                .unwrap()
                .time()
                + chrono::Duration::minutes(60)
        ));
        md.push_str(&format!("- trades since 2025-01-01: {}\n", trades));
        md.push_str(&format!("- total usd: {}\n\n", total.round_dp(2)));
        md.push_str("| month | usd |\n|---|---:|\n");
        for (m, v) in by_month {
            md.push_str(&format!("| {} | {} |\n", m, v.round_dp(2)));
        }
        md.push_str("\n");
    }

    std::fs::write(
        "reports/strategy_overviews/MACRO_SOUP_MNQ_MONTHLY_FROM_2025.md",
        md,
    )
    .unwrap_or_else(|e| panic!("failed writing report: {}", e));

    println!("Wrote reports/strategy_overviews/MACRO_SOUP_MNQ_MONTHLY_FROM_2025.md");
}
