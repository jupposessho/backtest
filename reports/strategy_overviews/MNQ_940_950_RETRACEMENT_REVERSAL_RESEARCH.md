# MNQ Retracement/Reversal Research (09:40-09:50 +/- 5m)

- Instrument: MNQ continuous 1m
- Data source: `assets/mnq_1m_cont.parquet`
- Timezone: New York
- Window tested: 09:35 to 09:55 (interpreting 09:40-09:50 +/- 5 minutes)
- Threshold: `> 30` points (implemented as `>= 30` points)

## Event Definition

For each trading day with enough data in the window, we measure the largest time-ordered reversal inside 09:35-09:55:

- Down reversal candidate: `high(i) - low(j)` with `j > i`
- Up reversal candidate: `high(j) - low(i)` with `j > i`

If either candidate is at least 30 points, the day is counted as an event day.

## Results

- Days with 09:35-09:55 data: `1289`
- Event days (>=30 pts either direction): `1256` (`97.44%`)
- Down reversal days (>=30 pts): `1110` (`86.11%`)
- Up reversal days (>=30 pts): `1159` (`89.91%`)
- Both directions >=30 pts same day: `1013` (`78.59%`)
- Event size (largest per event day): average `92.81` pts, max `476.50` pts

## Threshold x Window-Pad Sweep (30..100 pts)

- Base window: `09:40-09:50` NY
- Padding: symmetric `+/-N` minutes where `N in [0, 5]`
- Event definition: day counts as event if either down reversal or up reversal reaches threshold
- Threshold sweep: `30` to `100` in `5` point increments

| Pts | 0m (09:40-09:50) | 1m | 2m | 3m | 4m | 5m (09:35-09:55) |
|---:|---:|---:|---:|---:|---:|---:|
| 30 | 90.46% | 92.40% | 93.95% | 95.58% | 96.74% | 97.44% |
| 35 | 84.79% | 88.05% | 90.46% | 91.78% | 94.18% | 95.35% |
| 40 | 77.58% | 83.17% | 85.96% | 87.82% | 90.61% | 92.55% |
| 45 | 69.28% | 76.49% | 80.92% | 83.09% | 85.80% | 88.21% |
| 50 | 62.92% | 69.05% | 73.70% | 77.35% | 80.92% | 84.25% |
| 55 | 56.40% | 62.76% | 67.96% | 71.30% | 75.25% | 78.51% |
| 60 | 49.88% | 56.48% | 61.37% | 66.25% | 69.82% | 73.16% |
| 65 | 43.29% | 48.95% | 54.31% | 59.58% | 63.23% | 67.26% |
| 70 | 37.24% | 42.90% | 48.02% | 52.60% | 57.56% | 62.37% |
| 75 | 33.05% | 37.63% | 42.82% | 47.48% | 52.29% | 56.71% |
| 80 | 28.86% | 32.35% | 37.63% | 42.20% | 46.08% | 50.43% |
| 85 | 24.52% | 28.39% | 33.13% | 37.47% | 41.89% | 46.00% |
| 90 | 20.79% | 24.52% | 28.47% | 32.89% | 37.24% | 41.51% |
| 95 | 17.69% | 20.79% | 24.28% | 28.08% | 32.27% | 35.76% |
| 100 | 13.73% | 18.15% | 21.10% | 24.28% | 28.16% | 32.82% |

## Price-Time Entry Signal Scan

Goal: convert the high event frequency into a tradable entry by requiring both a specific minute and a price relationship before entering a 30-point fade.

Execution assumptions for this exploratory scan:

- Entry: next minute open after the signal bar
- Direction: fade the extension/touch
- Target: `30` points
- Stop/adverse cap: `20` points
- Max hold: `30` minutes, capped at `11:00` NY
- Slippage: `1` tick per side
- Commission: `$1.24` round trip, converted to `0.62` MNQ points
- Same-bar target/stop ambiguity: conservative stop-first handling
- Status: exploratory only, not promoted; needs true strategy implementation and out-of-sample validation before use

Families scanned:

