use std::fmt;

use rust_decimal::Decimal;

use super::{trade::Trade, trade_result::TradeResult};

pub struct BacktestResult {
    pub trades: Vec<Trade>,
}

impl BacktestResult {
    pub fn number_of_trades(&self) -> usize {
        self.trades.len()
    }
    pub fn result(&self, tr: TradeResult) -> usize {
        self.trades
            .clone()
            .into_iter()
            .filter(|x| x.result == tr)
            .collect::<Vec<_>>()
            .len()
    }
    pub fn profit_in_r(&self) -> Decimal {
        self.trades
            .clone()
            .into_iter()
            .map(|x| match x.result {
                TradeResult::Winner => x.rr().0,
                TradeResult::Expense => Decimal::from(-1),
                TradeResult::BreakEven => Decimal::from(0),
            })
            .sum()
    }
}

impl fmt::Debug for BacktestResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BacktestResult")
            .field("trades", &self.trades)
            .field("number_of_trades", &self.number_of_trades())
            .field("winners", &self.result(TradeResult::Winner))
            .field("expenses", &self.result(TradeResult::Expense))
            .field("break_evens", &self.result(TradeResult::BreakEven))
            .field("profit_in_r", &self.profit_in_r())
            .finish()
    }
}
