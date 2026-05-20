# Gold Mechanical Sweep (MGC) Summary

Date: 2026-05-20

## Scope
- Instrument: Gold continuous 1m source (`gold`)
- Contract sizing: MGC-equivalent point value (`$10` per 1.0 move)
- Cost model in runner: fixed round-turn commission plus slippage sweep
- Dataset span: 2021-03-07 23:00 UTC to 2026-03-05 23:59 UTC

## Donchian Focused Sweep
Ranking objective used during sweep:

```text
score = net_usd - 0.35 * max_dd_usd
```

Top config selected:

```text
entry=55 exit=20 sma=Some(200) atr_len=20 atr_mult=3.5
```

Performance (MGC, fixed 1 contract):
- trades: 11
- win rate: 54.55%
- net_usd: 7235.35
- max_dd_usd: 1481.60

## Strategy Ranking (MGC)
From latest run:

1. donchian_breakout: 7235.35
2. momentum_12m: 4405.60
3. ema_pullback_continuation: 3731.80
4. vol_expansion_squeeze: 3683.60
5. seasonal_window: 331.40
6. intraday_orb: -8035.60

## Realism Validation (Winner)
Winner slippage sensitivity (ticks per side):

1. 1 tick: net_usd 7257.35
2. 2 ticks: net_usd 7235.35
3. 3 ticks: net_usd 7213.35

Verdict: PASS

Detailed matrix exported to:

```text
reports/gold_mechanical_realism_matrix.csv
```
