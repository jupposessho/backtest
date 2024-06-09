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

use crate::model::trading_model::TradingModel;
use crate::model::{backtest_result::BacktestResult, candle_stick::DecimalVec};
use crate::model::{binance_klines_item::BinanceKlinesItem, candle_stick::CandleStick};
use crate::strategies::sfp::Sfp;
use crate::to_new_york_time;

fn execute<T: TradingModel>(model: T) -> BacktestResult {
    model.execute()
}

// TODO: extract to data loader
fn load_data() -> Vec<CandleStick> {
    let raw_data: Vec<BinanceKlinesItem> =
        serde_json::from_str(include_str!("../assets/eth15.json")).unwrap();

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

pub fn chart() -> Chart {
    let candlesticks = load_data();

    let category_data = candlesticks
        .iter()
        .map(|x| {
            to_new_york_time(x.open_time)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .collect::<Vec<_>>();
    let data = candlesticks
        .iter()
        .enumerate()
        .map(|(_, v)| {
            let item = v.clone();
            vec![item.open, item.close, item.low, item.high]
        })
        .collect::<Vec<_>>();

    //// SFP 15

    let sfp = Sfp { data: candlesticks };
    let result = execute(sfp);
    println!("============result {:#?}", result);

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
