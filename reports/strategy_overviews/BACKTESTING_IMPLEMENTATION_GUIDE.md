# Backtesting Implementation Guide

This is the default workflow for adding and validating new strategy models in this repo.

## 1) Core Principles

- Keep **research** and **champion** tracks separate.
- Optimize for **economic outcomes**, not just PF.
- Always evaluate **out-of-sample** (walk-forward), not just in-sample.
- For futures, report net in **$ per 1 micro contract**; for crypto, net in **%**.
- Prefer bounded, repeatable sweeps over ad-hoc one-offs.

## 2) Standard Metrics (must report)

- `net_result` + `net_unit` (`%` for crypto, `$` for futures)
- `profit_factor`
- `win_rate_%`
- `trades`
- `max_dd_%`
- `total_costs`
- `wf_train_net`
- `wf_test_net`
- `wf_test_pf`
- `final_verdict`

## 3) Minimum Promotion Gates

Use these as baseline gates for challengers:

- `trades >= 40`
- `profit_factor >= 1.2`
- `wf_test_pf >= 1.05`
- `wf_test_net > 0`
- net threshold by market:
  - crypto: `net_% >= 10`
  - futures: `net_$ >= 500` (1 micro equivalent, adjust by horizon)

If a row fails any gate, mark `HOLD`.

## 4) Two-Track Process

### Champion Track

- Keep one current best setup per strategy family.
- Recompute after major code/data changes.
- Do not replace champion unless challenger beats it on agreed gates.

### Research Track

- Run selective experiments only in promising buckets first.
- Explicitly tag each run with setup signature (entry mode, RR, filters, etc.).
- Promote only if challenger passes gates and beats champion.

## 5) Fast Feedback Loop

### Iteration mode

Use fast pass first:

```bash
cargo run --release --bin ttrades_matrix -- --fast
```

- Smaller bar caps
- Fewer slippage levels
- Used only for direction-finding

### Confirmation mode

Then run full pass:

```bash
cargo run --release --bin ttrades_matrix
```

- Final decision must use full pass

## 6) Sweep Design Defaults

### Always include

- Naive gate first (`fee=0`, `slippage=0`)
- Realism sweep (`slippage` ladder + configured fees)
- Walk-forward split (train/test)

### For reversal systems (default family)

- Confirmation families:
  - `cisd_only`
  - `ifvg_only`
  - `cisd_and_ifvg`
  - `cisd_or_ifvg`
- Entry variants:
  - `Close`
  - `ObLevel`
  - `ObMidpoint`
- Time filters:
  - weekday mask
  - killzones (`Off`, `NyOnly`, `LondonNy`)

## 7) Unit Handling Rules

- Crypto rows are ranked and compared by `%` net.
- Futures rows are ranked and compared by `$` net (1 micro).
- Never rank crypto and futures in one blended net score.
- Keep combined report for visibility, but do market-specific ranking files.

## 8) Data & Timeframe Construction

- Futures base source is 1m; construct HTF/LTF by resampling from 1m.
- Supported futures MTF sets currently:
  - `1m/15m`
  - `5m/1h`
  - `15m/4h`
- Crypto uses Binance JSON timeframe files directly.

## 9) Runtime & Performance Practices

- Always run release builds (`--release`).
- Load each dataset once and share with `Arc<Vec<CandleStick>>`.
- Use bounded parallelism (`min(available, 8)`).
- Skip deeper realism sweeps when naive gate is clearly bad.

## 10) Report Outputs (single source of truth)

Use `reports/strategy_overviews/`:

- `STRATEGY_VALIDATION_MATRIX.md`
- `TTRADES_FULLY_TESTED_RANKING.md`
- `PROMOTION_SHORTLIST.md`
- `CHAMPION_BASELINE.md`
- `CHALLENGER_RESULTS.md`
- `STRATEGY_BEST_SETUP_SUMMARY.md`

If you change matrix logic, regenerate all downstream summaries from the latest matrix.

## 11) Suggested Implementation Checklist for New Models

1. Implement strategy with configurable entry/confirmation/time filters.
2. Add runner and matrix integration.
3. Add naive gate + realism sweep + walk-forward columns.
4. Add market-aware net unit handling.
5. Run `--fast`, inspect promising buckets.
6. Run full matrix for finalists.
7. Update champion/challenger reports.
8. Make go/no-go decision; then move to next model.

## 12) Common Failure Modes

- High PF with too few trades (not economically useful).
- Great IS, weak OOS (`wf_test_pf < 1`).
- Opportunity increase that destroys edge quality.
- Mixing crypto `%` and futures `$` in one ranking.
- Stale downstream reports after matrix changes.

## 13) Quick Commands

```bash
# Fast exploration pass
cargo run --release --bin ttrades_matrix -- --fast

# Full decision pass
cargo run --release --bin ttrades_matrix

# Optional targeted research
cargo run --release --bin ttrades_targeted_grid
```
