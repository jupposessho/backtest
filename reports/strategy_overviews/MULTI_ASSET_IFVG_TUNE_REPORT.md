# Multi-Asset iFVG Tune (Recent Window)

Scope: `ttrades_fractal_mtf` close-entry + ifvg-only, on BTC/ETH/SOL 15m/4h.
Window: latest ~6 months in current asset JSON files.
Costs: strategy defaults with slippage ticks = 0.
Normalization: BTC shown at 0.1 coin, ETH at 1 coin, SOL at 10 coins.

## Top 5 Per Asset (Quick View)

### BTC (normalized at 0.1 BTC)

| rank | variant | net_6m_usd_normalized | positive_months | trades |
|---:|---|---:|---:|---:|
| 1 | close_ifvg_rr2_poi0_ob5_all_day | 208.90 | 4 | 227 |
| 2 | close_ifvg_rr1.8_poi0_ob5_all_day | 127.69 | 4 | 228 |
| 3 | close_ifvg_rr2_poi5_ob5_all_day | 115.41 | 4 | 305 |
| 4 | close_ifvg_rr2_poi0_ob0_all_day | 61.17 | 3 | 154 |
| 5 | close_ifvg_rr2_poi5_ob0_all_day | 44.64 | 4 | 194 |

### ETH (normalized at 1 ETH)

| rank | variant | net_6m_usd_normalized | positive_months | trades |
|---:|---|---:|---:|---:|
| 1 | close_ifvg_rr2_poi0_ob0_ny_only | 180.84 | 4 | 43 |
| 2 | close_ifvg_rr2_poi0_ob0_all_day | 157.97 | 3 | 128 |
| 3 | close_ifvg_rr1.8_poi0_ob0_ny_only | 153.62 | 4 | 43 |
| 4 | close_ifvg_rr2_poi0_ob5_ny_only | 140.61 | 3 | 49 |
| 5 | close_ifvg_rr1.8_poi0_ob0_all_day | 125.56 | 3 | 128 |

### SOL (normalized at 10 SOL)

| rank | variant | net_6m_usd_normalized | positive_months | trades |
|---:|---|---:|---:|---:|
| 1 | close_ifvg_rr2_poi10_ob10_ny_only | 55.14 | 4 | 8 |
| 2 | close_ifvg_rr1.8_poi10_ob10_ny_only | 43.05 | 4 | 8 |
| 3 | close_ifvg_rr1.5_poi10_ob10_ny_only | 39.74 | 5 | 8 |
| 4 | close_ifvg_rr2_poi10_ob5_ny_only | 37.26 | 3 | 7 |
| 5 | close_ifvg_rr1.8_poi10_ob5_ny_only | 27.16 | 3 | 7 |

## BTC

