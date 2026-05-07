# Champion Baseline

Sources:
- `reports/strategy_overviews/STRATEGY_VALIDATION_MATRIX.md`
- `reports/strategy_overviews/STRATEGY_BEST_SETUP_SUMMARY.md`
- strategy-specific reports in `reports/strategy_overviews/`
- `reports/strategy_overviews/MULTI_ASSET_IFVG_TUNE_REPORT.md`
- `reports/strategy_overviews/SOL_BEST_CURRENT_CONFIG.md`
- `reports/strategy_overviews/MACRO_SOUP_REALISM_REPORT.md`
- `reports/strategy_overviews/MACRO_SOUP_ROBUSTNESS.md`
- `reports/strategy_overviews/MACRO_SOUP_MNQ_1M_3M.md`
- `reports/strategy_overviews/MACRO_SOUP_MNQ_MONTHLY_FROM_2025.md`

Champion selection rule (crypto scorecard): best risk-adjusted score among `FULLY_TESTED` rows.
Risk-adjusted score = `(net_profit_% * profit_factor * win_rate_%) / max(max_dd_%,1)`.

## Best Result Per Strategy (What We Tried)

| strategy | market | best_asset | best_timeframe | best_net | unit | pf | win_rate_% | trades | wf_test_net | wf_test_pf | verdict | setup_id |
|---|---|---|---|---:|---|---:|---:|---:|---:|---:|---|---|
| ttrades_fractal_mtf | crypto | SOL | 15m/4h | 20.89 | % | 1.25 | 38.30 | 141 | 28.66 | 2.30 | FULLY_TESTED | `tt_mtf_sol_15m4h_cisd_rr2` |
| doji | futures | MNQ | 15m | 12924.24 (slip1) | $ | 1.97 (slip1) | 29.03 (slip1) | 1791 (slip1) | n/a | n/a | FULLY_TESTED | `doji_mnq_15m_classic_mc_sl12_tp300` |
| ema_wick_reclaim_mnq | futures | MNQ | 1m (EMA200 5m anchor) | 18672.08 (full dataset), 11533.18 (from 2025-01-01 tuned) | $ | n/a (custom $/trade model) | 22.25 (full), 24.49 (2025+ tuned) | 4197 (full), 1613 (2025+ tuned) | n/a | n/a | PARTIALLY_TESTED | `mnq_ema_wick_rr5_wick8_atr0.5_cap0.25_obw6_hold90_all_atr_obmid` |
| ttrades_fractal | futures | GOLD | 1m | -18838.46 | $ | 0.31 | 19.23 | 754 | -5247.04 | 0.41 | PARTIALLY_TESTED | `tt_single_gold_1m_rr2_fvg_cisd` |
| mc (manipulation candle/engulf) | crypto | BTC | 5m | negative (rep. balance -194092.84 to -3664305.62) | % | up to 0.94 | n/a | n/a | n/a | n/a | HOLD / PARTIALLY_TESTED | `mc_engulf_filters_reality` |
| ce | futures | MNQ | 1m | -27.17 avg test (best 8-fold cluster) | $ | 0.83 | n/a | 21 avg test | n/a | n/a | NOT_PROMOTED | `ce_london_research_8fold` |
| macro_soup | futures | MNQ | 1m / 3m (close-back-inside window sweep) | +1726.00 (1m best, from 2025-01-01), +1445.00 (3m best, from 2025-01-01) | $ | 3.81 (1m best), 3.95 (3m best) | 14.00 (1m best), 15.22 (3m best) | 50 (1m best), 46 (3m best) | n/a | n/a | RESEARCH_ONLY (fails robustness gate on baseline BTC pass) | `macro_soup_mnq_top_windows_2025plus` |
| orb_london_reversal | futures | MNQ | 30m ORB | 492.27 | $ | 1.19 | 34.94 | 312 | n/a | n/a | NOT_RECOMMENDED | `orb_london_mnq_30m_1400_close` |
| orb (reality-validated grid) | multi-market | BTC | 15m | no promotable variant after realism + rolling OOS | % | best OOS gate miss (pf windows <=2/5) | n/a | multi | rolling 5-window run completed | PF>=1.20 and profit_r>0 in >=4/5 failed on all asset/TF champions | KILL / FULLY_TESTED | `orb_realism_grid_oos_2026-05-04` |
| ob_engulfing | futures | MNQ | 15m | 4529.74 (points model), 33.92% (equity model) | $ / % | 1.52 | 43.33 | 120 | 12.17% (OOS split) | 1.42 | RESEARCH_ONLY (ETH transfer KILL) | `ob_engulf_mnq_15m_pairmid_rr2_sl32.5` |

## Champion Rows (Current)

| category | strategy | asset | timeframe | key_result |
|---|---|---|---|---|
| crypto score champion | ttrades_fractal_mtf | SOL | 15m/4h | net 20.89%, pf 1.25, wf_test_net 28.66%, wf_test_pf 2.30 |
| recent-window deployment champion (normalized sizing) | ttrades_fractal_mtf (portfolio combo) | BTC+ETH+SOL | 15m/4h | pooled last-6m +315.15 USD, positive months 4/6 (BTC 0.1, ETH 1, SOL 10) |
| futures cash champion (supplemental) | doji | MNQ | 15m | net 12924.24 USD (slip1), pf 1.97 |
| futures challenger (new) | ema_wick_reclaim_mnq | MNQ | 1m | net 11533.18 USD (2025+ tuned), monthly analysis available in MNQ EMA report |
| futures high hit-rate challenger (new) | ema_wick_reclaim_mnq | MNQ | 3m | net 5964.47 USD, win rate 30.35%, monthly +12/-3 (15 months) |

