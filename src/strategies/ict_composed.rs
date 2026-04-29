use crate::engine::execution::run_setups;
use crate::engine::ict::mss::MssDetector;
use crate::engine::ict::sweep::{SweepDetector, SweepSignal};
use crate::engine::types::{
    EntryModel, EntryPolicy, ExecutionConfig, SetupCandidate, StopModel, TargetModel, TrailingModel,
};
use crate::model::backtest_result::BacktestResult;
use crate::model::candle_stick::CandleStick;
use crate::model::decimal::DecimalVec;
use crate::model::position_direction::PositionDirection;
use crate::model::trading_model::TradingModel;
use rust_decimal::Decimal;

#[derive(Clone, Copy)]
pub enum IctEntryChoice {
    ObPrevOpen,
    ObPairMidpoint,
    OteMidpoint,
}

pub struct IctComposed {
    pub data: Vec<CandleStick>,
    pub rr_target: Decimal,
    pub sweep_lookback: usize,
    pub mss_confirm_window: usize,
    pub entry_choice: IctEntryChoice,
    pub entry_expiry_bars: usize,
    pub execution: ExecutionConfig,
}

impl IctComposed {
    fn to_entry_model(&self, direction: PositionDirection, signal: CandleStick, sweep: SweepSignal) -> EntryModel {
        match self.entry_choice {
            IctEntryChoice::ObPrevOpen => EntryModel::LimitByPolicy {
                policy: EntryPolicy::ObPrevOpen,
                expiry_bars: self.entry_expiry_bars,
            },
            IctEntryChoice::ObPairMidpoint => EntryModel::LimitByPolicy {
                policy: EntryPolicy::ObPairMidpoint,
                expiry_bars: self.entry_expiry_bars,
            },
            IctEntryChoice::OteMidpoint => {
                let move_range = match direction {
                    PositionDirection::Long => signal.close.0 - sweep.sweep_extreme.0,
                    PositionDirection::Short => sweep.sweep_extreme.0 - signal.close.0,
                };
                let (ote_low, ote_high) = match direction {
                    PositionDirection::Long => {
                        let low = signal.close.0 - move_range * Decimal::from_str_exact("0.786").unwrap();
                        let high = signal.close.0 - move_range * Decimal::from_str_exact("0.618").unwrap();
                        (DecimalVec(low), DecimalVec(high))
                    }
                    PositionDirection::Short => {
                        let low = signal.close.0 + move_range * Decimal::from_str_exact("0.618").unwrap();
                        let high = signal.close.0 + move_range * Decimal::from_str_exact("0.786").unwrap();
                        (DecimalVec(low), DecimalVec(high))
                    }
                };
                EntryModel::LimitByPolicy {
                    policy: EntryPolicy::OteMidpoint {
                        low: ote_low,
                        high: ote_high,
                    },
                    expiry_bars: self.entry_expiry_bars,
                }
            }
        }
    }
}

impl TradingModel for IctComposed {
    fn execute(&self) -> BacktestResult {
        let sweep_detector = SweepDetector {
            lookback: self.sweep_lookback,
        };
        let mss_detector = MssDetector {
            confirm_window: self.mss_confirm_window,
        };

        let mut active_sweep: Option<SweepSignal> = None;
        let mut setups: Vec<SetupCandidate> = vec![];

        let mut i = 0usize;
        while i < self.data.len() {
            if let Some(sweep) = active_sweep {
                if let Some(mss) = mss_detector.detect_signal(i, &self.data, sweep) {
                    let signal = self.data[mss.index];
                    let stop = match mss.direction {
                        PositionDirection::Long => sweep.sweep_extreme,
                        PositionDirection::Short => sweep.sweep_extreme,
                    };
                    setups.push(SetupCandidate {
                        direction: mss.direction,
                        signal_index: mss.index,
                        entry: self.to_entry_model(mss.direction, signal, sweep),
                        stop: StopModel::FixedPrice(stop),
                        target: TargetModel::FixedR(self.rr_target),
                        trailing: TrailingModel::BreakEvenAtR(Decimal::ONE),
                    });
                    active_sweep = None;
                } else if i > sweep.index + self.mss_confirm_window {
                    active_sweep = None;
                }
            } else {
                active_sweep = sweep_detector.detect_signal(i, &self.data);
            }
            i += 1;
        }

        let trades = run_setups(&self.data, &setups, &self.execution);
        BacktestResult {
            trades,
            capital: Decimal::from(1000),
        }
    }
}
