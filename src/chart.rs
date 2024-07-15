extern crate rust_decimal;
use charming::{
    component::{
        Axis, Brush, BrushType, DataZoom, DataZoomType, Feature, Grid, Legend, Toolbox,
        ToolboxDataZoom,
    },
    element::{
        AxisLine, AxisPointer, AxisPointerLink, AxisPointerType, AxisType, SplitArea, SplitLine,
        Tooltip, Trigger,
    },
    series::Candlestick,
    Chart,
};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::{
    model::{backtest_result::BacktestResult, decimal::DecimalVec, session::Session},
    parse_datetime,
};
use crate::{
    model::{binance_klines_item::BinanceKlinesItem, candle_stick::CandleStick},
    read_csv,
};
use crate::{
    model::{candle_ny::CandleNY, trading_model::TradingModel},
    strategies::macro_soup::MacroSoup,
};

fn execute<T: TradingModel>(model: T) -> BacktestResult {
    model.execute()
}

// TODO: extract to data loader
fn load_data() -> Vec<CandleStick> {
    let raw_data: Vec<BinanceKlinesItem> =
        serde_json::from_str(include_str!("../assets/eth15.json")).unwrap();
    // serde_json::from_str(include_str!("../assets/ETHUSDT_15m.json")).unwrap();

    raw_data
        .iter()
        .enumerate()
        .map(|(_, v)| CandleStick {
            open_time: v.open_time as i64,
            open: DecimalVec(Decimal::from_str(v.open.as_str()).unwrap()),
            high: DecimalVec(Decimal::from_str(v.high.as_str()).unwrap()),
            low: DecimalVec(Decimal::from_str(v.low.as_str()).unwrap()),
            close: DecimalVec(Decimal::from_str(v.close.as_str()).unwrap()),
            close_time: v.close_time as i64,
        })
        .collect::<Vec<_>>()
}

fn load_csv() -> Vec<CandleNY> {
    read_csv("/Users/jupposessho/develop/play/rust/backtest/assets/NDX_full_1min.txt").unwrap()
}

// fn round_to_nearest_15_minute(dt: DateTime<Tz>) -> (u32, u32) {
//     let minute = dt.minute();
//     let rounded_minute = (minute / 15) * 15;
//     (dt.hour(), rounded_minute)
// }

// pub fn high_low_statistics(klines: Vec<BinanceKlinesItem>) {
//     let mut grouped_by_day: HashMap<String, Vec<BinanceKlinesItem>> = HashMap::new();

//     for kline in &klines {
//         let open_time = to_new_york_time(kline.open_time as i64);
//         let day_key = format!("{}", open_time.format("%Y-%m-%d"));

//         grouped_by_day
//             .entry(day_key)
//             .or_insert_with(Vec::new)
//             .push(kline.clone());
//     }

//     let mut high_time_count: HashMap<(u32, u32), u32> = HashMap::new();
//     let mut low_time_count: HashMap<(u32, u32), u32> = HashMap::new();

//     for (_, klines) in grouped_by_day {
//         let mut high_time: Option<DateTime<Tz>> = None;
//         let mut low_time: Option<DateTime<Tz>> = None;
//         let mut highest_value = f64::MIN;
//         let mut lowest_value = f64::MAX;

//         for kline in klines {
//             let open_time = to_new_york_time(kline.open_time as i64);

//             let high = kline.high.parse::<f64>().unwrap_or(f64::MIN);
//             let low = kline.low.parse::<f64>().unwrap_or(f64::MAX);

//             if high > highest_value {
//                 highest_value = high;
//                 high_time = Some(open_time);
//             }

//             if low < lowest_value {
//                 lowest_value = low;
//                 low_time = Some(open_time);
//             }
//         }

//         if let Some(high_time) = high_time {
//             let rounded_high_time = round_to_nearest_15_minute(high_time);
//             *high_time_count.entry(rounded_high_time).or_insert(0) += 1;
//         }

//         if let Some(low_time) = low_time {
//             let rounded_low_time = round_to_nearest_15_minute(low_time);
//             *low_time_count.entry(rounded_low_time).or_insert(0) += 1;
//         }
//     }

