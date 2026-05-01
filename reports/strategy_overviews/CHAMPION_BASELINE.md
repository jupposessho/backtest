# Champion Baseline

Source: `reports/strategy_overviews/STRATEGY_VALIDATION_MATRIX.md` (latest run).

Champion selection rule: best risk-adjusted score among `FULLY_TESTED` rows.
Risk-adjusted score = `(net_profit_% * profit_factor * win_rate_%) / max(max_dd_%,1)`.

Note: this score is `%`-based and is directly comparable for crypto rows; futures `$` rows are tracked separately as cash champions.

| strategy | asset | timeframe | score | net_% | pf | win_rate_% | trades | max_dd_% | wf_test_net_% | wf_test_pf | setup |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| ttrades_fractal_mtf | SOL | 15m/4h | 60.91 | 20.89 | 1.25 | 38.30 | 141 | 16.42 | 28.66 | 2.30 | entry=ObMidpoint;confirm_mode=cisd_only;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] |

## Futures Cash Champion (Supplemental)

| strategy | asset | timeframe | net_$ | pf | win_rate_% | trades | wf_test_net_$ | wf_test_pf | setup |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| doji | MNQ | 15m | 12924.24 (slip1), 11487.34 (slip2), 9976.86 (slip3) | 1.97 (slip1), 1.92 (slip2), 1.86 (slip3) | 29.03 (slip1), 28.59 (slip2), 28.37 (slip3) | 1791 (slip1), 1791 (slip2), 1787 (slip3) | n/a | n/a | doji=classic;entry=market_close;max_sl_mode=limit_reprice;max_sl=12;tp_points=300;trail=10/10;max_trades_per_day=10;session=04:00-12:00;commission_rt=1.32;slippage=[1,2,3] |
