# Realism Validation Report

Backtest: MC Strategy (all variants) on BTC/USDT Binance  
Validator: realism_validation skill (§1-10)  
Date: 2026-04-29

---

## Executive Summary

**Verdict: NOT REALISTIC ENOUGH to iterate.** Multiple critical issues inflate results. The 592x champion strategy is almost certainly unachievable in live trading. Even the 19x 15m variant likely overstates by a large factor.

The two biggest problems are: **zero transaction costs** (on 3,000-10,000 trades with 1% compounding risk) and **the PrevOpen entry model** (limit fills at stale prices after price has moved away from the level).

---

## 1) Execution Model

### 1.1 Entry timing

**Close entry (`EntryMode::Close`):**  
Signal fires on `actual` bar (index `ind`), entry at `actual.close`.  
- The signal uses `actual` and `previous` (completed bars), so confirmation is from a completed bar.  
- Entry at `actual.close` means you're filling at the close of the bar that *just* produced the signal. This is **optimistic**: in practice you'd enter at the *next* bar's open.  
- **Impact**: Close entry is actually the worse-performing variant, so this optimism doesn't inflate the top results — but it means Close-mode results are still overstated.

**PrevOpen entry (`EntryMode::PrevOpen`):**  
Signal fires on `actual` bar. Entry is set at `previous.open` — a price from *two bars ago*. A `PendingLimit` is created with a fill window of 3 candles (`prev_open_fill_window_candles`).  
- Fill condition (`src/strategies/mc.rs:851`): `actual.low <= pending.entry && actual.high >= pending.entry` — price merely needs to touch the level.  
- **CRITICAL PROBLEM**: This is a limit order at `previous.open`. By the time the signal fires (at `actual` close), price has already moved significantly from `previous.open`. For a bullish engulfing signal, `actual.close > previous.body_top`, meaning price is now *above* `previous.open`. The limit order at `previous.open` (below current price) requires price to dip back down to fill — but that dip may not happen, or if it does, it may indicate the trade thesis is wrong.  
- **CRITICAL PROBLEM**: Even when price does "touch" the level (via OHLC), the backtest assumes a perfect fill at exactly `previous.open` with **zero slippage**. Real limit fills on crypto can slip, especially on 5m bars with thin liquidity at a specific tick.  
- **This is the single biggest source of fantasy edge.** All top 5 strategies use PrevOpen. The champion 592x result depends on getting systematically better entry prices than are achievable.

### 1.2 Exit timing / intrabar ambiguity

Both SL and TP checks use OHLC comparisons (`src/strategies/mc.rs:877-960`):

```
Long: if current_sl > actual.low → SL hit
      if tp < actual.high → TP hit
Short: if current_sl < actual.high → SL hit
       if tp > actual.low → TP hit
```

**When both SL and TP are touched in the same bar, the code checks SL first** (lines 879/920 before 908/949). This is the conservative default — **correct per skill §5.1**.

However: there's a subtle ordering issue. The trailing stop update (`apply_trailing`) runs *before* the SL/TP checks on the same bar. If trailing moves the SL up (for Long) on a bar where the low then hits the new SL, the trailing update and the exit happen in the same decision step. Whether the new SL should have been active for that bar's low is ambiguous — the code assumes it is, which is **conservative** (more exits at the tighter stop), so this is acceptable.

**Gap-through handling: MISSING.** If a bar opens beyond the stop loss (e.g., gap down on a long), the code checks `current_sl > actual.low` and exits — but it uses `actual.close_time` as the exit time and implicitly fills at the SL price. Per skill §5.3, you should fill at `open - slippage`, not at the stop price. **This overstates results on gap bars.**

### 1.3 Close-confirmed signals

Signals (`is_bullish_mc`, `is_bearish_mc`, `is_bullish_engulfing`, `is_bearish_engulfing`) use only `actual` and `previous` completed bars. No intrabar signal generation. **PASS.**

### 1.4 Next-bar-open entry

Neither entry mode implements next-bar-open fills. Close entry fills at same-bar close; PrevOpen fills at a stale level. **FAIL** — no realistic entry model exists as an option.

