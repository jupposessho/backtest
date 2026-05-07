# MNQ EMA Wick Reclaim Report

Date: 2026-05-05
Runner: `cargo run --release --bin ema_wick_reclaim_mnq`
Dataset: `assets/mnq_1m_cont.parquet`
Start date filter: `2025-01-01`
Contract: MNQ micro (1 contract, $2/point)
Execution costs: `$1.24` round-trip fees + `$1.00` round-trip slippage (stress-tested to `$1.50` and `$2.00`)

## Strategy Definition

- HTF filter: `EMA200(5m)` anchor for wick-through + close-back signal.
- LTF entries tested: `1m`, `3m`.
- Entry variants:
  - `imm`: immediate at signal close
  - `obmid`: wait for OB midpoint retest after signal candle
- Stop variants:
  - `wick`: wick-based stop (with min stop floor)
  - `atr`: ATR-derived stop floor
  - `hybrid`: max(wick stop, ATR floor)
- Additional filters tested:
  - session windows (`all`, `london`, `ny`, `ny_open`, `ny_late`)
  - min wick penetration (ticks)
  - ATR floor multiplier
  - cost-to-risk cap
  - regime filter, micro-confirmation, dynamic RR

## Baseline (Unfiltered)

- `1m`: trades `2604`, win `17.24%`, net `-$12,776.64`
- `3m`: trades `2160`, win `18.52%`, net `-$45,115.81`

Conclusion: raw wick reclaim is not viable on MNQ after costs.

## Top Results From Full Sweep

### Top 1m by Net

- Preset: `rr5 wick8 atr0.5 cap0.20 all atr obmid`
- Trades: `1337`
- Win rate: `23.34%`
- Net: `+$9,678.91`
- Avg/trade: `+$7.24`

### Top 3m by Net

- Preset: `rr5 wick8 atr0.5 cap0.20 ny atr obmid`
- Trades: `399`
- Win rate: `26.57%`
- Net: `+$7,186.38`
- Avg/trade: `+$18.01`

### Top 3m by Win Rate (Profitable)

- Preset: `rr4 wick8 atr0.5 cap0.20 ny atr obmid`
- Trades: `402`
- Win rate: `30.35%`
- Net: `+$5,964.47`
- Avg/trade: `+$14.84`

## EMA Period Comparison (Focused Profiles)

### 1m profile (`rr3 wick8 atr1 cap0.10 london`)

- `EMA200`: `+$459.19` (best in 1m compare set)
- `EMA100`: `-$83.45`
- `EMA150`: `-$1,415.68`
- `EMA250`: `-$1,501.87`
- `EMA300`: `-$1,271.25`

### 3m profile (`rr3 wick2 atr1 cap0.10 ny`)

- `EMA300`: `+$3,956.70` (best)
- `EMA200`: `+$2,241.03`
- `EMA250`: `+$1,775.18`
- `EMA100`: `-$622.37`
- `EMA150`: `-$2,226.79`

## Robustness and Monthly Breakdown

### TOP 1m net config monthly (`+13 / -2`)

- 2025-01: `$615.36`
- 2025-02: `$97.85`
- 2025-03: `$531.94`
- 2025-04: `$3,723.73`
- 2025-05: `$539.10`
- 2025-06: `$363.05`
- 2025-07: `$539.46`
- 2025-08: `-$473.34`
- 2025-09: `$838.10`
- 2025-10: `-$73.26`
- 2025-11: `$456.19`
- 2025-12: `$1,023.82`
- 2026-01: `$731.58`
- 2026-02: `$384.71`
- 2026-03: `$380.61`

### TOP 3m net config monthly (`+11 / -4`)

- 2025-01: `$865.37`
- 2025-02: `$438.37`
- 2025-03: `$515.59`
- 2025-04: `$929.66`
- 2025-05: `-$39.30`
- 2025-06: `$332.94`
- 2025-07: `$44.06`
- 2025-08: `$497.58`
- 2025-09: `-$466.15`
- 2025-10: `-$182.21`
- 2025-11: `$1,148.45`
- 2025-12: `$716.69`
- 2026-01: `$590.48`
- 2026-02: `$1,848.83`
- 2026-03: `-$53.99`

