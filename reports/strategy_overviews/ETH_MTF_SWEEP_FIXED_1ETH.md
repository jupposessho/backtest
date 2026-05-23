# ETH-only TTrades MTF Sweeps (Fixed 1 ETH)

- Strategy: ttrades_fractal_mtf
- Sizing: fixed 1 ETH per trade
- Costs: Binance standard fee config + slippage 1/2/3 ticks per side
- Timeframes: 5m/1h and 15m/4h

## Top 15 - 5m/1h

| rank | net_usd_1eth | pf | win_rate_% | trades | wins | mode | cisd | time_profile | opportunity | slip |
|---:|---:|---:|---:|---:|---:|---|---|---|---|---:|
| 1 | 34.78 | 1.88 | 42.86 | 7 | 3 | cisd_and_ifvg | body_flip | ny_weekdays | baseline | 1 |
| 2 | 34.64 | 1.87 | 42.86 | 7 | 3 | cisd_and_ifvg | body_flip | ny_weekdays | baseline | 2 |
| 3 | 34.50 | 1.87 | 42.86 | 7 | 3 | cisd_and_ifvg | body_flip | ny_weekdays | baseline | 3 |
| 4 | 34.46 | 1.72 | 42.86 | 7 | 3 | cisd_and_ifvg | strict_wick_break | ny_weekdays | baseline | 1 |
| 5 | 34.46 | 1.72 | 42.86 | 7 | 3 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | baseline | 1 |
| 6 | 34.32 | 1.72 | 42.86 | 7 | 3 | cisd_and_ifvg | strict_wick_break | ny_weekdays | baseline | 2 |
| 7 | 34.32 | 1.72 | 42.86 | 7 | 3 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | baseline | 2 |
| 8 | 34.18 | 1.71 | 42.86 | 7 | 3 | cisd_and_ifvg | strict_wick_break | ny_weekdays | baseline | 3 |
| 9 | 34.18 | 1.71 | 42.86 | 7 | 3 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | baseline | 3 |
| 10 | 30.51 | 1.67 | 44.44 | 9 | 4 | cisd_and_ifvg | body_flip | london_ny_weekdays | baseline | 1 |
| 11 | 30.33 | 1.67 | 44.44 | 9 | 4 | cisd_and_ifvg | body_flip | london_ny_weekdays | baseline | 2 |
| 12 | 30.15 | 1.66 | 44.44 | 9 | 4 | cisd_and_ifvg | body_flip | london_ny_weekdays | baseline | 3 |
| 13 | 30.02 | 1.46 | 40.00 | 10 | 4 | ifvg_only | body_flip | ny_weekdays | baseline | 1 |
| 14 | 30.02 | 1.46 | 40.00 | 10 | 4 | ifvg_only | strict_wick_break | ny_weekdays | baseline | 1 |
| 15 | 30.02 | 1.46 | 40.00 | 10 | 4 | ifvg_only | last_series_close_break | ny_weekdays | baseline | 1 |

## Top 15 - 15m/4h

| rank | net_usd_1eth | pf | win_rate_% | trades | wins | mode | cisd | time_profile | opportunity | slip |
|---:|---:|---:|---:|---:|---:|---|---|---|---|---:|
| 1 | 197.88 | 1.60 | 54.55 | 33 | 18 | cisd_and_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr15 | 1 |
| 2 | 197.22 | 1.59 | 54.55 | 33 | 18 | cisd_and_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr15 | 2 |
| 3 | 196.56 | 1.59 | 54.55 | 33 | 18 | cisd_and_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr15 | 3 |
| 4 | 189.87 | 1.35 | 50.00 | 56 | 28 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | more_hits_close_rr15 | 1 |
| 5 | 188.75 | 1.35 | 50.00 | 56 | 28 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | more_hits_close_rr15 | 2 |
| 6 | 187.63 | 1.34 | 50.00 | 56 | 28 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | more_hits_close_rr15 | 3 |
| 7 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | body_flip | ny_weekdays | more_hits_close_rr15 | 1 |
| 8 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | strict_wick_break | ny_weekdays | more_hits_close_rr15 | 1 |
| 9 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | last_series_close_break | ny_weekdays | more_hits_close_rr15 | 1 |
| 10 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | failure_swing | ny_weekdays | more_hits_close_rr15 | 1 |
| 11 | 171.32 | 1.39 | 52.17 | 46 | 24 | cisd_and_ifvg | last_series_close_break | ny_weekdays | more_hits_close_rr15 | 1 |
| 12 | 171.32 | 1.39 | 52.17 | 46 | 24 | cisd_or_ifvg | failure_swing | ny_weekdays | more_hits_close_rr15 | 1 |
| 13 | 170.40 | 1.39 | 52.17 | 46 | 24 | ifvg_only | body_flip | ny_weekdays | more_hits_close_rr15 | 2 |
| 14 | 170.40 | 1.39 | 52.17 | 46 | 24 | ifvg_only | strict_wick_break | ny_weekdays | more_hits_close_rr15 | 2 |
| 15 | 170.40 | 1.39 | 52.17 | 46 | 24 | ifvg_only | last_series_close_break | ny_weekdays | more_hits_close_rr15 | 2 |