- `open_offset_X`: price touches/reclaims `09:30 open +/- X` points
- `leg_ext_M_minleg_L`: extension of the dominant `09:30-09:39` opening leg; multiples roughly correspond to leg extreme (`1.00`) and negative-fib-style extensions (`1.33`, `1.50`, `1.66`, `2.00`, `2.50`)
- Triggers: `Touch` and `CloseBack`
- Entry minutes: `09:35-10:10` for open-offset anchors; `09:40-10:10` for opening-leg anchors to avoid look-ahead

Top full-dataset rows with at least 50 trades:

| Rank | Exp pts/trade | Trades | Win % | Avg MAE | Entry | Trigger | Family |
|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 5.95 | 81 | 53.09% | 16.25 | 10:04 | CloseBack | open_offset_50 |
| 2 | 5.73 | 96 | 53.12% | 15.35 | 10:08 | CloseBack | open_offset_25 |
| 3 | 4.88 | 60 | 51.67% | 16.75 | 09:52 | CloseBack | leg_ext_1.00_minleg_40 |
| 4 | 4.32 | 57 | 50.88% | 17.18 | 09:43 | CloseBack | open_offset_80 |
| 5 | 4.28 | 63 | 50.79% | 17.47 | 09:47 | CloseBack | leg_ext_1.33_minleg_30 |

Top 2025+ rows with at least 50 trades:

| Rank | Exp pts/trade | Trades | Win % | Avg MAE | Entry | Trigger | Family |
|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 2.61 | 91 | 46.15% | 17.95 | 10:03 | Touch | leg_ext_1.33_minleg_30 |
| 2 | 2.55 | 111 | 46.85% | 18.28 | 10:08 | Touch | leg_ext_1.00_minleg_40 |
| 3 | 2.39 | 94 | 45.74% | 18.19 | 10:03 | Touch | leg_ext_1.33_minleg_0 |
| 4 | 2.39 | 94 | 45.74% | 18.19 | 10:03 | Touch | leg_ext_1.33_minleg_20 |
| 5 | 2.30 | 78 | 46.15% | 18.34 | 10:08 | Touch | leg_ext_1.33_minleg_40 |

Robustness notes:

- No single exact minute looks dominant across all regimes. The useful cluster is later than the original `09:40-09:50` stat: roughly `09:47-10:08`, with 2025+ concentrating near `10:03-10:08`.
- `09:30 open +/- distance` close-back signals produce the strongest full-dataset expectancy, especially `+/-25` to `+/-50` point reclaims around `10:04-10:08`.
- Opening-leg extension signals are more structurally aligned with the manipulation-leg idea. The most stable broad candidate is `10:03 Touch leg_ext_1.33_minleg_30`: all-data `+1.55` pts/trade, pre-2025 `+1.15`, 2025+ `+2.61`, with 335 full-dataset trades.
- Some high-ranked rows are sparse and likely overfit. Example: `10:08 CloseBack open_offset_25` is positive across periods but only has 15 trades in 2025+.
- Some 2025+ winners fail pre-2025. Example: `10:08 Touch leg_ext_1.00_minleg_40` is `+2.55` pts/trade in 2025+ but `-1.02` pre-2025.

Current best lead:

- Use the opening-leg extension family, not the raw 06:00-09:00 sweep condition, as the next research path.
- Focus on entries after `10:00`, especially `10:03`, when price has touched the `1.33x` extension of the `09:30-09:39` dominant opening leg with minimum leg size around `30` points.
- Treat `09:30 open +/-25` to `+/-50` close-back around `10:04-10:08` as a secondary idea, but it needs more sample or looser timing before promotion.

## Limit-Fade Level Scan

Goal: test whether the 30-point reversal fact becomes tradable by placing a resting fade order at a predefined price level, instead of waiting for an exact signal minute.

Execution assumptions:

- Entry: first touch of level inside the allowed window
- Fill: conservative limit-style fade with `1/2/3` tick slippage sensitivity
- Direction: fade away from the level
- Same-bar ambiguity: stop-first
- Commission: `$1.24` round trip
- Metrics: points after slippage and commission
- Status: exploratory only; not promoted until implemented as a full strategy with walk-forward/holdout validation

Families scanned:

