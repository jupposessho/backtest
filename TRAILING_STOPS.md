# Trailing Stop Loss Implementation

## Overview

This document describes the trailing stop loss variants implemented for the MC (Manipulation Candle) trading strategy. Trailing stops progressively move the stop loss in favor of the trade as profit increases, locking in gains and reducing risk.

## Trailing Stop Modes

### 1. **None** (Baseline)
- No trailing stop logic
- Stop loss remains at initial level until 1R is reached
- At 1R, stop moves to break-even (entry price)
- Trade exits at either SL or TP (2R or 1.5R depending on config)

### 2. **BreakEven1R**
- **Rule**: Set SL to break even at 1R
- **Behavior**: Once price moves 1R in favor, immediately move SL to entry price
- **Use case**: Most conservative approach, eliminates risk as soon as 1R is reached

### 3. **Trail05RAt15R**
- **Rules**:
  - Set SL to break even at 1R
  - Set SL to 0.5R at 1.5R
- **Behavior**: 
  - At 1R: move SL to entry (0R profit protected)
  - At 1.5R: move SL to entry + 0.5R (locks in 0.5R profit)
- **Use case**: Moderate protection, secures half R once trade shows strong momentum

### 4. **Trail1RAt2R**
- **Rules**:
  - Set SL to break even at 1R
  - Set SL to 1R at 2R
- **Behavior**:
  - At 1R: move SL to entry (0R profit protected)
  - At 2R: move SL to entry + 1R (locks in 1R profit)
- **Use case**: Allows trade to breathe until full TP, then locks in 1R if reversal occurs

### 5. **Progressive** (Recommended)
- **Rules**:
  - Set SL to break even at 1R
  - Set SL to 0.5R at 1.5R
  - Set SL to 1R at 2R
  - Set SL to 1.5R at 2.5R
  - And so on (continues in 0.5R increments)
- **Behavior**: Dynamic trailing that moves SL forward by 0.5R for every 0.5R gained beyond 1R
- **Formula**: `SL = max(0, floor((attained_r - 1.0) / 0.5) * 0.5) R`
- **Use case**: Best balance between giving trades room and protecting profits

## Implementation Details

### Code Structure

The trailing stop logic is implemented in `backtest/src/strategies/mc.rs`:

```rust
pub enum TrailingStopMode {
    None,
    StepHalfR,              // Legacy half-R stepping
    BreakEven1R,            // BE at 1R only
    Trail05RAt15R,          // BE at 1R, 0.5R at 1.5R
    Trail1RAt2R,            // BE at 1R, 1R at 2R
    Progressive,            // Full progressive trailing
}
```

### Active Position Tracking

Each active position maintains:
- `position`: Core trade data (entry, direction, TP, etc.)
- `current_sl`: Dynamic stop loss that gets updated by trailing logic
- `initial_risk`: Entry - initial SL (used to calculate R multiples)

### Trailing Logic Flow

On every candle while a position is active:

1. Calculate `attained_r` = (High/Low - Entry) / initial_risk
2. Based on trailing mode, determine target SL in R multiples
3. Update `current_sl` if new SL is better (higher for longs, lower for shorts)
4. Check if price hits `current_sl` or TP
5. Exit accordingly

## Backtest Results Summary

### Key Observations

#### 1. **Break-Even Trades Increase**
Trailing stops create more break-even exits as trades get stopped out at entry after reaching 1R but before hitting TP.

Example (rev_daily_rr2_close):
- **No Trailing**: 0 break-evens, 36% win rate
- **BreakEven1R**: 124 break-evens (29% of trades), 21% win rate
- **Progressive**: 77 break-evens (18% of trades), 32% win rate

#### 2. **Win Rate Appears Lower**
Trailing stops reduce "technical" win rate because break-evens are not counted as winners, even though they protect capital.

#### 3. **Risk-Adjusted Performance**
Trailing stops reduce maximum drawdown in some cases by cutting losses faster, but may also reduce overall PnL if trades reverse after hitting intermediate targets.

