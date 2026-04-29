# Strategy Results Summary

Starting capital: $1,000 | Risk: 1% per trade (compounding) | Asset: BTC/USDT (Binance) | No fees/slippage included

---

## Best Strategy per Timeframe

| TF | Strategy | Balance | Return | Trades | Win% | Max DD |
|----|----------|---------|--------|--------|------|--------|
| **5m** | cont_ema200_engulf_rr2_prevopen | $592,170 | 592x | 10,064 | 35.8% | 74.0% |
| **15m** | cont_ema200_engulf_rr2_prevopen | $19,312 | 19x | 3,653 | 36.4% | 29.4% |
| **30m** | cont_struct_engulf_rr2_prevopen | $5,827 | 6x | 3,594 | 35.3% | 37.6% |
| **1h** | cont_ema200_rr2_prevopen | $4,560 | 5x | 452 | 44.9% | 12.4% |
| **4h** | cont_struct_rr2_prevopen | $1,608 | 2x | 229 | 40.6% | 11.4% |
| **12h** | cont_struct_engulf_rr2_prevopen | $1,087 | 1x | 161 | 35.4% | 16.7% |

---

## Top 10 Strategies Across All Timeframes

| # | TF | Strategy | Balance | Return | Trades | Win% | Max DD |
|---|----|----------|---------|--------|--------|------|--------|
| 1 | 5m | cont_ema200_engulf_rr2_prevopen | $592,170 | 592x | 10,064 | 35.8% | 74.0% |
| 2 | 5m | cont_ema200_engulf_rr2_close | $64,973 | 65x | 13,076 | 34.7% | 53.8% |
| 3 | 5m | cont_ema200_rr1.5_prevopen | $40,723 | 41x | 5,326 | 43.1% | 45.4% |
| 4 | 5m | cont_struct_engulf_rr2_prevopen | $33,209 | 33x | 17,628 | 34.3% | 96.4% |
| 5 | 5m | cont_ema200_rr2_prevopen | $27,021 | 27x | 5,280 | 35.8% | 56.0% |
| 6 | 15m | cont_ema200_engulf_rr2_prevopen | $19,312 | 19x | 3,653 | 36.4% | 29.4% |
| 7 | 15m | cont_struct_engulf_rr2_prevopen | $17,648 | 18x | 6,463 | 35.2% | 43.7% |
| 8 | 5m | cont_struct_rr2_prevopen | $11,833 | 12x | 9,785 | 34.5% | 92.4% |
| 9 | 15m | cont_struct_rr2_prevopen | $6,934 | 7x | 3,345 | 35.6% | 38.2% |
| 10 | 15m | cont_ema200_rr1.5_prevopen | $6,796 | 7x | 1,854 | 44.4% | 30.5% |

---

## Worst Performers (Avoid)

| Strategy | Balance | Loss |
|----------|---------|------|
| cont_struct_engulf_rr2_close (15m) | $40 | -96% |
| cont_struct_rr2_close (15m) | $100 | -90% |
| cont_ema200_rr2_close (15m) | $397 | -60% |

Pattern: Close entry + Continuation = poor results

---

## Trailing Stops (15m Baseline Comparison)

| Strategy | Baseline PnL% | Best Trailing PnL% | Verdict |
|----------|---------------|---------------------|---------|
| rev_daily_rr2_close | +34.6% | -29.4% (BE1R) | Trailing hurts |
| cont_ema200_rr2_prevopen | +317.7% | all deeply negative | Trailing hurts |
| cont_ema200_engulf_rr2_prevopen | +1831.2% | all deeply negative | Trailing hurts |
| cont_struct_rr2_prevopen | +593.4% | all deeply negative | Trailing hurts |

**Verdict**: Trailing stops reduce PnL across all strategies on this dataset. MC/Engulfing signals need full room to hit TP. Trailing does create 15-25% break-even exits (capital protection), but the net effect is negative.

---

## Key Findings

1. **Lower TF = higher returns**: 5m avg $85,895 vs 15m avg $6,664 (13x gap), driven by trade frequency / compounding
2. **Engulfing > MC pattern**: Top 4 strategies all use Engulfing
3. **PrevOpen >> Close entry**: 9 of top 10 use PrevOpen; Close entry with Continuation strategies is consistently negative
4. **Continuation > Reversal**: All top 5 are continuation; Reversal strategies mostly break even (~$1,200-$1,400)
5. **No trailing stops**: Baseline outperforms all trailing variants for this dataset
6. **Win rate doesn't predict profit**: 35.8% win rate champion (592x) vs 44.4% (7x) — frequency + compounding matters more
7. **Drawdown scales with return**: 5m best strategy has 74% DD; 1h best has only 12% DD

---

## Risk Profile Recommendations

| Profile | TF | Strategy | Expected | Max DD |
|---------|-----|----------|----------|--------|
| Aggressive | 5m | cont_ema200_engulf_rr2_prevopen | 592x | ~74% |
| Balanced | 15m | cont_ema200_engulf_rr2_prevopen | 19x | ~29% |
| Conservative | 1h | cont_ema200_rr2_prevopen | 5x | ~12% |
| Capital preservation | 4h | cont_struct_rr2_prevopen | 2x | ~11% |

---

Sources: `TOP_STRATEGIES.md`, `MULTI_TIMEFRAME_ANALYSIS.md`, `RESULTS_SUMMARY.md`, `TRAILING_STOPS_SUMMARY.md`, `TRAILING_STOPS.md`, `README_TRAILING_STOPS.md`