| variant | net_6m_usd_at_10 | net_6m_usd_normalized | positive_months | trades | wins | losses |
|---|---:|---:|---:|---:|---:|---:|
| close_ifvg_rr2_poi0_ob5_all_day | 20890.08 | 208.90 | 4 | 227 | 90 | 137 |
| close_ifvg_rr1.8_poi0_ob5_all_day | 12768.70 | 127.69 | 4 | 228 | 96 | 132 |
| close_ifvg_rr2_poi5_ob5_all_day | 11540.69 | 115.41 | 4 | 305 | 118 | 187 |
| close_ifvg_rr2_poi0_ob0_all_day | 6116.56 | 61.17 | 3 | 154 | 57 | 97 |
| close_ifvg_rr2_poi5_ob0_all_day | 4464.34 | 44.64 | 4 | 194 | 71 | 123 |
| close_ifvg_rr1.8_poi5_ob5_all_day | 1462.90 | 14.63 | 4 | 306 | 125 | 181 |
| close_ifvg_rr1.5_poi0_ob5_all_day | 586.64 | 5.87 | 4 | 228 | 103 | 125 |
| close_ifvg_rr1.8_poi0_ob0_all_day | -453.00 | -4.53 | 3 | 155 | 63 | 92 |
| close_ifvg_rr1.8_poi5_ob0_all_day | -898.92 | -8.99 | 4 | 195 | 77 | 118 |
| close_ifvg_rr2_poi10_ob5_all_day | -2743.81 | -27.44 | 4 | 402 | 149 | 253 |
| close_ifvg_rr2_poi10_ob0_all_day | -4154.34 | -41.54 | 3 | 239 | 88 | 151 |
| close_ifvg_rr2_poi10_ob5_ny_only | -4234.52 | -42.35 | 2 | 109 | 32 | 77 |
| close_ifvg_rr1.2_poi10_ob5_ny_only | -5717.06 | -57.17 | 3 | 109 | 47 | 62 |
| close_ifvg_rr2_poi0_ob10_all_day | -5995.65 | -59.96 | 3 | 297 | 113 | 184 |
| close_ifvg_rr1.8_poi10_ob5_ny_only | -6274.41 | -62.74 | 2 | 109 | 36 | 73 |
| close_ifvg_rr1.2_poi10_ob10_ny_only | -8256.38 | -82.56 | 3 | 137 | 62 | 75 |
| close_ifvg_rr1.5_poi5_ob0_all_day | -8943.81 | -89.44 | 4 | 195 | 84 | 111 |
| close_ifvg_rr1.5_poi10_ob5_ny_only | -9334.26 | -93.34 | 2 | 109 | 38 | 71 |
| close_ifvg_rr1.8_poi10_ob0_all_day | -10057.33 | -100.57 | 3 | 240 | 95 | 145 |
| close_ifvg_rr1.5_poi0_ob0_all_day | -10307.34 | -103.07 | 3 | 155 | 69 | 86 |
| close_ifvg_rr1.5_poi10_ob10_ny_only | -11047.15 | -110.47 | 2 | 137 | 51 | 86 |
| close_ifvg_rr1.2_poi0_ob5_all_day | -11595.42 | -115.95 | 4 | 229 | 108 | 121 |
| close_ifvg_rr2_poi10_ob10_ny_only | -12834.27 | -128.34 | 2 | 137 | 43 | 94 |
| close_ifvg_rr1.8_poi0_ob10_all_day | -12888.79 | -128.89 | 3 | 298 | 120 | 178 |
| close_ifvg_rr1.5_poi5_ob5_all_day | -13653.80 | -136.54 | 4 | 306 | 136 | 170 |
| close_ifvg_rr1.8_poi10_ob10_ny_only | -14874.17 | -148.74 | 2 | 137 | 47 | 90 |
| close_ifvg_rr1.2_poi5_ob0_all_day | -16988.69 | -169.89 | 4 | 196 | 92 | 104 |
| close_ifvg_rr2_poi0_ob0_ny_only | -18376.98 | -183.77 | 2 | 60 | 18 | 42 |
| close_ifvg_rr1.5_poi10_ob0_all_day | -18911.81 | -189.12 | 3 | 241 | 103 | 138 |
| close_ifvg_rr1.8_poi10_ob5_all_day | -19590.65 | -195.91 | 4 | 404 | 158 | 246 |
| close_ifvg_rr1.2_poi0_ob0_all_day | -20161.68 | -201.62 | 3 | 156 | 74 | 82 |
| close_ifvg_rr2_poi5_ob5_ny_only | -21694.23 | -216.94 | 2 | 82 | 23 | 59 |
| close_ifvg_rr1.2_poi10_ob0_all_day | -22081.20 | -220.81 | 3 | 242 | 114 | 128 |
| close_ifvg_rr1.8_poi0_ob0_ny_only | -22165.52 | -221.66 | 1 | 60 | 20 | 40 |
| close_ifvg_rr1.5_poi0_ob10_all_day | -23228.49 | -232.28 | 3 | 298 | 133 | 165 |
| close_ifvg_rr2_poi10_ob0_ny_only | -24527.65 | -245.28 | 1 | 85 | 26 | 59 |
| close_ifvg_rr1.8_poi5_ob5_ny_only | -24619.13 | -246.19 | 2 | 82 | 25 | 57 |
| close_ifvg_rr2_poi5_ob0_ny_only | -25386.33 | -253.86 | 1 | 72 | 20 | 52 |
| close_ifvg_rr1.2_poi0_ob0_ny_only | -26057.34 | -260.57 | 1 | 60 | 27 | 33 |
| close_ifvg_rr2_poi5_ob10_ny_only | -26065.00 | -260.65 | 2 | 105 | 33 | 72 |
| close_ifvg_rr2_poi0_ob10_ny_only | -26081.72 | -260.82 | 1 | 82 | 25 | 57 |
| close_ifvg_rr2_poi0_ob5_ny_only | -26081.72 | -260.82 | 1 | 65 | 18 | 47 |
| close_ifvg_rr1.5_poi0_ob0_ny_only | -27848.33 | -278.48 | 1 | 60 | 22 | 38 |
| close_ifvg_rr2_poi5_ob10_all_day | -28494.50 | -284.94 | 3 | 399 | 154 | 245 |
| close_ifvg_rr1.8_poi10_ob0_ny_only | -28542.00 | -285.42 | 1 | 85 | 29 | 56 |
| close_ifvg_rr1.8_poi5_ob10_ny_only | -28989.90 | -289.90 | 2 | 105 | 35 | 70 |
| close_ifvg_rr1.5_poi5_ob5_ny_only | -29006.48 | -290.06 | 2 | 82 | 27 | 55 |
| close_ifvg_rr1.8_poi5_ob0_ny_only | -29174.87 | -291.75 | 1 | 72 | 22 | 50 |
| close_ifvg_rr1.8_poi0_ob10_ny_only | -29870.25 | -298.70 | 1 | 82 | 27 | 55 |
| close_ifvg_rr1.8_poi0_ob5_ny_only | -29870.25 | -298.70 | 1 | 65 | 20 | 45 |
| close_ifvg_rr1.2_poi5_ob5_all_day | -31871.17 | -318.71 | 4 | 310 | 146 | 164 |
| close_ifvg_rr1.2_poi5_ob0_ny_only | -33066.69 | -330.67 | 1 | 72 | 31 | 41 |
| close_ifvg_rr1.2_poi10_ob0_ny_only | -33111.28 | -331.11 | 1 | 85 | 39 | 46 |
| close_ifvg_rr1.5_poi5_ob10_ny_only | -33377.25 | -333.77 | 2 | 105 | 38 | 67 |
| close_ifvg_rr1.2_poi5_ob5_ny_only | -33393.83 | -333.94 | 2 | 82 | 34 | 48 |
| close_ifvg_rr1.2_poi0_ob10_all_day | -33568.19 | -335.68 | 3 | 300 | 140 | 160 |
| close_ifvg_rr1.2_poi0_ob10_ny_only | -33762.08 | -337.62 | 1 | 82 | 36 | 46 |
| close_ifvg_rr1.2_poi0_ob5_ny_only | -33762.08 | -337.62 | 1 | 65 | 27 | 38 |
| close_ifvg_rr1.5_poi10_ob0_ny_only | -34563.54 | -345.64 | 1 | 85 | 31 | 54 |
| close_ifvg_rr1.5_poi5_ob0_ny_only | -34857.68 | -348.58 | 1 | 72 | 24 | 48 |
| close_ifvg_rr1.5_poi0_ob10_ny_only | -35553.06 | -355.53 | 1 | 82 | 30 | 52 |
| close_ifvg_rr1.5_poi0_ob5_ny_only | -35553.06 | -355.53 | 1 | 65 | 22 | 43 |
| close_ifvg_rr1.8_poi5_ob10_all_day | -35915.77 | -359.16 | 3 | 400 | 162 | 238 |
| close_ifvg_rr1.2_poi5_ob10_ny_only | -37764.59 | -377.65 | 2 | 105 | 46 | 59 |
| close_ifvg_rr1.5_poi10_ob5_all_day | -38024.39 | -380.24 | 4 | 406 | 174 | 232 |
| close_ifvg_rr1.2_poi10_ob5_all_day | -39136.60 | -391.37 | 4 | 410 | 190 | 220 |
| close_ifvg_rr1.5_poi5_ob10_all_day | -47047.69 | -470.48 | 3 | 400 | 179 | 221 |
| close_ifvg_rr2_poi10_ob10_all_day | -55615.96 | -556.16 | 3 | 528 | 198 | 330 |
| close_ifvg_rr1.2_poi5_ob10_all_day | -61280.28 | -612.80 | 2 | 405 | 191 | 214 |
| close_ifvg_rr1.8_poi10_ob10_all_day | -68837.01 | -688.37 | 2 | 530 | 208 | 322 |
| close_ifvg_rr1.2_poi10_ob10_all_day | -77505.57 | -775.06 | 2 | 538 | 249 | 289 |
| close_ifvg_rr1.5_poi10_ob10_all_day | -81832.06 | -818.32 | 1 | 533 | 230 | 303 |