- `open_offset_X`: fade `09:30 open +/- X` points
- `leg_ext_M_minleg_L`: fade extension of the dominant `09:30-09:39` opening leg
- `sixnine_offset_X`: fade `06:00-09:00` range high/low plus offset
- `sixnine_range_ext_M`: fade extension of the `06:00-09:00` range

Most useful robust candidate found:

| Candidate | Period | Slip | Exp pts/trade | Trades | Win % | Avg MAE |
|---|---|---:|---:|---:|---:|---:|
| `09:40-09:50 open_offset_100, target 30, stop 25, hold 15m` | All | 1 tick | 3.82 | 276 | 51.81% | 19.52 |
| same | All | 2 ticks | 3.11 | 276 | 51.45% | 19.81 |
| same | All | 3 ticks | 2.40 | 276 | 51.09% | 19.99 |
| same | Pre-2025 | 1 tick | 4.50 | 157 | 51.59% | 18.41 |
| same | Pre-2025 | 3 ticks | 3.11 | 157 | 50.96% | 19.02 |
| same | 2025+ | 1 tick | 2.92 | 119 | 52.10% | 21.00 |
| same | 2025+ | 3 ticks | 1.45 | 119 | 51.26% | 21.26 |

Interpretation: this is the first scan result that is both simple and positive across pre-2025, 2025+, and 1-3 tick slippage. It says: if MNQ is `100` points away from the `09:30` open during `09:40-09:50`, fade it with a `30` point target and `25` point stop, exiting after `15` minutes if unresolved.

Most useful manipulation-leg candidate:

| Candidate | Period | Slip | Exp pts/trade | Trades | Win % | Avg MAE |
|---|---|---:|---:|---:|---:|---:|
| `09:45-10:00 leg_ext_1.66_minleg_50, target 30, stop 25, hold 45m` | All | 1 tick | 3.45 | 159 | 53.46% | 20.15 |
| same | All | 2 ticks | 2.60 | 159 | 52.83% | 20.45 |
| same | All | 3 ticks | 1.41 | 159 | 51.57% | 20.73 |
| same | Pre-2025 | 1 tick | 2.50 | 97 | 51.55% | 20.47 |
| same | Pre-2025 | 3 ticks | 0.36 | 97 | 49.48% | 21.09 |
| same | 2025+ | 1 tick | 4.93 | 62 | 56.45% | 19.66 |
| same | 2025+ | 3 ticks | 3.04 | 62 | 54.84% | 20.16 |

Interpretation: this is closer to the original manipulation-leg hypothesis. It fades the `1.66x` extension of the dominant `09:30-09:39` leg, but only when that leg is at least `50` points. It is positive across periods, but pre-2025 profitability becomes thin under `3` ticks per side.

Rejected/weak observations:

- The strongest 2025+ `50` point target rows at `leg_ext_1.66` were negative pre-2025, so they look regime-specific rather than robust.
- The `06:00-09:00` range families did not dominate the robust top rows. This supports the prior conclusion: the old 06:00-09:00 relationship is not the best direct anchor for execution.
- Exact-minute confirmation entries remain weaker than resting level fades.

Current best next implementation target:

- Implement a real strategy variant for `open_offset_100` and `leg_ext_1.66_minleg_50` as separate modules or switchable configs.
- Use next-bar/limit realism, gap-through stop handling, 1/2/3 tick slippage, and commission.
- Validate with walk-forward folds and weekly/monthly stability before any promotion.

## Fixed Strategy Validation: `open_offset_100`

Implemented a dedicated validation runner for the strongest simple limit-fade lead.

Rule:

- Anchor: `09:30` open
- Entry levels: `09:30 open + 100` short, `09:30 open - 100` long
- Entry window: `09:40-09:50` NY
- Entry handling: first touch, one trade per day
- Target: `30` points
- Stop: `25` points
- Time exit: `15` minutes
- Commission: `$1.24` round trip
- Slippage: `1/2/3` ticks per side
- Gap-through stops: if bar opens beyond stop, fill at open plus exit slippage
- Same-bar ambiguity: stop-first

Validation result: rejected.

| Period | Slip | Trades | Win % | Gross pts | Net pts | Net exp/trade | Positive months |
|---|---:|---:|---:|---:|---:|---:|---:|
| All | 1 tick | 276 | 41.67% | -1857.75 | -2097.87 | -7.60 | 41.82% |
| All | 2 ticks | 276 | 40.94% | -1939.75 | -2248.87 | -8.15 | 40.00% |
| All | 3 ticks | 276 | 40.94% | -1952.25 | -2330.37 | -8.44 | 40.00% |
| Pre-2025 | 1 tick | 157 | 45.22% | -430.00 | -566.59 | -3.61 | 52.50% |
| Pre-2025 | 3 ticks | 157 | 44.59% | -497.50 | -712.59 | -4.54 | 52.50% |
| 2025+ | 1 tick | 119 | 36.97% | -1427.75 | -1531.28 | -12.87 | 13.33% |
| 2025+ | 3 ticks | 119 | 36.13% | -1454.75 | -1617.78 | -13.59 | 6.67% |

Walk-forward fold results at 1 tick slippage:

| Fold | Dates | Trades | Net pts | Exp/trade | Win % |
|---:|---|---:|---:|---:|---:|
| 1 | 2021-03-03 to 2022-03-01 | 34 | -208.83 | -6.14 | 41.18% |
| 2 | 2022-03-02 to 2023-03-03 | 62 | 121.31 | 1.96 | 50.00% |
| 3 | 2023-03-05 to 2024-03-03 | 16 | -59.17 | -3.70 | 31.25% |
| 4 | 2024-03-04 to 2025-03-03 | 60 | -669.95 | -11.17 | 41.67% |
| 5 | 2025-03-04 to 2026-03-03 | 104 | -1281.23 | -12.32 | 38.46% |

Realism verdict:

- `REJECTED`
- The prior scan was too optimistic because it was still a research approximation, not a full fixed-rule validation.
- Under realistic entry/exit handling, the edge disappears. The largest failure is 2025+, where trade frequency rises but expectancy becomes strongly negative.
- The setup catches days with large opening displacement, but a blind fade of `+/-100` is too early or too structurally naive. It needs either a better confirmation filter, a different stop model, or a later entry after rejection is visible.

Implication for next research:

- Do not promote raw `open_offset_100` as a tradable model.
- Re-test the manipulation-leg extension family (`leg_ext_1.66_minleg_50`) with the same dedicated realism runner before considering implementation.
- Explore confirmation after touch: close-back, failed continuation, second touch, or reclaim of the `09:40-09:50` range midpoint after the extreme.

## Fixed Strategy Validation: `leg_ext_1.66_minleg_50`

Completed the next-step strict validation using the same realism protocol as `open_offset_100`.

Rule:

- Build dominant `09:30-09:39` opening leg
- Require leg size `>= 50` points
- Compute extension at `1.66x` from `09:30` open in leg direction
- Fade first touch of that extension during `09:45-10:00`
- Target `30`, stop `25`, time exit `45` minutes, one trade/day
- Same realism: gap-through stop open fills, stop-first same-bar ordering, commission `$1.24`, slippage `1/2/3` ticks

Validation result: conditionally viable, but not yet promoted.

| Period | Slip | Trades | Win % | Gross pts | Net pts | Net exp/trade | Positive months |
|---|---:|---:|---:|---:|---:|---:|---:|
| All | 1 tick | 159 | 50.94% | 496.97 | 358.64 | 2.26 | 62.00% |
| All | 2 ticks | 159 | 50.31% | 441.47 | 263.39 | 1.66 | 62.00% |
| All | 3 ticks | 159 | 49.69% | 385.97 | 168.14 | 1.06 | 60.00% |
| Pre-2025 | 1 tick | 97 | 47.42% | 121.97 | 37.58 | 0.39 | 56.76% |
| Pre-2025 | 2 ticks | 97 | 46.39% | 66.47 | -42.17 | -0.43 | 56.76% |
| Pre-2025 | 3 ticks | 97 | 46.39% | 65.97 | -66.92 | -0.69 | 56.76% |
| 2025+ | 1 tick | 62 | 56.45% | 375.00 | 321.06 | 5.18 | 76.92% |
| 2025+ | 2 ticks | 62 | 56.45% | 375.00 | 305.56 | 4.93 | 76.92% |
| 2025+ | 3 ticks | 62 | 54.84% | 320.00 | 235.06 | 3.79 | 69.23% |

