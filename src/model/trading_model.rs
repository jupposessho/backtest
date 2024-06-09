use super::backtest_result::BacktestResult;

pub trait TradingModel {
    fn execute(&self) -> BacktestResult;
}
