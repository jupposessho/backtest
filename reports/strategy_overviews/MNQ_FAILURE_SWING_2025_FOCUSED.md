# MNQ Failure Swing 2025 Focused Sweep

- Symbol: MNQ
- Date filter: >= 2025-01-01 NY
- Strategy family: TTrades Fractal MTF with `failure_swing` reversal confirmation
- Costs: fixed fee $1.24 round-trip per 1 micro contract
- Slippage stress: 1 / 2 / 3 ticks per side

## Top Rows By Points/Week

| rank | timeframe | slippage | points/week | net_usd/week | trades | win_rate | pf_r | max_dd_usd | variant |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | 1m/15m | 1 | -39.17 | -138.02 | 2929 | 50.73 | 1.01 | 8563.13 | tf=1m/15m;trigger=continuation_break;entry=ob_mid;confirm=cisd_or_ifvg;rr=1;kz=named_kz;weekdays=tue_thu;poi=8;ob_tol=12;fs_lb=8;close_only=false;stop_bps=2;retest_bps=10;reclaim_bps=5000;htf_strict=false;htf_fvg=true;kz_hit=true;kz_lb=12;slip=1 |
| 2 | 1m/15m | 2 | -84.98 | -228.85 | 2891 | 49.71 | 0.95 | 14049.91 | tf=1m/15m;trigger=continuation_break;entry=ob_mid;confirm=cisd_or_ifvg;rr=1;kz=named_kz;weekdays=tue_thu;poi=8;ob_tol=12;fs_lb=8;close_only=false;stop_bps=2;retest_bps=10;reclaim_bps=5000;htf_strict=false;htf_fvg=true;kz_hit=true;kz_lb=12;slip=2 |
| 3 | 1m/15m | 3 | -128.69 | -315.44 | 2850 | 48.77 | 0.89 | 19309.05 | tf=1m/15m;trigger=continuation_break;entry=ob_mid;confirm=cisd_or_ifvg;rr=1;kz=named_kz;weekdays=tue_thu;poi=8;ob_tol=12;fs_lb=8;close_only=false;stop_bps=2;retest_bps=10;reclaim_bps=5000;htf_strict=false;htf_fvg=true;kz_hit=true;kz_lb=12;slip=3 |

## Robustness Ranking

| rank | min_points/week slip123 | min_net_usd/week slip123 | variant |
|---:|---:|---:|---|
| 1 | -128.69 | -315.44 | tf=1m/15m;trigger=continuation_break;entry=ob_mid;confirm=cisd_or_ifvg;rr=1;kz=named_kz;weekdays=tue_thu;poi=8;ob_tol=12;fs_lb=8;close_only=false;stop_bps=2;retest_bps=10;reclaim_bps=5000;htf_strict=false;htf_fvg=true;kz_hit=true;kz_lb=12 |
