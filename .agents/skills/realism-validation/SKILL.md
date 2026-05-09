# Skill: realism-validation

Use this skill at the start of any backtest implementation, optimization, or performance report.

Primary source of truth: `REALISM_VALIDATION.md`.

## Goal

Prevent optimistic/fantasy backtests. Enforce realistic execution assumptions and report gross vs net robustness before promoting any strategy result.

## Mandatory Rules

1. Costs are required.
- Always include commission/fee assumptions in runs.
- Report gross and net results side-by-side.

2. Slippage is required.
- Run at least 1 tick/side baseline.
- Run stress at 2 and 3 ticks/side.
- If edge collapses at 2 ticks, mark result execution-fragile.

3. Entry realism is required.
- Prefer next-bar-open capable entry models for realism baselines.
- If using level/limit entries (PrevOpen/OB levels), track and report fill rate and adverse slippage assumptions.

4. Stop realism is required.
- Implement gap-through stop behavior using open +/- slippage when price opens beyond stop.

5. Intrabar tie-breaker must be conservative.
- If TP and SL can both be touched in one bar, resolve as SL first.

6. Data integrity checks are required.
- Validate OHLC sanity.
- Validate monotonic timestamps and no duplicates.
- Validate expected bar spacing.

7. Reporting completeness is required.
- Include: trades, win rate, PF, gross points/R, net points/R, drawdown, and slippage sensitivity table.

8. Architecture discipline.
- Keep strategy signal generation separated from execution simulation and reporting.
- New work should avoid interleaving strategy and brokersim logic where practical.

## Promotion Gates

Do not mark a strategy variant as promoted unless all gates pass:

- Net positive after costs.
- Net positive at 1 tick/side.
- Acceptable degradation at 2 ticks/side.
- No dependency on unrealistic stale-price fills.
- Results stable across at least two time windows or splits.

## Required Output Block

Every backtest summary must include a "Realism Validation" block:

- Fees model used
- Slippage scenarios used
- Entry model realism note
- Gap-stop handling note
- Gross vs net comparison
- Sensitivity conclusion (robust / fragile)

## Fast/Full Workflow

Fast loop:
- Coarse sweep with baseline costs and 1 tick.

Full loop:
- Re-run finalists with full costs and 1/2/3 tick sensitivity.
- Publish realism block and gate verdict.
