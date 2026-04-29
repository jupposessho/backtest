use std::fmt;

use crate::engine::types::ExecutionConfig;
use crate::model::decimal::DecimalVec;
use crate::to_new_york_time;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;

use super::{position::Position, position_direction::PositionDirection, trade_result::TradeResult};

#[derive(Clone, Copy)]
pub struct Trade {
    pub direction: PositionDirection,
    pub open_time: i64,
    pub close_time: i64,
    pub entry: DecimalVec,
    pub sl: DecimalVec,
    pub tp: DecimalVec,
    pub result: TradeResult,
    pub commission: Decimal,
    pub slippage: Decimal,
    pub fees: Decimal,
}

impl Trade {
    pub fn rr(&self) -> DecimalVec {
        match self.direction {
            PositionDirection::Short => (self.entry - self.tp) / (self.sl - self.entry),
            PositionDirection::Long => (self.tp - self.entry) / (self.entry - self.sl),
        }
    }

    pub fn points(&self) -> DecimalVec {
        match self.direction {
            PositionDirection::Short => match self.result {
                TradeResult::Winner => self.entry - self.tp,
                TradeResult::Expense => self.entry - self.sl,
                TradeResult::BreakEven => DecimalVec::new(0),
            },
            PositionDirection::Long => match self.result {
                TradeResult::Winner => self.tp - self.entry,
                TradeResult::Expense => self.sl - self.entry,
                TradeResult::BreakEven => DecimalVec::new(0),
            },
        }
    }

    pub fn total_costs(&self) -> Decimal {
        self.commission + self.slippage + self.fees
    }

    pub fn gross_r(&self) -> Decimal {
        match self.result {
            TradeResult::Winner => self.rr().0,
            TradeResult::Expense => Decimal::from(-1),
            TradeResult::BreakEven => Decimal::ZERO,
        }
    }
}

impl fmt::Debug for Trade {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let o = to_new_york_time(self.open_time.clone())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let c = to_new_york_time(self.close_time.clone())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        f.debug_struct("Trade")
            .field("direction", &self.direction)
            .field("open_time", &o)
            .field("close_time", &c)
            .field("entry", &self.entry.0)
            .field("sl", &self.sl.0)
            .field("tp", &self.tp.0)
            .field("rr", &self.rr().0)
            .field("result", &self.result)
            .field("commission", &self.commission)
            .field("slippage", &self.slippage)
            .field("fees", &self.fees)
            .finish()
    }
}

impl Trade {
    pub(crate) fn from_position(position: Position, close_time: i64, result: TradeResult) -> Trade {
        Trade {
            direction: position.direction,
            open_time: position.open_time,
            close_time,
            entry: position.entry,
            sl: position.sl,
            tp: position.tp,
            result,
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO,
            fees: Decimal::ZERO,
        }
    }

    pub fn from_position_with_exit(
        position: Position,
        close_time: i64,
        exit: DecimalVec,
        result: TradeResult,
        execution: &ExecutionConfig,
    ) -> Trade {
        let notional = (position.entry.0 + exit.0) / Decimal::from(2);
        let commission = notional * execution.commission_rate_per_side * Decimal::from(2);
        let fees = notional * execution.fee_rate_per_side * Decimal::from(2);
        let slip = execution.tick_size
            * Decimal::from_i32(execution.slippage_ticks_per_side).unwrap_or(Decimal::ZERO);
        let slippage = slip.abs() * Decimal::from(2);

        Trade {
            direction: position.direction,
            open_time: position.open_time,
            close_time,
            entry: position.entry,
            sl: position.sl,
            tp: exit,
            result,
            commission,
            slippage,
            fees,
        }
    }
}
