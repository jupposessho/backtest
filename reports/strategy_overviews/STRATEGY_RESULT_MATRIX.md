# Strategy Result Matrix

Purpose: one-sheet matrix of tested strategies with best observed result, setup code, and verdict.

Notes:
- `best_result` is taken from each strategy's latest dedicated report.
- Units differ by runner (`%` equity model vs fixed-size `$` model); see `net_unit`.
- `setup_code` is a compact identifier for quick referencing.

| strategy | asset | timeframe/pair | net_result | net_unit | pf | win_rate_% | max_dd | setup_code | verdict |
|---|---|---|---:|---|---:|---:|---:|---|---|
| ttrades_fractal_mtf | SOL | 15m/4h | 20.89 | % | 1.25 | 38.30 | 16.42% | tt_mtf_sol_15m4h_cisd_rr2 | FULLY_TESTED |
| doji | MNQ | 15m | 12924.24 | $ | 1.97 | 29.03 | n/a | doji_mnq_15m_classic_mc_sl12_tp300 | FULLY_TESTED |
| ttrades_fractal | GOLD | 1m | -18838.46 | $ | 0.31 | 19.23 | n/a | tt_single_gold_1m_rr2_fvg_cisd | PARTIALLY_TESTED |
| ema_wick_reclaim_mnq | MNQ | 3m | -6000.49 | $ | n/a | 39.23 | n/a | mnq_ema_wick_rr2_wick8_atr0.5_cap0.10_all_atr_obmid | NOT_PROMOTABLE |
| mc (manipulation candle/engulf) | BTC | 5m | negative | % | <=0.94 | n/a | n/a | mc_engulf_filters_reality | HOLD / PARTIALLY_TESTED |
| mayne (fixed sizing sweep) | ETH | 4h/15m | 283.39 | $ (1 ETH fixed) | 10.02 | 80.00 | 31.43 USD | mayne_eth_4h15m_mss_sfp_htfswing_wick_rr075_ifvg6 | PARTIAL_RESULT (low trades=5) |
| ttrades_fractal_mtf (fixed sizing sweep) | ETH | 15m/4h | 196.56..197.88 | $ (1 ETH fixed) | 1.60 | 54.55 | n/a | tt_eth_15m4h_cisd_and_ifvg_strictwb_ny_rr15_close | PARTIAL_RESULT |
| mayne (fixed sizing sweep) | SOL | 1h/5m | 154.06 | $ (10 SOL fixed) | 17.28 | 80.00 | 9.46 USD | mayne_sol_1h5m_cisdbodyflip_ltfswing_htfswing_close_rr15_ifvg6 | PARTIAL_RESULT (low trades=5) |
| ttrades_fractal_mtf (fixed sizing sweep) | SOL | 15m/4h | 54.77..55.33 | $ (10 SOL fixed) | 1.72 | 64.29 | n/a | tt_sol_15m4h_cisd_only_strictwb_ny_rr15_close | PARTIAL_RESULT |

## Sources

- `reports/strategy_overviews/CHAMPION_BASELINE.md`
- `reports/strategy_overviews/STRATEGY_BEST_SETUP_SUMMARY.md`
- `reports/strategy_overviews/ETH_MTF_SWEEP_FIXED_1ETH.md`
- `reports/strategy_overviews/MAYNE_ETH_FIXED_1_SWEEP.md`
- `reports/strategy_overviews/SOL_TTRADES_FIXED_10_SWEEP.md`
- `reports/strategy_overviews/MAYNE_SOL_FIXED_10_SWEEP.md`
