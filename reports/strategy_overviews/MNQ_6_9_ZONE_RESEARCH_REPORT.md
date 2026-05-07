# MNQ 6-9 Range Zone Research

## Scope

- Instrument: MNQ continuous 1m (`assets/mnq_1m_cont.parquet`)
- Session anchor range: `06:00-08:59` New York time
- Research focus: extension-zone reversal behavior and a tradable first-touch strategy

## Zone Touch/Reversal Study

Definition:

- Zone touch: first touch (per side/day) of extension zone beyond the 6-9 range
- Reversal (EOD): after zone touch, price reaches opposite side of the 6-9 range by end of day
- Reversal (by 12:00): same criterion but completed before 12:00 NY

Sample size:

- Valid days: `1292`

Results:

### Zone `0.33-0.66R`

- Top first-touch reversals:
  - EOD: `354 / 849` = `41.70%`
  - By 12:00: `206 / 849` = `24.26%`
- Bottom first-touch reversals:
  - EOD: `368 / 838` = `43.91%`
  - By 12:00: `209 / 838` = `24.94%`
- Combined:
  - EOD: `722 / 1687` = `42.80%`
  - By 12:00: `415 / 1687` = `24.60%`

### Zone `1.33-1.66R`

- Top first-touch reversals:
  - EOD: `65 / 418` = `15.55%`
  - By 12:00: `17 / 418` = `4.07%`
- Bottom first-touch reversals:
  - EOD: `87 / 447` = `19.46%`
  - By 12:00: `20 / 447` = `4.47%`
- Combined:
  - EOD: `152 / 865` = `17.57%`
  - By 12:00: `37 / 865` = `4.28%`

Takeaway:

- `0.33-0.66R` is materially more reversal-prone than `1.33-1.66R`.
- Deep extension (`1.33-1.66R`) behaves more like continuation than reversal.

## First-Touch Strategy Prototype (0.33-0.66R)

Base strategy rules tested:

- First touch of `0.33-0.66R` zone only
- Reclaim + micro-MSS confirmation
- EMA50/EMA100 directional gate
- One trade per day max
- Costs: 1 tick slippage, 0.5 point round-trip commission
- Risk/exit structure:
  - Stop capped as a percent of 6-9 range
  - Partial TP at 1R; remainder targets opposite side of 6-9 range
  - Time exit by 12:00 NY

Tuning grid:

- Zone: `0.25-0.55`, `0.33-0.66`, `0.40-0.70`
- Confirm deadline: `10:59`, `11:59`
- Stop cap: `15%`, `20%`, `25%`
- TP1 fraction: `25%`, `33%`, `50%`

Chronological split:

- In-sample / out-of-sample: `70/30`

Best observed config (by OOS expectancy):

- `zone=0.33-0.66R`, `confirm<=10:59`, `stop_cap=20%`, `tp1=50%`
- IS: `n=30`, `WR=23.33%`, `exp=-0.118R`, `maxDD=14.85R`
- OOS: `n=17`, `WR=29.41%`, `exp=+0.120R`, `maxDD=3.61R`

Interpretation:

- OOS is positive but sample is small (`17` trades).
- IS remains negative in top configs.
- Current result is exploratory, not robust enough for deployment.

## Conclusion

- The zone behavior itself is informative and repeatable:
  - `0.33-0.66R` is the only extension band with meaningful reversal probability.
- The current tradable implementation around this zone is not yet robust.
- Next phase should prioritize robustness checks (rolling walk-forward and minimum OOS trade floor), not additional curve-fitting.

## Files Used

- `examples/mnq_zone_reversal_scan.rs`
- `examples/mnq_zone_reversal_strategy.rs`
