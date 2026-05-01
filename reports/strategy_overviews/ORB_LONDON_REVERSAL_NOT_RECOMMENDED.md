# ORB London Reversal - Not Recommended

Status: `NOT_RECOMMENDED` (do not promote)

Date: 2026-05-02

## Why this is not recommended

- The strategy can be tuned to produce positive net PnL on specific assets, but it is not robust across the futures basket.
- Under practical risk filters (`PF >= 1.15`, `net_usd >= 400`, `maxdd_usd <= 250`, `trades >= 200`), only one asset/configuration survives.
- MES fails risk-adjusted viability (low return vs high drawdown), so this is not suitable as a general framework candidate.

## Final optimization snapshot (1 micro contract)

- **MNQ (tradable)**: `orb=30m`, `session_close=14:00`, `min_excursion=20%`, `max_reenter=12`, `be/time_stop=off`
  - trades: `312`
  - win rate: `34.94%`
  - PF: `1.19`
  - net: `+492.27 USD`
  - maxDD: `213.81 USD`
- **MES (not tradable)**: best net variant `+128.06 USD` with `maxDD 620.69 USD` -> rejected.
- **GOLD (not tradable by current filter)**: strong net (`+999.10 USD`) but maxDD (`351.85 USD`) breaches `<= 250 USD` cap.

## Decision

- Keep code for reference/research only.
- Exclude ORB London Reversal from promotion shortlist and production candidates.
