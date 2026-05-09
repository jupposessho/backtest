# Strategy Best Setup Summary

Source: `reports/strategy_overviews/STRATEGY_VALIDATION_MATRIX.md` (latest run).
One row per strategy: best setup selected by verdict, walk-forward PF, PF, trades, then net result.

| strategy | market | best_asset | best_timeframe | net_result | net_unit | profit_factor | trades | wf_test_net | wf_test_pf | best_setup | verdict |
|---|---|---|---|---:|---|---:|---:|---:|---:|---|---|
| ttrades_fractal | futures | GOLD | 1m | -18838.46 | $ | 0.31 | 754 | -5247.04 | 0.41 | rr=[1,2,3] planned; current=rr=2,use_fvg=true,lookback=20,require_cisd=true;slippage=[0, 1, 2] | PARTIALLY_TESTED |
| ttrades_fractal_mtf | crypto | SOL | 15m/4h | 20.89 | % | 1.25 | 141 | 28.66 | 2.30 | entry=ObMidpoint;confirm_mode=cisd_only;time_filter=all_day_all_week;opportunity=baseline;rr=2;poi_pad_bps=0;ob_tol_bps=0;htf_bias+poi+cisd/ifvg+ob;slippage=[0, 1, 2] | FULLY_TESTED |
| doji | futures | MNQ | 15m | 31600.28 | $ | 12.48 (slip1), 10.89 (slip2) | 1205 | 18236.54 (slip1), 17555.98 (slip2) | 24.58 (slip1), 20.23 (slip2) | doji=classic;entry=market_close (risk-capped via max_sl=10);tp_points=200;trail=8/8;max_trades_per_day=10;slippage=[1, 2];commission_rt=1.32;wf_split=IS(2021-03-01..2023-12-31)/OOS(2024-01-01+) | FULLY_TESTED |
| ob_engulfing_mnq_overlap | futures | MNQ | 15m (05:00-09:15 NY) | 7206.50 gross points->USD (2025+) | $ | 1.47 gross (1 tick run net +6.36% / 2+ ticks negative) | 191 | mixed (3 positive chunks, 1 negative chunk) | n/a | entry=prev_open;overlap>=5%;rr=3.5;max_risk=25;fill=8;max_setups/day=2;trailing=none;slippage=1/2/3 tested | PARTIALLY_TESTED |
| ema_wick_reclaim_mnq | futures | MNQ | 3m (EMA200 5m anchor) | -6000.49 (2025+, conservative stop fill realism) | $ | n/a (custom $/trade model) | 910 | n/a | n/a | rr=2;wick>=8 ticks;atr_floor=0.5 ATR14;cost_cap=0.10R;session=all;stop=atr;entry=ob_mid_retest;commission_rt=1.24;slippage_rt=1.00;realism now uses SL-first + gap-through stop fill; no stable 2025/2026-positive config under realistic constraints | NOT_PROMOTABLE |

Note: ORB London Reversal research is marked `NOT_RECOMMENDED`; see `reports/strategy_overviews/ORB_LONDON_REVERSAL_NOT_RECOMMENDED.md`.