## Setup Details (Moved Out of Table)

- `tt_mtf_sol_15m4h_cisd_rr2`: `entry=ObMidpoint; confirm_mode=cisd_only; time_filter=all_day_all_week; opportunity=baseline; rr=2; poi_pad_bps=0; ob_tol_bps=0; slippage=[0,1,2]`.
- `close_ifvg_rr2_poi0_ob5_all_day` (BTC recent): `entry=Close; confirm_mode=ifvg_only; rr=2; poi_pad_bps=0; ob_tol_bps=5; killzone=all_day; sizing=0.1 BTC`.
- `close_ifvg_rr2_poi0_ob0_ny_only` (ETH recent): `entry=Close; confirm_mode=ifvg_only; rr=2; poi_pad_bps=0; ob_tol_bps=0; killzone=ny_only; sizing=1 ETH`.
- `close_ifvg_rr2_poi10_ob10_ny_only` (SOL recent): `entry=Close; confirm_mode=ifvg_only; rr=2; poi_pad_bps=10; ob_tol_bps=10; killzone=ny_only; sizing=10 SOL`.
- `portfolio_combo_top_1`: `BTC close_ifvg_rr2_poi0_ob5_all_day + ETH close_ifvg_rr2_poi0_ob0_ny_only + SOL close_ifvg_rr2_poi10_ob10_ny_only; pooled +315.15 USD over last 6 months`.
- `doji_mnq_15m_classic_mc_sl12_tp300`: `doji=classic; entry=market_close; max_sl_mode=limit_reprice; max_sl=12; tp_points=300; trail=10/10; max_trades_per_day=10; session=04:00-12:00; commission_rt=1.32; slippage=[1,2,3]`.
- `tt_single_gold_1m_rr2_fvg_cisd`: `rr=2; use_fvg=true; lookback=20; require_cisd=true; slippage=[0,1,2]`.
- `mc_engulf_filters_reality`: `MC/Engulfing reality runs with fee+slippage; tested FVG/narrow/quality filters; no robust profitable pocket`.
- `ce_london_research_8fold`: `London CE sweep with realistic costs, 8/10-fold WF gates; failed promotion under OOS and stress`.
- `macro_soup_mnq_top_windows_2025plus`: `setup family=close-back-inside; windows tested={15:50-16:50, 15:55-16:55, 16:05-17:05, 16:00-17:00, 16:10-17:10}; dataset=/Users/waff/develop/play/nq/mnq_1m_cont.parquet; quick mode=last 120000 1m bars; execution=slippage 1 tick, commission 0, fee 0; contract=MNQ 1 micro ($2/point); best 1m window=15:50 (+1726 from 2025-01-01), best 3m window=15:50 (+1445 from 2025-01-01); monthly profile still mixed (strong 2026-01 concentration); prior baseline MacroSoup robustness gate on BTC top windows: 0/9 pass (low positive-month rate, high monthly DD, non-positive total at fixed sizing).`.
- `orb_london_mnq_30m_1400_close`: `orb=30m; session_close=14:00; min_excursion=20%; max_reenter=12; be/time_stop=off`.
- `orb_realism_grid_oos_2026-05-04`: `entry=next_bar_open; intrabar=stop_first; gap_stop=adverse_open_fill; fees=maker+taker; slippage=1/2/3 ticks per side; datasets=MNQ(1m/5m/15m/1h resampled), BTC/ETH/SOL(5m/15m/1h/4h); final gate=rolling OOS 5 windows requiring PF>=1.20 and profit_r>0 in >=4/5`.
- `ob_engulf_mnq_15m_pairmid_rr2_sl32.5`: `session=NYAM; entry=PairMidpoint; quality body>=40% and range>=1.1x prev; rr=2.0; max_sl=32.5; mode=KeepEntryMoveStop`.
- `mnq_ema_wick_rr5_wick8_atr0.5_cap0.25_obw6_hold90_all_atr_obmid`: `entry_tf=1m; ema_anchor=EMA200(5m); trigger=wick_through_close_back; entry=OB midpoint retest (ob_wait=6); stop_mode=ATR; rr=5; min_wick=8 ticks; atr_floor_mult=0.5 (ATR14); cost_cap=0.25R; max_hold=90; session=all; commission_rt=1.24; slippage_rt=1.00; full_dataset=+$18672.08 (legacy full baseline reference); 2025+ tuned=+$11533.18 (1613 trades, 24.49% win)`.
- `mnq_ema_wick_rr4_wick8_atr0.5_cap0.20_ny_atr_obmid`: `entry_tf=3m; ema_anchor=EMA200(5m); trigger=wick_through_close_back; entry=OB midpoint retest; stop_mode=ATR; rr=4; min_wick=8 ticks; atr_floor_mult=0.5 (ATR14); cost_cap=0.20R; session=NY (09:30-11:30); commission_rt=1.24; slippage_rt=1.00; win_rate=30.35%; monthly=+12/-3`.

Note: table is normalized to one best row per strategy; alternate variants are retained in setup details.
