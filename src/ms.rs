extern crate rust_decimal;
use backtest::{
    candle_stick_loader::CandleStickLoader,
    engine::types::ExecutionConfig,
    execute,
    model::{
        backtest_result::BacktestResult, candle_stick::CandleStick, decimal::DecimalVec,
        session::Session, sl_trategy::SlStrategy,
    },
    parse_datetime,
    strategies::macro_soup::MacroSoup,
};
use rust_decimal::Decimal;

// fn load_csv() -> Vec<CandleStick> {
//     CandleStickLoader::load_csv("../assets/NDX_full_1min.txt").unwrap()
// }
fn load_binance() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        // "../assets/binance_BTC_1m_2017-08-17-2024-08-16.json"
        "../assets/binance_BTCUSDT_15m.json" // "../assets/binance_BTC_5m_2024-08-16-2025-03-01.json"
    ))
    // CandleStickLoader::load_binance(include_str!("../assets/BTCUSDT_1m.json"))
}

fn main() {
    // let candlesticks = load_csv();
    let candlesticks = load_binance();
    // let candlesticks = CandleStickLoader::load(include_str!("../assets/bitget_BTCUSDT_1m.json"));
    // println!("============cs {:#?}", candlesticks);

    let mut best_result: Option<BacktestResult> = None;
    let mut start = parse_datetime("2022-09-30 05:00:00").unwrap().time();
    let last = parse_datetime("2022-09-30 18:00:00").unwrap().time();
    while start < last {
        let session_end = start + chrono::Duration::minutes(60);
        let trading_model = MacroSoup {
            candles: candlesticks.clone(),
            rr_threshold: Decimal::from(3),
            be_threshold: Some(DecimalVec::new(2)),
            session: Session {
                start,
                end: session_end,
            },
            sl_strategy: SlStrategy::None,
            // sl_strategy: SlStrategy::Skip(DecimalVec::new(250)),
            // sl_strategy: SlStrategy::Limit(DecimalVec::new(500)),
            max_duration_min: 120,
            execution: ExecutionConfig {
                commission_rate_per_side: Decimal::new(1, 3),
                fee_rate_per_side: Decimal::ZERO,
                slippage_ticks_per_side: 1,
                tick_size: Decimal::new(1, 2),
            },
        };
        let result: BacktestResult = execute(trading_model);

        match best_result {
            Some(ref c) => {
                let p = result.profit_in_points();
                if p > Decimal::from(12000) {
                    println!("============result {:#?} {:#?}", start, result)
                }
                // if c.profit_in_r() < result.profit_in_r() {

                if c.profit_in_points() < p {
                    // if c.result(TradeResult::Winner) < result.result(TradeResult::Winner) {
                    best_result = Some(result.clone())
                }
            }
            None => best_result = Some(result.clone()),
        }
        start = start + chrono::Duration::minutes(5);
    }
    println!("============best_result {:#?}", best_result);
}
