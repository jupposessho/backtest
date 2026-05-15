# MNQ Killzone Midpoint Strategy Study (v2)

## Scope

- Instrument: MNQ continuous 1m (`assets/mnq_1m_cont.parquet`)
- Sample: `1292` session days (`2021-03-04` -> `2026-03-03`)
- Split: `904` train / `388` test
- Relationships tested:
  - `ASIA -> NYAM`
  - `LONDON -> NYAM`
  - `LUNCH -> NYPM`
  - `COMBINED` (first valid signal of those three per day)

## Prototype Rule

- Later box tags the earlier box midpoint from one side.
- Price must reclaim back to that same side within `N` bars.
- Entry: next bar open.
- Target:
  - long -> source box high
  - short -> source box low
- Stop:
  - structural sweep extreme during touch->reclaim sequence,
  - capped as a percent of source range.
- Costs:
  - 1 tick slippage,
  - 0.5 point round-trip commission.

Sweep dimensions:

- reclaim window: `1, 2, 3, 5` bars
- stop cap: `20%, 30%, 40%, 50%` of source range
- EMA50 gate: on/off
- exit: source target-zone end or session end
- reclaim body filter: `>=20%, >=30%, >=40%`
- minimum target RR filter: `>=0.30, >=0.50, >=0.70`
- max entry-distance-to-target as % of source range: `<=90%, <=75%, <=60%`
- target-box open must be inside source range: on/off

## Best Observed Results

### 1. LUNCH -> NYPM midpoint (strongest)

Best OOS cluster:

- `reclaim<=1`, `stop_cap=20%`, `ema=false`, `exit=target_end`, `body>=40%`
- Representative result:
  - IS: `n=101`, `WR=21.78%`, `exp=+0.167R`, `maxDD=30.42R`
  - OOS: `n=47`, `WR=25.53%`, `exp=+0.819R`, `maxDD=10.88R`

Interpretation:

- This is the strongest standalone relationship in the study.
- Win rate is low, but the move to the Lunch extreme is large enough relative to risk to support positive expectancy.
- The edge remains concentrated in very fast reclaim behavior (`<=1` bar), now with stronger reclaim-candle body quality.
- OOS robustness snapshot (same best config):
  - by year: `2024 +0.466R`, `2025 +1.209R`, `2026 -0.736R` (small `n=6`)
  - by weekday: strongest on Tue/Wed, weakest on Thu in this OOS slice

### 2. LONDON -> NYAM midpoint (improved but still unstable)

Best OOS cluster:

- `reclaim<=1`, `stop_cap=40%`, `ema=false`, `rr>=0.70`
- Best line:
  - IS: `n=75`, `WR=22.67%`, `exp=-0.180R`, `maxDD=17.88R`
  - OOS: `n=31`, `WR=38.71%`, `exp=+0.107R`, `maxDD=6.51R`

Interpretation:

- Structurally this relationship looked strong in the range study, and it does show positive OOS prototype behavior.
- But the train side remains negative, so this is not robust enough yet.
- Filters reduced drawdown and kept OOS positive, but IS remains negative.
- Candidate status remains `PARTIALLY_TESTED`.

### 3. ASIA -> NYAM midpoint (still negative)

Best line:

- `reclaim<=5`, `stop_cap=50%`, `ema=false`, `exit=target_end`, `body>=30%`, `open_in=true`
- IS: `n=44`, `WR=38.64%`, `exp=-0.287R`, `maxDD=13.72R`
- OOS: `n=33`, `WR=42.42%`, `exp=-0.077R`, `maxDD=10.67R`

Interpretation:

- Despite strong structural midpoint behavior in the earlier scan, the current executable prototype is still negative.
- The move quality is there, but the chosen execution/stop logic is not extracting it cleanly.

### 4. Combined model (OOS stronger, IS still negative)

Best OOS cluster:

- `reclaim<=1`, `stop_cap=40%`, `ema=false`, `body>=40%`, `rr>=0.70`, `open_in=true`
- Best line:
  - IS: `n=157`, `WR=23.57%`, `exp=-0.072R`, `maxDD=49.58R`
  - OOS: `n=81`, `WR=33.33%`, `exp=+0.440R`, `maxDD=12.52R`

Interpretation:

- The combined model is positive OOS, but still negative IS.
- It is being carried mainly by `LUNCH -> NYPM`; the morning relationships are not stable enough yet to lift the full sample.

## Main Conclusions

### 1. Midpoint remains a valid structural anchor

- The earlier relationship study showed midpoint reclaim is much more stable than extreme rejection.
- This prototype work confirms midpoint can support executable setups, especially in `LUNCH -> NYPM`.

### 2. LUNCH -> NYPM is still the current leader

- It is the only relationship that is positive both IS and OOS in the current sweep.
- The best behavior comes from immediate reclaim, without the EMA gate.

### 3. LONDON -> NYAM improved with quality filters but is not robust yet

- OOS is clearly interesting, but the sample is small and IS is still negative.
- This likely needs refined entry quality filters rather than broad parameter expansion.

### 4. ASIA -> NYAM is still not strategy-ready

- Good structural stats did not translate into positive prototype expectancy.
- That suggests the issue is execution design, not necessarily that the relationship is useless.

## Outputs

- Sweep CSV: `reports/strategy_overviews/mnq_killzone_midpoint_strategy_sweep.csv`

## Recommended Next Step

Focus on two paths only:

1. Promote `LUNCH -> NYPM midpoint` to walk-forward validation and month-by-month stability checks.
2. Keep iterating `LONDON -> NYAM midpoint` but only with strict quality gates and minimum sample floors.

Avoid spending more time on the current raw `ASIA -> NYAM` execution template until the entry logic is improved.

## Files

- Structural scan: `examples/mnq_killzone_relationships.rs`
- Prototype sweep: `examples/mnq_killzone_midpoint_strategy.rs`
- Structural report: `reports/strategy_overviews/MNQ_KILLZONE_RELATIONSHIPS_REPORT.md`
