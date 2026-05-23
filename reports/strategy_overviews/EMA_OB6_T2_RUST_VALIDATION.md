# EMA OB6 T2 Rust Validation

Date: 2026-05-23

## Scope

- Strategy: `15m_13_200_ema9_hl_mid_1030_ob6_t2`
- Dataset: `/Users/waff/develop/play/nq/mnq_1m_cont.parquet`
- Period: `2025-01-01+`
- Instrument: MNQ, 1 contract
- Runner: `cargo run --release --bin ema_ob6_t2_backtest`

## Engine Fixes Applied

- Added strict `LimitTradeThrough` fill mode to the shared Rust engine.
- Trade-through fill now requires one full tick beyond the limit, matching `tradovate_bot`:
  - long: `low <= entry - tick_size`
  - short: `high >= entry + tick_size`
- Added `PreviousClosePoints` trailing model for bot-style previous-close trailing.
- Kept stop-first intrabar precedence and gap-through stop fills.

## Python Parity Check

Winner preset parity is exact against the Python shared engine for the current promoted setup.

| Slippage | Trades | Win Rate | Net PnL | PF | Max DD |
|---:|---:|---:|---:|---:|---:|
| 1 tick | 358 | 55.6% | $8,886.89 | 2.05 | -$396.51 |
| 2 ticks | 358 | 54.7% | $8,528.89 | 1.99 | -$399.51 |
| 3 ticks | 358 | 53.9% | $8,170.89 | 1.93 | -$402.51 |

## First Filter Sweep

Best improvement found: skip Friday, keep full `10:30-15:25` entry window.

| Variant | Slippage | Trades | Win Rate | Net PnL | PF | Max DD |
|---|---:|---:|---:|---:|---:|---:|
| Baseline | 1 tick | 358 | 55.6% | $8,886.89 | 2.05 | -$396.51 |
| Baseline | 2 ticks | 358 | 54.7% | $8,528.89 | 1.99 | -$399.51 |
| Baseline | 3 ticks | 358 | 53.9% | $8,170.89 | 1.93 | -$402.51 |
| Skip Friday | 1 tick | 287 | 56.1% | $8,893.71 | 2.35 | -$321.12 |
| Skip Friday | 2 ticks | 287 | 55.7% | $8,606.71 | 2.29 | -$332.97 |
| Skip Friday | 3 ticks | 287 | 54.7% | $8,319.71 | 2.22 | -$348.97 |

## Realism Validation

- Fees: `$0.92` round trip per MNQ contract.
- Slippage: `1/2/3` ticks per side.
- Entry model: strict trade-through limit, one full tick beyond limit.
- Stop model: conservative stop-first intrabar ordering and gap-through open fills.
- Data checks: OHLC sanity and monotonic timestamp validation before run.
- Verdict before second sweep: `skip Friday` improves profit factor and drawdown without reducing 3-tick net robustness.

## Second Sweep

Executed in-process in Rust (`--sweep2`) on top of the validated baseline.

Sweep space:

- Weekday filters: `""`, `fri`, `mon,fri`, `tue,fri`, `wed,fri`, `thu,fri`, `mon`, `tue`, `wed`, `thu`
- Skip windows: `""`, `10:30-11:00`, `11:00-11:30`, `11:30-12:15`, `12:00-12:45`, `13:00-13:30`, `13:30-14:15`, `14:00-14:30`, `14:30-15:25`
- TP families:
  - fixed points: `120,130,140,150,160,170,180,200,220,250`
  - RR mode: `2.0,2.5,3.0,3.5,4.0,5.0,6.0`

Total tested: `1480` variants.

Top results:

| Rank | Filter | TP mode | TP value | 1-tick Net | 1-tick PF | 1-tick DD | 3-tick Net | 3-tick PF | 3-tick DD |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | skip `fri`, no intraday skip | fixed | 150 | $8,893.71 | 2.35 | -$321.12 | $8,319.71 | 2.22 | -$348.97 |
| 2 | skip `fri`, no intraday skip | fixed | 180 | $8,882.88 | 2.35 | -$321.12 | $8,310.88 | 2.22 | -$348.97 |
| 3 | skip `fri`, no intraday skip | fixed | 170 | $8,813.13 | 2.34 | -$321.12 | $8,241.13 | 2.21 | -$348.97 |
| 4 | skip `fri`, no intraday skip | fixed | 140 | $8,820.04 | 2.32 | -$321.12 | $8,244.04 | 2.19 | -$348.97 |
| 5 | baseline (no weekday/intraday skip) | fixed | 150 | $8,886.89 | 2.05 | -$396.51 | $8,170.89 | 1.93 | -$402.51 |

Best promoted result from second sweep remains:

- `skip_weekdays = fri`
- `skip_windows = ""`
- `tp_mode = fixed`
- `tp_pts = 150`

This variant improves drawdown materially and also improves/slightly preserves net across slippage stress.
