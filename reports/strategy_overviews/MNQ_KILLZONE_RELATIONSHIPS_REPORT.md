# MNQ Killzone Range Relationship Research

## Scope

- Instrument: MNQ continuous 1m (`assets/mnq_1m_cont.parquet`)
- Sample: `1292` session days (`2021-03-04` -> `2026-03-03`)
- Killzones:
  - `ASIA`: `20:00-23:59` NY
  - `LONDON`: `02:20-04:59` NY
  - `NYAM`: `09:30-10:59` NY
  - `LUNCH`: `12:00-12:59` NY
  - `NYPM`: `13:30-15:59` NY
- Session-day roll: `20:00` NY, so Asia is attached to the following London/NY date

## Method

For each ordered pair of killzones where the second box is later in the day:

- Build the source box high/low.
- Inside the later target box, detect the first touch of the source high and the first touch of the source low.
- Count a rejection when price touches that source boundary and then closes back inside it before the target box ends.
- Also test the source midpoint:
  - `from above`: price tags the midpoint from above, then reclaims back above it before the target box ends.
  - `from below`: price tags the midpoint from below, then reclaims back below it before the target box ends.
- Measure whether that rejection reaches the opposite side of the source box:
  - by target-box end,
  - by session-day end.

This is a structure study, not a tradable strategy. No fees/slippage were applied.

## Combined Results

| Pair | Touches | Rejections | Reject % | Opp by Target End | Opp by EOD |
| --- | ---: | ---: | ---: | ---: | ---: |
| ASIA -> LONDON | 1399 | 1161 | 82.99% | 20.07% | 77.35% |
| ASIA -> NYAM | 1694 | 1222 | 72.14% | 50.08% | 71.44% |
| ASIA -> LUNCH | 1337 | 483 | 36.13% | 24.64% | 59.83% |
| ASIA -> NYPM | 1429 | 683 | 47.80% | 42.17% | 49.93% |
| LONDON -> NYAM | 1681 | 1311 | 77.99% | 42.72% | 64.84% |
| LONDON -> LUNCH | 1284 | 497 | 38.71% | 14.49% | 50.50% |
| LONDON -> NYPM | 1378 | 673 | 48.84% | 32.39% | 37.74% |
| NYAM -> LUNCH | 815 | 529 | 64.91% | 1.13% | 17.77% |
| NYAM -> NYPM | 1103 | 795 | 72.08% | 8.55% | 13.46% |
| LUNCH -> NYPM | 1554 | 1344 | 86.49% | 31.92% | 39.51% |

## Main Patterns

### 0. Midpoints are cleaner than extremes

Across every pair, midpoint reclaims are more common than high/low rejections and convert to the source extremes much more often.

Examples:

- `ASIA -> NYAM`
  - extreme rejection: `72.14%`
  - midpoint reclaim: `92.52%`
  - midpoint reaches source extreme by NYAM end: `74.42%`

- `LONDON -> NYAM`
  - extreme rejection: `77.99%`
  - midpoint reclaim: `92.05%`
  - midpoint reaches source extreme by NYAM end: `69.11%`

- `LUNCH -> NYPM`
  - extreme rejection: `86.49%`
  - midpoint reclaim: `94.23%`
  - midpoint reaches source extreme by NYPM end: `61.05%`

Interpretation:

- The midpoint is a more reliable continuation / directional acceptance reference than the outer high/low is a full reversal reference.
- If the goal is to model the relationship between time boxes, the midpoint is likely the better anchor for a tradable rule than the extreme alone.

### 1. Overnight highs/lows matter most to the morning sessions

- `ASIA -> LONDON` rejects very often: `82.99%` of touches.
- `ASIA -> NYAM` is also strong: `72.14%` rejection rate, with `50.08%` reaching the opposite side of the Asia box before NYAM ends.
- `LONDON -> NYAM` is similar: `77.99%` rejection rate, `42.72%` opposite-side completion by NYAM end.
- With midpoint added, these are even stronger:
  - `ASIA -> NYAM` midpoint-to-extreme by target end: `74.42%`
  - `LONDON -> NYAM` midpoint-to-extreme by target end: `69.11%`

Interpretation:

- Asia and London look like real liquidity references for the next major session.
- NYAM is the best later box for turning an overnight sweep into an actual two-sided move.

