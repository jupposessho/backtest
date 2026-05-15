use rust_decimal::Decimal;

use crate::engine::types::ExecutionConfig;
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::position_direction::PositionDirection;
use crate::model::trade::Trade;
use crate::model::trade_result::TradeResult;
use crate::model::trading_model::TradingModel;
use crate::model::trigger_type::TriggerType;
use crate::strategies::lib::{first_swing, is_swing_low};

use super::lib::{add_to_swings, find_sfp_high, find_sfp_low, is_swing_high, trigger_mayne};

#[derive(Clone, Copy, Debug)]
pub enum ReversalPattern {
    Mss,
    Ob,
    CisdBodyFlip,
    CisdStrictWickBreak,
    CisdLastSeriesCloseBreak,
    IfvgOnly,
    CisdStrictWickBreakAndIfvg,
    CisdStrictWickBreakOrIfvg,
}

#[derive(Clone, Copy, Debug)]
pub enum SlVariant {
    SfpExtreme,
    LtfRecentSwing,
}

#[derive(Clone, Copy, Debug)]
pub enum TpVariant {
    OpposingHtfSwing,
    OpposingLtfSwing,
}

pub struct Mayne {
    pub rr_threshold: Decimal,
    pub trigger_type: TriggerType,
    pub reversal_pattern: ReversalPattern,
    pub sl_variant: SlVariant,
    pub tp_variant: TpVariant,
    pub ifvg_max_confirm_bars: usize,
    pub htf_data: Vec<CandleStick>,
    pub ltf_data: Vec<CandleStick>,
    pub execution: ExecutionConfig,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MayneDiagnostics {
    pub htf_sfp_hits: usize,
    pub reversal_pass: usize,
    pub ifvg_found: usize,
    pub ifvg_distance_reject: usize,
    pub ltf_trigger_hit: usize,
    pub rr_pass: usize,
    pub trades: usize,
    pub winners: usize,
    pub expenses: usize,
}

impl Mayne {
    fn has_reversal_confirmation(&self, index: usize, direction: PositionDirection) -> bool {
        if index == 0 || index >= self.htf_data.len() {
            return false;
        }

        let current = self.htf_data[index];
        let previous = self.htf_data[index - 1];

        match self.reversal_pattern {
            ReversalPattern::Mss => match direction {
                PositionDirection::Short => current.close < previous.low,
                PositionDirection::Long => current.close > previous.high,
            },
            ReversalPattern::Ob => match direction {
                PositionDirection::Short => {
                    previous.close > previous.open
                        && current.close < current.open
                        && current.close < previous.low
                }
                PositionDirection::Long => {
                    previous.close < previous.open
                        && current.close > current.open
                        && current.close > previous.high
                }
            },
            ReversalPattern::CisdBodyFlip => {
                if index < 3 {
                    return false;
                }
                let prev1 = self.htf_data[index - 1];
                let prev2 = self.htf_data[index - 2];
                let prev3 = self.htf_data[index - 3];
                match direction {
                    PositionDirection::Long => {
                        let bearish_count = (prev1.close < prev1.open) as i32
                            + (prev2.close < prev2.open) as i32
                            + (prev3.close < prev3.open) as i32;
                        bearish_count >= 2 && current.close > current.open
                    }
                    PositionDirection::Short => {
                        let bullish_count = (prev1.close > prev1.open) as i32
                            + (prev2.close > prev2.open) as i32
                            + (prev3.close > prev3.open) as i32;
                        bullish_count >= 2 && current.close < current.open
                    }
                }
            }
            ReversalPattern::CisdStrictWickBreak
            | ReversalPattern::CisdStrictWickBreakAndIfvg
            | ReversalPattern::CisdStrictWickBreakOrIfvg
            | ReversalPattern::IfvgOnly => {
                if index < 3 {
                    return false;
                }
                let prev1 = self.htf_data[index - 1];
                let prev2 = self.htf_data[index - 2];
                let prev3 = self.htf_data[index - 3];
                match direction {
                    PositionDirection::Long => {
                        let level = [prev3, prev2, prev1]
                            .iter()
                            .filter(|c| c.close < c.open)
                            .map(|c| c.close)
                            .max_by(|a, b| a.0.cmp(&b.0));
                        if let Some(l) = level {
                            current.high > l
                        } else {
                            false
                        }
                    }
                    PositionDirection::Short => {
                        let level = [prev3, prev2, prev1]
                            .iter()
                            .filter(|c| c.close > c.open)
                            .map(|c| c.close)
                            .min_by(|a, b| a.0.cmp(&b.0));
                        if let Some(l) = level {
                            current.low < l
                        } else {
                            false
                        }
                    }
                }
            }
            ReversalPattern::CisdLastSeriesCloseBreak => {
                if index < 3 {
                    return false;
                }
                let prev1 = self.htf_data[index - 1];
                let prev2 = self.htf_data[index - 2];
                let prev3 = self.htf_data[index - 3];
                match direction {
                    PositionDirection::Long => {
                        let level = [prev3, prev2, prev1]
                            .iter()
                            .rev()
                            .find(|c| c.close < c.open)
                            .map(|c| c.close);
                        if let Some(l) = level {
                            current.close > l
                        } else {
                            false
                        }
                    }
                    PositionDirection::Short => {
                        let level = [prev3, prev2, prev1]
                            .iter()
                            .rev()
                            .find(|c| c.close > c.open)
                            .map(|c| c.close);
                        if let Some(l) = level {
                            current.close < l
                        } else {
                            false
                        }
                    }
                }
            }
        }
    }

