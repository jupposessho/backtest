use chrono::NaiveTime;
use rust_decimal::Decimal;

use crate::engine::context::MarketContext;
use crate::engine::detection::SetupDetector;
use crate::engine::execution::run_setups;
use crate::engine::types::{
    EntryModel, ExecutionConfig, SetupCandidate, StopModel, TargetModel, TrailingModel,
};
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position_direction::PositionDirection;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;

#[derive(Clone, Copy)]
pub enum DojiType {
    Classic,
    Strict,
    LongLegged,
    Dragonfly,
    Gravestone,
    Loose,
}

#[derive(Clone, Copy)]
pub enum DojiEntryMode {
    MidpointLimit,
    MarketClose,
}

#[derive(Clone, Copy)]
pub enum DojiTargetMode {
    RunnerR(Decimal),
    FixedPoints(Decimal),
}

#[derive(Clone, Copy)]
pub enum MaxSlMode {
    MarketStopCap,
    LimitReprice,
}

#[derive(Clone, Copy)]
pub struct DojiConfig {
    pub doji_type: DojiType,
    pub body_pct_max: Decimal,
    pub stop_buffer_ticks: i32,
    pub limit_timeout_bars: usize,
    pub trail_activate_points: Decimal,
    pub trail_distance_points: Decimal,
    pub max_trades_per_day: usize,
    pub session_start: NaiveTime,
    pub session_end: NaiveTime,
    pub entry_mode: DojiEntryMode,
    pub target_mode: DojiTargetMode,
    pub max_sl_points: Option<Decimal>,
    pub max_sl_mode: MaxSlMode,
    pub execution: ExecutionConfig,
}

impl Default for DojiConfig {
    fn default() -> Self {
        Self {
            doji_type: DojiType::Classic,
            body_pct_max: Decimal::from(5),
            stop_buffer_ticks: 1,
            limit_timeout_bars: 5,
            trail_activate_points: Decimal::from(10),
            trail_distance_points: Decimal::from(10),
            max_trades_per_day: 3,
            session_start: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            session_end: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
            entry_mode: DojiEntryMode::MidpointLimit,
            target_mode: DojiTargetMode::RunnerR(Decimal::from(100)),
            max_sl_points: None,
            max_sl_mode: MaxSlMode::MarketStopCap,
            execution: ExecutionConfig::default(),
        }
    }
}

pub struct Doji {
    pub data: Vec<CandleStick>,
    pub config: DojiConfig,
}

struct DojiDetector {
    config: DojiConfig,
}

impl DojiDetector {
    fn is_doji(&self, body_pct: Decimal, upper_wick_pct: Decimal, lower_wick_pct: Decimal) -> bool {
        if matches!(self.config.doji_type, DojiType::Loose) {
            return body_pct <= Decimal::from(20);
        }
        if body_pct > self.config.body_pct_max {
            return false;
        }
        match self.config.doji_type {
            DojiType::Strict => upper_wick_pct > Decimal::ZERO && lower_wick_pct > Decimal::ZERO,
            DojiType::LongLegged => {
                upper_wick_pct >= Decimal::from(30) && lower_wick_pct >= Decimal::from(30)
            }
            DojiType::Dragonfly => upper_wick_pct < Decimal::from(5) && lower_wick_pct >= Decimal::from(30),
            DojiType::Gravestone => lower_wick_pct < Decimal::from(5) && upper_wick_pct >= Decimal::from(30),
            _ => true,
        }
    }

    fn detect_direction(&self, lower_wick_pct: Decimal, upper_wick_pct: Decimal) -> PositionDirection {
        match self.config.doji_type {
            DojiType::Dragonfly => return PositionDirection::Long,
            DojiType::Gravestone => return PositionDirection::Short,
            _ => {}
        }

        let dominance = Decimal::new(15, 1);
        if lower_wick_pct > upper_wick_pct * dominance {
            PositionDirection::Long
        } else if upper_wick_pct > lower_wick_pct * dominance {
            PositionDirection::Short
        } else if lower_wick_pct >= upper_wick_pct {
            PositionDirection::Long
        } else {
            PositionDirection::Short
        }
    }
}

