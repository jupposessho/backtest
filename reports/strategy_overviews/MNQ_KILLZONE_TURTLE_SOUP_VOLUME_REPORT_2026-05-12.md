# MNQ Killzone Turtle Soup + Volume Report (2026-05-12)

## Scope
- Instrument/data: MNQ continuous 1m CSV with volume (`/Users/waff/develop/play/nq/mnq_1m_cont.csv`)
- Entries: Turtle soup at killzone session levels (ASIA/LONDON/NYAM/NYPM)
- Timeframes tested: 1m and 3m
- Realism: fees + slippage (1/2/3 ticks per side), SL-first intrabar tie-break, next-bar-open entry

## Key Fixes Applied
1. Fixed tick-size bug that previously forced zero trades.
   - Wrong: `Decimal::from_parts(25, 2, 0, false, 2)`
   - Correct: `Decimal::from_parts(25, 0, 0, false, 2)` (= 0.25)
2. Enforced prior completed session-level usage and session pairing.
3. Added turnover controls:
   - Cooldown bars (`1m=30`, `3m=10`)
   - One-trade-per-session-direction cap per NY day
4. Added quality/regime filters:
   - Session range tick filter (`40..=1200`)
   - Previous day range regime filter (`120..=2200`)
5. Reverted asymmetric exits to single-target execution after degradation.

## Baseline Config (current)
- `min_sweep_ticks=2`
- `vol_mult=1.5` (SMA20 multiplier)
- `min_target_ticks=20`

## Latest Results

### 1m
- slip1: trades=2896, win%=21.7, PF=1.08, gross$=+9,934.04, net$=-3,040.64
- slip2: trades=2897, win%=21.7, PF=1.00, gross$=+709.16, net$=-20,629.93
- slip3: trades=2897, win%=21.7, PF=0.93, gross$=-7,654.71, net$=-36,560.70

### 3m
- slip1: trades=2245, win%=30.2, PF=1.09, gross$=+7,696.63, net$=+17.35
- slip2: trades=2244, win%=30.3, PF=1.03, gross$=+2,761.51, net$=-9,393.99
- slip3: trades=2243, win%=30.1, PF=0.97, gross$=-2,085.24, net$=-18,389.44

## 3m Parameter Sweep (summary)
- Grid dimensions:
  - `sweep_ticks`: 1/2/3
  - `vol_mult`: 1.2/1.3/1.5
  - `min_target_ticks`: 20/30/40
- Outcome:
  - No configuration produced positive net at slip2.
  - Closest region: `sweep_ticks=2`, `vol_mult=1.5`, `min_target_ticks=20` (slip1 ~ flat positive, slip2 clearly negative).

## Realism Validation
- Fees model: `$0.62/side`
- Slippage scenarios: `1/2/3` ticks per side
- Entry model: next-bar-open
- Intrabar tie-breaker: SL first when TP/SL both touched
- Gross vs net: reported side-by-side
- Gate verdict: **execution-fragile** (fails slip2 net-positive gate)

## Conclusion
- Strategy now operational and correctly backtested (trade generation fixed).
- Current rule set does not pass realism promotion criteria due to slip2 degradation.
- Status: `PARTIALLY_TESTED` / `NOT_PROMOTED`.
