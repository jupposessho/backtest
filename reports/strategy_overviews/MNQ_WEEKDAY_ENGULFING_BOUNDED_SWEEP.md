# MNQ Weekday Engulfing Bounded Sweep

- Status: PARTIALLY_TESTED
- Promotion: NOT_PROMOTED
- Strategy: Weekday engulfing rules from Edgeful table (15m MNQ, NY session)
- Runtime: optimized sweep with shared `Arc<Vec<CandleStick>>` + `rayon` parallel config evaluation

## Setup

- Symbol: MNQ continuous 1m parquet, resampled to 15m
- Position sizing: 5 MNQ contracts (`$10` per point)
- Risk gate: max loss `$250` per trade
- Baseline rule family: per-weekday `TP%`, `SL%`, `min/max engulf%`, direction, entry cutoff
- Sweep bounds: TP nudge `0.90/1.00/1.10`, SL nudge `0.90/1.00/1.10`, min engulf shift `-25/0/+25`, max engulf nudge `0.90/1.00/1.10`, cutoff shift `-30/0/+30` minutes

## Key Results

- Total variants tested: 244
- Best variant: `tp=1.10 sl=1.10 min_add=0 max=1 cutoff=0m`
  - trades: 148
  - win rate: 69.59%
  - PF(R): 1.15
  - net points: +86.41
  - net PnL USD: +$864.10
  - payoff ratio (avg win / avg loss): 0.491
- Edgeful exact table baseline: `edgeful_table_exact`
  - trades: 161
  - win rate: 65.22%
  - PF(R): 0.95
  - net points: -11.12
  - net PnL USD: -$111.20
  - payoff ratio: 0.526

## Why Not Promoted

- High win-rate but weak payoff profile in tested neighborhood (wins smaller than losses).
- Robust shortlist gate produced zero candidates:
  - payoff >= 0.7
  - trades >= 120
- Result: no configuration passed the robustness threshold for promotion.

## Artifacts

- Full ranked sweep CSV: `reports/strategy_overviews/MNQ_WEEKDAY_ENGULFING_BOUNDED_SWEEP.csv`
- Robust shortlist CSV: `reports/strategy_overviews/MNQ_WEEKDAY_ENGULFING_BOUNDED_SWEEP_ROBUST.csv`