### TOP 3m win-rate config monthly (`+12 / -3`)

- 2025-01: `$602.98`
- 2025-02: `$270.76`
- 2025-03: `$440.54`
- 2025-04: `$1,292.35`
- 2025-05: `-$130.84`
- 2025-06: `$328.03`
- 2025-07: `$169.37`
- 2025-08: `$282.61`
- 2025-09: `-$362.47`
- 2025-10: `$43.24`
- 2025-11: `$815.65`
- 2025-12: `$319.36`
- 2026-01: `$547.53`
- 2026-02: `$1,399.36`
- 2026-03: `-$53.99`

## Slippage Stress

### TOP 1m net

- `$1.0` RT slippage: `trades=1337`, `win=23.34%`, `net=+$9,678.91`
- `$1.5` RT slippage: `trades=1066`, `win=23.45%`, `net=+$8,454.48`
- `$2.0` RT slippage: `trades=850`, `win=23.65%`, `net=+$7,627.82`

### TOP 3m net

- `$1.0` RT slippage: `trades=399`, `win=26.57%`, `net=+$7,186.38`
- `$1.5` RT slippage: `trades=390`, `win=26.15%`, `net=+$6,829.76`
- `$2.0` RT slippage: `trades=371`, `win=25.34%`, `net=+$6,147.46`

### TOP 3m win-rate

- `$1.0` RT slippage: `trades=402`, `win=30.35%`, `net=+$5,964.47`
- `$1.5` RT slippage: `trades=393`, `win=29.77%`, `net=+$5,584.05`
- `$2.0` RT slippage: `trades=374`, `win=28.88%`, `net=+$4,966.08`

## Improvement Trial (Regime + Micro-confirm + Dynamic RR)

Applied to selected top configs:

- `IMPROVED 1m`
  - Trades: `1402`
  - Win rate: `29.60%`
  - Net: `+$6,977.25`
  - Monthly: `+12 / -3`

- `IMPROVED 3m`
  - Trades: `372`
  - Win rate: `31.72%`
  - Net: `+$4,273.51`
  - Monthly: `+11 / -3` (14 active months)

Interpretation: improvements increased win rate, but reduced absolute net compared with max-net presets.

## Current Recommendation

- **Max net objective**: use `1m rr5 wick8 atr0.5 cap0.20 all atr obmid`.
- **Higher hit-rate objective**: use `3m rr4 wick8 atr0.5 cap0.20 ny atr obmid`.
- **Balanced objective**: use improved `1m` variant (higher hit-rate, still strong positive net).

Status: `PARTIALLY_TESTED` (good in-sample and monthly breakdown on this dataset; still needs strict out-of-sample/live-forward validation before promotion).

## Final Presets (Locked)

From targeted optimization around winners (trade-count constrained search):

- **`MNQ_EMA_WICK_MAX_NET_1M`**
  - setup: `1m rr5 wick8 atr0.5 cap0.25 obw6 hold90 all atr obmid`
  - trades: `1613`
  - win rate: `24.49%`
  - net: `+$11,533.18`
  - avg/trade: `+$7.15`

- **`MNQ_EMA_WICK_MAX_NET_3M`**
  - setup: `3m ema200 rr5 wick10 atr0.5 cap0.25 obw6 hold90 ny atr obmid`
  - trades: `385`
  - win rate: `27.53%`
  - net: `+$7,475.56`
  - avg/trade: `+$19.42`

- **`MNQ_EMA_WICK_BALANCED_1M`**
  - setup: `max-net 1m + selective filters ed1 bp25 rg5`
  - trades: `1346`
  - win rate: `24.67%`
  - net: `+$10,426.21`
  - avg/trade: `+$7.75`

- **`MNQ_EMA_WICK_BALANCED_3M`**
  - setup: `max-net 3m + selective filter ed2`
  - trades: `380`
  - win rate: `27.63%`
  - net: `+$7,255.10`
  - avg/trade: `+$19.09`

Interpretation:
- heavy all-5-filter mode improved hit rate but over-pruned opportunity,
- selective filtering around winners gives better trade quality with minimal net sacrifice.
