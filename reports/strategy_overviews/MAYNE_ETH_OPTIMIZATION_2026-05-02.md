# Mayne ETH Optimization (2026-05-02)

## Scope

- Strategy: `Mayne` with HTF SFP + reversal confirmation and LTF trigger logic.
- Added reversal modes: `IfvgOnly`, `CisdStrictWickBreakAndIfvg`, `CisdStrictWickBreakOrIfvg`.
- Added configurable LTF iFVG invalidation distance: `ifvg_max_confirm_bars`.
- Added diagnostics funnel counters per config.

## Data and Window

- ETH 12h/1h run used last 5000 HTF candles.
- Window (UTC): `2019-04-13 18:00:00` to `2026-02-17 23:59:59`.
- ETH 4h/15m and 1h/5m optimization pass used last 3000 HTF candles.

## Best Setups

### ETH 12h / 1h

- Best PnL: **+50.80%**
- Config:
  - `reversal_pattern=CisdStrictWickBreakOrIfvg`
  - `sl_variant=SfpExtreme`
  - `tp_variant=OpposingHtfSwing`
  - `trigger_type=Wick`
  - `rr_threshold=1.0`
  - `ifvg_max_confirm_bars=6`
- Stats: `39 trades`, `20 winners`, `19 expenses`, `profit_in_r=53.82`.

### ETH 4h / 15m

- Best PnL: **+13.11%**
- Config:
  - `reversal_pattern=CisdStrictWickBreak`
  - `sl_variant=SfpExtreme`
  - `tp_variant=OpposingHtfSwing`
  - `trigger_type=Wick`
  - `rr_threshold=2.0`
  - `ifvg_max_confirm_bars in {6,12,24,48,96}` (same best result)
- Stats: `7 trades`, `4 winners`, `3 expenses`, `profit_in_r=17.51`.

### ETH 1h / 5m

- Best PnL: **+7.05%**
- Config:
  - `reversal_pattern=Mss`
  - `sl_variant=LtfRecentSwing`
  - `tp_variant=OpposingHtfSwing`
  - `trigger_type=Wick`
  - `rr_threshold in {0.75, 1.0}`
  - `ifvg_max_confirm_bars in {6,12,24,48,96}`
- Stats: `3 trades`, `3 winners`, `0 expenses`, `profit_in_r=8.70`.

## Verdict

- Current ETH champion is **12h/1h** with `CisdStrictWickBreakOrIfvg + Wick + SfpExtreme + OpposingHtfSwing + RR=1.0 + ifvg_max=6`.
- This is the highest observed return with meaningful trade count in the tested grid.