### Example Comparison: Reversal Daily RR2 Close

| Mode | Trades | Win Rate | Winners | Losers | B/E | Max DD | PnL% |
|------|--------|----------|---------|--------|-----|--------|------|
| None | 416 | 36.05% | 150 | 266 | 0 | 19.19% | +34.58% |
| BreakEven1R | 426 | 21.12% | 90 | 212 | 124 | 29.42% | -29.42% |
| Trail05RAt15R | 427 | 29.03% | 124 | 213 | 90 | 46.91% | -46.91% |
| Trail1RAt2R | 426 | 24.41% | 104 | 212 | 110 | 40.80% | -40.80% |
| Progressive | 428 | 32.24% | 138 | 213 | 77 | 37.07% | -36.55% |

**Insight**: For this particular strategy on this dataset, trailing stops reduced performance. This suggests the original MC signals benefit from giving trades full room to hit 2R targets.

### Example Comparison: Continuation EMA200 RR2 PrevOpen

| Mode | Trades | Win Rate | Winners | Losers | B/E | Max DD | PnL% |
|------|--------|----------|---------|--------|-----|--------|------|
| None | 1836 | 36.27% | 666 | 1170 | 0 | 45.47% | +317.72% |
| (Trailing variants show similar reduction in PnL with increased break-evens) |

## Usage

To use trailing stops in your backtest:

```rust
use backtest::strategies::mc::{TrailingStopConfig, TrailingStopMode};

let config = McConfig {
    // ... other config
    trailing_stop: TrailingStopConfig {
        mode: TrailingStopMode::Progressive,
    },
    // ...
};
```

## Recommendations

### When to Use Trailing Stops

1. **High volatility markets**: When reversals are common after 1R-2R moves
2. **Lower confidence setups**: When you want to secure profits quickly
3. **Risk management**: When drawdown reduction is more important than max profit
4. **Partial exits**: Combined with position sizing (not yet implemented)

### When NOT to Use Trailing Stops

1. **Strong directional trends**: When price typically runs to full TP
2. **High win rate strategies**: When the edge comes from letting winners run
3. **Low R:R setups**: When you need full TP to maintain profitability

### Best Practices

1. **Test both approaches**: Always compare trailing vs non-trailing on your specific data
2. **Consider market regime**: Different modes may work better in different market conditions
3. **Monitor break-even rate**: High b/e rate (>20%) may indicate trailing is too aggressive
4. **Use with position sizing**: Trailing works best when combined with partial exits (future feature)

## Future Enhancements

### Potential Improvements

1. **Time-based trailing**: Only trail after trade has been open for X candles
2. **ATR-based trailing**: Adjust trail distance based on volatility
3. **Partial exits**: Close 50% at 1R, trail remaining 50%
4. **Asymmetric trailing**: Different rules for longs vs shorts
5. **Session-aware trailing**: Tighter trailing during volatile sessions
6. **Parabolic SAR integration**: Use SAR as dynamic trailing stop

### Configuration Ideas

```rust
pub struct AdvancedTrailingConfig {
    pub mode: TrailingStopMode,
    pub min_time_candles: usize,      // Don't trail until X candles open
    pub atr_multiplier: Option<f64>,   // Trail at ATR * multiplier
    pub partial_exit_at_1r: bool,      // Close 50% at 1R
}
```

## Conclusion

Trailing stops are a double-edged sword:
- **Pros**: Reduce risk, lock in profits, protect capital
- **Cons**: May stop out winners early, reduce average R

The best trailing strategy depends on:
1. Your strategy's natural behavior (how far do winners typically run?)
2. Market conditions (trending vs ranging)
3. Risk tolerance (capital preservation vs max profit)
4. Trading style (aggressive vs conservative)

**Bottom line**: Always backtest both trailing and non-trailing variants to see which performs better on YOUR specific setup and data.