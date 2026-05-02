# MC Strategy Report

Date: 2026-05-02

## Scope

- Strategy family: `mc` (Manipulation Candle + Engulfing)
- Runner used: `cargo run --release --bin mc`
- Data: BTCUSDT `5m, 15m, 30m, 1h, 4h, 12h` (embedded assets)
- Reality settings (default):
  - `commission_rate_per_side = 0.001`
  - `slippage_ticks_per_side = 1`
  - `tick_size = 0.01`

## What Was Added

- Core-engine compliance already in place (`run_setups` path).
- Added optional naive toggle in runner (`MC_NAIVE=1`) for diagnostics.
- Added new filter variants in sweep runner:
  - `flt_engulf_ema200_rr2_close_fvg`
  - `flt_engulf_ema200_rr2_prevopen_fvg`
  - `flt_engulf_ema200_rr2_close_narrow`
  - `flt_engulf_ema200_rr2_close_quality`
  - `flt_engulf_ema200_rr2_prevopen_quality`
- Added signal-quality config in strategy:
  - `min_body_to_range`
  - `min_range_to_prev_range`
  - `min_range_to_avg_range`
  - `avg_range_lookback`

## Key Findings

### 1) Reality-mode result (fees + slippage)

- All tested MC/Engulfing variants remain unprofitable under reality settings.
- No variant reached robust profitability (`PF < 1` across meaningful cases).
- Added filters reduced trade counts but did not improve net edge after costs.

### 2) Naive diagnostic (`MC_NAIVE=1`)

- Several variants become positive in naive mode.
- Interpretation: some gross signal edge exists, but it is too thin and gets erased by realistic execution friction.

### 3) Filter experiments

- FVG filter and narrow time window did not produce a profitable reality-mode pocket.
- Signal-quality gates removed many trades, but also removed too many winners.

## Representative Results (Reality Mode)

5m examples:

- `cont_ema200_engulf_rr2_close`: `PF 0.94`, balance `-3,677,617.96`
- `flt_engulf_ema200_rr2_close_fvg`: `PF 0.94`, balance `-3,664,305.62`
- `flt_engulf_ema200_rr2_close_narrow`: `PF 0.87`, balance `-703,226.69`
- `flt_engulf_ema200_rr2_close_quality`: `PF 0.86`, balance `-194,092.84`

## Verdict

- Status: `PARTIALLY_TESTED`
- Promotion: `HOLD`
- Practical conclusion: not ready for champion track under current execution realism.

## Next Recommended Work

1. Add cost-aware pre-trade gates (minimum expected move/risk after friction).
2. Focus sweeps on higher-TF, lower-turnover pockets first.
3. Run break-even friction analysis (max slippage/commission the best naive variants can absorb).
