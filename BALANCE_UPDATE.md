# Balance Display Update - Summary

## What Changed

The output now shows **final balance** instead of percentage PnL, making results easier to understand at a glance.

### Before (PnL%)
```
case                          trades   win_rate   ...   pnl%
cont_ema200_engulf_rr2_prevopen    3653      36.38%   ...   +1831.17%
```

### After (Final Balance)
```
case                          trades   win_rate   ...   balance
cont_ema200_engulf_rr2_prevopen    3653      36.38%   ...   19311.70
```

---

## How It Works

**Starting Balance**: $1,000  
**Risk Model**: 1% of current balance per trade  
**Compounding**: Yes (risk grows as balance grows)

### Example Calculation

```
Trade 1:  Balance = $1,000  →  Risk = $10  (1%)
Trade 50: Balance = $1,500  →  Risk = $15  (1%)
Trade 100: Balance = $2,300  →  Risk = $23  (1%)
...
Final:    Balance = $19,311.70
```

---

## Top 5 Strategies (by Final Balance)

| Rank | Strategy | Final Balance | From $1,000 |
|------|----------|---------------|-------------|
| 🥇 1 | cont_ema200_engulf_rr2_prevopen | **$19,311.70** | +$18,311.70 |
| 🥈 2 | cont_struct_engulf_rr2_prevopen | **$17,647.80** | +$16,647.80 |
| 🥉 3 | cont_struct_rr2_prevopen | **$6,934.47** | +$5,934.47 |
| 4 | cont_ema200_rr1.5_prevopen | **$6,796.24** | +$5,796.24 |
| 5 | cont_ema200_rr2_prevopen | **$4,177.26** | +$3,177.26 |

---

## Reading the Results

### Profitable Strategies (Balance > $1,000)
These strategies made money:
- cont_ema200_engulf_rr2_prevopen: $19,311.70 (19x return!)
- cont_struct_engulf_rr2_prevopen: $17,647.80 (17.6x return!)
- cont_struct_rr2_prevopen: $6,934.47 (6.9x return)
- And more...

### Break-Even Strategies (Balance ≈ $1,000)
These strategies roughly broke even:
- rev_daily_rr1.5_prevopen: $1,032.83

### Losing Strategies (Balance < $1,000)
These strategies lost money:
- cont_ema200_rr2_close: $397.23 (lost 60%)
- cont_struct_rr2_close: $99.84 (lost 90%)
- Many trailing stop variants

---

## Why Balance Is Better Than Percentage

### Easier to Understand
- **Balance**: "$19,311.70" → immediately know your account value
- **Percentage**: "+1831%" → need to calculate: $1,000 × 19.31 = $19,311

### More Intuitive
- "I'd have $19,311" is clearer than "I'd make 1831%"
- Easy to compare: $19,311 vs $17,647 vs $6,934

### Real-World Applicable
- Shows actual dollar amount you'd have
- Easy to scale: 10x capital = 10x balance shown
- Direct answer to "how much would I make?"

---

## Implementation Details

### Code Changes
**File**: `backtest/src/mc.rs`

**Changed**:
1. `Stats` struct: `pnl_pct` → `final_balance`
2. `equity_metrics()`: Returns final capital instead of percentage
3. Starting capital: Now hardcoded to $1,000 (was configurable)
4. Output format: Shows balance with 2 decimal places

**Before**:
```rust
let (pnl_pct, max_dd, ...) = equity_metrics(trades, result.capital);
// pnl_pct = (capital - start) / start * 100

Stats { pnl_pct, ... }
```

**After**:
```rust
let (final_balance, max_dd, ...) = equity_metrics(trades, Decimal::from(1000));
// final_balance = capital (after all trades)

Stats { final_balance, ... }
```

### Output Format
```rust
println!(
    "{:<28} ... {:>12.2}",
    stats.label, stats.final_balance
);
```

Result: Right-aligned, 2 decimal places, 12 characters wide

---

## How to Run

```bash
cd backtest
cargo run --release --bin mc
```

Output shows:
- Baseline strategies (no trailing)
- Trailing stop variants
- Legend and balance calculation notes

---

## Important Notes

### This Is Backtest Data
- ✅ Shows what WOULD have happened historically
- ❌ NOT a guarantee of future performance
- ⚠️ Past performance ≠ future results

### Not Included
- Transaction fees
- Slippage
- Spread costs
- Execution delays
- Market impact

Real trading results will be lower!

### Risk Management
The 1% risk model means:
- At $1,000: risk $10/trade (0.5 BTC at $20k)
- At $10,000: risk $100/trade (5 BTC at $20k)
- At $19,311: risk $193/trade (9.6 BTC at $20k)

Position sizes grow as balance grows (compounding).

---

## Quick Reference

### Starting Capital
**$1,000** (hardcoded in `equity_metrics()`)

### Risk Per Trade
**1%** of current balance (compounding)

### Best Strategy
**cont_ema200_engulf_rr2_prevopen**
- Final Balance: $19,311.70
- Return: 19.3x (1,831% gain)
- Trades: 3,653
- Win Rate: 36.38%

### Worst Strategy
**cont_struct_engulf_rr2_close**
- Final Balance: $39.91
- Loss: -96%
- Don't use this!

---

## Comparison: Before vs After

### Example Strategy: cont_ema200_engulf_rr2_prevopen

**Before (PnL%)**:
```
pnl%: +1831.17%
```
→ Need to calculate: $1,000 × (1 + 18.3117) = $19,311.70

**After (Balance)**:
```
balance: 19311.70
```
→ Immediately see: $19,311.70 final value

**Improvement**: Instant clarity, no mental math needed!

---

## Summary

✅ **Changed**: pnl% → final balance  
✅ **Starting Capital**: $1,000  
✅ **Risk Model**: 1% per trade  
✅ **Top Strategy**: $19,311.70 (cont_ema200_engulf_rr2_prevopen)  
✅ **Benefit**: Easier to understand results at a glance  

---

**Status**: ✅ Complete  
**Date**: 2024  
**Implementation**: `backtest/src/mc.rs`  
**Testing**: All 30+ variants working correctly