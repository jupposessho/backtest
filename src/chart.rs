extern crate rust_decimal;

use charming::{
    component::{
        Axis, DataZoom, DataZoomType, Feature, Grid, Legend, Toolbox, ToolboxDataZoom,
    },
    element::{
        AxisLine, AxisPointer, AxisPointerLink, AxisPointerType, AxisType, SplitArea, SplitLine,
        Tooltip, Trigger,
    },
    series::{Candlestick, Scatter},
    Chart,
};
use charming::datatype::NumericValue;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

use crate::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{candle_stick::CandleStick, trade::Trade},
    strategies::mc::{
        EntryMode, ExecutionConfig, FvgConfig, LevelFilters, MarketEntryMode, Mc, McConfig,
        McMode, SignalPattern, SignalQualityConfig, TimeWindow, TrailingStopConfig, TrendFilter,
    },
    to_new_york_time,
};

fn load_binance() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!(
        "../assets/binance_BTCUSDT_15m.json"
    ))
}

fn recent_candles(candles: &[CandleStick], days: i64) -> Vec<CandleStick> {
    if candles.is_empty() {
        return vec![];
    }
    let last_close_time = candles.last().unwrap().close_time;
    let cutoff = last_close_time - (days * 24 * 60 * 60);
    candles
        .iter()
        .copied()
        .filter(|c| c.open_time >= cutoff)
        .collect()
}

fn find_candle_index(candles: &[CandleStick], timestamp: i64) -> Option<usize> {
    candles
        .iter()
        .position(|c| c.open_time <= timestamp && c.close_time >= timestamp)
        .or_else(|| candles.iter().position(|c| c.open_time >= timestamp))
}

fn mc_trades(data: Vec<CandleStick>) -> Vec<Trade> {
    let config = McConfig {
        mode: McMode::ContinuationEma200,
        entry_mode: EntryMode::PrevOpen,
        rr_target: rust_decimal::Decimal::from_f32(1.5).unwrap(),
        trade_window: Some(TimeWindow::default()),
        level_filters: LevelFilters {
            enabled: false,
            ..LevelFilters::default()
        },
        trend_filter: TrendFilter::Ema { fast: 50, slow: 200 },
        fvg_filter: FvgConfig {
            enabled: false,
            ..FvgConfig::default()
        },
        signal_quality: SignalQualityConfig::default(),
        daily_open_time: chrono::NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        prev_open_fill_window_candles: 3,
        pattern: SignalPattern::Mc,
        trailing_stop: TrailingStopConfig::default(),
        execution: ExecutionConfig {
            market_entry: MarketEntryMode::NextBarOpen,
            ..ExecutionConfig::default()
        },
    };

    let model = Mc { data, config };
    let result = execute(model);
    result.trades
}

pub fn chart() -> Chart {
    let all_candles = load_binance();
    let candles = recent_candles(&all_candles, 90);
    let trades = mc_trades(all_candles);

    let category_data = candles
        .iter()
        .map(|x| {
            to_new_york_time(x.open_time)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .collect::<Vec<_>>();

    let data = candles
        .iter()
        .map(|v| vec![v.open, v.close, v.low, v.high])
        .collect::<Vec<_>>();

    let mut entry_points: Vec<Vec<NumericValue>> = vec![];
    let mut exit_points: Vec<Vec<NumericValue>> = vec![];

    for t in trades.iter() {
        if let Some(entry_idx) = find_candle_index(&candles, t.open_time) {
            if let Some(price) = t.entry.0.to_f64() {
                entry_points.push(vec![
                    NumericValue::Float(entry_idx as f64),
                    NumericValue::Float(price),
                ]);
            }
        }
        if let Some(exit_idx) = find_candle_index(&candles, t.close_time) {
            let exit_price = match t.result {
                crate::model::trade_result::TradeResult::Winner => t.tp.0.to_f64(),
                crate::model::trade_result::TradeResult::Expense => t.sl.0.to_f64(),
                crate::model::trade_result::TradeResult::BreakEven => t.entry.0.to_f64(),
            };
            if let Some(price) = exit_price {
                exit_points.push(vec![
                    NumericValue::Float(exit_idx as f64),
                    NumericValue::Float(price),
                ]);
            }
        }
    }

    Chart::new()
        .legend(
            Legend::new()
                .bottom(10)
                .left("center")
                .data(vec!["Candles", "Entries", "Exits"]),
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
                    .data_zoom(ToolboxDataZoom::new().y_axis_index("none")),
            ),
        )
        .grid(Grid::new().left("10%").right("8%").bottom(120))
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
                .end(100)
                .min_value_span(10),
        )
        .series(Candlestick::new().name("Candles").data(data))
        .series(
            Scatter::new()
                .name("Entries")
                .data(entry_points)
                .symbol_size(8),
        )
        .series(
            Scatter::new()
                .name("Exits")
                .data(exit_points)
                .symbol_size(8),
        )
}