## Robust Top 20 (ranked by worst-case slip net)

| rank | timeframe | net_min_usd | net_avg_usd | net_max_usd | pf | win_rate_% | trades | wins | mode | cisd | time_profile | opportunity |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|
| 1 | 15m/4h | 196.56 | 197.22 | 197.88 | 1.60 | 54.55 | 33 | 18 | cisd_and_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr15 |
| 2 | 15m/4h | 187.63 | 188.75 | 189.87 | 1.35 | 50.00 | 56 | 28 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | more_hits_close_rr15 |
| 3 | 15m/4h | 169.48 | 170.40 | 171.32 | 1.39 | 52.17 | 46 | 24 | cisd_and_ifvg | last_series_close_break | ny_weekdays | more_hits_close_rr15 |
| 4 | 15m/4h | 169.48 | 170.40 | 171.32 | 1.39 | 52.17 | 46 | 24 | cisd_or_ifvg | failure_swing | ny_weekdays | more_hits_close_rr15 |
| 5 | 15m/4h | 169.48 | 170.40 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | body_flip | ny_weekdays | more_hits_close_rr15 |
| 6 | 15m/4h | 169.48 | 170.40 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | failure_swing | ny_weekdays | more_hits_close_rr15 |
| 7 | 15m/4h | 169.48 | 170.40 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | last_series_close_break | ny_weekdays | more_hits_close_rr15 |
| 8 | 15m/4h | 169.48 | 170.40 | 171.32 | 1.39 | 52.17 | 46 | 24 | ifvg_only | strict_wick_break | ny_weekdays | more_hits_close_rr15 |
| 9 | 15m/4h | 168.65 | 169.45 | 170.25 | 1.43 | 50.00 | 40 | 20 | cisd_and_ifvg | body_flip | ny_weekdays | more_hits_close_rr15 |
| 10 | 15m/4h | 147.44 | 149.90 | 152.36 | 1.16 | 48.78 | 123 | 60 | cisd_only | strict_wick_break | ny_weekdays | more_hits_close_rr15 |
| 11 | 15m/4h | 123.54 | 125.02 | 126.50 | 1.18 | 48.65 | 74 | 36 | cisd_and_ifvg | last_series_close_break | london_ny_weekdays | more_hits_close_rr15 |
| 12 | 15m/4h | 123.54 | 125.02 | 126.50 | 1.18 | 48.65 | 74 | 36 | cisd_or_ifvg | failure_swing | london_ny_weekdays | more_hits_close_rr15 |
| 13 | 15m/4h | 123.54 | 125.02 | 126.50 | 1.18 | 48.65 | 74 | 36 | ifvg_only | body_flip | london_ny_weekdays | more_hits_close_rr15 |
| 14 | 15m/4h | 123.54 | 125.02 | 126.50 | 1.18 | 48.65 | 74 | 36 | ifvg_only | failure_swing | london_ny_weekdays | more_hits_close_rr15 |
| 15 | 15m/4h | 123.54 | 125.02 | 126.50 | 1.18 | 48.65 | 74 | 36 | ifvg_only | last_series_close_break | london_ny_weekdays | more_hits_close_rr15 |
| 16 | 15m/4h | 123.54 | 125.02 | 126.50 | 1.18 | 48.65 | 74 | 36 | ifvg_only | strict_wick_break | london_ny_weekdays | more_hits_close_rr15 |
| 17 | 15m/4h | 121.79 | 123.05 | 124.31 | 1.20 | 47.62 | 63 | 30 | cisd_and_ifvg | body_flip | london_ny_weekdays | more_hits_close_rr15 |
| 18 | 15m/4h | 120.36 | 123.08 | 125.80 | 1.12 | 48.53 | 136 | 66 | cisd_or_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr15 |
| 19 | 15m/4h | 55.52 | 56.52 | 57.52 | 1.12 | 62.00 | 50 | 31 | cisd_and_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr12 |
| 20 | 5m/1h | 34.50 | 34.64 | 34.78 | 1.88 | 42.86 | 7 | 3 | cisd_and_ifvg | body_flip | ny_weekdays | baseline |

## Trade Density Top 20 (positive worst-case slip)

