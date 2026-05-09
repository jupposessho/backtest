# MNQ EMA Wick Reclaim Report

Date: 2026-05-09  
Runner: `cargo run --release --bin ema_wick_reclaim_mnq`  
Dataset: `assets/mnq_1m_cont.parquet`  
Primary window: `2025-01-01+`  
Contract model: MNQ micro (`$2/point`)  
Costs: `$1.24` round-trip fees + `$1.00` round-trip slippage

## Final Realism Status

This strategy family is **not promotable** after conservative execution revalidation.

Conservative updates used in the final run:

- same-bar SL/TP conflict resolved with SL-first behavior,
- gap-through stop handling (adverse open fill beyond stop),
- unchanged causal entry logic (retest + confirm, then next-bar entry behavior in OB modes).

## Current Best Legacy Preset (Revalidated)

Preset: `3m rr2 wick8 atr0.5 cap0.10 all atr obmid`

- trades: `910`
- win rate: `39.23%`
- net: `-$6,000.49`
- avg/trade: `-$6.59`
- monthly profile: `+4 / -11` (15 months)

This preset was previously positive under less conservative stop execution. Under conservative fills it is negative.

## Expanded Recovery Attempts (Realistic Engine)

Additional sweeps were run around the same family with realistic fills (EMA anchors, session variants, asymmetric wick thresholds, retest mode variants, ATR period variants, hold variants).

Result:

- **0 passers** for joint constraint `net_2025 > 0` and `net_2026 > 0` with minimum activity thresholds.

## Runtime-Optimized Re-Run Confirmation

The full runner was re-executed after parallel runtime optimization (`Arc` shared datasets + Rayon sweeps).

- full sweep completed end-to-end (no timeout),
- `CURRENT BEST 3m (post-fix)` remained unchanged at `-$6,000.49` (`910` trades, `39.23%` win),
- some localized positive pockets appeared in intermediate slices, but they did not clear full realism/promotion constraints.

## Verdict

- Classification: `NOT_PROMOTABLE`
- Action: keep as archived research reference only.
- Recommendation: do not deploy this strategy family without a materially new hypothesis layer.