### 2. Lunch rejects prior extremes, but usually does not deliver full follow-through

- `ASIA -> LUNCH`: only `36.13%` of touches reject.
- `LONDON -> LUNCH`: `38.71%` reject.
- Even after rejection, opposite-side completion inside Lunch is weak:
  - `ASIA -> LUNCH`: `24.64%`
  - `LONDON -> LUNCH`: `14.49%`

Interpretation:

- Lunch behaves more like a pause/compression zone than a reliable expansion box.
- It can reject old extremes, but that rejection often needs the afternoon to do anything meaningful.
- But midpoint behavior inside Lunch is still usable:
  - `ASIA -> LUNCH` midpoint reclaim -> source extreme by Lunch end: `48.93%`
  - `LONDON -> LUNCH` midpoint reclaim -> source extreme by Lunch end: `39.69%`

### 3. NYAM extremes are sticky

- `NYAM -> LUNCH`: `64.91%` rejection rate, but only `17.77%` reach the opposite side by EOD.
- `NYAM -> NYPM`: `72.08%` rejection rate, but only `13.46%` reach the opposite side by EOD.

Interpretation:

- Later sessions often wick and reclaim back through NYAM high/low.
- But a rejection of NYAM levels is usually not enough to traverse the full NYAM range.
- NYAM appears structurally dominant: its extremes are often probed and reclaimed, but the full box tends to hold.
- Midpoint version is materially healthier than extreme version:
  - `NYAM -> LUNCH` midpoint reclaim -> NYAM extreme by EOD: `49.93%`
  - `NYAM -> NYPM` midpoint reclaim -> NYAM extreme by EOD: `38.58%`

Interpretation:

- Later sessions often cannot fully traverse NYAM, but they do rotate between the NYAM midpoint and one edge much more often.
- That suggests NYAM may behave less like a full range reversal framework and more like a half-range distribution framework.

### 4. Lunch is highly rejectable in NYPM, but not a full-box traversal setup

- `LUNCH -> NYPM` has the highest rejection rate in the whole scan: `86.49%`.
- But opposite-side completion is still only moderate:
  - `31.92%` by NYPM end
  - `39.51%` by EOD

Interpretation:

- NYPM often fades Lunch extremes.
- That fade is real, but usually not a complete end-to-end traversal of the Lunch box.
- Midpoint improves this relationship a lot:
  - `LUNCH -> NYPM` midpoint reclaim -> Lunch extreme by NYPM end: `61.05%`
  - vs extreme rejection -> opposite side by NYPM end: `31.92%`

### 5. Rejections are usually quick, but afternoon ones take longer

Average bars from first touch to reclaim:

- Fastest: `LONDON -> NYAM` (`7.90` bars), `NYAM -> LUNCH` (`8.13`), `ASIA -> LUNCH` (`8.98`), `ASIA -> NYAM` (`9.27`)
- Slowest: `LONDON -> NYPM` (`24.79`), `ASIA -> NYPM` (`22.54`), `NYAM -> NYPM` (`18.85`)

Interpretation:

- Morning rejections tend to resolve quickly.
- Afternoon interactions with earlier boxes are slower and less decisive.
- Midpoint reclaims are generally much faster than extreme rejections, often in `2-5` bars.

## Practical Takeaways

- Best structural relationship: overnight box (`ASIA` or `LONDON`) swept and rejected during `NYAM`.
- Best structural relationship for extremes: overnight box (`ASIA` or `LONDON`) swept and rejected during `NYAM`.
- Best structural relationship for midpoint continuation: any of `ASIA -> NYAM`, `LONDON -> NYAM`, or `LUNCH -> NYPM`.
- Strongest raw rejection behavior: `LUNCH -> NYPM`, but with only moderate full-box completion.
- Weakest full reversal behavior: any setup expecting `LUNCH` or `NYPM` to fully reverse `NYAM`.
- If you want a full opposite-side traversal, focus on `ASIA/LONDON -> NYAM`, not on later-session fades of NYAM.
- If you want a higher-frequency structural rule, midpoint reclaim is stronger than extreme rejection in every pair tested.

## Files

- Scanner: `examples/mnq_killzone_relationships.rs`
- Prior reference study: `reports/strategy_overviews/MNQ_6_9_ZONE_RESEARCH_REPORT.md`
