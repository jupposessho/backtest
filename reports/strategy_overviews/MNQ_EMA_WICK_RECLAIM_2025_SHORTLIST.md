# MNQ EMA Wick Reclaim 2025 Shortlist

- Symbol: MNQ
- Date filter: >= 2025-01-01
- Costs: $1.24 round-trip fee + slippage stress $1.00 / $1.50 / $2.00
- Source report: `reports/strategy_overviews/MNQ_EMA_WICK_RECLAIM_2025_VALIDATION.md`

## Deduping Note

Several top rows in the validator are effectively the same outcome cluster.
In particular, the top `ema300 rr5 ny_am wick immediate` variants with `atr_floor_mult` in `0.75/1.0/1.25`
produced identical or near-identical results because the wick-defined stop dominated the ATR floor in those trades.

This shortlist keeps only materially distinct candidates.

## Primary

- Label: `neighbor_3m_ema300_rr5_wick6_atr0.75_cap0.10_ny_am_wick_immediate`
- Config:
  - timeframe: `3m`
  - EMA: `300`
  - RR: `5`
  - min wick: `6` ticks
  - ATR floor mult: `0.75`
  - max cost R: `0.10`
  - session: `NY_AM`
  - stop mode: `Wick`
  - entry mode: `Immediate`

Stress summary:

| slip RT $ | trades | win rate % | net USD | avg USD | +months | -months | max DD USD |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1.0 | 420 | 25.71 | 10777.20 | 25.66 | 11 | 4 | 2485.94 |
| 1.5 | 420 | 25.71 | 10567.20 | 25.16 | 11 | 4 | 2538.94 |
| 2.0 | 420 | 25.71 | 10357.20 | 24.66 | 11 | 4 | 2591.94 |

Why primary:

- Highest net profit in the focused neighborhood sweep.
- Better monthly consistency than the earlier shortlist (`11` positive months, `4` negative).
- Much lower drawdown than the earlier `3m_top_sweep_net` candidate.
- Remains strongly profitable across the full slippage stress band.

## Backup

- Label: `neighbor_3m_ema300_rr5_wick2_atr0.75_cap0.10_ny_am_wick_immediate`
- Config:
  - timeframe: `3m`
  - EMA: `300`
  - RR: `5`
  - min wick: `2` ticks
  - ATR floor mult: `0.75`
  - max cost R: `0.10`
  - session: `NY_AM`
  - stop mode: `Wick`
  - entry mode: `Immediate`

Stress summary:

| slip RT $ | trades | win rate % | net USD | avg USD | +months | -months | max DD USD |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1.0 | 434 | 25.81 | 10727.84 | 24.72 | 11 | 4 | 2550.64 |
| 1.5 | 434 | 25.81 | 10510.84 | 24.22 | 11 | 4 | 2606.14 |
| 2.0 | 434 | 25.81 | 10293.84 | 23.72 | 11 | 4 | 2661.64 |

Why backup:

- Slightly more trades than the primary.
- Nearly identical slippage robustness.
- Slightly worse drawdown and slightly lower net than the primary.

## Retired Prior Leaders

The previous top shortlist candidates were surpassed by the new `ema300 rr5 ny_am wick immediate` cluster:

- `3m_top_sweep_net`: lower net and higher drawdown.
- `3m_ema300_baseline`: strong but clearly weaker than the new neighborhood winners.
- `3m_ema200_baseline`: still promotable, but dominated by the `ema300 rr5` neighborhood.

## Final Recommendation

Use this ordering for future validation and deployment-style checks:

1. Primary: `3m ema300 rr5 wick6 atr0.75 cap0.10 ny_am wick immediate`
2. Backup: `3m ema300 rr5 wick2 atr0.75 cap0.10 ny_am wick immediate`
