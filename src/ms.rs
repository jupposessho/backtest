extern crate rust_decimal;
use backtest::{
    execute,
    model::{candle_ny::CandleNY, decimal::DecimalVec, session::Session},
    parse_datetime, read_csv,
    strategies::macro_soup::MacroSoup,
};
use rust_decimal::Decimal;

fn load_csv() -> Vec<CandleNY> {
    read_csv("/Users/jupposessho/develop/play/rust/backtest/assets/NDX_full_1min.txt").unwrap()
}

fn main() {
    let candlesticks = load_csv();
    println!("============cs {:#?}", candlesticks);

    let sfp = MacroSoup {
        candles: candlesticks.clone(),
        rr_threshold: Decimal::from(3),
        be_threshold: Some(DecimalVec::new(2)),
        session: Session {
            start: parse_datetime("2022-09-30 09:50:00").unwrap().time(),
            end: parse_datetime("2022-09-30 10:10:00").unwrap().time(),
        },
        max_duration_min: 30,
    };
    let result = execute(sfp);
    println!("============result {:#?}", result);
}
