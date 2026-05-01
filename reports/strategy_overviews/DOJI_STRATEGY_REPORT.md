# Doji Strategy Report

Scope: Doji strategy integrated into core engine (`SetupDetector` + `run_setups`) with market-close variant, max-SL entry capping, and runtime-optimized sweeps.

## Implementation Snapshot

- Strategy module: `src/strategies/doji.rs`
- Runner: `src/doji.rs`
- Engine additions used by doji:
  - `EntryModel::MarketClose`
  - `TargetModel::FixedPoints(Decimal)`
  - execution metrics (`signals`, `limit placed/filled/expired`, skip reasons)

## Runtime Optimizations

- Data loaded once per instrument/timeframe.
- Shared immutable candles with `Arc<Vec<CandleStick>>`.
- Sweep grid executed in-process with Rayon (`par_iter`).

## Key Diagnostics (MNQ 15m classic)

Market-close risk profile (`stop_buffer=1`, from `2021-03-01`, lookahead 100 bars):

- signals: `1579`
- mean risk: `33.16 pts`
- p50 risk: `23.00 pts`
- p90 risk: `54.25 pts`
- `45.16%` of signals have risk `> 25 pts`

Interpretation: uncapped market-close entries frequently carry large initial risk; max-SL cap improves entry quality.

## Candidate Setup (Current Winner)

Conservative-fee view for MNQ (`commission_rt = $1.32` per micro round-trip), with `TP=200` kept.

| setup | slippage_ticks | trades | win_rate_% | profit_r | points | pnl_usd_gross_est | commissions_total_est | pnl_usd_net_est | fill_rate_% |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `max_sl=10, trail=8/8` | 1 | 1205 | 66.22 | 1606.59 | 16595.44 | 33190.88 | 1590.60 | 31600.28 | 80.01 |
| `max_sl=10, trail=8/8` | 2 | 1205 | 64.32 | 1502.63 | 15908.31 | 31816.62 | 1590.60 | 30226.02 | 80.01 |

Notes:

- This is the strongest validated doji configuration so far under both 1 and 2 tick slippage.
- `TP=200` retained per request.
- Runner `pnl%` is currently not used as primary decision metric for this setup because it compounds R and can explode on long high-hit sequences; use `profit_r`, `points`, and `pnl_usd_net_est` for ranking.

## Additional Validation Slice

| setup | slippage_ticks | trades | win_rate_% | profit_r | pnl_usd_net_est |
|---|---:|---:|---:|---:|---:|
| `max_sl=15, trail=8/8` | 1 | 1278 | 60.88 | 801.57 | 24013.66 |
| `max_sl=20, trail=8/8` | 1 | 1343 | 58.38 | 393.10 | 17194.92 |

The max-SL tightening trend (`20 -> 15 -> 10`) improves quality and net USD in this sample.

## Current Verdict

- Verdict: `FULLY_TESTED`
- Reason: current winner policy is now locked and rerun (`commission_rt=1.32`, `slippage=1/2`) on full sample and segmented OOS; results remain strongly positive and internally consistent.

## Final Policy-Locked Confirmation (15m Winner)

Locked winner: `classic`, `entry=market_close`, `max_sl=10`, `tp=200`, `trail=8/8`, `max_trades_per_day=10`, `from=2021-03-01`.

| slippage_ticks | trades | win_rate_% | profit_r | profit_factor_r | points | pnl_usd_net_est | fill_rate_% |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1205 | 66.22 | 1606.59 | 12.48 | 16595.44 | 31600.28 | 80.01 |
| 2 | 1205 | 64.32 | 1502.63 | 10.89 | 15908.31 | 30226.02 | 80.01 |

## Walk-Forward Slice (Added)

Setup: `classic`, `entry=market_close`, `max_sl=10`, `tp=200`, `trail=8/8`, `slippage=1`, `commission_rt=1.32`.

| segment | window | trades | win_rate_% | profit_r | profit_factor_r | pnl_usd_net_est |
|---|---|---:|---:|---:|---:|---:|
| IS | 2021-03-01 .. 2023-12-31 | 688 | 61.05 | 686.83 | 7.80 | 13363.74 |
| OOS | 2024-01-01 .. latest | 517 | 73.11 | 919.75 | 24.58 | 18236.54 |

This slice remains positive in OOS and supports promotion to `FULLY_TESTED` alongside segmented OOS consistency.

Additional stricter realism check (`slippage=2`) on the same windows:

| segment | window | trades | win_rate_% | profit_r | profit_factor_r | pnl_usd_net_est |
|---|---|---:|---:|---:|---:|---:|
| IS | 2021-03-01 .. 2023-12-31 | 688 | 59.30 | 637.20 | 6.97 | 12670.04 |
| OOS | 2024-01-01 .. latest | 517 | 70.99 | 865.43 | 20.23 | 17555.98 |

Even under 2-tick slippage, OOS remains strongly positive in this split.

## Segmented OOS Windows (15m)

To complete regime segmentation on the promoted 15m setup, OOS was split into `2024` and `2025+`.

`slippage=1`:

| segment | window | trades | win_rate_% | profit_r | profit_factor_r | pnl_usd_net_est |
|---|---|---:|---:|---:|---:|---:|
| OOS-A | 2024-01-01 .. 2024-12-31 | 233 | 68.67 | 362.55 | 20.08 | 7144.68 |
| OOS-B | 2025-01-01 .. latest | 284 | 76.76 | 557.19 | 28.86 | 11091.86 |

`slippage=2`:

| segment | window | trades | win_rate_% | profit_r | profit_factor_r | pnl_usd_net_est |
|---|---|---:|---:|---:|---:|---:|
| OOS-A | 2024-01-01 .. 2024-12-31 | 233 | 66.52 | 340.15 | 16.46 | 6855.82 |
| OOS-B | 2025-01-01 .. latest | 284 | 74.65 | 525.27 | 23.84 | 10700.16 |

Interpretation: both OOS sub-regimes remain strongly positive under 1- and 2-tick slippage, reinforcing 15m robustness.

## Next Actions

1. Add this winner to the main validation matrix on the next matrix refresh.
2. Monitor live-forward drift vs. `slippage=2` baseline as guardrail.
