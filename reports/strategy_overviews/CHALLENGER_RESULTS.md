# Challenger Results

Source: `reports/strategy_overviews/STRATEGY_VALIDATION_MATRIX.md` (latest run).

Promotion policy:
- gates: `trades>=40`, `net_%>=10`, `pf>=1.2`, `wf_test_pf>=1.05`, `wf_test_net_%>0`
- champion-compare: `net_% > champion_net_%` and `wf_test_pf >= champion_wf_test_pf`

Champion reference: `SOL 15m/4h` net `20.89%`, wf_test_pf `2.30`

| status | strategy | asset | timeframe | score | net_% | pf | trades | wf_test_net_% | wf_test_pf | setup |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 348.94 | 11.99 | 1.93 | 27 | -4.01 | 0.25 | entry=ObLevel;confirm_mode=ifvg_only;time_filter=london_ny_weekdays;opportunity=more_hits_ob_level_rr15;rr=1.5;poi_pad_bps=10;ob_tol_bps=8;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 343.16 | 5.61 | 2.29 | 9 | 0.00 | 0.00 | entry=ObMidpoint;confirm_mode=ifvg_only;time_filter=ny_weekdays;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 304.44 | 9.32 | 2.03 | 17 | 0.78 | 1.72 | entry=ObMidpoint;confirm_mode=ifvg_only;time_filter=london_ny_weekdays;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 139.33 | 6.01 | 1.44 | 25 | -2.68 | 0.51 | entry=ObMidpoint;confirm_mode=cisd_and_ifvg;time_filter=london_ny_weekdays;opportunity=more_hits_ob_mid_rr15;rr=1.5;poi_pad_bps=10;ob_tol_bps=8;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 131.89 | 7.68 | 1.42 | 33 | -3.73 | 0.42 | entry=ObMidpoint;confirm_mode=ifvg_only;time_filter=london_ny_weekdays;opportunity=more_hits_ob_mid_rr15;rr=1.5;poi_pad_bps=10;ob_tol_bps=8;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 128.93 | 4.85 | 1.48 | 19 | -2.96 | 0.32 | entry=ObLevel;confirm_mode=cisd_and_ifvg;time_filter=london_ny_weekdays;opportunity=more_hits_ob_level_rr15;rr=1.5;poi_pad_bps=10;ob_tol_bps=8;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 117.25 | 2.68 | 1.82 | 6 | 0.00 | 0.00 | entry=ObMidpoint;confirm_mode=cisd_and_ifvg;time_filter=ny_weekdays;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 15m/4h | 109.14 | 7.08 | 1.48 | 24 | -3.37 | 0.36 | entry=ObMidpoint;confirm_mode=cisd_only;time_filter=ny_weekdays;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 15m/4h | 109.14 | 7.08 | 1.48 | 24 | -3.37 | 0.36 | entry=ObMidpoint;confirm_mode=cisd_or_ifvg;time_filter=ny_weekdays;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 103.00 | 17.21 | 1.19 | 148 | -0.60 | 0.96 | entry=ObMidpoint;confirm_mode=ifvg_only;time_filter=all_day_all_week;opportunity=more_hits_ob_mid_rr15;rr=1.5;poi_pad_bps=10;ob_tol_bps=8;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 80.89 | 12.77 | 1.40 | 48 | 4.32 | 2.29 | entry=ObMidpoint;confirm_mode=cisd_and_ifvg;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 15m/4h | 61.92 | 1.61 | 1.70 | 4 | 0.00 | 0.00 | entry=ObMidpoint;confirm_mode=ifvg_only;time_filter=london_ny_weekdays;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 58.75 | 4.23 | 1.55 | 13 | 0.78 | 1.72 | entry=ObMidpoint;confirm_mode=cisd_and_ifvg;time_filter=london_ny_weekdays;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | SOL | 15m/4h | 47.03 | 18.47 | 1.22 | 143 | 28.66 | 2.30 | entry=ObMidpoint;confirm_mode=cisd_or_ifvg;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
| HOLD | ttrades_fractal_mtf | ETH | 5m/1h | 41.83 | 7.09 | 1.12 | 97 | -1.27 | 0.89 | entry=ObMidpoint;confirm_mode=cisd_and_ifvg;time_filter=all_day_all_week;opportunity=more_hits_ob_mid_rr15;rr=1.5;poi_pad_bps=10;ob_tol_bps=8;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |
