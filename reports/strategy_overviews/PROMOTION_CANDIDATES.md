# Promotion Candidates

Selection rules used in this pass:

- Minimum walk-forward test trades: `>= 10`
- Ranked by `wf_test_net` then `wf_test_pf`
- Tagging:
  - `PROMOTE`: `FULLY_TESTED` and (`wf_test_net > 0`, `wf_test_pf >= 1.5`)
  - `WATCH`: positive `wf_test_net` with `wf_test_pf >= 1.2`, but not promotion-grade yet
  - `REJECT`: everything else

## Shortlist

| tag | source | strategy/config | asset | timeframe | wf_test_net | wf_test_pf | test_trades | notes |
|---|---|---|---|---|---:|---:|---:|---|
| PROMOTE | validation_matrix | ttrades_fractal_mtf | SOL | 15m/4h | 28.66 | 2.30 | 90 | FULLY_TESTED candidate from matrix |
| WATCH | ce_sweep | london_only_s8_rr2.5/1.2_w3_h48_t1_v1_q0_r0 | MNQ | 1m->5m | 55.94 | 1.89 | 11 | Best CE OOS net, but train side negative |
| WATCH | ce_sweep | london_only_s8_rr2.5/1.2_w3_h36_t1_v1_q0_r0 | MNQ | 1m->5m | 50.93 | 1.81 | 11 | Similar shape, still weak train robustness |
| WATCH | ce_sweep | london_ny_s8_rr2.5/1.2_w3_h48_t1_v1_q0_r0 | MNQ | 1m->5m | 42.60 | 1.57 | 12 | Better trade count than many CE rows |
| WATCH | validation_matrix | ttrades_fractal_mtf | SOL | 15m/4h | 8.13 | 4.73 | 79 | Very high PF, lower net than top SOL config |
| WATCH | validation_matrix | ttrades_fractal_mtf | ETH | 5m/1h | 4.32 | 2.29 | 73 | Positive WF metrics, PARTIALLY_TESTED |
| WATCH | validation_matrix | ttrades_fractal_mtf | SOL | 15m/4h | 2.92 | 1.95 | 32 | Smaller edge, still positive |
| REJECT | validation_matrix | ttrades_fractal_mtf | ETH | 15m/4h | 8.94 | 1.27 | 157 | Positive WF test but negative overall net (`-14.40`) |

## Recommendation

- Promote only the `SOL 15m/4h` `ttrades_fractal_mtf` configuration with `FULLY_TESTED` verdict.
- Keep CE rows in `WATCH` until train-side robustness improves (current top CE rows have strongly negative train net).
- Next gate to apply before final promotion commit:
  - increase min test trades to `>= 20`,
  - rerun with stricter fee/slippage assumptions,
  - require non-negative train net for CE candidates.
