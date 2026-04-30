# Promotion Shortlist

Source: `reports/strategy_overviews/STRATEGY_VALIDATION_MATRIX.md` (FULL mode)

Thresholds:
- strategy: `ttrades_fractal_mtf`
- trades >= 40
- net_profit_% >= 10
- profit_factor >= 1.2
- wf_test_pf >= 1.05

| rank | asset | timeframe | net_% | pf | trades | wf_test_net_% | wf_test_pf | setup |
|---:|---|---|---:|---:|---:|---:|---:|---|
| 1 | SOL | 15m/4h | 20.89 | 1.25 | 141 | 28.66 | 2.30 | entry=ObMidpoint;confirm_mode=cisd_only;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| 2 | SOL | 15m/4h | 18.47 | 1.22 | 143 | 28.66 | 2.30 | entry=ObMidpoint;confirm_mode=cisd_or_ifvg;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| 3 | ETH | 5m/1h | 12.77 | 1.40 | 48 | 4.32 | 2.29 | entry=ObMidpoint;confirm_mode=cisd_and_ifvg;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