//     let mut high_time_count: Vec<_> = high_time_count.into_iter().collect();
//     high_time_count.sort_by(|a, b| b.1.cmp(&a.1));

//     let mut low_time_count: Vec<_> = low_time_count.into_iter().collect();
//     low_time_count.sort_by(|a, b| b.1.cmp(&a.1));

//     println!("High of the day forms at (15-minute intervals):");
//     for ((hour, minute), count) in high_time_count {
//         println!("Time: {:02}:{:02} Count: {}", hour, minute, count);
//     }

//     println!("Low of the day forms at (15-minute intervals):");
//     for ((hour, minute), count) in low_time_count {
//         println!("Time: {:02}:{:02} Count: {}", hour, minute, count);
//     }
// }

pub fn chart() -> Chart {
    // let candlesticks = load_data();

    // let category_data = candlesticks
    //     .iter()
    //     .map(|x| {
    //         to_new_york_time(x.open_time)
    //             .format("%Y-%m-%d %H:%M:%S")
    //             .to_string()
    //     })
    //     .collect::<Vec<_>>();
    // let data = candlesticks
    //     .iter()
    //     .enumerate()
    //     .map(|(_, v)| {
    //         let item = v.clone();
    //         vec![item.open, item.close, item.low, item.high]
    //     })
    //     .collect::<Vec<_>>();

    // //// SFP 15

    // let sfp = Sfp {
    //     rr_treshold: Decimal::from(2),
    //     data: candlesticks,
    // };
    // let result = execute(sfp);
    // println!("============result {:#?}", result);

    let candlesticks = load_csv();

    let sfp = MacroSoup {
        candles: candlesticks.clone(),
        rr_threshold: Decimal::from(3),
        session: Session {
            start: parse_datetime("2022-09-30 09:50:00").unwrap().time(),
            end: parse_datetime("2022-09-30 10:10:00").unwrap().time(),
        },
        max_duration_min: 30,
    };
    let result = execute(sfp);
    println!("============result {:#?}", result);
    let category_data = candlesticks
        .iter()
        .map(|x| x.open_time.format("%Y-%m-%d %H:%M:%S").to_string())
        .collect::<Vec<_>>();
    let data = candlesticks
        .iter()
        .enumerate()
        .map(|(_, v)| {
            let item = v.clone();
            vec![item.open, item.close, item.low, item.high]
        })
        .collect::<Vec<_>>();

    Chart::new()
        .legend(
            Legend::new()
                .bottom(10)
                .left("center")
                .data(vec!["Dow-Jones index"]),
        )
        .tooltip(
            Tooltip::new()
                .trigger(Trigger::Axis)
                .axis_pointer(AxisPointer::new().type_(AxisPointerType::Cross)),
        )
        .axis_pointer(AxisPointer::new().link(vec![AxisPointerLink::new().x_axis_index("all")]))
        .toolbox(
            Toolbox::new().feature(
                Feature::new()
                    .data_zoom(ToolboxDataZoom::new().y_axis_index("none"))
                    .brush(Brush::new().type_(vec![BrushType::LineX, BrushType::Clear])),
            ),
        )
        .grid(Grid::new().left("10%").right("8%").bottom(150))
        .x_axis(
            Axis::new()
                .type_(AxisType::Category)
                .data(category_data)
                .boundary_gap(false)
                .axis_line(AxisLine::new().on_zero(false))
                .split_line(SplitLine::new().show(false))
                .min("dataMin")
                .max("dataMax")
                .axis_pointer(AxisPointer::new().z(100)),
        )
        .y_axis(
            Axis::new()
                .scale(true)
                .split_area(SplitArea::new().show(true)),
        )
        .data_zoom(
            DataZoom::new()
                .type_(DataZoomType::Inside)
                .start(98)
                .end(100)
                .min_value_span(10),
        )
        .data_zoom(
            DataZoom::new()
                .type_(DataZoomType::Slider)
                .bottom(60)
                .start(98)
                .start(98)
                .end(100)
                .min_value_span(10),
        )
        .series(Candlestick::new().data(data.clone()))
}