---

## 2) Costs: Commissions + Fees

**ZERO costs modeled.** No commission, no fees, no slippage anywhere in the codebase.

`BacktestResult.pnl()` (`src/model/backtest_result.rs:44-56`) compounds balance purely on R-multiples:
```rust
TradeResult::Winner => acc + acc * r * x.rr().0
TradeResult::Expense => acc - acc * r
```

No cost deduction per trade. With 3,653 trades (15m champion) or 10,064 trades (5m champion), even Binance's 0.1% maker/taker fee per side compounds devastatingly:

**Rough impact estimate (15m champion, 3,653 trades)**:
- Round-trip cost: 0.1% × 2 sides = 0.2% per trade on notional
- Average notional per trade scales with balance (compounding)
- After costs, the 1,831% PnL likely drops by 30-50%+ on net basis

**Rough impact estimate (5m champion, 10,064 trades)**:
- Double the trade count = roughly double the cost drag
- The 59,117% PnL could drop by 50-70%+ with realistic costs

**This is a mandatory fix. Per skill §2, report both gross and net.**

---

## 3) Slippage and Spread Modeling

**ZERO slippage modeled.** No tick-based slippage, no spread, no adverse fill modeling.

Per skill §3, minimum viable model is 1 tick per side (baseline), 2 ticks (stress), 3 ticks (extreme).

For Binance BTC/USDT on 5m candles:
- Typical spread: ~$0.10-$1.00 depending on time of day
- Tick size: $0.01
- Slippage on limit orders: typically 1-3 ticks in normal conditions, more during volatility
- Slippage on market orders: 2-5+ ticks

**The PrevOpen entry is especially sensitive**: it's a limit order at a specific price from 2 bars ago. If it fills, it may fill with slippage in the adverse direction. The fact that the backtest assumes zero slippage on 10,000 fills is a major overstatement.

**Stress test needed**: Run champion strategy with 1, 2, 3 ticks per side slippage. If PnL collapses at 2 ticks, the edge is an execution illusion.

---

## 4) Session / Timezone / Market Structure

### 4.1 Timezones

Timestamps are converted to `America/New_York` via `to_new_york_time()` (`src/lib.rs:14-18`). **PASS.**

The trading window filter uses `NaiveTime` in NY timezone (default 05:00-16:00 NY). The daily open is set to 19:00 NY (7pm ET), which corresponds to the traditional crypto "daily open" at midnight UTC. **Reasonable.**

### 4.2 Holidays and early closes

**Not handled.** Crypto trades 24/7 so this is less of an issue than equities, but:
- Exchange maintenance windows (Binance has had outages)
- Flash crash events where liquidity vanishes
- These would cause gap-through stops that the code doesn't handle correctly

### 4.3 Session boundaries for crypto

The `trade_window` filter (default 05:00-16:00 NY) limits entries to NY trading hours, which is good for filtering low-liquidity periods. But the backtest doesn't account for wider spreads outside NY hours affecting existing positions. **Minor issue.**

---

## 5) Fill Feasibility Checks

### 5.1 Stop/target tie-breaker

SL is checked before TP in the same bar. **PASS (conservative).**

### 5.2 Level fills after confirmation — PrevOpen

**FAIL.** This is the most problematic fill assumption in the entire backtest:

1. Signal confirms at `actual.close` (e.g., bullish engulfing: close > previous body top)
2. Entry is placed at `previous.open` — which is *below* current price for a bullish signal
3. The fill window is only 3 candles
4. Fill is assumed at exactly `previous.open` with zero slippage

**Why this is unrealistic:**
- If price just closed above previous body top (engulfing), it's now well above `previous.open`
- A limit buy at `previous.open` requires price to retrace down to that level
- If it does retrace, that's often a sign the engulfing failed — the market is rejecting the signal
- Even if filled, the fill price on a fast-moving market often slips past the limit
- The backtest counts this as a *better* entry than Close, giving a larger R-multiple, which is the core driver of the 592x result

