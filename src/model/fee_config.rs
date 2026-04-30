use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

/// Configuration for trading fees/commissions
#[derive(Debug, Clone, Copy)]
pub struct FeeConfig {
    /// Maker fee rate (for limit order entries), as a percentage (e.g., 0.02 for 0.02%)
    pub maker_fee_pct: Decimal,
    /// Taker fee rate (for market order exits at TP/SL), as a percentage (e.g., 0.05 for 0.05%)
    pub taker_fee_pct: Decimal,
}

impl FeeConfig {
    /// Create a new fee configuration with specified rates
    ///
    /// # Arguments
    /// * `maker_fee_pct` - Maker fee as a percentage (e.g., 0.02 for 0.02%)
    /// * `taker_fee_pct` - Taker fee as a percentage (e.g., 0.05 for 0.05%)
    pub fn new(maker_fee_pct: f32, taker_fee_pct: f32) -> Self {
        Self {
            maker_fee_pct: Decimal::from_f32(maker_fee_pct).unwrap(),
            taker_fee_pct: Decimal::from_f32(taker_fee_pct).unwrap(),
        }
    }

    /// Standard Binance-like fee structure
    /// Maker: 0.02%, Taker: 0.06%
    pub fn binance_standard() -> Self {
        Self::new(0.02, 0.06)
    }

    /// Binance VIP 1 fee structure
    /// Maker: 0.016%, Taker: 0.04%
    pub fn binance_vip1() -> Self {
        Self::new(0.016, 0.04)
    }

    /// Binance VIP 2 fee structure
    /// Maker: 0.014%, Taker: 0.035%
    pub fn binance_vip2() -> Self {
        Self::new(0.014, 0.035)
    }

    /// Zero fees for testing ideal conditions
    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    /// Legacy fee structure (maker 0%, taker 0.005%)
    /// Used in older backtests before commission implementation
    pub fn legacy() -> Self {
        Self::new(0.0, 0.005)
    }

    /// Conservative high-fee estimate
    /// Maker: 0.05%, Taker: 0.1%
    pub fn conservative() -> Self {
        Self::new(0.05, 0.1)
    }

    /// Calculate maker fee for a given price
    pub fn maker_fee(&self, price: Decimal) -> Decimal {
        price * self.maker_fee_pct / Decimal::from(100)
    }

    /// Calculate taker fee for a given price
    pub fn taker_fee(&self, price: Decimal) -> Decimal {
        price * self.taker_fee_pct / Decimal::from(100)
    }
}

impl Default for FeeConfig {
    /// Default to Binance standard fees
    fn default() -> Self {
        Self::binance_standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculation() {
        let fees = FeeConfig::binance_standard();
        let price = Decimal::from(100);

        // 0.02% of 100 = 0.02
        assert_eq!(fees.maker_fee(price), Decimal::from_f32(0.02).unwrap());

        // 0.06% of 100 = 0.06
        assert_eq!(fees.taker_fee(price), Decimal::from_f32(0.06).unwrap());
    }

    #[test]
    fn test_zero_fees() {
        let fees = FeeConfig::zero();
        let price = Decimal::from(100);

        assert_eq!(fees.maker_fee(price), Decimal::ZERO);
        assert_eq!(fees.taker_fee(price), Decimal::ZERO);
    }

    #[test]
    fn test_vip_tiers() {
        let vip1 = FeeConfig::binance_vip1();
        assert_eq!(vip1.maker_fee_pct, Decimal::from_f32(0.016).unwrap());
        assert_eq!(vip1.taker_fee_pct, Decimal::from_f32(0.04).unwrap());

        let vip2 = FeeConfig::binance_vip2();
        assert_eq!(vip2.maker_fee_pct, Decimal::from_f32(0.014).unwrap());
        assert_eq!(vip2.taker_fee_pct, Decimal::from_f32(0.035).unwrap());
    }
}