    fn ltf_index_for_sfp_candle(&self, sfp_candle: CandleStick) -> Option<usize> {
        self.ltf_data.iter().position(|x| {
            x.open_time == sfp_candle.open_time && x.close_time == sfp_candle.close_time
        })
    }

    fn find_ltf_anchor_for_htf(
        &self,
        htf_candle: CandleStick,
        direction: PositionDirection,
    ) -> Option<CandleStick> {
        let mut best: Option<CandleStick> = None;
        for c in self.ltf_data.iter().copied() {
            if c.open_time >= htf_candle.open_time && c.close_time <= htf_candle.close_time {
                best = match (best, direction) {
                    (None, _) => Some(c),
                    (Some(prev), PositionDirection::Short) => {
                        if c.high > prev.high {
                            Some(c)
                        } else {
                            Some(prev)
                        }
                    }
                    (Some(prev), PositionDirection::Long) => {
                        if c.low < prev.low {
                            Some(c)
                        } else {
                            Some(prev)
                        }
                    }
                };
            }
        }
        best
    }

    fn has_ifvg_confirmation_ltf(
        &self,
        ltf_sfp_index: usize,
        expected_direction: PositionDirection,
    ) -> Option<usize> {
        if ltf_sfp_index < 3 || ltf_sfp_index >= self.ltf_data.len() {
            return None;
        }

        let start = ltf_sfp_index.saturating_sub(self.ifvg_max_confirm_bars.max(3));
        let end = (ltf_sfp_index + self.ifvg_max_confirm_bars).min(self.ltf_data.len() - 1);

        match expected_direction {
            PositionDirection::Long => {
                let mut i = start;
                while i + 2 < end {
                    let c1 = self.ltf_data[i];
                    let c3 = self.ltf_data[i + 2];
                    if c3.high < c1.low {
                        let gap_high = c1.low;
                        let mut k = (i + 3).max(ltf_sfp_index.saturating_sub(1));
                        while k <= end {
                            let previous = self.ltf_data[k - 1];
                            let current = self.ltf_data[k];
                            if previous.close <= gap_high && current.close > gap_high {
                                return Some(k.saturating_sub(i));
                            }
                            k += 1;
                        }
                    }
                    i += 1;
                }
            }
            PositionDirection::Short => {
                let mut i = start;
                while i + 2 < end {
                    let c1 = self.ltf_data[i];
                    let c3 = self.ltf_data[i + 2];
                    if c3.low > c1.high {
                        let gap_low = c1.high;
                        let mut k = (i + 3).max(ltf_sfp_index.saturating_sub(1));
                        while k <= end {
                            let previous = self.ltf_data[k - 1];
                            let current = self.ltf_data[k];
                            if previous.close >= gap_low && current.close < gap_low {
                                return Some(k.saturating_sub(i));
                            }
                            k += 1;
                        }
                    }
                    i += 1;
                }
            }
        }

        None
    }

    fn reversal_gate(
        &self,
        htf_index: usize,
        direction: PositionDirection,
        ltf_sfp_index: usize,
        diagnostics: &mut MayneDiagnostics,
    ) -> bool {
        let cisd_strict = self.has_reversal_confirmation(htf_index, direction)
            && matches!(
                self.reversal_pattern,
                ReversalPattern::CisdStrictWickBreak
                    | ReversalPattern::CisdStrictWickBreakAndIfvg
                    | ReversalPattern::CisdStrictWickBreakOrIfvg
            );
        let base_reversal = self.has_reversal_confirmation(htf_index, direction);
        let ifvg_distance = self.has_ifvg_confirmation_ltf(ltf_sfp_index, direction);
        let ifvg_ok = if let Some(distance) = ifvg_distance {
            if distance <= self.ifvg_max_confirm_bars {
                diagnostics.ifvg_found += 1;
                true
            } else {
                diagnostics.ifvg_distance_reject += 1;
                false
            }
        } else {
            false
        };

        let passed = match self.reversal_pattern {
            ReversalPattern::IfvgOnly => ifvg_ok,
            ReversalPattern::CisdStrictWickBreakAndIfvg => cisd_strict && ifvg_ok,
            ReversalPattern::CisdStrictWickBreakOrIfvg => cisd_strict || ifvg_ok,
            _ => base_reversal,
        };

        if passed {
            diagnostics.reversal_pass += 1;
        }

        passed
    }

