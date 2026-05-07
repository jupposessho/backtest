# Porting TODO

Scope: missing strategy ports into this Rust backtest repo, especially parity gaps vs `~/develop/play/nq` and strategies that still lack full validation/reporting.

## Not Ported From `~/develop/play/nq` Yet

- [ ] `AsiaSweep` preset family (`nq/src/main.rs`)
  - Reversal session spec exists in NQ runner; no direct Rust strategy/preset parity yet.

- [ ] `Cr` preset family (`nq/src/main.rs`)
  - Not implemented as a first-class strategy/preset in `backtest`.

- [ ] `PatPo3` preset family (`nq/src/main.rs`)
  - Not implemented as a first-class strategy/preset in `backtest`.

- [ ] `Crt1m` preset family (`nq/src/main.rs`)
  - Not implemented as a first-class strategy/preset in `backtest`.

- [ ] `range_rejection_strategy.py` (`nq/range_rejection_strategy.py`)
  - Conceptually close to `orb_london_reversal`, but no explicit close-back-inside range rejection port.

- [ ] `no_wick_strategy.py` (`nq/no_wick_strategy.py`)
  - No dedicated Rust equivalent.

- [ ] `5min_asia_gold_strategy.py` (`nq/5min_asia_gold_strategy.py`)
  - No dedicated Rust equivalent.

- [ ] `tradovate_bot/ema_strategy.py`
  - Live-bot oriented EMA+OB logic not ported as a standalone Rust backtest strategy.

- [ ] `tradovate_bot/range_strategy.py`
  - Live-bot range rejection module not ported as standalone Rust strategy.

## Partially Ported (Preset-Level Only)

- [~] `MultiOrb` / `FinalBoss` / `Orb30m15m`
  - Added as ORB preset aliases in `src/orb_variants.rs`.
  - Still not a dedicated strategy module with one-to-one NQ runner behavior.

## Implemented But Missing Full Validation Pack

- [ ] `ict` (`src/strategies/ict.rs`)
  - Not exported in `src/strategies/mod.rs`.
  - Missing dedicated strategy-overview variant grid + OOS verdict.

- [ ] `crypto_momentum_rider` (`src/strategies/crypto_momentum_rider.rs`)
  - Not exported in `src/strategies/mod.rs`.
  - Missing dedicated strategy-overview variant grid + OOS verdict.

- [ ] `macro_soup` (`src/strategies/macro_soup.rs`)
  - Implemented and runnable via `src/ms.rs`.
  - Missing full variant grid + rolling OOS promotion gate report.

- [ ] `ict_composed` (`src/strategies/ict_composed.rs`)
  - Implemented.
  - Missing full variant grid + rolling OOS promotion gate report.

- [ ] `sfp` (`src/strategies/sfp.rs`)
  - Implemented.
  - Missing full variant grid + rolling OOS promotion gate report.

## Notes

- `orb` realism/OOS coverage exists (`reports/strategy_overviews/ORB_VARIANTS_GRID.md`), current verdict remains non-promotable.
- `orb_london_reversal` is documented in `reports/strategy_overviews/ORB_LONDON_REVERSAL_NOT_RECOMMENDED.md`.
- Recent multi-asset iFVG research is documented in `reports/strategy_overviews/MULTI_ASSET_IFVG_TUNE_REPORT.md`.
