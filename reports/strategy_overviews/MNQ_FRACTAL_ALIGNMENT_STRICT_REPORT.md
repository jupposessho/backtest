# MNQ Fractal Alignment Strict Report

- Strategy: `mnq_fractal_alignment_sweep`
- Market: MNQ futures (`assets/mnq_1m_cont.parquet`)
- Primary model: 1m execution, 3m + 27m context (PO3-inspired)
- Realism: next-bar-open entries, fixed commission, slippage 1/2/3 ticks per side, gap-through stop handling

## What Was Implemented

- Added strict BAG controls (`real_bag_only`, minimum BAG gap ticks, multi-bar BAG scan)
- Added entry variants (`breakout`, `rmb_retest`, `breakout_or_rmb`)
- Added literal RMB zone construction from CSD/swept candles
- Added inversion quality filters (minimum body ticks and close-position threshold)
- Added timing + regime gates (bars-after-inversion limit, optional anchor expansion)
- Added stop buffer sweep in ticks

## Strict Redesign Outcome

The strict redesign improved setup frequency from very sparse single-digit counts to around 30-40 trades in some strict-lite combinations, but those higher-frequency rows were net negative.

Best strict-only rows with positive net remained low sample size (roughly 6-12 trades) and are not promotable under realism standards.

## Trade-Count Gate Validation

Runs with strict mode and output filtering by minimum trades:

- `--min-trades 30`: no promotable strict row with stable profitability
- `--min-trades 50`: no rows matched
- `--min-trades 100`: no rows matched

## Current Verdict

- Strict BAG mode (`real_bag_only`) remains too sparse for deployment-grade confidence on current definition.
- Relaxing strict gates can increase trade count, but currently degrades edge quality.
- Keep strict BAG as a research branch; do not promote as production candidate yet.

## Next Direction

- If strict-only must be preserved, redesign setup semantics further (alternative fractal completion logic and/or BAG origin logic), then rerun with hard gates (`trades >= 100`, positive net at 2-tick slippage).
- If deployment speed is the priority, use fallback-BAG mode as the active candidate and keep strict mode for continued R&D.
