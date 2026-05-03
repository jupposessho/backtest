# OB Engulfing Progress Report

Detailed variant grid: `reports/strategy_overviews/OB_ENGULFING_TARGETED_GRID.md`.

## MNQ Production Candidate
- Session: NYAM (09:15-12:15)
- Pattern: Engulfing (OB style), Entry: PairMidpoint
- Quality: body>=40%, range>=1.1x prev
- RR: 2.0
- Max SL: 32.5 points, mode: KeepEntryMoveStop
- Full-data stats: trades=120, win%=43.33, profit_r=35.70, pf_r=1.52, pnl%=33.92
- Net points: 2264.87
- Net profit (MNQ $2/point, 1 contract): $4529.74

## ETH Transfer Test
Transferred the same setup to ETH (5m/15m/1h/4h). For crypto, both full-day and NYAM windows were tested.

| Variant | Trades | Win% | Profit R | PF R | PnL% |
|---|---:|---:|---:|---:|---:|
| ETH 5m full_day | 1162 | 34.85 | 50.30 | 1.07 | 43.97 |
| ETH 5m nyam | 170 | 34.71 | 6.75 | 1.06 | 4.78 |
| ETH 15m full_day | 703 | 35.56 | 46.02 | 1.10 | 45.41 |
| ETH 15m nyam | 105 | 20.95 | -39.07 | 0.53 | -33.10 |
| ETH 1h full_day | 376 | 31.91 | -16.18 | 0.94 | -18.72 |
| ETH 1h nyam | 37 | 29.73 | -4.01 | 0.85 | -4.34 |
| ETH 4h full_day | 201 | 40.30 | 41.95 | 1.35 | 48.30 |
| ETH 4h nyam | 31 | 41.94 | 7.99 | 1.44 | 7.88 |

## ETH 15m Refinement Top 5
| Variant | Trades | Win% | Profit R | PF R | PnL% |
|---|---:|---:|---:|---:|---:|
| ETH 15m rr=2.0 sl=27.5 body=0.45 rprev=0 | 961 | 35.59 | 63.45 | 1.10 | 67.77 |
| ETH 15m rr=2.0 sl=30 body=0.45 rprev=0 | 961 | 35.59 | 63.45 | 1.10 | 67.78 |
| ETH 15m rr=2.0 sl=32.5 body=0.45 rprev=0 | 961 | 35.59 | 63.45 | 1.10 | 67.78 |
| ETH 15m rr=2.2 sl=27.5 body=0.45 rprev=0 | 960 | 33.23 | 59.43 | 1.09 | 59.39 |
| ETH 15m rr=1.8 sl=27.5 body=0.45 rprev=0 | 961 | 37.98 | 59.33 | 1.10 | 62.72 |

## ETH 15m Robustness Split (winner)
- Split timestamp: 1637026200
- Train: trades=534, win%=33.52, profit_r=1.68, pf_r=1.00, pnl%=-4.69
- Test: trades=427, win%=38.17, profit_r=61.76, pf_r=1.23, pnl%=76.10

## ETH 4h Refinement Top 5
| Variant | Trades | Win% | Profit R | PF R | PnL% |
|---|---:|---:|---:|---:|---:|
| ETH 4h rr=2.2 sl=30 body=0.45 rprev=0 | 262 | 37.79 | 54.71 | 1.34 | 66.72 |
| ETH 4h rr=2.2 sl=30 body=0.45 rprev=1.1 | 186 | 39.78 | 50.75 | 1.45 | 61.81 |
| ETH 4h rr=2.2 sl=30 body=0.40 rprev=1.1 | 199 | 39.20 | 50.55 | 1.42 | 61.21 |
| ETH 4h rr=2.2 sl=30 body=0.40 rprev=0 | 279 | 36.92 | 50.50 | 1.29 | 59.50 |
| ETH 4h rr=2.2 sl=27.5 body=0.45 rprev=0 | 261 | 37.16 | 49.31 | 1.30 | 58.02 |

## ETH 4h Robustness Split (winner)
- Split timestamp: 1637172000
- Train: trades=146, win%=38.36, profit_r=33.12, pf_r=1.37, pnl%=36.45
- Test: trades=116, win%=37.07, profit_r=21.58, pf_r=1.30, pnl%=22.10

## Final Cycle: Rolling OOS Verdict
Criteria: OOS PF>=1.20 and OOS profit_r>0 in >=4/5 windows.

### ETH 15m base
- Verdict: KILL (profit windows 3/5, pf windows 3/5)
- W1: trades=208 profit_r=-1.58 pf_r=0.99 pnl%=-3.97
- W2: trades=201 profit_r=-24.69 pf_r=0.83 pnl%=-23.69
- W3: trades=229 profit_r=31.93 pf_r=1.22 pnl%=33.84
- W4: trades=157 profit_r=34.86 pf_r=1.37 pnl%=38.92
- W5: trades=167 profit_r=21.92 pf_r=1.21 pnl%=21.99

### ETH 15m ema50/200
- Verdict: KILL (profit windows 3/5, pf windows 3/5)
- W1: trades=208 profit_r=-1.58 pf_r=0.99 pnl%=-3.97
- W2: trades=201 profit_r=-24.69 pf_r=0.83 pnl%=-23.69
- W3: trades=229 profit_r=31.93 pf_r=1.22 pnl%=33.84
- W4: trades=157 profit_r=34.86 pf_r=1.37 pnl%=38.92
- W5: trades=167 profit_r=21.92 pf_r=1.21 pnl%=21.99

### ETH 4h base
- Verdict: KILL (profit windows 4/5, pf windows 3/5)
- W1: trades=53 profit_r=10.97 pf_r=1.33 pnl%=10.78
- W2: trades=51 profit_r=-3.04 pf_r=0.92 pnl%=-3.61
- W3: trades=67 profit_r=28.99 pf_r=1.78 pnl%=32.27
- W4: trades=43 profit_r=4.99 pf_r=1.18 pnl%=4.50
- W5: trades=48 profit_r=12.79 pf_r=1.44 pnl%=12.86

### ETH 4h ema50/200
- Verdict: KILL (profit windows 4/5, pf windows 3/5)
- W1: trades=53 profit_r=10.97 pf_r=1.33 pnl%=10.78
- W2: trades=51 profit_r=-3.04 pf_r=0.92 pnl%=-3.61
- W3: trades=67 profit_r=28.99 pf_r=1.78 pnl%=32.27
- W4: trades=43 profit_r=4.99 pf_r=1.18 pnl%=4.50
- W5: trades=48 profit_r=12.79 pf_r=1.44 pnl%=12.86
