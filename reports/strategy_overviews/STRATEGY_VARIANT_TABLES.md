# Strategy Variant Tables

Purpose: provide per-strategy variant tables in the same spirit as `TTRADES_TARGETED_GRID_SOL_MTF.md`, so tested variants are easy to scan.

Note: for some older runs, full raw sweep rows were not persisted. In those cases, this file includes all recoverable variant rows from strategy reports and validation outputs.

## doji (MNQ 15m, realism-fixed)

| variant_id | doji_type | entry | max_sl_mode | max_sl | tp_points | trail_activate | trail_dist | max_trades_day | session | commission_rt | slippage_ticks | trades | win_rate_% | profit_r | pf_r | pnl_usd_net_est | verdict |
|---|---|---|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| doji_champion_slip1 | classic | market_close | limit_reprice | 12 | 300 | 10 | 10 | 10 | 04:00-12:00 | 1.32 | 1 | 1791 | 29.03 | 1023.57 | 1.97 | 12924.24 | PROMOTED |
| doji_champion_slip2 | classic | market_close | limit_reprice | 12 | 300 | 10 | 10 | 10 | 04:00-12:00 | 1.32 | 2 | 1791 | 28.59 | 966.87 | 1.92 | 11487.34 | PROMOTED |
| doji_champion_slip3 | classic | market_close | limit_reprice | 12 | 300 | 10 | 10 | 10 | 04:00-12:00 | 1.32 | 3 | 1787 | 28.37 | 907.03 | 1.86 | 9976.86 | PROMOTED |

Source: `DOJI_STRATEGY_REPORT.md`.

## mc / engulfing (BTC multi-TF reality pass)

| variant_id | family | market | asset | timeframe | mode | pattern | entry | rr | filter_bundle | execution | key_result | verdict |
|---|---|---|---|---|---|---|---|---:|---|---|---|---|
| mc_cont_ema200_engulf_close | mc | crypto | BTC | 5m | ContinuationEma200 | Engulfing | Close | 2.0 | none | fee+slippage | PF 0.94, unprofitable | HOLD |
| mc_flt_engulf_ema200_close_fvg | mc | crypto | BTC | 5m | ContinuationEma200 | Engulfing | Close | 2.0 | FVG on | fee+slippage | PF 0.94, unprofitable | HOLD |
| mc_flt_engulf_ema200_close_narrow | mc | crypto | BTC | 5m | ContinuationEma200 | Engulfing | Close | 2.0 | narrow window | fee+slippage | PF 0.87, unprofitable | HOLD |
| mc_flt_engulf_ema200_close_quality | mc | crypto | BTC | 5m | ContinuationEma200 | Engulfing | Close | 2.0 | signal quality gates | fee+slippage | PF 0.86, unprofitable | HOLD |

Source: `MC_STRATEGY_REPORT.md`.

## ce (London research promotion gate)

| variant_id | folds | mode | min_avg_test_trades | commission_mult | slippage_mult | avg_test_net_usd | avg_test_pf | survivor_count | verdict |
|---|---:|---|---:|---:|---:|---:|---:|---:|---|
| ce_london_8fold_base | 8 | research-london | 20 | 100 | 100 | -27.17 | 0.83 | >0 | FAIL |
| ce_london_10fold_base | 10 | research-london | 20 | 100 | 100 | n/a | n/a | 0 | FAIL |
| ce_london_8fold_stress125 | 8 | research-london | 20 | 125 | 125 | -39.77 | 0.76 | >0 | FAIL |
| ce_london_10fold_stress125 | 10 | research-london | 20 | 125 | 125 | n/a | n/a | 0 | FAIL |

Source: `CE_PROMOTION_REPORT.md`.

## orb_london_reversal (final optimization snapshot)

| variant_id | asset | orb_window | session_close | min_excursion | max_reenter | be_stop | time_stop | trades | win_rate_% | pf | net_usd | maxdd_usd | verdict |
|---|---|---|---|---:|---:|---|---|---:|---:|---:|---:|---:|---|
| orb_mnq_best | MNQ | 30m | 14:00 | 20 | 12 | off | off | 312 | 34.94 | 1.19 | 492.27 | 213.81 | TRADABLE_CANDIDATE_ONLY |
| orb_mes_best | MES | tuned | tuned | tuned | tuned | mixed | mixed | n/a | n/a | n/a | 128.06 | 620.69 | REJECT |
| orb_gold_best | GOLD | tuned | tuned | tuned | tuned | mixed | mixed | n/a | n/a | n/a | 999.10 | 351.85 | REJECT |