Walk-forward folds (chronological 5-fold, 1 tick):

| Fold | Dates | Trades | Net pts | Exp/trade | Win % |
|---:|---|---:|---:|---:|---:|
| 1 | 2021-03-03 to 2022-03-01 | 24 | 39.12 | 1.63 | 50.00% |
| 2 | 2022-03-02 to 2023-03-03 | 37 | 78.47 | 2.12 | 51.35% |
| 3 | 2023-03-05 to 2024-03-03 | 14 | 49.14 | 3.51 | 50.00% |
| 4 | 2024-03-04 to 2025-03-03 | 35 | 29.55 | 0.84 | 48.57% |
| 5 | 2025-03-04 to 2026-03-03 | 49 | 162.37 | 3.31 | 53.06% |

Walk-forward folds (3 ticks):

| Fold | Net pts | Exp/trade |
|---:|---:|---:|
| 1 | -27.88 | -1.16 |
| 2 | 59.47 | 1.61 |
| 3 | 41.64 | 2.97 |
| 4 | -42.95 | -1.23 |
| 5 | 137.87 | 2.81 |

Realism verdict:

- `PARTIALLY VIABLE`
- Strong in 2025+ and positive on full dataset, but pre-2025 turns slightly negative at 2-3 tick slippage.
- Better than `open_offset_100` by a large margin under the same realism checks.
- Not robust enough yet for promotion because slippage sensitivity around pre-2025 is marginal and fold dispersion remains high.

Immediate next step:

- Keep this as base candidate and add one structural confirmation filter (close-back or failed continuation) to reduce adverse entries.
- Re-run the same strict protocol and require pre-2025 non-negative at 2 ticks plus positive fold-aggregate stability.

## Confirmation Filter Test: Close-Back

Applied the requested next step on top of `leg_ext_1.66_minleg_50`:

- Require touch of the extension **and** close-back across the level in `09:45-10:00`
- Enter next bar after confirmation
- Keep same exits and realism protocol (`TP=30`, `SL=25`, `hold=45m`, fee, 1/2/3 tick slippage, stop-first, gap-through)

Result: this improves robustness and passes the practical gate.

| Period | Slip | Trades | Win % | Net pts | Net exp/trade | Positive months |
|---|---:|---:|---:|---:|---:|---:|
| All | 1 tick | 132 | 56.06% | 686.73 | 5.20 | 68.75% |
| All | 2 ticks | 132 | 55.30% | 598.48 | 4.53 | 68.75% |
| All | 3 ticks | 132 | 54.55% | 510.23 | 3.87 | 66.67% |
| Pre-2025 | 1 tick | 82 | 53.66% | 330.23 | 4.03 | 66.67% |
| Pre-2025 | 2 ticks | 82 | 52.44% | 254.48 | 3.10 | 66.67% |
| Pre-2025 | 3 ticks | 82 | 52.44% | 233.73 | 2.85 | 66.67% |
| 2025+ | 1 tick | 50 | 60.00% | 356.50 | 7.13 | 75.00% |
| 2025+ | 2 ticks | 50 | 60.00% | 344.00 | 6.88 | 75.00% |
| 2025+ | 3 ticks | 50 | 58.00% | 276.50 | 5.53 | 66.67% |

Walk-forward fold check (all data):

- 1 tick: `[-0.87, 11.45, 6.17, 2.58, 5.75]` pts/trade by fold
- 2 ticks: `[-3.62, 11.20, 5.90, 2.33, 5.50]`
- 3 ticks: `[-3.87, 10.95, 5.64, 0.18, 5.25]`

Interpretation:

- One early fold remains weak/negative, but aggregate pre-2025 and 2025+ are both positive across 1-3 ticks.
- Compared with unfiltered `leg_ext_1.66_minleg_50`, close-back reduces trade count and materially improves expectancy and monthly profile.
- Compared with `open_offset_100`, this is decisively better under strict realism.

Current status:

- `CANDIDATE_FOR_PROMOTION_AFTER_FINAL_HARDENING`
- Next hardening step: add a simple continuation-failure guard (e.g., no immediate re-break of extension in next 1-2 bars) and re-check fold-1 weakness.

## Architecture Expansion Check: 09:35 Multi-Module Blends (2025+)

Given the weekly target (`80-100` pts) was not reachable in the prior `09:40-09:50` scheduler family, we ran a broader architecture check with independent `09:35` modules and multi-entry variants.

Runs:

```bash
cargo run --release --example mnq_935_portfolio_blend_weekly
cargo run --release --example mnq_935_combined_modules_weekly
cargo run --release --example mnq_935_filtered_multi_entry_weekly
```

Key outputs:

- `mnq_935_portfolio_blend_weekly` (daily cap sweep): best `avg_pts_w = 12.14`, `pct_ge_100 = 4.76%`.
- `mnq_935_combined_modules_weekly` (reversal + continuation): best `avg_pts_w = 21.19`, `pct_ge_100 = 14.29%`, `max_week = 150`.
- `mnq_935_filtered_multi_entry_weekly` (single filtered config with trade/cooldown grid): best observed `avg_pts_w = 18.57` (`max_trades=6`, `cooldown=3`), `pct_ge_100 = 14.29%`.

Interpretation:

- Throughput increases versus the earlier strict `09:40-09:50` family, but distribution still misses the objective by a wide margin.
- Even the best 2025+ averages cluster around `~12` to `~21` pts/week, far below `80-100`.
- This confirms the same bottleneck: more lanes help, but current lane quality/correlation is still insufficient for the weekly target under strict realism.

Decision update:

- Keep current `leg_ext_1.66 + close-back` as a valid submodule.
- Continue with architecture expansion (additional independent setup families and session windows), not parameter micro-tuning inside the same module family.

## Session Expansion Check: 09:00-09:15 Transition Family

To test a pre-open to open-transition lane as an additional independent family, we ran three dedicated `09:00-09:15` examples.

Runs:

```bash
cargo run --release --example mnq_900_915_transition_strategy
cargo run --release --example mnq_900_915_extension_scan
cargo run --release --example mnq_900_915_transition_robustness
```

Results summary:

- `mnq_900_915_transition_strategy`:
  - Some OOS slices show small positive expectancy, but IS is consistently negative in top rows.
  - Best displayed OOS row: `exp=+0.164R` with `wr=14.04%` (high-variance profile, weak for production).
- `mnq_900_915_extension_scan` (event tendency, not execution-ready strategy):
  - `1.0R` band combined reversal rate: `69.79%` (`1414/2026`).
  - `2.0-2.5R` band combined reversal rate: `54.50%` (`927/1701`).
  - `4.0-4.5R` band combined reversal rate: `32.21%` (`381/1183`).
  - Interpretation: reversal tendency decays sharply as extension deepens.
- `mnq_900_915_transition_robustness` (rolling WFO gate):
  - All tested configs fail robustness (`pass_folds=0/6`).
  - OOS weighted expectancy is negative for every top candidate (best shown `-0.126R`).

Verdict:

- `09:00-09:15` transition family is not promotable under current rules.
- It may remain useful as a context/regime feature (state input), but not as a standalone tradable lane for weekly target contribution.
- Keep portfolio focus on stronger families (`09:35` module blends and `leg_ext_1.66 + close-back`) while expanding into other independent NY windows.

## Main Path Continuation: 06:00-09:00 First-Break Reversal Family

To continue adding independent lanes, we tested the `06:00-09:00` first-break reversal family with filter and execution scans, then re-checked weekly-target capability in the 2025+ sweep framework.

Runs:

```bash
cargo run --release --example mnq_reversal_filter_scan
cargo run --release --example mnq_reversal_exec_scan
cargo run --release --example mnq_reversal_2025plus_100pts_week_sweep
```

Key findings:

