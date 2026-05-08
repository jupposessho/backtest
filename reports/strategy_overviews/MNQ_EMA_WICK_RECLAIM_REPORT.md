# MNQ EMA Wick Reclaim Report

Date: 2026-05-08
Runner: `cargo run --release --bin ema_wick_reclaim_mnq`
Dataset: `assets/mnq_1m_cont.parquet`
Primary analysis window: `2025-01-01+`
Contract: MNQ micro (1 contract, $2/point)
Costs: `$1.24` round-trip fees + `$1.00` round-trip slippage

## Important Realism Fixes Applied

The earlier report values were optimistic and are now obsolete. The runner was fixed in three key places:

- OB midpoint retest entry now executes at **next-bar open** after touch+confirm (removed optimistic midpoint fill).
- `max_hold_bars` exits now realize PnL at the timeout bar close (removed fake flat exits).
- Slippage stress no longer changes trade admission via `cost_r` filter (selection uses fixed `cost_filter_slippage_rt`).

All numbers below are post-fix.

## Baseline (Unfiltered)

- `1m`: trades `2604`, win `17.24%`, net `-$12,776.64`
- `3m`: trades `2160`, win `18.52%`, net `-$45,115.81`

Raw wick reclaim is not viable on MNQ after costs.

## Post-Fix Full Sweep Leaders (2025+)

### Best 1m by Net

- setup: `rr3 wick2 atr0.5 cap0.10 all hybrid obmid`
- trades: `1157`
- win rate: `29.30%`
- net: `+$7,384.77`
- avg/trade: `+$6.38`

### Best 3m by Net

- setup: `rr2 wick8 atr0.5 cap0.10 all atr obmid`
- trades: `910`
- win rate: `39.23%`
- net: `+$18,674.05`
- avg/trade: `+$20.52`

## Targeted Search Around Winners (2025+)

Trade-floor constrained search around high-performing neighborhoods:

- `Top 1m targeted`: `1m rr4 wick6 atr0.4 cap0.20 obw6 hold90 all`
  - trades: `1054`
  - win rate: `24.57%`
  - net: `+$5,039.68`

- `Top 3m targeted`: `3m ema200 rr4 wick6 atr0.4 cap0.15 obw8 hold90 all`
  - trades: `934`
  - win rate: `27.94%`
  - net: `+$16,107.48`

## Selective Bad-Trade Filter Pass

Filtering around the new best presets shows a typical trade-off:

- win rate can be nudged up,
- but aggressive filtering generally reduces net.

Examples from constrained filter pass:

- `1m ed1 bp25 rg5 ...` -> win `24.67%`, net `+$10,426.21` on an older base profile.
- `3m ed2 ...` -> win `27.63%`, net `+$7,255.10` on an older base profile.

These are useful quality levers, but post-fix best net currently comes from the simpler 3m leader above.

## Full-Dataset Check (Post-Fix, Legacy Preset References)

For previously used top presets (not current champions):

- `FULL DATASET TOP 1m net config`: `+$9,059.53` (`3967` trades, `19.99%` win)
- `FULL DATASET TOP 3m net config`: `+$1,648.38` (`1636` trades, `19.56%` win)

These are kept only as historical references after realism fixes, not as current recommended setups.

## Current Recommendation

- **Primary candidate**: `3m rr2 wick8 atr0.5 cap0.10 all atr obmid` (post-fix top net + high win rate).
- **Secondary candidate**: `1m rr3 wick2 atr0.5 cap0.10 all hybrid obmid`.

Status: `PARTIALLY_TESTED`.

Before implementation/deployment:
- run strict walk-forward/OOS on the new post-fix leaders,
- re-run monthly and slippage stress on these exact updated presets,
- validate live-fill assumptions against brokerage execution logs.

## Monthly Breakdown (Current Best Post-Fix 3m)

Preset: `3m rr2 wick8 atr0.5 cap0.10 all atr obmid`

- trades: `910`
- win rate: `39.23%`
- net: `+$18,674.05`
- monthly profile: `+8 / -7` (15 months)

Monthly PnL:

- 2025-01: `-$298.18`
- 2025-02: `$527.19`
- 2025-03: `$4,020.66`
- 2025-04: `$67.63`
- 2025-05: `-$313.70`
- 2025-06: `$1,018.90`
- 2025-07: `-$75.33`
- 2025-08: `-$189.35`
- 2025-09: `$9,317.53`
- 2025-10: `-$58.00`
- 2025-11: `-$157.41`
- 2025-12: `$4,102.21`
- 2026-01: `-$99.29`
- 2026-02: `$410.11`
- 2026-03: `$401.07`