## ETH

| variant | net_6m_usd_at_10 | net_6m_usd_normalized | positive_months | trades | wins | losses |
|---|---:|---:|---:|---:|---:|---:|
| close_ifvg_rr2_poi0_ob0_ny_only | 1808.42 | 180.84 | 4 | 43 | 21 | 22 |
| close_ifvg_rr2_poi0_ob0_all_day | 1579.74 | 157.97 | 3 | 128 | 47 | 81 |
| close_ifvg_rr1.8_poi0_ob0_ny_only | 1536.20 | 153.62 | 4 | 43 | 22 | 21 |
| close_ifvg_rr2_poi0_ob5_ny_only | 1406.10 | 140.61 | 3 | 49 | 23 | 26 |
| close_ifvg_rr1.8_poi0_ob0_all_day | 1255.59 | 125.56 | 3 | 128 | 51 | 77 |
| close_ifvg_rr1.8_poi0_ob5_ny_only | 1163.24 | 116.32 | 3 | 49 | 24 | 25 |
| close_ifvg_rr2_poi5_ob0_all_day | 1149.89 | 114.99 | 3 | 150 | 54 | 96 |
| close_ifvg_rr1.5_poi0_ob0_ny_only | 1127.85 | 112.79 | 4 | 43 | 22 | 21 |
| close_ifvg_rr2_poi5_ob0_ny_only | 1126.90 | 112.69 | 3 | 47 | 22 | 25 |
| close_ifvg_rr1.5_poi0_ob5_ny_only | 1104.92 | 110.49 | 4 | 49 | 25 | 24 |
| close_ifvg_rr2_poi0_ob5_all_day | 1015.47 | 101.55 | 4 | 178 | 65 | 113 |
| close_ifvg_rr1.8_poi5_ob0_ny_only | 884.05 | 88.40 | 3 | 47 | 23 | 24 |
| close_ifvg_rr2_poi10_ob0_all_day | 848.10 | 84.81 | 4 | 181 | 69 | 112 |
| close_ifvg_rr1.8_poi5_ob0_all_day | 825.74 | 82.57 | 3 | 150 | 58 | 92 |
| close_ifvg_rr1.5_poi0_ob0_all_day | 769.38 | 76.94 | 3 | 128 | 57 | 71 |
| close_ifvg_rr2_poi5_ob5_ny_only | 760.35 | 76.03 | 2 | 58 | 25 | 33 |
| close_ifvg_rr1.2_poi0_ob0_ny_only | 719.51 | 71.95 | 4 | 43 | 24 | 19 |
| close_ifvg_rr1.2_poi0_ob5_ny_only | 703.92 | 70.39 | 4 | 49 | 28 | 21 |
| close_ifvg_rr1.5_poi10_ob0_all_day | 695.20 | 69.52 | 3 | 181 | 84 | 97 |
| close_ifvg_rr1.8_poi0_ob5_all_day | 659.01 | 65.90 | 4 | 178 | 69 | 109 |
| close_ifvg_rr2_poi0_ob10_ny_only | 590.88 | 59.09 | 2 | 62 | 27 | 35 |
| close_ifvg_rr2_poi5_ob5_all_day | 585.63 | 58.56 | 4 | 217 | 77 | 140 |
| close_ifvg_rr1.8_poi5_ob5_ny_only | 542.12 | 54.21 | 2 | 58 | 26 | 32 |
| close_ifvg_rr1.5_poi5_ob5_ny_only | 520.75 | 52.07 | 3 | 58 | 28 | 30 |
| close_ifvg_rr1.5_poi5_ob0_ny_only | 519.76 | 51.98 | 3 | 47 | 24 | 23 |
| close_ifvg_rr1.8_poi10_ob0_all_day | 514.84 | 51.48 | 3 | 181 | 75 | 106 |
| close_ifvg_rr1.8_poi0_ob10_ny_only | 372.65 | 37.27 | 2 | 62 | 28 | 34 |
| close_ifvg_rr2_poi0_ob10_all_day | 364.18 | 36.42 | 4 | 224 | 82 | 142 |
| close_ifvg_rr1.5_poi10_ob5_ny_only | 341.33 | 34.13 | 3 | 72 | 35 | 37 |
| close_ifvg_rr1.5_poi10_ob0_ny_only | 341.33 | 34.13 | 3 | 56 | 29 | 27 |
| close_ifvg_rr1.5_poi5_ob0_all_day | 339.53 | 33.95 | 3 | 150 | 66 | 84 |
| close_ifvg_rr1.2_poi0_ob0_all_day | 283.16 | 28.32 | 3 | 128 | 61 | 67 |
| close_ifvg_rr1.8_poi5_ob5_all_day | 229.17 | 22.92 | 4 | 218 | 82 | 136 |
| close_ifvg_rr2_poi10_ob5_ny_only | 206.65 | 20.66 | 2 | 72 | 31 | 41 |
| close_ifvg_rr2_poi10_ob0_ny_only | 206.65 | 20.66 | 2 | 56 | 26 | 30 |
| close_ifvg_rr1.5_poi10_ob10_ny_only | 189.80 | 18.98 | 3 | 91 | 45 | 46 |
| close_ifvg_rr2_poi10_ob5_all_day | 183.21 | 18.32 | 3 | 264 | 100 | 164 |
| close_ifvg_rr2_poi5_ob10_ny_only | 161.03 | 16.10 | 2 | 74 | 32 | 42 |
| close_ifvg_rr1.2_poi5_ob5_ny_only | 156.69 | 15.67 | 3 | 58 | 31 | 27 |
| close_ifvg_rr1.2_poi5_ob0_ny_only | 155.47 | 15.55 | 3 | 47 | 26 | 21 |
| close_ifvg_rr1.5_poi0_ob5_all_day | 124.32 | 12.43 | 4 | 178 | 79 | 99 |
| close_ifvg_rr1.2_poi10_ob0_all_day | 113.69 | 11.37 | 3 | 181 | 91 | 90 |
| close_ifvg_rr2_poi10_ob10_ny_only | 55.12 | 5.51 | 2 | 91 | 40 | 51 |
| close_ifvg_rr1.5_poi0_ob10_ny_only | 45.31 | 4.53 | 2 | 62 | 29 | 33 |
| close_ifvg_rr1.8_poi0_ob10_all_day | -4.12 | -0.41 | 4 | 224 | 87 | 137 |
| close_ifvg_rr1.8_poi10_ob5_ny_only | -11.58 | -1.16 | 2 | 72 | 32 | 40 |
| close_ifvg_rr1.8_poi10_ob0_ny_only | -11.58 | -1.16 | 2 | 56 | 27 | 29 |
| close_ifvg_rr2_poi10_ob10_all_day | -36.88 | -3.69 | 3 | 350 | 135 | 215 |
| close_ifvg_rr1.5_poi10_ob5_all_day | -50.47 | -5.05 | 3 | 266 | 121 | 145 |
| close_ifvg_rr1.8_poi5_ob10_ny_only | -57.20 | -5.72 | 2 | 74 | 34 | 40 |
| close_ifvg_rr1.2_poi10_ob5_ny_only | -67.64 | -6.76 | 3 | 72 | 38 | 34 |
| close_ifvg_rr1.2_poi10_ob0_ny_only | -67.64 | -6.76 | 3 | 56 | 31 | 25 |
| close_ifvg_rr1.2_poi5_ob0_all_day | -146.69 | -14.67 | 3 | 150 | 70 | 80 |
| close_ifvg_rr1.8_poi10_ob10_ny_only | -163.11 | -16.31 | 2 | 91 | 42 | 49 |
| close_ifvg_rr1.5_poi10_ob10_all_day | -178.29 | -17.83 | 2 | 353 | 163 | 190 |
| close_ifvg_rr1.8_poi10_ob5_all_day | -182.36 | -18.24 | 3 | 265 | 107 | 158 |
| close_ifvg_rr1.2_poi10_ob10_ny_only | -219.17 | -21.92 | 3 | 91 | 51 | 40 |
| close_ifvg_rr2_poi5_ob10_all_day | -253.28 | -25.33 | 4 | 278 | 102 | 176 |
| close_ifvg_rr1.2_poi0_ob10_ny_only | -282.03 | -28.20 | 2 | 62 | 35 | 27 |
| close_ifvg_rr1.5_poi5_ob5_all_day | -305.52 | -30.55 | 4 | 218 | 94 | 124 |
| close_ifvg_rr1.5_poi5_ob10_ny_only | -384.54 | -38.45 | 2 | 74 | 36 | 38 |
| close_ifvg_rr1.2_poi0_ob5_all_day | -410.37 | -41.04 | 3 | 178 | 87 | 91 |
| close_ifvg_rr1.8_poi10_ob10_all_day | -481.40 | -48.14 | 3 | 351 | 144 | 207 |
| close_ifvg_rr1.5_poi0_ob10_all_day | -556.56 | -55.66 | 3 | 224 | 99 | 125 |
| close_ifvg_rr1.8_poi5_ob10_all_day | -621.58 | -62.16 | 4 | 279 | 108 | 171 |
| close_ifvg_rr1.2_poi10_ob5_all_day | -680.46 | -68.05 | 3 | 266 | 134 | 132 |
| close_ifvg_rr1.2_poi5_ob10_ny_only | -711.87 | -71.19 | 2 | 74 | 42 | 32 |
| close_ifvg_rr1.2_poi5_ob5_all_day | -840.21 | -84.02 | 3 | 218 | 104 | 114 |
| close_ifvg_rr1.5_poi5_ob10_all_day | -884.37 | -88.44 | 3 | 279 | 123 | 156 |
| close_ifvg_rr1.2_poi10_ob10_all_day | -961.47 | -96.15 | 2 | 355 | 183 | 172 |
| close_ifvg_rr1.2_poi0_ob10_all_day | -1109.00 | -110.90 | 2 | 225 | 112 | 113 |
| close_ifvg_rr1.2_poi5_ob10_all_day | -1471.57 | -147.16 | 2 | 280 | 139 | 141 |

