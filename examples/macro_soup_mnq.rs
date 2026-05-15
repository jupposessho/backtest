use backtest::{
    candle_stick_loader::{CandleDataSource, CandleStickLoader},
    engine::types::ExecutionConfig,
    model::{
        candle_stick::CandleStick, decimal::DecimalVec, session::Session, sl_trategy::SlStrategy,
        trade_result::TradeResult, trading_model::TradingModel,
    },
    parse_datetime,
    strategies::macro_soup::MacroSoup,
};
use rust_decimal::Decimal;

#[derive(Clone)]
struct Row {
    timeframe: String,
    start: String,
    end: String,
    trades: usize,
    win_rate: Decimal,
    profit_r: Decimal,
    pf_r: Decimal,
    points: Decimal,
    usd_est: Decimal,
}

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

fn run_scan(timeframe: &str, candles: &[CandleStick]) -> Vec<Row> {
    let top_windows = ["16:00", "15:50", "15:55", "16:05", "16:10"];

    let mut rows = Vec::new();
    for hm in top_windows {
        let start = parse_datetime(&format!("2022-09-30 {}:00", hm))
            .unwrap()
            .time();
        let session_end = start + chrono::Duration::minutes(60);
        let model = MacroSoup {
            candles: candles.to_vec(),
            rr_threshold: Decimal::from(3),
            be_threshold: Some(DecimalVec::new(2)),
            session: Session {
                start,
                end: session_end,
            },
            sl_strategy: SlStrategy::None,
            max_duration_min: 120,
            execution: ExecutionConfig {
                commission_rate_per_side: Decimal::ZERO,
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: 1,
                tick_size: Decimal::new(25, 2), // MNQ tick size 0.25
            },
        };
        let result = model.execute();
        let winners = result.result(TradeResult::Winner);
        let losers = result.result(TradeResult::Expense);
        let trades = result.number_of_trades();
        let win_rate = if trades == 0 {
            Decimal::ZERO
        } else {
            Decimal::from(winners as u32) / Decimal::from(trades as u32) * Decimal::new(100, 0)
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
        let points = result.profit_in_points();
        let usd_est = points * Decimal::from(2); // MNQ $2 per point

        rows.push(Row {
            timeframe: timeframe.to_string(),
            start: start.format("%H:%M").to_string(),
            end: session_end.format("%H:%M").to_string(),
            trades,
            win_rate,
            profit_r: result.profit_in_r(),
            pf_r,
            points,
            usd_est,
        });
    }

    rows.sort_by(|a, b| b.profit_r.cmp(&a.profit_r).then(b.pf_r.cmp(&a.pf_r)));
    rows
}

fn main() {
    let path = "/Users/waff/develop/play/nq/mnq_1m_cont.parquet";
    let mut mnq_1m = CandleStickLoader::load_source(CandleDataSource::ParquetPath(path))
        .unwrap_or_else(|e| panic!("failed to load MNQ parquet at {}: {}", path, e));

    // Quick mode: limit history to speed up iteration.
    let keep_last = 120_000usize;
    if mnq_1m.len() > keep_last {
        let drop_n = mnq_1m.len() - keep_last;
        mnq_1m.drain(0..drop_n);
    }

    let mnq_3m = resample_minutes(&mnq_1m, 3);

    let rows_1m = run_scan("1m", &mnq_1m);
    let rows_3m = run_scan("3m", &mnq_3m);

    let mut md = String::new();
    md.push_str("# MacroSoup MNQ (Close-Back-Inside)\n\n");
    md.push_str("Dataset: `/Users/waff/develop/play/nq/mnq_1m_cont.parquet`\n");
    md.push_str("Mode: quick (last 120000 1m bars, top 5 session windows)\n");
    md.push_str("Execution: slippage=1 tick (0.25), commission=0, fee=0\n");
    md.push_str("Session windows: top windows from prior MacroSoup runs\n\n");

    md.push_str("## Top setups - 1m\n\n");
    md.push_str("| start | end | trades | win_rate_% | profit_r | pf_r | points | usd_est |\n");
    md.push_str("|---|---|---:|---:|---:|---:|---:|---:|\n");
    for r in rows_1m.iter().take(10) {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.start,
            r.end,
            r.trades,
            r.win_rate.round_dp(2),
            r.profit_r.round_dp(2),
            r.pf_r.round_dp(2),
            r.points.round_dp(2),
            r.usd_est.round_dp(2)
        ));
    }

    md.push_str("\n## Top setups - 3m\n\n");
    md.push_str("| start | end | trades | win_rate_% | profit_r | pf_r | points | usd_est |\n");
    md.push_str("|---|---|---:|---:|---:|---:|---:|---:|\n");
    for r in rows_3m.iter().take(10) {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.start,
            r.end,
            r.trades,
            r.win_rate.round_dp(2),
            r.profit_r.round_dp(2),
            r.pf_r.round_dp(2),
            r.points.round_dp(2),
            r.usd_est.round_dp(2)
        ));
    }

    std::fs::write("reports/strategy_overviews/MACRO_SOUP_MNQ_1M_3M.md", md)
        .unwrap_or_else(|e| panic!("failed writing report: {}", e));

    println!("Wrote reports/strategy_overviews/MACRO_SOUP_MNQ_1M_3M.md");
}
