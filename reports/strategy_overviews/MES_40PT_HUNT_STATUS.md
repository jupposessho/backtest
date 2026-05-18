# MES 40pt Hunt Status

Date: 2026-05-19

## Objective

- Target: `40 points/week` on `MES` with `1 micro` under realistic fills/costs.

## Latest Structural Switch: ORB Variants

- Runner: `orb_variants_mes`
- Data: `assets/mes_1m_cont.parquet`
- Realism: next-bar-open, conservative intrabar handling, fee model, slippage `1/2/3` ticks.
- Metric: robust `min(points/week)` across slippage `1/2/3`.

## Result

- Best robust setup: `grid_or15_rr2p5_opp_aw120_holdnone` on `1m`
- Robust min points/week: `1.96`
- Gap to target: `38.04`
- Variants with robust `>= 40 points/week`: `0`

## Notes

- A points/week unit bug was corrected in the MES ORB runner (seconds vs milliseconds denominator).
- After the fix, no ORB variant is close to the 40pt/week target.

## Verdict

- Status: `HOLD`
- Reason: no robust MES candidate close to target under realism constraints.