- `mnq_reversal_filter_scan` (pattern quality layer):
  - Baseline first-break fade: `win_rate=55.74%`, `expectancy=0.115R`.
  - Strong filter candidates appear at the pattern layer (for example reclaim window + capped overshoot + compressed range), with best listed around `expectancy~0.38R` and `~250-300` trades.
  - But the same run's net ranking under next-open execution and stop-first handling is still negative even before fees/slippage (best shown `net_exp=-0.096R`).
- `mnq_reversal_exec_scan` (execution variants):
  - Top rows show large positive `exp` with very low win rate (around `9-12%`), indicating these rows are not directly comparable to weekly points without a dedicated realism-normalized validator.
  - Treat this output as directional hypothesis generation, not promotion evidence.
- `mnq_reversal_2025plus_100pts_week_sweep` (objective-level check):
  - Exhaustive run over `10,584` combos on `2025+` (`365` days).
  - Best result: `avg_pts_w=11.00`, `median_pts_w=3.76`, `%weeks>=100=16.13%`.
  - This remains far below the target distribution (`80-100` avg weekly points).

Verdict:

- `06:00-09:00` first-break reversal remains non-promotable for the weekly objective under strict execution realism.
- Pattern-level edges do not survive translation to realistic execution quality at required weekly scale.
- Keep as secondary research branch only; do not allocate primary portfolio weight.

Main-path implication:

- The objective gap persists across `09:40-09:50`, `09:35` blends, `09:00-09:15` transition, and `06:00-09:00` first-break families.
- Next expansion should prioritize genuinely independent post-open lanes (for example `10:00-11:30` and late-session mean-reversion/rejection families) with immediate weekly-objective scoring under strict realism gates.

## Main Path Tests: Post-Open + Late-Session Families

Executed the requested next block for post-open and late-session candidates.

Runs:

```bash
cargo run --release --example mnq_reversal_mtf_scan
cargo run --release --example mnq_reversal_htf_orderflow_scan
cargo run --release --example mnq_zone_reversal_scan
cargo run --release --example mnq_zone_reversal_strategy
cargo run --release --example mnq_killzone_midpoint_strategy
cargo run --release --example mnq_killzone_relationships
```

### Post-open reversal scans (`10:00-11:30` candidate families)

- `mnq_reversal_mtf_scan`:
  - Broadly negative across most pattern families under realistic costs.
  - Best observed row (`3m ob`, `rr=2`, `tstop=11`, `slip=1`, `comm=0.25`) is only `exp=+0.052R` with `323` trades.
- `mnq_reversal_htf_orderflow_scan`:
  - HTF structure gating improves some OB rows but remains sparse.
  - Best row (`5m ob + htf_structure`, `rr=2`, `tstop=11`, `slip=1`, `comm=0.25`) is `exp=+0.133R` with only `49` trades.

Interpretation: post-open pattern lanes show small positive pockets, but current quality and sample depth are insufficient for direct weekly-target contribution.

### Zone and killzone late-session lanes

- `mnq_zone_reversal_scan` (event tendency layer):
  - `0.33-0.66R` zone combined EOD reversal rate: `42.80%`.
  - `1.33-1.66R` zone combined EOD reversal rate: `17.57%`.
  - Reversal probability decays sharply with deeper zone extension.
- `mnq_zone_reversal_strategy` (execution layer):
  - No robust IS/OOS-positive configuration.
  - Typical pattern: IS negative, OOS mixed/small-positive, not stable enough for promotion.
- `mnq_killzone_midpoint_strategy`:
  - `LUNCH->NYPM` subgroup shows promising OOS (`exp=+0.819R`, `n=47`),
  - but corresponding IS is weak (`exp=+0.167R` with large drawdown) and combined variants remain IS negative.
  - Robustness slice shows non-trivial regime dispersion (`2026` OOS negative in shown best subgroup).
- `mnq_killzone_relationships`:
  - Relationship stats confirm strong structural rejection frequencies (especially midpoint reclaim behaviors),
  - but this is descriptive/event-level evidence and not a tradable weekly-objective pass by itself.

### Verdict for this test block

- No new family in this block clears promotion standards for the weekly objective.
- Best actionable signal from this block is to treat midpoint relationship features as filters/inputs for stronger existing modules, not standalone lanes yet.
- Weekly target gap remains open; no tested post-open or late-session family here materially closes it under strict realism.