## SOL

| variant | net_6m_usd_at_10 | net_6m_usd_normalized | positive_months | trades | wins | losses |
|---|---:|---:|---:|---:|---:|---:|
| close_ifvg_rr2_poi10_ob10_ny_only | 55.14 | 55.14 | 4 | 8 | 4 | 4 |
| close_ifvg_rr1.8_poi10_ob10_ny_only | 43.05 | 43.05 | 4 | 8 | 4 | 4 |
| close_ifvg_rr1.5_poi10_ob10_ny_only | 39.74 | 39.74 | 5 | 8 | 5 | 3 |
| close_ifvg_rr2_poi10_ob5_ny_only | 37.26 | 37.26 | 3 | 7 | 3 | 4 |
| close_ifvg_rr1.8_poi10_ob5_ny_only | 27.16 | 27.16 | 3 | 7 | 3 | 4 |
| close_ifvg_rr1.5_poi10_ob5_ny_only | 26.83 | 26.83 | 4 | 7 | 4 | 3 |
| close_ifvg_rr2_poi0_ob10_all_day | 26.44 | 26.44 | 5 | 22 | 9 | 13 |
| close_ifvg_rr2_poi0_ob10_ny_only | 25.52 | 25.52 | 2 | 5 | 2 | 3 |
| close_ifvg_rr2_poi5_ob10_ny_only | 25.52 | 25.52 | 2 | 5 | 2 | 3 |
| close_ifvg_rr1.5_poi0_ob10_ny_only | 20.61 | 20.61 | 3 | 5 | 3 | 2 |
| close_ifvg_rr1.5_poi5_ob10_ny_only | 20.61 | 20.61 | 3 | 5 | 3 | 2 |
| close_ifvg_rr1.2_poi10_ob10_ny_only | 19.84 | 19.84 | 4 | 8 | 5 | 3 |
| close_ifvg_rr1.8_poi0_ob10_ny_only | 17.63 | 17.63 | 2 | 5 | 2 | 3 |
| close_ifvg_rr1.8_poi5_ob10_ny_only | 17.63 | 17.63 | 2 | 5 | 2 | 3 |
| close_ifvg_rr2_poi10_ob10_all_day | 17.50 | 17.50 | 3 | 32 | 12 | 20 |
| close_ifvg_rr1.2_poi10_ob5_ny_only | 9.90 | 9.90 | 3 | 7 | 4 | 3 |
| close_ifvg_rr2_poi10_ob0_ny_only | 9.90 | 9.90 | 2 | 6 | 2 | 4 |
| close_ifvg_rr2_poi0_ob0_all_day | 7.93 | 7.93 | 3 | 13 | 5 | 8 |
| close_ifvg_rr1.5_poi0_ob0_ny_only | 7.70 | 7.70 | 2 | 4 | 2 | 2 |
| close_ifvg_rr1.5_poi0_ob5_ny_only | 7.70 | 7.70 | 2 | 4 | 2 | 2 |
| close_ifvg_rr1.5_poi5_ob0_ny_only | 7.70 | 7.70 | 2 | 4 | 2 | 2 |
| close_ifvg_rr1.5_poi5_ob5_ny_only | 7.70 | 7.70 | 2 | 4 | 2 | 2 |
| close_ifvg_rr2_poi0_ob0_ny_only | 7.64 | 7.64 | 1 | 4 | 1 | 3 |
| close_ifvg_rr2_poi0_ob5_ny_only | 7.64 | 7.64 | 1 | 4 | 1 | 3 |
| close_ifvg_rr2_poi5_ob0_ny_only | 7.64 | 7.64 | 1 | 4 | 1 | 3 |
| close_ifvg_rr2_poi5_ob5_ny_only | 7.64 | 7.64 | 1 | 4 | 1 | 3 |
| close_ifvg_rr1.8_poi0_ob10_all_day | 7.42 | 7.42 | 3 | 22 | 9 | 13 |
| close_ifvg_rr1.2_poi0_ob10_ny_only | 7.00 | 7.00 | 2 | 5 | 3 | 2 |
| close_ifvg_rr1.2_poi5_ob10_ny_only | 7.00 | 7.00 | 2 | 5 | 3 | 2 |
| close_ifvg_rr1.5_poi10_ob0_ny_only | 6.85 | 6.85 | 3 | 6 | 3 | 3 |
| close_ifvg_rr2_poi5_ob10_all_day | 4.46 | 4.46 | 5 | 25 | 9 | 16 |
| close_ifvg_rr2_poi5_ob0_all_day | 3.58 | 3.58 | 3 | 14 | 5 | 9 |
| close_ifvg_rr1.8_poi10_ob0_ny_only | 2.75 | 2.75 | 2 | 6 | 2 | 4 |
| close_ifvg_rr1.8_poi0_ob0_ny_only | 1.73 | 1.73 | 1 | 4 | 1 | 3 |
| close_ifvg_rr1.8_poi0_ob5_ny_only | 1.73 | 1.73 | 1 | 4 | 1 | 3 |
| close_ifvg_rr1.8_poi5_ob0_ny_only | 1.73 | 1.73 | 1 | 4 | 1 | 3 |
| close_ifvg_rr1.8_poi5_ob5_ny_only | 1.73 | 1.73 | 1 | 4 | 1 | 3 |
| close_ifvg_rr2_poi0_ob5_all_day | -0.95 | -0.95 | 3 | 14 | 5 | 9 |
| close_ifvg_rr1.2_poi0_ob0_ny_only | -2.94 | -2.94 | 1 | 4 | 2 | 2 |
| close_ifvg_rr1.2_poi0_ob5_ny_only | -2.94 | -2.94 | 1 | 4 | 2 | 2 |
| close_ifvg_rr1.2_poi5_ob0_ny_only | -2.94 | -2.94 | 1 | 4 | 2 | 2 |
| close_ifvg_rr1.2_poi5_ob5_ny_only | -2.94 | -2.94 | 1 | 4 | 2 | 2 |
| close_ifvg_rr1.8_poi0_ob0_all_day | -3.80 | -3.80 | 3 | 13 | 5 | 8 |
| close_ifvg_rr1.2_poi10_ob0_ny_only | -5.66 | -5.66 | 2 | 6 | 3 | 3 |
| close_ifvg_rr1.5_poi0_ob10_all_day | -6.29 | -6.29 | 3 | 22 | 10 | 12 |
| close_ifvg_rr1.5_poi0_ob0_all_day | -6.57 | -6.57 | 4 | 13 | 6 | 7 |
| close_ifvg_rr1.8_poi10_ob10_all_day | -7.06 | -7.06 | 3 | 32 | 12 | 20 |
| close_ifvg_rr1.8_poi5_ob0_all_day | -8.15 | -8.15 | 3 | 14 | 5 | 9 |
| close_ifvg_rr1.5_poi5_ob0_all_day | -10.92 | -10.92 | 4 | 14 | 6 | 8 |
| close_ifvg_rr1.8_poi0_ob5_all_day | -12.68 | -12.68 | 3 | 14 | 5 | 9 |
| close_ifvg_rr1.8_poi5_ob10_all_day | -14.56 | -14.56 | 3 | 25 | 9 | 16 |
| close_ifvg_rr2_poi10_ob0_all_day | -14.80 | -14.80 | 2 | 18 | 6 | 12 |
| close_ifvg_rr1.5_poi0_ob5_all_day | -15.45 | -15.45 | 4 | 14 | 6 | 8 |
| close_ifvg_rr2_poi10_ob5_all_day | -21.37 | -21.37 | 2 | 23 | 7 | 16 |
| close_ifvg_rr2_poi5_ob5_all_day | -22.93 | -22.93 | 2 | 17 | 5 | 12 |
| close_ifvg_rr1.2_poi0_ob0_all_day | -25.94 | -25.94 | 3 | 13 | 6 | 7 |
| close_ifvg_rr1.8_poi10_ob0_all_day | -27.77 | -27.77 | 2 | 18 | 6 | 12 |
| close_ifvg_rr1.5_poi5_ob10_all_day | -28.27 | -28.27 | 1 | 25 | 10 | 15 |
| close_ifvg_rr1.5_poi10_ob10_all_day | -29.08 | -29.08 | 3 | 32 | 13 | 19 |
| close_ifvg_rr1.2_poi5_ob0_all_day | -30.29 | -30.29 | 3 | 14 | 6 | 8 |
| close_ifvg_rr1.5_poi10_ob0_all_day | -32.41 | -32.41 | 3 | 18 | 7 | 11 |
| close_ifvg_rr1.8_poi5_ob5_all_day | -34.66 | -34.66 | 2 | 17 | 5 | 12 |
| close_ifvg_rr1.2_poi0_ob5_all_day | -34.82 | -34.82 | 3 | 14 | 6 | 8 |
| close_ifvg_rr1.2_poi0_ob10_all_day | -36.60 | -36.60 | 2 | 22 | 10 | 12 |
| close_ifvg_rr1.8_poi10_ob5_all_day | -37.29 | -37.29 | 2 | 23 | 7 | 16 |
| close_ifvg_rr1.5_poi5_ob5_all_day | -37.43 | -37.43 | 2 | 17 | 6 | 11 |
| close_ifvg_rr1.5_poi10_ob5_all_day | -46.36 | -46.36 | 3 | 23 | 8 | 15 |
| close_ifvg_rr1.2_poi10_ob0_all_day | -53.65 | -53.65 | 2 | 18 | 7 | 11 |
| close_ifvg_rr1.2_poi5_ob5_all_day | -56.80 | -56.80 | 2 | 17 | 6 | 11 |
| close_ifvg_rr1.2_poi5_ob10_all_day | -58.58 | -58.58 | 1 | 25 | 10 | 15 |
| close_ifvg_rr1.2_poi10_ob10_all_day | -67.71 | -67.71 | 2 | 32 | 13 | 19 |
| close_ifvg_rr1.2_poi10_ob5_all_day | -72.02 | -72.02 | 2 | 23 | 8 | 15 |
