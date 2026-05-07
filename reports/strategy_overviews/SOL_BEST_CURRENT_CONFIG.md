# SOL Best Current Config (Recent Window)

This report captures the best-performing SOL-only variant found in the recent-window tuning pass.

## Selected Config

- Strategy family: `ttrades_fractal_mtf`
- Market: `SOLUSDT`
- Timeframes: `15m / 4h`
- Entry variant: `Close`
- Reversal confirm mode: `IfvgOnly`
- RR target: `2.0`
- Killzone: `NyOnly`
- POI padding: `10 bps`
- OB sweep tolerance: `10 bps`
- Slippage ticks per side: `0`
- Tick size: `0.001`
- Position size used in this report: `10 SOL` per trade

## Why this config

From the focused recent-window sweep, this row had the highest 6-month net among tuned iFVG close variants while maintaining positive month consistency.

- Variant id: `close_ifvg_rr2_poi10_ob10_ny_only`
- 6-month net PnL: `+$55.14`
- Positive months: `4/5`
- Trades: `8`

## Monthly Net PnL (USD, 10 SOL)

| Month | Net USD |
|---|---:|
| 2025-11 | 22.62 |
| 2025-12 | 10.18 |
| 2026-01 | 18.55 |
| 2026-02 | -7.28 |
| 2026-03 | 11.07 |
| **Total** | **55.14** |

## Reverse-direction sanity check

The same setup forced to opposite direction was clearly worse.

- Reversed 5-month net: `-$87.16`
- Reversed positive months: `1/5`

## Repro commands

- Tune pass that identified this row:
  - `cargo run --release --bin sol_mtf_ifvg_tune`
- Original vs reversed validation for tuned winners:
  - `cargo run --release --bin sol_reverse_tuned_check`

## Caveats

- Sample is short and sparse (only `8` trades in the evaluated window).
- Results are sensitive to regime and data window; rerun periodically as new candles arrive.
- This is a SOL-only view; do not assume pooled cross-asset behavior from this row.