**Per skill §1.2**: "If performance only exists in the optimistic model, the edge is fragile." The top strategies *only* work with PrevOpen. Close entry on the same strategies produces dramatically worse results (champion Close variant: $1,229 vs $592,170).

### 5.3 Gap-through stops

**NOT HANDLED.** If a bar opens beyond the stop, the code exits at the SL price, not at `open - slippage`. This understates losses on gap events.

---

## 6) Data Integrity Validation

**Not implemented.** The code loads JSON files via `serde_json::from_str().unwrap()` with no validation:

- No OHLC sanity check (`low <= min(open,close) <= high`)
- No duplicate timestamp check
- No bar spacing validation
- No missing bar detection

The data comes from Binance API (via `loader_binance`), so it's likely clean, but there's no programmatic verification. **Should add at minimum an OHLC sanity assert on load.**

---

## 7) Reporting Completeness

| Required output | Present? |
|----------------|----------|
| Entry/exit timestamps | Partial (stored in Trade but not always printed) |
| Direction | Yes |
| Quantity / position size | **NO** — fixed 1% risk model, no per-trade qty |
| Stop and target | Yes (in Trade struct) |
| Exit reason (TP/SL/BE) | Yes |
| PnL gross | Yes (R-multiples) |
| PnL net (after costs) | **NO** |
| R-multiple (realized) | Yes |
| Profit factor | Computed in runners |
| Max drawdown | Computed in runners |
| Sensitivity table (slippage sweep) | **NO** |

**Missing**: net PnL, per-trade notional/qty, slippage sensitivity table.

---

## 8) Sanity Checks (Estimated)

Since the code can't be built without assets, these are analytical estimates rather than ran results:

### 8.1 Slippage cliff test

The 5m champion (10,064 trades) is extremely sensitive. At 1 tick ($0.01) per side on BTC at ~$30,000:
- Per-trade cost: ~$0.02 on entry + exit
- On 10,064 trades with compounding: the cumulative drag is substantial
- At 2 ticks per side: likely cuts the 592x result by 50%+
- **Highly execution-sensitive.** The edge is thin on a per-trade basis (35% win rate, 2R target) and relies on volume.

### 8.2 Entry model test

| Entry mode | 5m Champion PnL | 15m Champion PnL |
|------------|-----------------|------------------|
| PrevOpen (optimistic) | 592x | 19x |
| Close (realistic-ish) | 65x | 1.2x |

**The 9x gap on 5m and 16x gap on 15m between PrevOpen and Close confirms the edge is fragile and concentrated in the optimistic fill model.**

### 8.3 Cost-only test

No commission model exists. Adding 0.1% taker fee per side (0.2% round trip) on 10,064 trades with compounding:
- Each round trip deducts 0.2% of notional
- With 35% win rate at 2R: expected gross R per trade = 0.35×2 - 0.65×1 = +0.05R
- After 0.2% round-trip cost on notional that's ~2x the risk amount, net R per trade drops to roughly +0.01R or negative
- **Commission alone may kill the 5m strategy.**

### 8.4 Time cutoff test

The trade window filter already exists (default 05:00-16:00 NY). This is already a form of cutoff. No analysis of whether narrowing or widening it materially changes results.

---

## 9) Architecture: Strategy / BrokerSim / Reporter Separation

**FAIL.** The MC strategy (`src/strategies/mc.rs`) directly:
- Generates signals
- Manages pending limit fills (broker logic)
- Applies trailing stops (broker logic)
- Checks SL/TP exits (broker logic)
- Computes PnL via `BacktestResult.pnl()` (reporter logic)

All concerns are interleaved in one 1,154-line `execute()` method. There's no separation between "what the strategy decides" and "how the broker fills it." This makes it impossible to swap fill models, add slippage, or test different execution assumptions without modifying strategy code.

**Per skill §9**: keep Strategy, BrokerSim, and Reporter separate.

---

## 10) Per-Strategy Summary

### MC Strategy (`src/strategies/mc.rs`) — ACTIVELY USED