    pub fn execute_with_diagnostics(&self) -> (BacktestResult, MayneDiagnostics) {
        let mut diagnostics = MayneDiagnostics::default();
        let mut swing_lows: Vec<CandleStick> = vec![];
        let mut swing_highs: Vec<CandleStick> = vec![];
        let mut trades: Vec<Trade> = vec![];

        let mut ind = 0;
        while ind < self.htf_data.len() {
            if ind > 0 && ind < self.htf_data.len() - 1 {
                let actual = self.htf_data[ind];
                let previous = self.htf_data[ind - 1];
                let next = self.htf_data[ind + 1];

                if let Some(prev_swing_low) = swing_lows.iter().last() {
                    if find_sfp_high(actual, &swing_highs).is_some() {
                        diagnostics.htf_sfp_hits += 1;
                        if let Some(ltf_sfp_candle) =
                            self.find_ltf_anchor_for_htf(actual, PositionDirection::Short)
                        {
                            if let Some(ltf_sfp_index) =
                                self.ltf_index_for_sfp_candle(ltf_sfp_candle)
                            {
                                if self.reversal_gate(
                                    ind,
                                    PositionDirection::Short,
                                    ltf_sfp_index,
                                    &mut diagnostics,
                                ) {
                                    let (mut previous_candles, next_candles): (Vec<_>, Vec<_>) =
                                        self.ltf_data
                                            .clone()
                                            .into_iter()
                                            .partition(|x| x.open_time < ltf_sfp_candle.open_time);
                                    previous_candles.reverse();
                                    if let Some(prev_ltf_swing_low) =
                                        first_swing(previous_candles.clone(), is_swing_low)
                                    {
                                        let sl = match self.sl_variant {
                                            SlVariant::SfpExtreme => actual.high,
                                            SlVariant::LtfRecentSwing => {
                                                first_swing(previous_candles.clone(), is_swing_high)
                                                    .map(|c| c.high)
                                                    .unwrap_or(actual.high)
                                            }
                                        };
                                        let tp = match self.tp_variant {
                                            TpVariant::OpposingHtfSwing => prev_swing_low.low,
                                            TpVariant::OpposingLtfSwing => prev_ltf_swing_low.low,
                                        };
                                        let before = trades.len();
                                        trigger_mayne(
                                            PositionDirection::Short,
                                            self.trigger_type,
                                            prev_ltf_swing_low.low,
                                            sl,
                                            tp,
                                            self.rr_threshold,
                                            next_candles,
                                            &mut trades,
                                            &self.execution,
                                        );
                                        if trades.len() > before {
                                            diagnostics.ltf_trigger_hit += 1;
                                            diagnostics.rr_pass += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(prev_swing_high) = swing_highs.iter().last() {
                    if find_sfp_low(actual, &swing_lows).is_some() {
                        diagnostics.htf_sfp_hits += 1;
                        if let Some(ltf_sfp_candle) =
                            self.find_ltf_anchor_for_htf(actual, PositionDirection::Long)
                        {
                            if let Some(ltf_sfp_index) =
                                self.ltf_index_for_sfp_candle(ltf_sfp_candle)
                            {
                                if self.reversal_gate(
                                    ind,
                                    PositionDirection::Long,
                                    ltf_sfp_index,
                                    &mut diagnostics,
                                ) {
                                    let (mut previous_candles, next_candles): (Vec<_>, Vec<_>) =
                                        self.ltf_data
                                            .clone()
                                            .into_iter()
                                            .partition(|x| x.open_time < ltf_sfp_candle.open_time);
                                    previous_candles.reverse();
                                    if let Some(prev_ltf_swing_high) =
                                        first_swing(previous_candles.clone(), is_swing_high)
                                    {
                                        let sl = match self.sl_variant {
                                            SlVariant::SfpExtreme => actual.low,
                                            SlVariant::LtfRecentSwing => {
                                                first_swing(previous_candles.clone(), is_swing_low)
                                                    .map(|c| c.low)
                                                    .unwrap_or(actual.low)
                                            }
                                        };
                                        let tp = match self.tp_variant {
                                            TpVariant::OpposingHtfSwing => prev_swing_high.high,
                                            TpVariant::OpposingLtfSwing => prev_ltf_swing_high.high,
                                        };
                                        let before = trades.len();
                                        trigger_mayne(
                                            PositionDirection::Long,
                                            self.trigger_type,
                                            prev_ltf_swing_high.high,
                                            sl,
                                            tp,
                                            self.rr_threshold,
                                            next_candles,
                                            &mut trades,
                                            &self.execution,
                                        );
                                        if trades.len() > before {
                                            diagnostics.ltf_trigger_hit += 1;
                                            diagnostics.rr_pass += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                add_to_swings(&mut swing_lows, &mut swing_highs, actual, previous, next)
            }
            ind += 1;
        }

        let result = BacktestResult {
            trades,
            capital: Decimal::from(1000),
        };
        diagnostics.trades = result.number_of_trades();
        diagnostics.winners = result.result(TradeResult::Winner);
        diagnostics.expenses = result.result(TradeResult::Expense);

        (result, diagnostics)
    }
}

impl TradingModel for Mayne {
    fn execute(&self) -> BacktestResult {
        self.execute_with_diagnostics().0
    }
}
