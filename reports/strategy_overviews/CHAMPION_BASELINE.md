# Champion Baseline

Source: `reports/strategy_overviews/STRATEGY_VALIDATION_MATRIX.md` (latest run).

Champion selection rule: best risk-adjusted score among `FULLY_TESTED` rows.
Risk-adjusted score = `(net_profit_% * profit_factor * win_rate_%) / max(max_dd_%,1)`.

| strategy | asset | timeframe | score | net_% | pf | win_rate_% | trades | max_dd_% | wf_test_net_% | wf_test_pf | setup |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| ttrades_fractal_mtf | SOL | 15m/4h | 60.91 | 20.89 | 1.25 | 38.30 | 141 | 16.42 | 28.66 | 2.30 | entry=ObMidpoint;confirm_mode=cisd_only;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
