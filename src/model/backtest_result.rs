use std::fmt;

use rust_decimal::{prelude::FromPrimitive, Decimal};

use super::{trade::Trade, trade_result::TradeResult};

#[derive(Clone)]
pub struct BacktestResult {
    pub trades: Vec<Trade>,
    pub capital: Decimal,
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
        let r: Decimal = self
            .trades
            .clone()
            .into_iter()
            .map(|x| match x.result {
                TradeResult::Winner => x.rr().0,
                TradeResult::Expense => Decimal::from(-1),
                TradeResult::BreakEven => Decimal::from(0),
            })
            .sum();
        r.trunc_with_scale(2)
    }

    pub fn profit_in_points(&self) -> Decimal {
        let r: Decimal = self.trades.clone().into_iter().map(|x| x.points().0).sum();
        r.trunc_with_scale(2)
    }

    pub fn pnl(&self) -> Decimal {
        let r = Decimal::from_f32(0.01).unwrap();
        let result = self
            .trades
            .clone()
            .iter()
            .fold(self.capital, |acc, &x| match x.result {
                TradeResult::Winner => acc + acc * r * x.rr().0.trunc_with_scale(2),
                TradeResult::Expense => acc - acc * r,
                TradeResult::BreakEven => acc,
            });
        ((result - self.capital) / self.capital * Decimal::from(100)).trunc_with_scale(2)
    }
}

impl fmt::Debug for BacktestResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BacktestResult")
            // .field("trades", &self.trades)
            .field("number_of_trades", &self.number_of_trades())
            .field("winners", &self.result(TradeResult::Winner))
            .field("expenses", &self.result(TradeResult::Expense))
            .field("break_evens", &self.result(TradeResult::BreakEven))
            .field("profit_in_r", &self.profit_in_r())
            .field("points", &self.profit_in_points())
            .field("pnl_pct", &self.pnl())
            .finish()
    }
}
