# AGENTS.md

## Build & Run

- **Build**: `cargo build --release` (always use `--release`; debug builds are very slow due to numeric computation)
- **Run main backtest**: `cargo run --release --bin mc_summary` (top-9 strategies across timeframes, ~2-3 min)
- **Run full analysis**: `cargo run --release --bin mc` (all variants, ~10-15 min)
- **Other bins**: `loader` (Mexc data fetch), `loader_binance` (Binance data fetch), `gallery` (axum chart server on :5555), `ms` (MacroSoup strategy)

## Tests

- **Tests are currently broken**: `cargo test` fails because `assets/` is gitignored and `include_str!` in binary targets and `chart.rs` embeds those files at compile time. Tests in `src/strategies/lib.rs` and `src/strategies/macro_soup.rs` are unit tests on logic only but the lib crate still fails to compile without the asset files present.
- **To run tests**: You need the `assets/` directory populated with Binance JSON kline files (e.g. `assets/binance_BTCUSDT_5m.json`). There is no public download script — use `cargo run --release --bin loader_binance` to fetch them.
- **To run only strategy tests (if assets exist)**: `cargo test --lib strategies`

## Architecture

- **Rust binary crate** with a shared library (`src/lib.rs`). Multiple `[[bin]]` targets defined in `Cargo.toml`.
- **Core strategy**: `src/strategies/mc.rs` — the MC (Manipulation Candle) strategy. Implements the `TradingModel` trait from `src/model/trading_model.rs`.
- **Model layer**: `src/model/` — `CandleStick`, `Position`, `Trade`, `BacktestResult`, `DecimalVec` (newtype around `rust_decimal::Decimal`).
- **Shared strategy utilities**: `src/strategies/lib.rs` — swing detection, trade execution, session filtering.
- **Entry points**:
  - `mc` / `mc_summary` — main analysis runners
  - `ms` — MacroSoup strategy runner
  - `gallery` — chart visualization web server
  - `loader` / `loader_binance` — exchange data fetchers (async, use clap for CLI args)

## Key Conventions

- **All prices use `DecimalVec`** (not raw `Decimal`) — always wrap/unwrap via `.0` when interacting with model fields.
- **Data is embedded at compile time** via `include_str!` pointing to `../assets/*.json`. The `assets/` dir is gitignored so builds fail without it.
- **Time handling**: All timestamps are converted to New York timezone (`America/New_York`).
- **`binance-rs` crate** is pulled from a git dependency, not crates.io.
- **The `serde` cfg feature** referenced in `candle_stick.rs` and `binance_klines_item.rs` does not exist in `Cargo.toml` — this produces `unexpected_cfgs` warnings. The `serde` derive still works because `serde` is a direct dependency; the `cfg_attr` is redundant.

## Adding a New Strategy or Timeframe

1. Add data file: `assets/binance_BTCUSDT_XXm.json`
2. Add a `load_binance_XXm()` function with `include_str!` in the relevant binary
3. Add to the `timeframes` vector in `main()`
4. For a new strategy: implement `TradingModel` trait, add module to `src/strategies/mod.rs`
