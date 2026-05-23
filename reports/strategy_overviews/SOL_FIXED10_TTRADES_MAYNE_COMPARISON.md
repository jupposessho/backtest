# SOL Fixed-10 Strategy Comparison

Run context:
- Asset: `SOLUSDT`
- Sizing: fixed `10` contracts
- Costs: per-strategy defaults used in runners

## Tested Variants Saved

- TTrades MTF full sweep report: `reports/strategy_overviews/SOL_TTRADES_FIXED_10_SWEEP.md`
  - Coverage: `864` rows (`2` timeframe pairs x reversal/confirmation/time/opportunity combos x slippage `1/2/3`)
- Mayne full sweep report: `reports/strategy_overviews/MAYNE_SOL_FIXED_10_SWEEP.md`
  - Coverage: `3200` rows (`2` timeframe pairs x pattern/sl/tp/trigger/rr/ifvg grids)

## Winner Setup (Current)

Winner by net USD across the two sweeps:

- Strategy: `Mayne`
- Pair: `SOL 1h/5m`
- Net: `+154.06` USD
- Max DD: `9.46` USD
- PF: `17.28`
- Trades: `5`
- Win rate: `80%`
- Config: `pat=CisdBodyFlip;sl=LtfRecentSwing;tp=OpposingHtfSwing;trig=Close;rr=1.5;ifvg_max=6`

Reference row:
- `reports/strategy_overviews/MAYNE_SOL_FIXED_10_SWEEP.md:5`

## Best TTrades Baseline (for comparison)

- Strategy: `ttrades_fractal_mtf`
- Pair: `15m/4h`
- Robust net (slip1/2/3): `54.77 / 55.05 / 55.33` USD
- Trades: `14`
- Win rate: `64.29%`
- PF: `1.72`
- Config: `cisd_only + strict_wick_break + ny_weekdays + more_hits_close_rr15`

Reference row:
- `reports/strategy_overviews/SOL_TTRADES_FIXED_10_SWEEP.md:5`

## Practical Read

- Mayne currently has higher top-end net but much lower activity (`5` trades).
- TTrades currently has lower net but higher activity (`14-23` in top rows).
