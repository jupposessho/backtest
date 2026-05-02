# CE Strategy Promotion Gate Report

Date: 2026-05-02

## Scope

- Ported CE strategy from `~/develop/play/nq` into this repo.
- Added realistic per-trade round-trip costs into CE backtest flow.
- Evaluated with walk-forward validation and trade-count gates.
- Ran London-focused research with stricter confirmation filters.

## Implementation Summary

- New strategy module: `src/strategies/ce.rs`
  - CE swing detection and 5m execution path
  - Killzone/session gating
  - Volatility and impulse filters
  - Optional rejection confirmation at CE touch
  - Cost model fields:
    - `commission_round_trip_usd`
    - `slippage_round_trip_usd`
- New runner: `src/ce_sweep.rs`
  - Runtime data loading (`--parquet` / `--csv`)
  - Walk-forward evaluation
  - Multi-fold rolling research mode
  - Cost stress multipliers
  - Minimum average test-trades gate

## Promotion-Gate Runs

Dataset used: `mnq_1m_cont.parquet` (from NQ project)

### 1) 8-fold, baseline costs

Command:

```bash
cargo run --release --bin ce_sweep -- --parquet "/Users/waff/develop/play/nq/mnq_1m_cont.parquet" --research-london --folds 8 --max-bars 400000 --min-avg-test-trades 20 --top 8
```

Best cluster outcome:

- Avg test net USD: about `-27.17`
- Avg test PF: `0.83`
- Avg test trades: `21`

Status: failed (negative OOS)

### 2) 10-fold, baseline costs

Command:

```bash
cargo run --release --bin ce_sweep -- --parquet "/Users/waff/develop/play/nq/mnq_1m_cont.parquet" --research-london --folds 10 --max-bars 400000 --min-avg-test-trades 20 --top 8
```

Outcome:

- No configurations survived `min_avg_test_trades >= 20`.

Status: failed (insufficient robust survivors)

### 3) 8-fold, stressed costs (+25%)

Command:

```bash
cargo run --release --bin ce_sweep -- --parquet "/Users/waff/develop/play/nq/mnq_1m_cont.parquet" --research-london --folds 8 --max-bars 400000 --min-avg-test-trades 20 --commission-mult 125 --slippage-mult 125 --top 8
```

Best cluster outcome:

- Avg test net USD: about `-39.77`
- Avg test PF: `0.76`

Status: failed (edge degrades under realistic stress)

### 4) 10-fold, stressed costs (+25%)

Command:

```bash
cargo run --release --bin ce_sweep -- --parquet "/Users/waff/develop/play/nq/mnq_1m_cont.parquet" --research-london --folds 10 --max-bars 400000 --min-avg-test-trades 20 --commission-mult 125 --slippage-mult 125 --top 8
```

Outcome:

- No configurations survived `min_avg_test_trades >= 20`.

Status: failed

## Final Verdict

`NOT_PROMOTED`

Reason:

- Candidate configurations do not maintain positive out-of-sample net USD under stricter fold validation.
- Results further weaken under a moderate cost-stress test.
- Robust survivor count is insufficient at 10-fold with trade-count guardrails.

## Recommended Next Research (Optional)

- Add higher-timeframe bias and structure confirmation at CE touch.
- Introduce adaptive no-trade filters for low-liquidity/low-volatility windows.
- Re-test using fixed contract sizing and explicit tick-value accounting to mirror execution assumptions.
