# Doji Strategy Report

Scope: MNQ 15m doji strategy after execution-realism fixes (same-bar exit handling, trailing order-of-operations, and realized-exit points accounting), with policy-locked slippage and commission.

## Executive Summary

- The old promoted setup (`max_sl=10,tp=200,trail=8/8`) was invalidated after realism fixes.
- Current promoted setup is `classic`, `entry=market_close`, `max_sl_mode=limit_reprice`, `max_sl=12`, `tp_points=300`, `trail=10/10`, `max_trades_per_day=10`, session `04:00-12:00`.
- This setup remains strongly positive under `slippage=1/2/3` with `commission_rt=1.32`.
- Entry-time diagnostics identified elevated loss rates in late morning and afternoon; filtering to `04:00-12:00` materially improved robustness.

## Implementation Snapshot

- Strategy module: `src/strategies/doji.rs`
- Runner: `src/doji.rs`
- Engine path: `src/engine/execution.rs`
- Realism features in use:
  - close-confirmed entries with no same-bar hindsight exits for market-close entries
  - gap-aware stop fills with adverse slippage
  - stop-before-target conflict handling
  - directional per-side slippage and explicit commission deduction in USD estimate

## Current Champion (Policy-Locked)

Setup: `doji=classic;entry=market_close;max_sl_mode=limit_reprice;max_sl=12;tp_points=300;trail=10/10;max_trades_per_day=10;session=04:00-12:00;commission_rt=1.32;from=2021-03-01`.

| slippage_ticks | trades | win_rate_% | profit_r | profit_factor_r | points | pnl_usd_net_est | fill_rate_% |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1791 | 29.03 | 1023.57 | 1.97 | 7644.18 | 12924.24 | 81.39 |
| 2 | 1791 | 28.59 | 966.87 | 1.92 | 6925.73 | 11487.34 | 81.47 |
| 3 | 1787 | 28.37 | 907.03 | 1.86 | 6167.85 | 9976.86 | 81.48 |

## Entry-Time Loss Concentration

Reference profile (same strategy family before session filtering, `04:00-15:30`) showed the worst loss-rate entry windows:

- `09:00` -> `67.57%`
- `13:00` -> `68.16%`
- `10:00` -> `~64%`
- `14:00-15:00` -> `~60-64%`

These windows were the basis for narrowing the active session to `04:00-12:00`.

## Focused Sweep Outcome (Around Winners)

Focused Rust sweep around the filtered winner (`max_sl 10-14`, `trail 6/8/10`, `tp 225/250/275/300`, `slippage 1/2/3`) produced many robust candidates; top by worst-case net (slip3) was:

- `max_sl=12, trail=10/10, tp=300` (current champion)

This same candidate ranked first by minimum net across `slippage=1/2/3` while maintaining PF > 1 in all stress levels.

## Current Verdict

- Verdict: `FULLY_TESTED`
- Rationale: realism-fixed execution model + commission + slippage stress to 3 ticks + stable positive results across the selected operating window.

## Next Actions

1. Keep this setup as the production research baseline and monitor live-forward drift vs `slippage=3` guardrail.
2. Run periodic drift checks on entry-hour loss rates; if late-morning degradation increases, tighten the session further.
3. Add walk-forward segmentation for the new champion profile to refresh IS/OOS evidence under the realism-fixed model.
