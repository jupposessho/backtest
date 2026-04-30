use chrono::{Datelike, NaiveDate, Timelike};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::fee_config::FeeConfig;
use crate::model::position::Position;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;
use crate::to_new_york_time;
use crate::engine::types::ExecutionConfig;
use std::sync::Arc;

/// TTrades Fractal Model Strategy
/// Based on: https://ttrades.com/the-only-trading-strategy-you-need-for-2026/
pub struct TTradesFractal {
    pub data: Arc<Vec<CandleStick>>,
    pub config: FractalConfig,
}

#[derive(Clone)]
pub struct FractalConfig {
    /// Target risk-to-reward ratio (default: 2.0)
    pub rr_target: Decimal,
    /// Fee configuration
    pub fee_config: FeeConfig,
    /// Enable FVG detection
    pub use_fvg: bool,
    /// Minimum candles to look back for structure
    pub lookback_candles: usize,
    /// Require CISD (Change in State of Delivery) confirmation
    pub require_cisd: bool,
    pub slippage_ticks_per_side: i32,
    pub tick_size: Decimal,
}

impl Default for FractalConfig {
    fn default() -> Self {
        Self {
            rr_target: Decimal::from(2),
            fee_config: FeeConfig::default(),
            use_fvg: true,
            lookback_candles: 20,
            require_cisd: true,
            slippage_ticks_per_side: 0,
            tick_size: Decimal::new(1, 2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DailyBias {
    BullishContinuation,
    BearishContinuation,
    BullishReversal,
    BearishReversal,
    None,
}

#[derive(Debug, Clone)]
struct FairValueGap {
    high: DecimalVec,
    low: DecimalVec,
    candle_index: usize,
    direction: PositionDirection, // Bullish FVG or Bearish FVG
}

#[derive(Debug, Clone)]
struct SwingPoint {
    price: DecimalVec,
    candle_index: usize,
    is_high: bool, // true for swing high, false for swing low
}

impl TTradesFractal {
    fn execution_config(&self) -> ExecutionConfig {
        let hundred = Decimal::from(100);
        ExecutionConfig {
            commission_rate_per_side: self.config.fee_config.maker_fee_pct / hundred,
            fee_rate_per_side: self.config.fee_config.taker_fee_pct / hundred,
            slippage_ticks_per_side: self.config.slippage_ticks_per_side,
            tick_size: self.config.tick_size,
        }
    }

    /// Determine daily bias based on previous day's candle
    fn determine_daily_bias(&self, current_index: usize) -> DailyBias {
        if current_index < 2 {
            return DailyBias::None;
        }

        let current = self.data[current_index];
        let current_date = to_new_york_time(current.open_time).date_naive();

        // Find previous day's candle (last candle of previous day)
        let mut prev_day_index = None;
        for i in (0..current_index).rev() {
            let candle_date = to_new_york_time(self.data[i].open_time).date_naive();
            if candle_date < current_date {
                prev_day_index = Some(i);
                break;
            }
        }

        if prev_day_index.is_none() {
            return DailyBias::None;
        }

        let prev_day_idx = prev_day_index.unwrap();
        let prev_day = self.data[prev_day_idx];

        // Find the daily high and low of the previous day
        let mut prev_day_high = prev_day.high;
        let mut prev_day_low = prev_day.low;

        for i in 0..=prev_day_idx {
            let candle_date = to_new_york_time(self.data[i].open_time).date_naive();
            if candle_date == to_new_york_time(prev_day.open_time).date_naive() {
                if self.data[i].high > prev_day_high {
                    prev_day_high = self.data[i].high;
                }
                if self.data[i].low < prev_day_low {
                    prev_day_low = self.data[i].low;
                }
            }
        }

        let close = current.close;

        // Bullish Continuation: Close above previous day's high
        if close > prev_day_high {
            return DailyBias::BullishContinuation;
        }

        // Bearish Continuation: Close below previous day's low
        if close < prev_day_low {
            return DailyBias::BearishContinuation;
        }

        // Check for reversal patterns (sweep and close back)
        // Bullish Reversal: Sweep below prev day low, then close back above it
        if current.low < prev_day_low && close > prev_day_low {
            return DailyBias::BullishReversal;
        }

        // Bearish Reversal: Sweep above prev day high, then close back below it
        if current.high > prev_day_high && close < prev_day_high {
            return DailyBias::BearishReversal;
        }

        DailyBias::None
    }

    /// Detect Fair Value Gaps (FVG)
    fn detect_fvgs(&self, start_index: usize, end_index: usize) -> Vec<FairValueGap> {
        let mut fvgs = Vec::new();

        if end_index < start_index + 3 {
            return fvgs;
        }

        for i in start_index..end_index - 2 {
            let candle1 = self.data[i];
            let _candle2 = self.data[i + 1];
            let candle3 = self.data[i + 2];

            // Bullish FVG: Gap between candle1 high and candle3 low (candle2 doesn't fill it)
            if candle3.low > candle1.high {
                fvgs.push(FairValueGap {
                    high: candle3.low,
                    low: candle1.high,
                    candle_index: i + 2,
                    direction: PositionDirection::Long,
                });
            }

            // Bearish FVG: Gap between candle1 low and candle3 high (candle2 doesn't fill it)
            if candle3.high < candle1.low {
                fvgs.push(FairValueGap {
                    high: candle1.low,
                    low: candle3.high,
                    candle_index: i + 2,
                    direction: PositionDirection::Short,
                });
            }
        }

        fvgs
    }

    /// Detect swing highs and lows
    fn detect_swing_points(&self, start_index: usize, end_index: usize) -> Vec<SwingPoint> {
        let mut swing_points = Vec::new();

        if end_index < start_index + 3 {
            return swing_points;
        }

        for i in start_index + 1..end_index - 1 {
            let prev = self.data[i - 1];
            let current = self.data[i];
            let next = self.data[i + 1];

            // Swing High: Current high is higher than both neighbors
            if current.high > prev.high && current.high > next.high {
                swing_points.push(SwingPoint {
                    price: current.high,
                    candle_index: i,
                    is_high: true,
                });
            }

            // Swing Low: Current low is lower than both neighbors
            if current.low < prev.low && current.low < next.low {
                swing_points.push(SwingPoint {
                    price: current.low,
                    candle_index: i,
                    is_high: false,
                });
            }
        }

        swing_points
    }

    /// Check if current price is in a FVG zone
    fn is_in_fvg(&self, price: DecimalVec, fvgs: &[FairValueGap]) -> Option<FairValueGap> {
        for fvg in fvgs.iter().rev() {
            // Check if price is within FVG range
            if price >= fvg.low && price <= fvg.high {
                return Some(fvg.clone());
            }
        }
        None
    }

    /// Detect Change in State of Delivery (CISD)
    /// This occurs when we see a series of candles in one direction, then a reversal pattern
    fn detect_cisd(
        &self,
        current_index: usize,
        direction: PositionDirection,
        lookback: usize,
    ) -> bool {
        if current_index < lookback {
            return false;
        }

        let start = current_index.saturating_sub(lookback);
        let mut consecutive_count = 0;

        match direction {
            PositionDirection::Long => {
                // Looking for bullish CISD: previously bearish, now turning bullish
                for i in start..current_index {
                    if self.data[i].close < self.data[i].open {
                        consecutive_count += 1;
                    }
                }
                // Need at least 2-3 bearish candles before, then bullish reversal
                consecutive_count >= 2 && self.data[current_index].close > self.data[current_index].open
            }
            PositionDirection::Short => {
                // Looking for bearish CISD: previously bullish, now turning bearish
                for i in start..current_index {
                    if self.data[i].close > self.data[i].open {
                        consecutive_count += 1;
                    }
                }
                // Need at least 2-3 bullish candles before, then bearish reversal
                consecutive_count >= 2 && self.data[current_index].close < self.data[current_index].open
            }
        }
    }

    /// Detect Continuation Order Block
    /// Forms when price sweeps a short-term high/low and closes through opposing candles
    fn detect_continuation_order_block(
        &self,
        current_index: usize,
        direction: PositionDirection,
    ) -> Option<DecimalVec> {
        if current_index < 5 {
            return None;
        }

        let current = self.data[current_index];
        let swing_points = self.detect_swing_points(
            current_index.saturating_sub(10),
            current_index,
        );

        match direction {
            PositionDirection::Long => {
                // For bullish continuation: sweep below recent low, then close bullish
                for swing in swing_points.iter().rev() {
                    if !swing.is_high {
                        // Found a swing low
                        // Check if current candle swept below it
                        if current.low < swing.price && current.close > current.open {
                            // Swept the low and closed bullish
                            return Some(swing.price);
                        }
                    }
                }
            }
            PositionDirection::Short => {
                // For bearish continuation: sweep above recent high, then close bearish
                for swing in swing_points.iter().rev() {
                    if swing.is_high {
                        // Found a swing high
                        // Check if current candle swept above it
                        if current.high > swing.price && current.close < current.open {
                            // Swept the high and closed bearish
                            return Some(swing.price);
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if we should avoid trading (choppy conditions)
    fn should_avoid_trading(&self, current_index: usize, lookback: usize) -> bool {
        if current_index < lookback {
            return true;
        }

        let start = current_index.saturating_sub(lookback);
        let mut range_high = self.data[start].high;
        let mut range_low = self.data[start].low;

        // Calculate range
        for i in start..current_index {
            if self.data[i].high > range_high {
                range_high = self.data[i].high;
            }
            if self.data[i].low < range_low {
                range_low = self.data[i].low;
            }
        }

        let range = range_high.0 - range_low.0;

        // Check if price is stuck in middle third of range (choppy)
        let upper_third = range_low.0 + (range / Decimal::from(3) * Decimal::from(2));
        let lower_third = range_low.0 + (range / Decimal::from(3));

        let current_price = self.data[current_index].close.0;

        // If price is in middle third, consider it choppy
        current_price > lower_third && current_price < upper_third
    }
}

impl TradingModel for TTradesFractal {
    fn execute(&self) -> BacktestResult {
        let mut trades = Vec::new();
        let mut position: Option<Position> = None;
        let execution = self.execution_config();

        for ind in 1..self.data.len() {
            let actual = self.data[ind];

            // Check if we have an open position and manage it
            if let Some(pos) = position {
                match pos.direction {
                    PositionDirection::Short => {
                        // Check SL hit
                        if pos.sl < actual.high {
                            trades.push(Trade::from_position_with_exit(
                                pos,
                                actual.close_time,
                                pos.sl,
                                TradeResult::Expense,
                                &execution,
                            ));
                            position = None;
                        }
                        // Check TP hit
                        else if pos.tp > actual.low {
                            trades.push(Trade::from_position_with_exit(
                                pos,
                                actual.close_time,
                                pos.tp,
                                TradeResult::Winner,
                                &execution,
                            ));
                            position = None;
                        }
                    }
                    PositionDirection::Long => {
                        // Check SL hit
                        if pos.sl > actual.low {
                            trades.push(Trade::from_position_with_exit(
                                pos,
                                actual.close_time,
                                pos.sl,
                                TradeResult::Expense,
                                &execution,
                            ));
                            position = None;
                        }
                        // Check TP hit
                        else if pos.tp < actual.high {
                            trades.push(Trade::from_position_with_exit(
                                pos,
                                actual.close_time,
                                pos.tp,
                                TradeResult::Winner,
                                &execution,
                            ));
                            position = None;
                        }
                    }
                }
            }

            // Only look for new trades if no position is open
            if position.is_none() {
                // Step 1: Determine daily bias
                let bias = self.determine_daily_bias(ind);

                if bias == DailyBias::None {
                    continue;
                }

                // Step 2: Check if we should avoid trading (choppy conditions)
                if self.should_avoid_trading(ind, self.config.lookback_candles) {
                    continue;
                }

                // Step 3: Detect FVGs and swing points
                let start_index = ind.saturating_sub(self.config.lookback_candles);
                let fvgs = if self.config.use_fvg {
                    self.detect_fvgs(start_index, ind)
                } else {
                    Vec::new()
                };

                let swing_points = self.detect_swing_points(start_index, ind);

                // Step 4: Look for entry based on bias
                match bias {
                    DailyBias::BullishContinuation | DailyBias::BullishReversal => {
                        // Looking for bullish entry

                        // Check if price is retracing into a point of interest
                        let in_fvg = if self.config.use_fvg {
                            self.is_in_fvg(actual.close, &fvgs)
                        } else {
                            None
                        };

                        // Check for swing low nearby
                        let near_swing_low = swing_points.iter().rev().find(|s| {
                            !s.is_high && (actual.close.0 - s.price.0).abs() < (s.price.0 * Decimal::from_f32(0.01).unwrap())
                        });

                        if in_fvg.is_some() || near_swing_low.is_some() {
                            // Check for CISD if required
                            let cisd_confirmed = if self.config.require_cisd {
                                self.detect_cisd(ind, PositionDirection::Long, 5)
                            } else {
                                true
                            };

                            if cisd_confirmed {
                                // Check for continuation order block
                                if let Some(ob_level) = self.detect_continuation_order_block(ind, PositionDirection::Long) {
                                    // Entry signal confirmed - create position
                                    let entry = actual.close;
                                    let sl = ob_level - DecimalVec(Decimal::from_f32(0.0001).unwrap()); // Just below order block
                                    let risk = entry - sl;
                                    let tp = entry + DecimalVec(risk.0 * self.config.rr_target);

                                    position = Some(Position {
                                        direction: PositionDirection::Long,
                                        open_time: actual.open_time,
                                        entry,
                                        sl,
                                        tp,
                                        at_break_even: false,
                                    });
                                }
                            }
                        }
                    }
                    DailyBias::BearishContinuation | DailyBias::BearishReversal => {
                        // Looking for bearish entry

                        // Check if price is retracing into a point of interest
                        let in_fvg = if self.config.use_fvg {
                            self.is_in_fvg(actual.close, &fvgs)
                        } else {
                            None
                        };

                        // Check for swing high nearby
                        let near_swing_high = swing_points.iter().rev().find(|s| {
                            s.is_high && (actual.close.0 - s.price.0).abs() < (s.price.0 * Decimal::from_f32(0.01).unwrap())
                        });

                        if in_fvg.is_some() || near_swing_high.is_some() {
                            // Check for CISD if required
                            let cisd_confirmed = if self.config.require_cisd {
                                self.detect_cisd(ind, PositionDirection::Short, 5)
                            } else {
                                true
                            };

                            if cisd_confirmed {
                                // Check for continuation order block
                                if let Some(ob_level) = self.detect_continuation_order_block(ind, PositionDirection::Short) {
                                    // Entry signal confirmed - create position
                                    let entry = actual.close;
                                    let sl = ob_level + DecimalVec(Decimal::from_f32(0.0001).unwrap()); // Just above order block
                                    let risk = sl - entry;
                                    let tp = entry - DecimalVec(risk.0 * self.config.rr_target);

                                    position = Some(Position {
                                        direction: PositionDirection::Short,
                                        open_time: actual.open_time,
                                        entry,
                                        sl,
                                        tp,
                                        at_break_even: false,
                                    });
                                }
                            }
                        }
                    }
                    DailyBias::None => {}
                }
            }
        }

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fractal_config_default() {
        let config = FractalConfig::default();
        assert_eq!(config.rr_target, Decimal::from(2));
        assert_eq!(config.use_fvg, true);
        assert_eq!(config.require_cisd, true);
    }
}
