# MacroSoup Robustness Gate

Gates:
- min average trades/month >= 8
- positive month rate >= 55%
- max monthly equity drawdown (USD, 0.1 BTC) <= 2500
- total USD (0.1 BTC) > 0
- PF >= 1.20

- Stress shortlist: top 3 windows from MACRO_SOUP_REALISM_REPORT.md
- Tested stress rows: 9
- Pass rows: 0

## Top PASS (deduped)

| rank | start | end | slip | fee_mult | trades | win_rate_% | pf_r | profit_r | total_usd_0p1 | pos_months | months | max_monthly_dd |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|

## Top FAIL (why)

| start | end | slip | fee_mult | total_usd_0p1 | pf_r | fail_reason |
|---|---|---:|---:|---:|---:|---|
| 15:50 | 16:50 | 1 | 100 | -8847.72 | 4.77 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 15:55 | 16:55 | 1 | 100 | -8847.72 | 4.77 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 15:50 | 16:50 | 2 | 100 | -8852.09 | 4.77 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 15:55 | 16:55 | 2 | 100 | -8852.09 | 4.77 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 15:50 | 16:50 | 3 | 100 | -8856.45 | 4.77 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 15:55 | 16:55 | 3 | 100 | -8856.45 | 4.77 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 16:00 | 17:00 | 1 | 100 | -10128.74 | 4.64 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 16:00 | 17:00 | 2 | 100 | -10133.34 | 4.64 | low_pos_month_rate,high_monthly_dd,non_positive_total |
| 16:00 | 17:00 | 3 | 100 | -10137.94 | 4.64 | low_pos_month_rate,high_monthly_dd,non_positive_total |
