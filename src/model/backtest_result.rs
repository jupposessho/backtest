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
        let r: Decimal = self.trades.clone().into_iter().map(|x| x.gross_r()).sum();
        r.trunc_with_scale(2)
    }

    pub fn costs_total(&self) -> Decimal {
        self.trades
            .iter()
            .map(|x| x.total_costs())
            .sum::<Decimal>()
            .trunc_with_scale(2)
    }

    pub fn profit_in_points(&self) -> Decimal {
        let r: Decimal = self.trades.clone().into_iter().map(|x| x.points().0).sum();
        r.trunc_with_scale(2)
    }

    pub fn pnl(&self) -> Decimal {
        let r = Decimal::from_f32(0.01).unwrap();
        let result = self.trades.clone().iter().fold(self.capital, |acc, &x| {
            let gross_change = acc * r * x.gross_r().trunc_with_scale(4);
            acc + gross_change - x.total_costs()
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