## Fusion Test: 09:35 Core + Midpoint Relationship Filter

Implemented and ran a direct fusion pass using the requested default direction:

- Base core: `mnq_935_combined_modules_weekly` logic (reversal + continuation blend)
- Added filter: prior-day `LUNCH->NYPM` midpoint reclaim gate approximating the best midpoint candidate profile (`reclaim<=1`, `stop_cap~20%`, `body>=40%`, `rr>=0.30`)
- Realism in fused runner: slippage sweep `1/2/3` ticks and commission `0.62` points RT
- File: `examples/mnq_935_midpoint_fusion_weekly.rs`
- Run:

```bash
cargo run --release --example mnq_935_midpoint_fusion_weekly
```

Results (2025+):

- Best row: `cont_stop=20`, `cont_target=30`, `slip=1`
  - `avg_pts_w=13.17`, `med_pts_w=11.90`, `%weeks>=80=25.00`, `%weeks>=100=12.50`, `worst_week=-64.34`
- At `slip=2`: best `avg_pts_w=11.21`
- At `slip=3`: best `avg_pts_w=5.59`
- Many parameter rows turn negative by `slip=3`.

Interpretation:

- The midpoint gate did not improve the weekly objective; it reduced sample/throughput (only `8` weeks represented, `10` qualified days) and lowered mean weekly points versus ungated core.
- This fusion path does not close the `80-100 pts/week` gap and is not promotable as-is.

## Follow-up Fusion: Same-Day Non-Lookahead Gate

Ran a stricter same-day gate to avoid prior-day carryover dependence:

- Gate logic: `06:00-09:00` first-break reclaim by `09:35`, reclaim within `<=2` bars, overshoot cap `<=35%` of `06:00-09:00` range.
- Integrated into the same `09:35` core blend and scored on weekly objective metrics.
- File: `examples/mnq_935_same_day_gate_fusion_weekly.rs`

Run:

```bash
cargo run --release --example mnq_935_same_day_gate_fusion_weekly
```

Result: this version is worse than both baseline and prior midpoint fusion.

- Best displayed row: `avg_pts_w=-4.76` (still negative), `med_pts_w=-12.44`, `%weeks>=80=12.50`, `%weeks>=100=12.50`.
- Several rows are deeply negative with large downside tails (worst weeks near `-150` points).

Verdict:

- Same-day gate as implemented is rejected.

## Union-Lanes + Weekly-Objective Ranking

Implemented requested combined experiment:

- Union architecture: keep `09:35` core lane always tradable, add optional independent `LUNCH->NYPM` midpoint lane as an additive module.
- Objective-first ranking:
  - primary: `% weeks >= 80`
  - secondary: `% weeks >= 100`
  - tertiary: worst week
  - then average week
- File: `examples/mnq_union_lanes_weekly_objective.rs`

Run:

```bash
cargo run --release --example mnq_union_lanes_weekly_objective
```

Top row from this ranking:

- `use_lane_b=true, cont_stop=20, cont_target=30, slip=2`
- `avg_pts_w=26.84`, `med_pts_w=35.04`, `%weeks>=80=33.33`, `%weeks>=100=28.57`, `worst_week=-152.44`

Interpretation:

- Directional improvement vs prior non-union baseline (`~10-21` avg/week).
- Still far from target (`80-100` avg/week) and still carries severe downside-tail risk.

Current status update:

- `UNION_LANES_OBJECTIVE_PASS_PARTIAL` (improved ranking metrics)
- `TARGET_NOT_MET` (weekly mean and tail risk still fail promotion standard)

## Repro

Run:

```bash
cargo run --release --example mnq_940_950_reversal_retracement_stats
```

Code:

- `examples/mnq_940_950_reversal_retracement_stats.rs`
- `examples/mnq_940_price_time_signal_scan.rs`
- `examples/mnq_940_limit_fade_level_scan.rs`
- `examples/mnq_open_offset_100_validation.rs`
- `examples/mnq_legext_166_validation.rs`
- `examples/mnq_legext_166_closeback_validation.rs`