impl SetupDetector for DojiDetector {
    fn detect(&self, ind: usize, data: &[CandleStick], _ctx: &MarketContext) -> Vec<SetupCandidate> {
        let c = data[ind];
        let ny = to_new_york_time(c.open_time);
        let t = ny.time();
        if t < self.config.session_start || t >= self.config.session_end {
            return vec![];
        }

        let day = ny.date_naive();
        let daily_setups = data
            .iter()
            .copied()
            .take(ind)
            .filter(|x| to_new_york_time(x.open_time).date_naive() == day)
            .filter(|x| {
                let xt = to_new_york_time(x.open_time).time();
                xt >= self.config.session_start && xt < self.config.session_end
            })
            .filter(|x| {
                let range = x.high.0 - x.low.0;
                if range <= Decimal::ZERO {
                    return false;
                }
                let body = (x.close.0 - x.open.0).abs();
                let body_pct = body / range * Decimal::from(100);
                let upper_wick = x.high.0 - x.open.0.max(x.close.0);
                let lower_wick = x.open.0.min(x.close.0) - x.low.0;
                let upper_wick_pct = upper_wick / range * Decimal::from(100);
                let lower_wick_pct = lower_wick / range * Decimal::from(100);
                self.is_doji(body_pct, upper_wick_pct, lower_wick_pct)
            })
            .count();

        if daily_setups >= self.config.max_trades_per_day {
            return vec![];
        }

        let range = c.high.0 - c.low.0;
        if range <= Decimal::ZERO {
            return vec![];
        }

        let body = (c.close.0 - c.open.0).abs();
        let body_pct = body / range * Decimal::from(100);
        let upper_wick = c.high.0 - c.open.0.max(c.close.0);
        let lower_wick = c.open.0.min(c.close.0) - c.low.0;
        let upper_wick_pct = upper_wick / range * Decimal::from(100);
        let lower_wick_pct = lower_wick / range * Decimal::from(100);

        if !self.is_doji(body_pct, upper_wick_pct, lower_wick_pct) {
            return vec![];
        }

        let direction = self.detect_direction(lower_wick_pct, upper_wick_pct);
        let midpoint_entry = DecimalVec((c.open.0 + c.close.0) / Decimal::from(2));
        let mut entry_px = match self.config.entry_mode {
            DojiEntryMode::MidpointLimit => midpoint_entry,
            DojiEntryMode::MarketClose => c.close,
        };
        let buffer = Decimal::from(self.config.stop_buffer_ticks) * self.config.execution.tick_size;
        let mut sl = match direction {
            PositionDirection::Long => DecimalVec(c.low.0 - buffer),
            PositionDirection::Short => DecimalVec(c.high.0 + buffer),
        };

        let initial_risk = match direction {
            PositionDirection::Long => entry_px.0 - sl.0,
            PositionDirection::Short => sl.0 - entry_px.0,
        };

        if let Some(max_sl) = self.config.max_sl_points {
            if initial_risk > max_sl {
                match self.config.max_sl_mode {
                    MaxSlMode::MarketStopCap => {
                        sl = match direction {
                            PositionDirection::Long => DecimalVec(entry_px.0 - max_sl),
                            PositionDirection::Short => DecimalVec(entry_px.0 + max_sl),
                        };
                    }
                    MaxSlMode::LimitReprice => {
                        entry_px = match direction {
                            PositionDirection::Long => DecimalVec(sl.0 + max_sl),
                            PositionDirection::Short => DecimalVec(sl.0 - max_sl),
                        };
                    }
                }
            }
        }

        let trailing = {
            let risk = match direction {
                PositionDirection::Long => entry_px.0 - sl.0,
                PositionDirection::Short => sl.0 - entry_px.0,
            };
            if risk <= Decimal::ZERO {
                TrailingModel::None
            } else {
                TrailingModel::ProgressiveHalfR {
                    start_r: self.config.trail_activate_points / risk,
                    step_r: self.config.trail_distance_points / risk,
                }
            }
        };

        let use_repriced_limit = self.config.max_sl_points.is_some()
            && matches!(self.config.max_sl_mode, MaxSlMode::LimitReprice)
            && initial_risk > self.config.max_sl_points.unwrap_or(Decimal::ZERO);

        let entry = if use_repriced_limit {
            EntryModel::LimitTouch {
                price: entry_px,
                expiry_bars: self.config.limit_timeout_bars,
            }
        } else {
            match self.config.entry_mode {
                DojiEntryMode::MidpointLimit => EntryModel::LimitTouch {
                    price: entry_px,
                    expiry_bars: self.config.limit_timeout_bars,
                },
                DojiEntryMode::MarketClose => EntryModel::MarketClose,
            }
        };

        let target = match self.config.target_mode {
            DojiTargetMode::RunnerR(r) => TargetModel::FixedR(r),
            DojiTargetMode::FixedPoints(points) => TargetModel::FixedPoints(points),
        };

        vec![SetupCandidate {
            direction,
            signal_index: ind,
            entry,
            stop: StopModel::FixedPrice(sl),
            target,
            trailing,
        }]
    }
}

impl TradingModel for Doji {
    fn execute(&self) -> BacktestResult {
        let setups = self.detect_setups();
        let trades = run_setups(&self.data, &setups, &self.config.execution);
        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}

impl Doji {
    pub fn detect_setups(&self) -> Vec<SetupCandidate> {
        let detector = DojiDetector {
            config: self.config,
        };
        let ctx = MarketContext::default();
        let mut setups = Vec::new();
        let mut ind = 0usize;
        while ind < self.data.len() {
            setups.extend(detector.detect(ind, &self.data, &ctx));
            ind += 1;
        }
        setups
    }
}
