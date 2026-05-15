# MNQ Failure Swing 2025 Sweep

- Symbol: MNQ
- Date filter: >= 2025-01-01 NY
- Strategy family: TTrades Fractal MTF with `failure_swing` reversal confirmation
- Costs: fixed fee $1.24 round-trip per 1 micro contract
- Slippage stress: 1 / 2 / 3 ticks per side

## Top Rows By Points/Week

| rank | timeframe | slippage | points/week | net_usd/week | trades | win_rate | pf_r | max_dd_usd | variant |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | 1m/15m | 1 | 55.83 | 68.60 | 2113 | 56.60 | 1.57 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0;slip=1 |
| 2 | 1m/15m | 2 | 55.83 | 68.60 | 2113 | 56.60 | 1.57 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0;slip=2 |
| 3 | 1m/15m | 3 | 55.83 | 68.60 | 2113 | 56.60 | 1.57 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0;slip=3 |
| 4 | 1m/15m | 1 | 55.43 | 67.74 | 2116 | 56.47 | 1.56 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0;slip=1 |
| 5 | 1m/15m | 2 | 55.43 | 67.74 | 2116 | 56.47 | 1.56 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0;slip=2 |
| 6 | 1m/15m | 3 | 55.43 | 67.74 | 2116 | 56.47 | 1.56 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0;slip=3 |
| 7 | 1m/15m | 1 | 55.36 | 67.57 | 2118 | 56.42 | 1.55 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=false;stop_bps=0;slip=1 |
| 8 | 1m/15m | 2 | 55.36 | 67.57 | 2118 | 56.42 | 1.55 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=false;stop_bps=0;slip=2 |
| 9 | 1m/15m | 3 | 55.36 | 67.57 | 2118 | 56.42 | 1.55 | 257.80 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=false;stop_bps=0;slip=3 |
| 10 | 1m/15m | 1 | 52.03 | 62.62 | 2034 | 56.15 | 1.54 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=true;stop_bps=0;slip=1 |
| 11 | 1m/15m | 2 | 52.03 | 62.62 | 2034 | 56.15 | 1.54 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=true;stop_bps=0;slip=2 |
| 12 | 1m/15m | 3 | 52.03 | 62.62 | 2034 | 56.15 | 1.54 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=true;stop_bps=0;slip=3 |
| 13 | 1m/15m | 1 | 51.95 | 62.41 | 2037 | 56.11 | 1.53 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=true;stop_bps=0;slip=1 |
| 14 | 1m/15m | 2 | 51.95 | 62.41 | 2037 | 56.11 | 1.53 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=true;stop_bps=0;slip=2 |
| 15 | 1m/15m | 3 | 51.95 | 62.41 | 2037 | 56.11 | 1.53 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=true;stop_bps=0;slip=3 |
| 16 | 1m/15m | 1 | 51.93 | 62.34 | 2038 | 56.08 | 1.53 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=true;stop_bps=0;slip=1 |
| 17 | 1m/15m | 2 | 51.93 | 62.34 | 2038 | 56.08 | 1.53 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=true;stop_bps=0;slip=2 |
| 18 | 1m/15m | 3 | 51.93 | 62.34 | 2038 | 56.08 | 1.53 | 260.32 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=true;stop_bps=0;slip=3 |
| 19 | 1m/15m | 1 | 35.26 | 54.14 | 804 | 59.95 | 1.80 | 160.88 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0;slip=1 |
| 20 | 1m/15m | 2 | 35.26 | 54.14 | 804 | 59.95 | 1.80 | 160.88 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0;slip=2 |
| 21 | 1m/15m | 3 | 35.26 | 54.14 | 804 | 59.95 | 1.80 | 160.88 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0;slip=3 |
| 22 | 1m/15m | 1 | 35.01 | 53.58 | 807 | 59.73 | 1.78 | 160.88 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0;slip=1 |
| 23 | 1m/15m | 2 | 35.01 | 53.58 | 807 | 59.73 | 1.78 | 160.88 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0;slip=2 |
| 24 | 1m/15m | 3 | 35.01 | 53.58 | 807 | 59.73 | 1.78 | 160.88 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0;slip=3 |
| 25 | 1m/15m | 1 | 34.95 | 53.41 | 809 | 59.58 | 1.77 | 160.88 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=36;close_only=false;stop_bps=0;slip=1 |

## Robustness Ranking

| rank | min_points/week slip123 | min_net_usd/week slip123 | variant |
|---:|---:|---:|---|
| 1 | 55.83 | 68.60 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0 |
| 2 | 55.43 | 67.74 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0 |
| 3 | 55.36 | 67.57 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=false;stop_bps=0 |
| 4 | 52.03 | 62.62 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=true;stop_bps=0 |
| 5 | 51.95 | 62.41 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=true;stop_bps=0 |
| 6 | 51.93 | 62.34 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=true;stop_bps=0 |
| 7 | 35.26 | 54.14 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0 |
| 8 | 35.01 | 53.58 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0 |
| 9 | 34.95 | 53.41 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=36;close_only=false;stop_bps=0 |
| 10 | 32.26 | 48.89 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=12;close_only=true;stop_bps=0 |
| 11 | 32.06 | 48.46 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=24;close_only=true;stop_bps=0 |
| 12 | 32.04 | 48.39 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=ny_only;poi=5;ob_tol=5;fs_lb=36;close_only=true;stop_bps=0 |
| 13 | 27.99 | 18.13 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=false;stop_bps=1 |
| 14 | 27.60 | 17.26 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=false;stop_bps=1 |
| 15 | 27.45 | 16.93 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=false;stop_bps=1 |
| 16 | 26.00 | 15.60 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=24;close_only=true;stop_bps=1 |
| 17 | 25.94 | 15.45 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=36;close_only=true;stop_bps=1 |
| 18 | 25.91 | 15.49 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=5;ob_tol=5;fs_lb=12;close_only=true;stop_bps=1 |
| 19 | 25.90 | 39.33 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=0;ob_tol=5;fs_lb=12;close_only=false;stop_bps=0 |
| 20 | 25.71 | 38.90 | tf=1m/15m;entry=ob_mid;confirm=cisd_or_ifvg;rr=1.2;kz=london_ny;poi=0;ob_tol=5;fs_lb=24;close_only=false;stop_bps=0 |