Source: `ORB_LONDON_REVERSAL_NOT_RECOMMENDED.md`.

## orb (reality-validated cross-asset grid)

| variant_id | scope | assets | timeframes | realism_model | slippage_sweep | tested_variants | final_gate | final_result |
|---|---|---|---|---|---|---:|---|---|
| orb_realism_grid_oos_2026-05-04 | ORB durations/SL/RR/active-window/hold/retest families | MNQ, BTC, ETH, SOL | MNQ: 1m/5m/15m/1h; Crypto: 5m/15m/1h/4h | next-bar-open entries, stop-first intrabar, gap-aware stop fills, maker+taker fees | 1/2/3 ticks per side | 31,440 rows | rolling OOS (5 equal windows), require PF>=1.20 and profit_r>0 in >=4/5 windows | ALL_CHAMPIONS_KILL |

Notes:
- Full row-level tested variants are in `ORB_VARIANTS_GRID.md` (realism-only report; no optimistic pre-realism rows).
- Final cycle in that report shows per asset/timeframe champion and `PROMOTE/KILL` verdict.

## ob_engulfing (MNQ + ETH transfer)

### MNQ 15m fixed-SL grid

| variant_id | session | entry | rr | quality | sl_cap | sl_mode | trades | win_rate_% | profit_r | pf_r | pnl_% | verdict |
|---|---|---|---:|---|---:|---|---:|---:|---:|---:|---:|---|
| ob_mnq_sl30_keep_entry | NYAM | PairMidpoint | 2.0 | body>=0.40 & range>=1.1x prev | 30.0 | KeepEntryMoveStop | 120 | 41.67 | 29.70 | 1.42 | 25.85 | BEST_MNQ |
| ob_mnq_sl27p5_keep_entry | NYAM | PairMidpoint | 2.0 | body>=0.40 & range>=1.1x prev | 27.5 | KeepEntryMoveStop | 120 | 41.67 | 29.69 | 1.42 | 25.84 | PASS |
| ob_mnq_sl30_keep_stop | NYAM | PairMidpoint | 2.0 | body>=0.40 & range>=1.1x prev | 30.0 | KeepStopMoveEntry | 82 | 43.90 | 25.70 | 1.56 | 23.39 | PASS |
| ob_mnq_sl25_keep_entry | NYAM | PairMidpoint | 2.0 | body>=0.40 & range>=1.1x prev | 25.0 | KeepEntryMoveStop | 120 | 40.00 | 23.69 | 1.33 | 18.31 | PASS |

### ETH transfer baseline (same core setup)

| variant_id | asset | timeframe | session | trades | win_rate_% | profit_r | pf_r | pnl_% | verdict |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| ob_eth_5m_full | ETH | 5m | full_day | 1162 | 34.85 | 50.30 | 1.07 | 43.97 | HOLD |
| ob_eth_15m_full | ETH | 15m | full_day | 703 | 35.56 | 46.02 | 1.10 | 45.41 | HOLD |
| ob_eth_1h_full | ETH | 1h | full_day | 376 | 31.91 | -16.18 | 0.94 | -18.72 | REJECT |
| ob_eth_4h_full | ETH | 4h | full_day | 201 | 40.30 | 41.95 | 1.35 | 48.30 | HOLD |

### ETH refinement winners

| variant_id | timeframe | rr | sl_cap | min_body_to_range | min_range_to_prev | trades | win_rate_% | profit_r | pf_r | pnl_% | verdict |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| ob_eth_15m_ref_best | 15m | 2.0 | 30.0 | 0.45 | 0 | 961 | 35.59 | 63.45 | 1.10 | 67.78 | CANDIDATE |
| ob_eth_4h_ref_best | 4h | 2.2 | 30.0 | 0.45 | 0 | 262 | 37.79 | 54.71 | 1.34 | 66.72 | CANDIDATE |

### ETH rolling OOS final cycle

| variant_id | profit_windows_pos | pf_windows_ge_1p2 | final_verdict |
|---|---:|---:|---|
| ob_eth_15m_base | 3/5 | 3/5 | KILL |
| ob_eth_15m_ema50_200 | 3/5 | 3/5 | KILL |
| ob_eth_4h_base | 4/5 | 3/5 | KILL |
| ob_eth_4h_ema50_200 | 4/5 | 3/5 | KILL |

Source: `OB_ENGULFING_MNQ_ETH_REPORT.md`.