| Check | Status | Notes |
|-------|--------|-------|
| Close-confirmed signals | PASS | Uses completed bars only |
| Next-bar-open entry option | FAIL | Not implemented |
| Stop wins same-bar conflicts | PASS | SL checked before TP |
| Gap-through stops | FAIL | Fills at SL price, not open |
| Per-side costs | FAIL | Zero costs |
| Slippage modeling | FAIL | Zero slippage |
| Spread modeling | FAIL | No spread |
| PrevOpen fill realism | **CRITICAL FAIL** | Limit at stale price, zero slippage, optimistic fill |
| Data integrity validation | FAIL | No OHLC checks |
| Net PnL reporting | FAIL | Gross only |
| Slippage sensitivity | FAIL | Not tested |
| Architecture separation | FAIL | Strategy/broker/reporter interleaved |

### MacroSoup (`src/strategies/macro_soup.rs`) — SECONDARY

| Check | Status | Notes |
|-------|--------|-------|
| Entry model | FAIL | Enters at `actual.close` on the bar that triggers — same-bar hindsight |
| Same-bar SL/TP | PARTIAL | Checks SL before TP but no gap handling |
| Costs | FAIL | Zero |
| Slippage | FAIL | Zero |
| Session handling | PASS | Uses NY timezone with session filter |

Additional issue: `trigger_or_invalidation` uses `actual.close` as entry for trades triggered by a sweep — but the sweep is detected on the same bar, so there's no next-bar delay. The entry at `actual.close` uses the close of the bar that just formed the trigger, which is optimistic.

### Mayne (`src/strategies/mayne.rs`) — SECONDARY

| Check | Status | Notes |
|-------|--------|-------|
| Entry model | FAIL | Uses `trigger_mayne` which enters at `tc.close` — same issue |
| Costs | FAIL | Zero |
| Architecture | FAIL | All interleaved |

### SFP (`src/strategies/sfp.rs`) — SECONDARY

| Check | Status | Notes |
|-------|--------|-------|
| Entry model | FAIL | Enters at `actual.close` on signal bar |
| Same-bar SL/TP | **BUG** | Both SL and TP are checked with separate `if` statements (not `if/else if`), so both can trigger on the same bar, creating two trades from one position |
| Costs | FAIL | Zero |

The SFP bug at `sfp.rs:37-53`: after SL triggers and sets `position = None`, the TP check still runs on the now-None position via the earlier `unwrap()` — actually this would panic. On closer look, the SL sets `position = None` but then the TP check `if trade.tp > actual.low` still references `trade` (the unwrapped value from before), so both trades get pushed. This is a **duplicate trade bug**.

---

## Priority Fixes (Ordered by Impact)

1. **Add transaction costs** — Even a simple 0.1% per-side model will show whether the edge survives. This is mandatory before any optimization.

2. **Add slippage model** — Minimum 1 tick per side. Run stress tests at 2 and 3 ticks. If the 5m champion dies at 2 ticks, the edge is execution-dependent.

3. **Fix PrevOpen fill model** — Either:
   - Add slippage to the limit fill (at minimum 1-2 ticks adverse)
   - Add a "next-bar-open" entry mode as a realistic baseline
   - Track fill rate: what % of PendingLimits actually fill? (The 3-candle window may overstate fills.)

4. **Handle gap-through stops** — Fill at `open ± slippage` when bar opens beyond SL.

5. **Separate BrokerSim from Strategy** — Extract fill logic (pending limits, SL/TP checks, trailing updates) into a separate module so you can swap execution models without touching strategy code.

6. **Add data integrity checks** — OHLC sanity, duplicate timestamps, bar spacing.

7. **Fix SFP double-trade bug** — Use `else if` for SL/TP checks, or check position.is_none() between them.

8. **Add net PnL reporting** — Show gross and net side by side.

---

## Bottom Line

The backtest currently removes zero execution friction from the results. On 5m with 10,064 trades, the compounded fantasy edge from PrevOpen fills + zero costs likely overstates real-world performance by **5-10x or more**. The 15m strategies (3,653 trades) are more survivable but still lack any cost accounting.

**Before optimizing any parameter**, implement costs + slippage and confirm the edge persists. If it doesn't, the strategy needs fundamental redesign, not tuning.
