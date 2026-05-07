# Strategy Parameter Grids

Purpose: centralized reference of which parameters were tested for each strategy family, in a compact grid-style format similar to targeted sweep reports.

## ttrades_fractal

| parameter | tested values |
|---|---|
| asset | `BTC`, `ETH`, `SOL`, `MNQ`, `MES`, `GOLD` |
| timeframe | crypto: `5m`, `15m`, `1h`, `4h`; futures: `1m` |
| rr | primarily `2` (naive baseline pass) |
| use_fvg | `true` |
| lookback | `20` |
| require_cisd | `true` |
| slippage_ticks | `[0, 1, 2]` (coverage tracked in matrix) |
| fee/commission realism | gate-first approach: realism skipped when naive fails |

Source: `STRATEGY_VALIDATION_MATRIX.md` rows for `ttrades_fractal`.

## ttrades_fractal_mtf

| parameter | tested values |
|---|---|
| asset | `BTC`, `ETH`, `SOL`, `MNQ`, `MES`, `GOLD` |
| tf_pair | crypto: `5m/1h`, `15m/4h`; futures: `1m/15m`, `5m/1h`, `15m/4h` |
| entry_variant | `Close`, `ObLevel`, `ObMidpoint` |
| confirm_mode | `cisd_only`, `ifvg_only`, `cisd_and_ifvg`, `cisd_or_ifvg` |
| time_filter | `all_day_all_week`, `ny_weekdays`, `london_ny_weekdays` |
| opportunity presets | `baseline`, `more_hits_close_rr15`, `more_hits_ob_level_rr15`, `more_hits_ob_mid_rr15`, `more_hits_close_rr12` |
| rr | `1.2`, `1.5`, `2` |
| poi_pad_bps | `0`, `5`, `10` |
| ob_tol_bps | `0`, `5`, `8`, `10` |
| slippage_ticks | `[0, 1, 2]` |
| fee_profile | `zero`, `binance_std`, `conservative` (explicitly enumerated in targeted grids) |

Primary sources: `STRATEGY_VALIDATION_MATRIX.md`, `TTRADES_TARGETED_GRID_SOL_MTF.md`.

## doji

| parameter | tested values |
|---|---|
| instrument | `MNQ` (primary), exploratory `ETH` |
| timeframe | `15m` (MNQ), `1h` (exploratory ETH) |
| doji_type | `classic` (champion path) |
| entry_mode | `market_close` (promoted), plus sweep around alternatives in runner history |
| max_sl_mode | `limit_reprice` (promoted) |
| max_sl_points | focused around `10` to `14` |
| tp_points | focused around `225`, `250`, `275`, `300` |
| trailing activate/distance | focused around `6/6`, `8/8`, `10/10` |
| max_trades_per_day | up to `10` (promoted path) |
| session | broad `04:00-15:30` diagnostics, narrowed to `04:00-12:00` |
| slippage_ticks | `1`, `2`, `3` |
| commission_rt_usd | `1.32` |

Source: `DOJI_STRATEGY_REPORT.md`.

## mc (Manipulation Candle / Engulfing)

| parameter | tested values |
|---|---|
| pattern | `Mc`, `Engulfing` |
| mode | `ReversalDaily`, `ContinuationEma200`, `ContinuationStructure`, `Auto` |
| entry_mode | `Close`, `PrevOpen`, `PairMidpoint`, `PairExtreme` |
| rr_target | broad `1.5`, `1.8`, `2.0`, `2.1`, `2.2` plus baseline variants |
| trade_window/session | full session and segmented windows (`PM`, `NYAM`, `NYPM`) |
| trend_filter | `None`, `Ema(50/200)`, `MarketStructure` |
| trailing_stop | `None`, `Progressive`, `BreakEven1R`, `StepHalfR`, `Trail05RAt15R` |
| prev_open_fill_window_candles | tested around `3`, `4`, `5`, `6`, `8` |
| signal_quality.min_body_to_range | `0`, `0.35`, `0.40`, `0.45` |
| signal_quality.min_range_to_prev_range | `0`, `1.1`, `1.2` |
| signal_quality.min_range_to_avg_range | tested in quality pass (`1.15x avg20`) |
| fvg_filter | on/off variants in MC runner |
| execution.slippage_ticks | `1` default in recent OB/MC runs |
| max_sl_points | `20`, `22.5`, `25`, `27.5`, `30`, `32.5` |
| max_sl_mode | `KeepEntryMoveStop`, `KeepStopMoveEntry` |

Sources: `MC_STRATEGY_REPORT.md`, `OB_ENGULFING_MNQ_ETH_REPORT.md`.

## ce

| parameter | tested values |
|---|---|
| market | `MNQ` (London-focused research dataset) |
| validation folds | `8`, `10` |
| max_bars | `400000` |
| min_avg_test_trades gate | `20` |
| cost stress multipliers | baseline (`100%`), stressed (`125%`) for commission and slippage |
| mode | `--research-london` |
| output ranking depth | `--top 8` |

Source: `CE_PROMOTION_REPORT.md`.

## orb_london_reversal

| parameter | tested values |
|---|---|
| orb_window | optimized; best shown at `30m` |
| session_close | optimized; best shown at `14:00` |
| min_excursion | optimized; best shown at `20%` |
| max_reenter | optimized; best shown at `12` |
| be/time_stop | on/off explored; best shown with both `off` |
| assets | `MNQ`, `MES`, `GOLD` |
| trade filters | practical viability gates: `PF`, net USD, maxDD, trades |

Source: `ORB_LONDON_REVERSAL_NOT_RECOMMENDED.md`.

## ob_engulfing (MNQ + ETH transfer)

| parameter | tested values |
|---|---|
| instrument | `MNQ`, transfer to `ETH` |
| timeframe | MNQ `15m`; ETH `5m`, `15m`, `1h`, `4h` |
| entry_mode | `PrevOpen`, `PairMidpoint`, `PairExtreme`, `Close` |
| rr_target | `1.5`, `1.7`, `1.8`, `1.9`, `2.0`, `2.1`, `2.2` |
| session window | full-day and segmented (`PM`, `NYAM`, `NYPM`) |
| trailing_stop | `None`, `Progressive`, `BreakEven1R`, `StepHalfR`, `Trail05RAt15R` |
| quality.min_body_to_range | `0`, `0.35`, `0.40`, `0.45` |
| quality.min_range_to_prev_range | `0`, `1.1`, `1.2` |
| max_sl_points | `20`, `22.5`, `25`, `27.5`, `30`, `32.5` |
| max_sl_mode | `KeepEntryMoveStop`, `KeepStopMoveEntry` |
| walk-forward style checks | 50/50 split and rolling `5` window OOS verdict cycle |
| final gate | OOS `PF>=1.20` and `profit_r>0` in `>=4/5` windows |

Source: `OB_ENGULFING_MNQ_ETH_REPORT.md`.
