# Strategy Overviews

This folder contains the latest generated strategy evaluation reports.

- `STRATEGY_VALIDATION_MATRIX.md` - cross-asset strategy matrix with final verdicts.
- `TTRADES_TARGETED_GRID_SOL_MTF.md` - focused SOL MTF parameter sweep report.
- `TTRADES_FULLY_TESTED_RANKING.md` - ranked shortlist of fully tested candidates.

Writers updated:

- `cargo run --release --bin ttrades_matrix [-- --fast]` now writes to this folder.
- `cargo run --release --bin ttrades_targeted_grid` now writes to this folder.
