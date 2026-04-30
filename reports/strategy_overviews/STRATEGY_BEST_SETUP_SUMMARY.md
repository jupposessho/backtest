# Strategy Best Setup Summary

Source: `reports/strategy_overviews/STRATEGY_VALIDATION_MATRIX.md` (latest run).
One row per strategy: best setup selected by verdict, walk-forward PF, PF, trades, then net result.

| strategy | market | best_asset | best_timeframe | net_result | net_unit | profit_factor | trades | wf_test_net | wf_test_pf | best_setup | verdict |
|---|---|---|---|---:|---|---:|---:|---:|---:|---|---|
| ttrades_fractal | futures | GOLD | 1m | -18838.46 | $ | 0.31 | 754 | -5247.04 | 0.41 | rr=[1,2,3] planned; current=rr=2,use_fvg=true,lookback=20,require_cisd=true;slippage=[0, 1, 2] | PARTIALLY_TESTED |
| ttrades_fractal_mtf | crypto | SOL | 15m/4h | 20.89 | % | 1.25 | 141 | 28.66 | 2.30 | entry=ObMidpoint;confirm_mode=cisd_only;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] | FULLY_TESTED |
