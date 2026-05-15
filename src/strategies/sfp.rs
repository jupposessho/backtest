use rust_decimal::Decimal;

use crate::engine::context::MarketContext;
use crate::engine::detection::SetupDetector;
use crate::engine::execution::run_setups;
use crate::engine::types::{
    EntryModel, ExecutionConfig, SetupCandidate, StopModel, TargetModel, TrailingModel,
};
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::position_direction::PositionDirection;
use crate::model::trading_model::TradingModel;

use super::lib::add_to_swings;

pub struct Sfp {
    pub rr_treshold: Decimal,
    pub data: Vec<CandleStick>,
}

struct SfpDetector {
    rr_treshold: Decimal,
}

impl SetupDetector for SfpDetector {
    fn detect(
        &self,
        ind: usize,
        data: &[CandleStick],
        _ctx: &MarketContext,
    ) -> Vec<SetupCandidate> {
        if ind == 0 || ind >= data.len() - 1 {
            return vec![];
        }

        let actual = data[ind];
        let previous = data[ind - 1];
        let next = data[ind + 1];

        let mut swing_lows: Vec<CandleStick> = vec![];
        let mut swing_highs: Vec<CandleStick> = vec![];
        let mut i = 1usize;
        while i < ind {
            let p = data[i - 1];
            let a = data[i];
            let n = data[i + 1];
            add_to_swings(&mut swing_lows, &mut swing_highs, a, p, n);
            i += 1;
        }

        add_to_swings(&mut swing_lows, &mut swing_highs, actual, previous, next);

        let mut out = Vec::new();

        let sfp_high = swing_highs.iter().any(|x| {
            x.close_time < actual.close_time && x.high < actual.high && x.high > actual.close
        });
        if sfp_high {
            if let Some(prev_low) = swing_lows.last() {
                let entry = actual.close;
                let sl = actual.high;
                let tp = prev_low.low;
                let rr = (entry - tp) / (sl - entry);
                if rr.0 >= self.rr_treshold {
                    out.push(SetupCandidate {
                        direction: PositionDirection::Short,
                        signal_index: ind,
                        entry: EntryModel::SignalClose,
                        stop: StopModel::FixedPrice(sl),
                        target: TargetModel::FixedPrice(tp),
                        trailing: TrailingModel::None,
                        max_hold_bars: None,
                    });
                }
            }
        }

        let sfp_low = swing_lows.iter().any(|x| {
            x.close_time < actual.close_time && x.low > actual.low && x.low < actual.close
        });
        if sfp_low {
            if let Some(prev_high) = swing_highs.last() {
                let entry = actual.close;
                let sl = actual.low;
                let tp = prev_high.high;
                let rr = (tp - entry) / (entry - sl);
                if rr.0 >= self.rr_treshold {
                    out.push(SetupCandidate {
                        direction: PositionDirection::Long,
                        signal_index: ind,
                        entry: EntryModel::SignalClose,
                        stop: StopModel::FixedPrice(sl),
                        target: TargetModel::FixedPrice(tp),
                        trailing: TrailingModel::None,
                        max_hold_bars: None,
                    });
                }
            }
        }

        out
    }
}

impl TradingModel for Sfp {
    fn execute(&self) -> BacktestResult {
        let detector = SfpDetector {
            rr_treshold: self.rr_treshold,
        };
        let mut setups = Vec::new();
        let ctx = MarketContext::default();
        let mut ind = 0usize;
        while ind < self.data.len() {
            setups.extend(detector.detect(ind, &self.data, &ctx));
            ind += 1;
        }

        let execution = ExecutionConfig {
            commission_rate_per_side: Decimal::new(1, 3),
            fee_rate_per_side: Decimal::ZERO,
            slippage_ticks_per_side: 1,
            tick_size: Decimal::new(1, 2),
        };
        let trades = run_setups(&self.data, &setups, &execution);

        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}