| rank | timeframe | trades | wins | win_rate_% | pf | net_min_usd | net_avg_usd | net_max_usd | mode | cisd | time_profile | opportunity |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|
| 1 | 15m/4h | 136 | 66 | 48.53 | 1.12 | 120.36 | 123.08 | 125.80 | cisd_or_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr15 |
| 2 | 15m/4h | 123 | 60 | 48.78 | 1.16 | 147.44 | 149.90 | 152.36 | cisd_only | strict_wick_break | ny_weekdays | more_hits_close_rr15 |
| 3 | 15m/4h | 89 | 51 | 57.30 | 1.01 | 3.98 | 5.76 | 7.54 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | more_hits_close_rr12 |
| 4 | 15m/4h | 83 | 50 | 60.24 | 1.05 | 23.63 | 25.29 | 26.95 | cisd_and_ifvg | strict_wick_break | ny_weekdays | ultra_hits_close_rr10 |
| 5 | 15m/4h | 74 | 36 | 48.65 | 1.18 | 123.54 | 125.02 | 126.50 | cisd_and_ifvg | last_series_close_break | london_ny_weekdays | more_hits_close_rr15 |
| 6 | 15m/4h | 74 | 36 | 48.65 | 1.18 | 123.54 | 125.02 | 126.50 | cisd_or_ifvg | failure_swing | london_ny_weekdays | more_hits_close_rr15 |
| 7 | 15m/4h | 74 | 36 | 48.65 | 1.18 | 123.54 | 125.02 | 126.50 | ifvg_only | body_flip | london_ny_weekdays | more_hits_close_rr15 |
| 8 | 15m/4h | 74 | 36 | 48.65 | 1.18 | 123.54 | 125.02 | 126.50 | ifvg_only | failure_swing | london_ny_weekdays | more_hits_close_rr15 |
| 9 | 15m/4h | 74 | 36 | 48.65 | 1.18 | 123.54 | 125.02 | 126.50 | ifvg_only | last_series_close_break | london_ny_weekdays | more_hits_close_rr15 |
| 10 | 15m/4h | 74 | 36 | 48.65 | 1.18 | 123.54 | 125.02 | 126.50 | ifvg_only | strict_wick_break | london_ny_weekdays | more_hits_close_rr15 |
| 11 | 15m/4h | 63 | 30 | 47.62 | 1.20 | 121.79 | 123.05 | 124.31 | cisd_and_ifvg | body_flip | london_ny_weekdays | more_hits_close_rr15 |
| 12 | 15m/4h | 56 | 28 | 50.00 | 1.35 | 187.63 | 188.75 | 189.87 | cisd_and_ifvg | strict_wick_break | london_ny_weekdays | more_hits_close_rr15 |
| 13 | 15m/4h | 50 | 31 | 62.00 | 1.12 | 55.52 | 56.52 | 57.52 | cisd_and_ifvg | strict_wick_break | ny_weekdays | more_hits_close_rr12 |
| 14 | 15m/4h | 46 | 24 | 52.17 | 1.39 | 169.48 | 170.40 | 171.32 | cisd_and_ifvg | last_series_close_break | ny_weekdays | more_hits_close_rr15 |
| 15 | 15m/4h | 46 | 24 | 52.17 | 1.39 | 169.48 | 170.40 | 171.32 | cisd_or_ifvg | failure_swing | ny_weekdays | more_hits_close_rr15 |
| 16 | 15m/4h | 46 | 24 | 52.17 | 1.39 | 169.48 | 170.40 | 171.32 | ifvg_only | body_flip | ny_weekdays | more_hits_close_rr15 |
| 17 | 15m/4h | 46 | 24 | 52.17 | 1.39 | 169.48 | 170.40 | 171.32 | ifvg_only | failure_swing | ny_weekdays | more_hits_close_rr15 |
| 18 | 15m/4h | 46 | 24 | 52.17 | 1.39 | 169.48 | 170.40 | 171.32 | ifvg_only | last_series_close_break | ny_weekdays | more_hits_close_rr15 |
| 19 | 15m/4h | 46 | 24 | 52.17 | 1.39 | 169.48 | 170.40 | 171.32 | ifvg_only | strict_wick_break | ny_weekdays | more_hits_close_rr15 |
| 20 | 15m/4h | 40 | 20 | 50.00 | 1.43 | 168.65 | 169.45 | 170.25 | cisd_and_ifvg | body_flip | ny_weekdays | more_hits_close_rr15 |

## Objective: High Trade Count (trades>=200, PF>=1.0)

| rank | timeframe | trades | wins | win_rate_% | pf | net_min_usd | net_avg_usd | net_max_usd | mode | cisd | time_profile | opportunity |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|---|---|
| - | - | - | - | - | - | - | - | - | - | - | - | no qualifying preset in current grid |

