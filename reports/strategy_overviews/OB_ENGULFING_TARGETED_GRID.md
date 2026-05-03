# OB Engulfing Targeted Grid

| section | variant | asset | timeframe | split | trades | win_rate_% | profit_r | pf_r | pnl_% | verdict |
|---|---|---|---|---|---:|---:|---:|---:|---:|---|
| mnq_quick | quick prev_open | MNQ | 15m | full | 390 | 32.56 | -11.11 | 0.96 | -33.39 | TESTED |
| mnq_quick | quick pair_mid | MNQ | 15m | full | 402 | 31.34 | -25.09 | 0.91 | -40.98 | TESTED |
| mnq_quick | quick close | MNQ | 15m | full | 422 | 31.04 | -30.04 | 0.90 | -45.54 | TESTED |
| mnq_session | session PM 06:00-09:15 | MNQ | 15m | full | 122 | 28.69 | -17.63 | 0.80 | -22.63 | TESTED |
| mnq_session | session NYAM 09:15-12:15 | MNQ | 15m | full | 189 | 34.92 | 8.12 | 1.07 | -3.87 | TESTED |
| mnq_session | session NYPM 12:15-15:30 | MNQ | 15m | full | 129 | 27.91 | -21.77 | 0.77 | -26.49 | TESTED |
| mnq_quality | quality off (baseline) | MNQ | 15m | full | 390 | 32.56 | -11.11 | 0.96 | -33.39 | TESTED |
| mnq_quality | quality body>=35% | MNQ | 15m | full | 343 | 33.82 | 3.35 | 1.01 | -18.92 | TESTED |
| mnq_quality | quality body>=40%, range>=1.1x prev | MNQ | 15m | full | 243 | 36.21 | 19.74 | 1.13 | 4.45 | TESTED |
| mnq_quality | quality body>=45%, range>=1.15x avg20 | MNQ | 15m | full | 200 | 34.50 | 6.17 | 1.05 | -6.76 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill4 trailnone q0 | MNQ | 15m | full | 388 | 36.34 | -37.76 | 0.85 | -49.93 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill4 trailnone q1 | MNQ | 15m | full | 239 | 41.00 | 4.69 | 1.03 | -9.64 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill4 trailprog q0 | MNQ | 15m | full | 391 | 32.74 | -32.20 | 0.85 | -46.14 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill4 trailprog q1 | MNQ | 15m | full | 239 | 35.56 | 1.25 | 1.01 | -12.59 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill6 trailnone q0 | MNQ | 15m | full | 403 | 36.72 | -35.32 | 0.86 | -49.08 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill6 trailnone q1 | MNQ | 15m | full | 248 | 40.32 | 0.66 | 1.00 | -14.08 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill6 trailprog q0 | MNQ | 15m | full | 406 | 33.25 | -30.76 | 0.87 | -45.82 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill6 trailprog q1 | MNQ | 15m | full | 248 | 35.48 | -1.28 | 0.99 | -15.46 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill8 trailnone q0 | MNQ | 15m | full | 402 | 36.32 | -39.27 | 0.85 | -51.39 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill8 trailnone q1 | MNQ | 15m | full | 248 | 39.92 | -1.80 | 0.99 | -16.37 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill8 trailprog q0 | MNQ | 15m | full | 405 | 32.84 | -34.70 | 0.85 | -48.27 | TESTED |
| mnq_bounded | sweep prev_open rr1.5 fill8 trailprog q1 | MNQ | 15m | full | 248 | 35.08 | -3.75 | 0.97 | -17.71 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill4 trailnone q0 | MNQ | 15m | full | 379 | 33.51 | -25.49 | 0.90 | -43.52 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill4 trailnone q1 | MNQ | 15m | full | 235 | 37.87 | 12.97 | 1.09 | -1.85 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill4 trailprog q0 | MNQ | 15m | full | 391 | 32.74 | -22.05 | 0.90 | -40.24 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill4 trailprog q1 | MNQ | 15m | full | 239 | 35.56 | 7.75 | 1.06 | -6.60 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill6 trailnone q0 | MNQ | 15m | full | 394 | 34.01 | -20.95 | 0.92 | -40.87 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill6 trailnone q1 | MNQ | 15m | full | 244 | 37.70 | 12.33 | 1.08 | -3.14 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill6 trailprog q0 | MNQ | 15m | full | 406 | 33.25 | -19.80 | 0.91 | -39.18 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill6 trailprog q1 | MNQ | 15m | full | 248 | 35.48 | 6.11 | 1.05 | -8.79 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill8 trailnone q0 | MNQ | 15m | full | 396 | 33.84 | -22.95 | 0.91 | -42.43 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill8 trailnone q1 | MNQ | 15m | full | 245 | 37.55 | 11.33 | 1.07 | -4.30 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill8 trailprog q0 | MNQ | 15m | full | 408 | 33.09 | -21.80 | 0.91 | -40.76 | TESTED |
| mnq_bounded | sweep prev_open rr1.8 fill8 trailprog q1 | MNQ | 15m | full | 249 | 35.34 | 5.11 | 1.04 | -9.88 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill4 trailnone q0 | MNQ | 15m | full | 375 | 32.00 | -17.05 | 0.93 | -37.37 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill4 trailnone q1 | MNQ | 15m | full | 234 | 36.32 | 19.79 | 1.13 | 5.16 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill4 trailprog q0 | MNQ | 15m | full | 388 | 32.99 | -13.65 | 0.94 | -34.43 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill4 trailprog q1 | MNQ | 15m | full | 238 | 35.71 | 12.45 | 1.10 | -2.09 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill6 trailnone q0 | MNQ | 15m | full | 390 | 32.56 | -11.11 | 0.96 | -33.39 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill6 trailnone q1 | MNQ | 15m | full | 243 | 36.21 | 19.74 | 1.13 | 4.45 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill6 trailprog q0 | MNQ | 15m | full | 403 | 33.50 | -10.20 | 0.96 | -32.31 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill6 trailprog q1 | MNQ | 15m | full | 247 | 35.63 | 11.41 | 1.09 | -3.77 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill8 trailnone q0 | MNQ | 15m | full | 392 | 32.40 | -13.11 | 0.95 | -35.10 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill8 trailnone q1 | MNQ | 15m | full | 244 | 36.07 | 18.74 | 1.12 | 3.20 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill8 trailprog q0 | MNQ | 15m | full | 405 | 33.33 | -12.20 | 0.95 | -34.05 | TESTED |
| mnq_bounded | sweep prev_open rr2.0 fill8 trailprog q1 | MNQ | 15m | full | 248 | 35.48 | 10.41 | 1.08 | -4.93 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill4 trailnone q0 | MNQ | 15m | full | 413 | 38.01 | -21.82 | 0.91 | -38.87 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill4 trailnone q1 | MNQ | 15m | full | 252 | 40.87 | 4.76 | 1.03 | -9.55 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill4 trailprog q0 | MNQ | 15m | full | 423 | 33.57 | -11.24 | 0.95 | -31.28 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill4 trailprog q1 | MNQ | 15m | full | 257 | 36.19 | 10.80 | 1.08 | -3.62 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill6 trailnone q0 | MNQ | 15m | full | 413 | 38.01 | -21.82 | 0.91 | -38.87 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill6 trailnone q1 | MNQ | 15m | full | 252 | 40.87 | 4.76 | 1.03 | -9.55 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill6 trailprog q0 | MNQ | 15m | full | 423 | 33.57 | -11.24 | 0.95 | -31.28 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill6 trailprog q1 | MNQ | 15m | full | 257 | 36.19 | 10.80 | 1.08 | -3.62 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill8 trailnone q0 | MNQ | 15m | full | 413 | 38.01 | -21.82 | 0.91 | -38.87 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill8 trailnone q1 | MNQ | 15m | full | 252 | 40.87 | 4.76 | 1.03 | -9.55 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill8 trailprog q0 | MNQ | 15m | full | 423 | 33.57 | -11.24 | 0.95 | -31.28 | TESTED |
| mnq_bounded | sweep pair_mid rr1.5 fill8 trailprog q1 | MNQ | 15m | full | 257 | 36.19 | 10.80 | 1.08 | -3.62 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill4 trailnone q0 | MNQ | 15m | full | 405 | 33.33 | -28.15 | 0.90 | -43.07 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill4 trailnone q1 | MNQ | 15m | full | 250 | 36.80 | 6.92 | 1.04 | -7.47 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill4 trailprog q0 | MNQ | 15m | full | 420 | 33.81 | -7.07 | 0.97 | -28.21 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill4 trailprog q1 | MNQ | 15m | full | 256 | 36.33 | 16.96 | 1.13 | 2.58 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill6 trailnone q0 | MNQ | 15m | full | 405 | 33.33 | -28.15 | 0.90 | -43.07 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill6 trailnone q1 | MNQ | 15m | full | 250 | 36.80 | 6.92 | 1.04 | -7.47 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill6 trailprog q0 | MNQ | 15m | full | 420 | 33.81 | -7.07 | 0.97 | -28.21 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill6 trailprog q1 | MNQ | 15m | full | 256 | 36.33 | 16.96 | 1.13 | 2.58 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill8 trailnone q0 | MNQ | 15m | full | 405 | 33.33 | -28.15 | 0.90 | -43.07 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill8 trailnone q1 | MNQ | 15m | full | 250 | 36.80 | 6.92 | 1.04 | -7.47 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill8 trailprog q0 | MNQ | 15m | full | 420 | 33.81 | -7.07 | 0.97 | -28.21 | TESTED |
| mnq_bounded | sweep pair_mid rr1.8 fill8 trailprog q1 | MNQ | 15m | full | 256 | 36.33 | 16.96 | 1.13 | 2.58 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill4 trailnone q0 | MNQ | 15m | full | 402 | 31.34 | -25.09 | 0.91 | -40.98 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill4 trailnone q1 | MNQ | 15m | full | 249 | 34.94 | 11.35 | 1.07 | -2.98 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill4 trailprog q0 | MNQ | 15m | full | 419 | 33.65 | 0.89 | 1.00 | -21.46 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill4 trailprog q1 | MNQ | 15m | full | 256 | 36.33 | 23.16 | 1.18 | 9.55 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill6 trailnone q0 | MNQ | 15m | full | 402 | 31.34 | -25.09 | 0.91 | -40.98 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill6 trailnone q1 | MNQ | 15m | full | 249 | 34.94 | 11.35 | 1.07 | -2.98 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill6 trailprog q0 | MNQ | 15m | full | 419 | 33.65 | 0.89 | 1.00 | -21.46 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill6 trailprog q1 | MNQ | 15m | full | 256 | 36.33 | 23.16 | 1.18 | 9.55 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill8 trailnone q0 | MNQ | 15m | full | 402 | 31.34 | -25.09 | 0.91 | -40.98 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill8 trailnone q1 | MNQ | 15m | full | 249 | 34.94 | 11.35 | 1.07 | -2.98 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill8 trailprog q0 | MNQ | 15m | full | 419 | 33.65 | 0.89 | 1.00 | -21.46 | TESTED |
| mnq_bounded | sweep pair_mid rr2.0 fill8 trailprog q1 | MNQ | 15m | full | 256 | 36.33 | 23.16 | 1.18 | 9.55 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill3 trailnone | MNQ | 15m | full | 106 | 41.51 | 12.48 | 1.20 | 6.26 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill3 trailprog | MNQ | 15m | full | 106 | 35.85 | 6.30 | 1.12 | 0.06 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill3 trailbe1r | MNQ | 15m | full | 106 | 32.08 | 5.51 | 1.11 | -0.75 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill3 trailstep_half_r | MNQ | 15m | full | 106 | 35.85 | 6.30 | 1.12 | 0.06 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill3 trailtrail05_at15 | MNQ | 15m | full | 106 | 44.34 | 12.16 | 1.21 | 6.05 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill4 trailnone | MNQ | 15m | full | 113 | 41.59 | 13.54 | 1.21 | 7.12 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill4 trailprog | MNQ | 15m | full | 113 | 37.17 | 8.06 | 1.15 | 1.59 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill4 trailbe1r | MNQ | 15m | full | 113 | 32.74 | 7.58 | 1.14 | 1.07 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill4 trailstep_half_r | MNQ | 15m | full | 113 | 37.17 | 8.06 | 1.15 | 1.59 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill4 trailtrail05_at15 | MNQ | 15m | full | 113 | 45.13 | 13.92 | 1.22 | 7.68 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill5 trailnone | MNQ | 15m | full | 116 | 40.52 | 10.54 | 1.15 | 3.66 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill5 trailprog | MNQ | 15m | full | 117 | 35.90 | 5.06 | 1.09 | -1.73 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill5 trailbe1r | MNQ | 15m | full | 116 | 31.90 | 4.58 | 1.08 | -2.19 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill5 trailstep_half_r | MNQ | 15m | full | 117 | 35.90 | 5.06 | 1.09 | -1.73 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill5 trailtrail05_at15 | MNQ | 15m | full | 117 | 43.59 | 9.92 | 1.15 | 3.06 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill6 trailnone | MNQ | 15m | full | 120 | 40.00 | 9.24 | 1.13 | 2.08 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill6 trailprog | MNQ | 15m | full | 121 | 35.54 | 3.76 | 1.06 | -3.24 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill6 trailbe1r | MNQ | 15m | full | 120 | 31.67 | 3.28 | 1.05 | -3.69 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill6 trailstep_half_r | MNQ | 15m | full | 121 | 35.54 | 3.76 | 1.06 | -3.24 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill6 trailtrail05_at15 | MNQ | 15m | full | 121 | 42.98 | 8.62 | 1.12 | 1.48 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill8 trailnone | MNQ | 15m | full | 121 | 39.67 | 8.24 | 1.11 | 0.94 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill8 trailprog | MNQ | 15m | full | 122 | 35.25 | 2.76 | 1.04 | -4.32 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill8 trailbe1r | MNQ | 15m | full | 121 | 31.40 | 2.28 | 1.04 | -4.76 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 35.25 | 2.76 | 1.04 | -4.32 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.7 fill8 trailtrail05_at15 | MNQ | 15m | full | 122 | 42.62 | 7.62 | 1.11 | 0.35 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill3 trailnone | MNQ | 15m | full | 106 | 41.51 | 16.88 | 1.27 | 11.08 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill3 trailprog | MNQ | 15m | full | 106 | 35.85 | 5.70 | 1.11 | -0.55 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill3 trailbe1r | MNQ | 15m | full | 106 | 30.19 | 5.32 | 1.10 | -0.93 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill3 trailstep_half_r | MNQ | 15m | full | 106 | 35.85 | 5.70 | 1.11 | -0.55 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill3 trailtrail05_at15 | MNQ | 15m | full | 106 | 44.34 | 12.26 | 1.21 | 6.14 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill4 trailnone | MNQ | 15m | full | 113 | 41.59 | 18.24 | 1.28 | 12.32 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill4 trailprog | MNQ | 15m | full | 113 | 37.17 | 7.76 | 1.14 | 1.27 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill4 trailbe1r | MNQ | 15m | full | 113 | 30.97 | 7.69 | 1.14 | 1.19 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill4 trailstep_half_r | MNQ | 15m | full | 113 | 37.17 | 7.76 | 1.14 | 1.27 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill4 trailtrail05_at15 | MNQ | 15m | full | 113 | 45.13 | 14.32 | 1.23 | 8.09 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill5 trailnone | MNQ | 15m | full | 116 | 40.52 | 15.24 | 1.22 | 8.70 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill5 trailprog | MNQ | 15m | full | 117 | 35.90 | 4.76 | 1.08 | -2.04 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill5 trailbe1r | MNQ | 15m | full | 116 | 30.17 | 4.69 | 1.08 | -2.06 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill5 trailstep_half_r | MNQ | 15m | full | 117 | 35.90 | 4.76 | 1.08 | -2.04 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill5 trailtrail05_at15 | MNQ | 15m | full | 117 | 43.59 | 10.32 | 1.16 | 3.46 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill6 trailnone | MNQ | 15m | full | 120 | 40.00 | 14.04 | 1.20 | 7.16 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill6 trailprog | MNQ | 15m | full | 121 | 35.54 | 3.56 | 1.06 | -3.45 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill6 trailbe1r | MNQ | 15m | full | 120 | 30.00 | 3.49 | 1.06 | -3.46 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill6 trailstep_half_r | MNQ | 15m | full | 121 | 35.54 | 3.56 | 1.06 | -3.45 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill6 trailtrail05_at15 | MNQ | 15m | full | 121 | 42.98 | 9.12 | 1.13 | 1.98 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill8 trailnone | MNQ | 15m | full | 121 | 39.67 | 13.04 | 1.18 | 5.96 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill8 trailprog | MNQ | 15m | full | 122 | 35.25 | 2.56 | 1.04 | -4.52 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill8 trailbe1r | MNQ | 15m | full | 121 | 29.75 | 2.49 | 1.04 | -4.54 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 35.25 | 2.56 | 1.04 | -4.52 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.8 fill8 trailtrail05_at15 | MNQ | 15m | full | 122 | 42.62 | 8.12 | 1.12 | 0.85 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill3 trailnone | MNQ | 15m | full | 106 | 39.62 | 15.49 | 1.24 | 9.41 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill3 trailprog | MNQ | 15m | full | 106 | 35.85 | 4.50 | 1.09 | -1.86 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill3 trailbe1r | MNQ | 15m | full | 106 | 28.30 | 4.73 | 1.09 | -1.64 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill3 trailstep_half_r | MNQ | 15m | full | 106 | 35.85 | 4.50 | 1.09 | -1.86 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill3 trailtrail05_at15 | MNQ | 15m | full | 106 | 44.34 | 11.76 | 1.20 | 5.49 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill4 trailnone | MNQ | 15m | full | 113 | 39.82 | 17.15 | 1.25 | 10.97 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill4 trailprog | MNQ | 15m | full | 113 | 37.17 | 6.86 | 1.12 | 0.24 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill4 trailbe1r | MNQ | 15m | full | 113 | 29.20 | 7.40 | 1.13 | 0.77 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill4 trailstep_half_r | MNQ | 15m | full | 113 | 37.17 | 6.86 | 1.12 | 0.24 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill4 trailtrail05_at15 | MNQ | 15m | full | 113 | 45.13 | 14.12 | 1.23 | 7.75 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill5 trailnone | MNQ | 15m | full | 116 | 38.79 | 14.15 | 1.20 | 7.40 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill5 trailprog | MNQ | 15m | full | 117 | 35.90 | 3.86 | 1.07 | -3.05 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill5 trailbe1r | MNQ | 15m | full | 116 | 28.45 | 4.40 | 1.08 | -2.48 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill5 trailstep_half_r | MNQ | 15m | full | 117 | 35.90 | 3.86 | 1.07 | -3.05 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill5 trailtrail05_at15 | MNQ | 15m | full | 117 | 43.59 | 10.12 | 1.15 | 3.13 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill6 trailnone | MNQ | 15m | full | 120 | 38.33 | 13.05 | 1.18 | 5.97 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill6 trailprog | MNQ | 15m | full | 121 | 35.54 | 2.76 | 1.05 | -4.34 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill6 trailbe1r | MNQ | 15m | full | 120 | 28.33 | 3.30 | 1.05 | -3.78 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill6 trailstep_half_r | MNQ | 15m | full | 121 | 35.54 | 2.76 | 1.05 | -4.34 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill6 trailtrail05_at15 | MNQ | 15m | full | 121 | 42.98 | 9.02 | 1.13 | 1.76 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill8 trailnone | MNQ | 15m | full | 121 | 38.02 | 12.05 | 1.16 | 4.79 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill8 trailprog | MNQ | 15m | full | 122 | 35.25 | 1.76 | 1.03 | -5.41 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill8 trailbe1r | MNQ | 15m | full | 121 | 28.10 | 2.30 | 1.04 | -4.85 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 35.25 | 1.76 | 1.03 | -5.41 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr1.9 fill8 trailtrail05_at15 | MNQ | 15m | full | 122 | 42.62 | 8.02 | 1.11 | 0.62 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill3 trailnone | MNQ | 15m | full | 105 | 39.05 | 17.69 | 1.28 | 11.92 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill3 trailprog | MNQ | 15m | full | 105 | 36.19 | 5.20 | 1.10 | -1.08 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill3 trailbe1r | MNQ | 15m | full | 105 | 27.62 | 6.74 | 1.13 | 0.41 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill3 trailstep_half_r | MNQ | 15m | full | 105 | 36.19 | 5.20 | 1.10 | -1.08 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill3 trailtrail05_at15 | MNQ | 15m | full | 105 | 44.76 | 11.66 | 1.20 | 5.48 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill4 trailnone | MNQ | 15m | full | 112 | 39.29 | 19.65 | 1.29 | 13.87 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill4 trailprog | MNQ | 15m | full | 112 | 37.50 | 7.86 | 1.15 | 1.35 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill4 trailbe1r | MNQ | 15m | full | 112 | 28.57 | 9.70 | 1.18 | 3.20 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill4 trailstep_half_r | MNQ | 15m | full | 112 | 37.50 | 7.86 | 1.15 | 1.35 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill4 trailtrail05_at15 | MNQ | 15m | full | 112 | 45.54 | 14.32 | 1.23 | 8.08 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill5 trailnone | MNQ | 15m | full | 115 | 38.26 | 16.65 | 1.23 | 10.21 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill5 trailprog | MNQ | 15m | full | 116 | 36.21 | 4.86 | 1.09 | -1.97 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill5 trailbe1r | MNQ | 15m | full | 115 | 27.83 | 6.70 | 1.12 | -0.12 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill5 trailstep_half_r | MNQ | 15m | full | 116 | 36.21 | 4.86 | 1.09 | -1.97 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill5 trailtrail05_at15 | MNQ | 15m | full | 116 | 43.97 | 10.32 | 1.16 | 3.44 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill6 trailnone | MNQ | 15m | full | 119 | 37.82 | 15.65 | 1.21 | 8.86 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill6 trailprog | MNQ | 15m | full | 120 | 35.83 | 3.86 | 1.06 | -3.17 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill6 trailbe1r | MNQ | 15m | full | 119 | 27.73 | 5.70 | 1.10 | -1.34 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill6 trailstep_half_r | MNQ | 15m | full | 120 | 35.83 | 3.86 | 1.06 | -3.17 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill6 trailtrail05_at15 | MNQ | 15m | full | 120 | 43.33 | 9.32 | 1.14 | 2.18 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill8 trailnone | MNQ | 15m | full | 120 | 37.50 | 14.65 | 1.20 | 7.65 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill8 trailprog | MNQ | 15m | full | 121 | 35.54 | 2.86 | 1.05 | -4.25 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill8 trailbe1r | MNQ | 15m | full | 120 | 27.50 | 4.70 | 1.08 | -2.44 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill8 trailstep_half_r | MNQ | 15m | full | 121 | 35.54 | 2.86 | 1.05 | -4.25 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.0 fill8 trailtrail05_at15 | MNQ | 15m | full | 121 | 42.98 | 8.32 | 1.12 | 1.04 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill3 trailnone | MNQ | 15m | full | 105 | 37.14 | 15.59 | 1.24 | 9.48 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill3 trailprog | MNQ | 15m | full | 105 | 36.19 | 5.50 | 1.11 | -0.76 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill3 trailbe1r | MNQ | 15m | full | 105 | 23.81 | 1.25 | 1.02 | -4.96 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill3 trailstep_half_r | MNQ | 15m | full | 105 | 36.19 | 5.50 | 1.11 | -0.76 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill3 trailtrail05_at15 | MNQ | 15m | full | 105 | 44.76 | 11.56 | 1.20 | 5.40 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill4 trailnone | MNQ | 15m | full | 112 | 37.50 | 17.86 | 1.26 | 11.72 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill4 trailprog | MNQ | 15m | full | 112 | 37.50 | 8.46 | 1.16 | 1.99 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill4 trailbe1r | MNQ | 15m | full | 112 | 25.00 | 4.51 | 1.08 | -2.02 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill4 trailstep_half_r | MNQ | 15m | full | 112 | 37.50 | 8.46 | 1.16 | 1.99 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill4 trailtrail05_at15 | MNQ | 15m | full | 112 | 45.54 | 14.52 | 1.24 | 8.33 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill5 trailnone | MNQ | 15m | full | 115 | 36.52 | 14.86 | 1.20 | 8.13 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill5 trailprog | MNQ | 15m | full | 116 | 36.21 | 5.46 | 1.10 | -1.34 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill5 trailbe1r | MNQ | 15m | full | 115 | 24.35 | 1.51 | 1.03 | -5.19 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill5 trailstep_half_r | MNQ | 15m | full | 116 | 36.21 | 5.46 | 1.10 | -1.34 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill5 trailtrail05_at15 | MNQ | 15m | full | 116 | 43.97 | 10.52 | 1.16 | 3.69 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill6 trailnone | MNQ | 15m | full | 119 | 36.13 | 13.96 | 1.18 | 6.91 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill6 trailprog | MNQ | 15m | full | 120 | 35.83 | 4.56 | 1.08 | -2.46 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill6 trailbe1r | MNQ | 15m | full | 119 | 24.37 | 0.61 | 1.01 | -6.26 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill6 trailstep_half_r | MNQ | 15m | full | 120 | 35.83 | 4.56 | 1.08 | -2.46 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill6 trailtrail05_at15 | MNQ | 15m | full | 120 | 43.33 | 9.62 | 1.14 | 2.53 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill8 trailnone | MNQ | 15m | full | 120 | 35.83 | 12.96 | 1.17 | 5.72 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill8 trailprog | MNQ | 15m | full | 121 | 35.54 | 3.56 | 1.06 | -3.54 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill8 trailbe1r | MNQ | 15m | full | 120 | 24.17 | -0.38 | 0.99 | -7.30 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill8 trailstep_half_r | MNQ | 15m | full | 121 | 35.54 | 3.56 | 1.06 | -3.54 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.1 fill8 trailtrail05_at15 | MNQ | 15m | full | 121 | 42.98 | 8.62 | 1.12 | 1.38 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill3 trailnone | MNQ | 15m | full | 105 | 36.19 | 16.31 | 1.24 | 10.12 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill3 trailprog | MNQ | 15m | full | 105 | 36.19 | 6.60 | 1.13 | 0.29 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill3 trailbe1r | MNQ | 15m | full | 105 | 22.86 | 1.57 | 1.03 | -4.73 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill3 trailstep_half_r | MNQ | 15m | full | 105 | 36.19 | 6.60 | 1.13 | 0.29 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill3 trailtrail05_at15 | MNQ | 15m | full | 105 | 44.76 | 11.06 | 1.19 | 4.76 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill4 trailnone | MNQ | 15m | full | 112 | 36.61 | 18.88 | 1.27 | 12.72 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill4 trailprog | MNQ | 15m | full | 112 | 37.50 | 9.86 | 1.18 | 3.40 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill4 trailbe1r | MNQ | 15m | full | 112 | 24.11 | 5.13 | 1.10 | -1.49 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill4 trailstep_half_r | MNQ | 15m | full | 112 | 37.50 | 9.86 | 1.18 | 3.40 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill4 trailtrail05_at15 | MNQ | 15m | full | 112 | 45.54 | 14.32 | 1.23 | 8.00 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill5 trailnone | MNQ | 15m | full | 115 | 35.65 | 15.88 | 1.21 | 9.09 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill5 trailprog | MNQ | 15m | full | 116 | 36.21 | 6.86 | 1.12 | 0.01 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill5 trailbe1r | MNQ | 15m | full | 115 | 23.48 | 2.13 | 1.04 | -4.67 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill5 trailstep_half_r | MNQ | 15m | full | 116 | 36.21 | 6.86 | 1.12 | 0.01 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill5 trailtrail05_at15 | MNQ | 15m | full | 116 | 43.97 | 10.32 | 1.16 | 3.37 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill6 trailnone | MNQ | 15m | full | 119 | 35.29 | 15.08 | 1.20 | 7.97 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill6 trailprog | MNQ | 15m | full | 120 | 35.83 | 6.06 | 1.10 | -1.01 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill6 trailbe1r | MNQ | 15m | full | 119 | 23.53 | 1.33 | 1.02 | -5.65 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill6 trailstep_half_r | MNQ | 15m | full | 120 | 35.83 | 6.06 | 1.10 | -1.01 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill6 trailtrail05_at15 | MNQ | 15m | full | 120 | 43.33 | 9.52 | 1.14 | 2.30 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill8 trailnone | MNQ | 15m | full | 120 | 35.00 | 14.08 | 1.18 | 6.77 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill8 trailprog | MNQ | 15m | full | 121 | 35.54 | 5.06 | 1.08 | -2.11 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill8 trailbe1r | MNQ | 15m | full | 120 | 23.33 | 0.33 | 1.01 | -6.70 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill8 trailstep_half_r | MNQ | 15m | full | 121 | 35.54 | 5.06 | 1.08 | -2.11 | TESTED |
| mnq_nyam_fine | nyam_fine prev_open rr2.2 fill8 trailtrail05_at15 | MNQ | 15m | full | 121 | 42.98 | 8.52 | 1.12 | 1.16 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill3 trailnone | MNQ | 15m | full | 122 | 44.26 | 23.52 | 1.35 | 18.27 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill3 trailprog | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill3 trailbe1r | MNQ | 15m | full | 123 | 35.77 | 17.56 | 1.31 | 11.33 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill3 trailstep_half_r | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill3 trailtrail05_at15 | MNQ | 15m | full | 122 | 45.90 | 19.17 | 1.29 | 13.14 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill4 trailnone | MNQ | 15m | full | 122 | 44.26 | 23.52 | 1.35 | 18.27 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill4 trailprog | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill4 trailbe1r | MNQ | 15m | full | 123 | 35.77 | 17.56 | 1.31 | 11.33 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill4 trailstep_half_r | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill4 trailtrail05_at15 | MNQ | 15m | full | 122 | 45.90 | 19.17 | 1.29 | 13.14 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill5 trailnone | MNQ | 15m | full | 122 | 44.26 | 23.52 | 1.35 | 18.27 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill5 trailprog | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill5 trailbe1r | MNQ | 15m | full | 123 | 35.77 | 17.56 | 1.31 | 11.33 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill5 trailstep_half_r | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill5 trailtrail05_at15 | MNQ | 15m | full | 122 | 45.90 | 19.17 | 1.29 | 13.14 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill6 trailnone | MNQ | 15m | full | 122 | 44.26 | 23.52 | 1.35 | 18.27 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill6 trailprog | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill6 trailbe1r | MNQ | 15m | full | 123 | 35.77 | 17.56 | 1.31 | 11.33 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill6 trailstep_half_r | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill6 trailtrail05_at15 | MNQ | 15m | full | 122 | 45.90 | 19.17 | 1.29 | 13.14 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill8 trailnone | MNQ | 15m | full | 122 | 44.26 | 23.52 | 1.35 | 18.27 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill8 trailprog | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill8 trailbe1r | MNQ | 15m | full | 123 | 35.77 | 17.56 | 1.31 | 11.33 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill8 trailstep_half_r | MNQ | 15m | full | 123 | 40.65 | 19.19 | 1.34 | 13.26 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.7 fill8 trailtrail05_at15 | MNQ | 15m | full | 122 | 45.90 | 19.17 | 1.29 | 13.14 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill3 trailnone | MNQ | 15m | full | 120 | 41.67 | 19.73 | 1.28 | 13.93 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill3 trailprog | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill3 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 17.57 | 1.31 | 11.44 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill3 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill3 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 19.77 | 1.30 | 13.90 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill4 trailnone | MNQ | 15m | full | 120 | 41.67 | 19.73 | 1.28 | 13.93 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill4 trailprog | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill4 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 17.57 | 1.31 | 11.44 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill4 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill4 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 19.77 | 1.30 | 13.90 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill5 trailnone | MNQ | 15m | full | 120 | 41.67 | 19.73 | 1.28 | 13.93 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill5 trailprog | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill5 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 17.57 | 1.31 | 11.44 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill5 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill5 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 19.77 | 1.30 | 13.90 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill6 trailnone | MNQ | 15m | full | 120 | 41.67 | 19.73 | 1.28 | 13.93 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill6 trailprog | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill6 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 17.57 | 1.31 | 11.44 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill6 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill6 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 19.77 | 1.30 | 13.90 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill8 trailnone | MNQ | 15m | full | 120 | 41.67 | 19.73 | 1.28 | 13.93 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill8 trailprog | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill8 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 17.57 | 1.31 | 11.44 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 20.59 | 1.37 | 14.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.8 fill8 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 19.77 | 1.30 | 13.90 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill3 trailnone | MNQ | 15m | full | 120 | 41.67 | 24.73 | 1.35 | 19.83 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill3 trailprog | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill3 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 21.67 | 1.39 | 16.15 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill3 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill3 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 24.17 | 1.37 | 19.07 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill4 trailnone | MNQ | 15m | full | 120 | 41.67 | 24.73 | 1.35 | 19.83 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill4 trailprog | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill4 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 21.67 | 1.39 | 16.15 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill4 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill4 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 24.17 | 1.37 | 19.07 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill5 trailnone | MNQ | 15m | full | 120 | 41.67 | 24.73 | 1.35 | 19.83 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill5 trailprog | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill5 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 21.67 | 1.39 | 16.15 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill5 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill5 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 24.17 | 1.37 | 19.07 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill6 trailnone | MNQ | 15m | full | 120 | 41.67 | 24.73 | 1.35 | 19.83 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill6 trailprog | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill6 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 21.67 | 1.39 | 16.15 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill6 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill6 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 24.17 | 1.37 | 19.07 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill8 trailnone | MNQ | 15m | full | 120 | 41.67 | 24.73 | 1.35 | 19.83 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill8 trailprog | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill8 trailbe1r | MNQ | 15m | full | 121 | 33.88 | 21.67 | 1.39 | 16.15 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 24.59 | 1.44 | 19.67 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr1.9 fill8 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 24.17 | 1.37 | 19.07 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill3 trailnone | MNQ | 15m | full | 120 | 40.83 | 26.73 | 1.38 | 22.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill3 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill3 trailbe1r | MNQ | 15m | full | 121 | 31.40 | 19.78 | 1.35 | 13.99 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill3 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill3 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 22.57 | 1.35 | 17.17 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill4 trailnone | MNQ | 15m | full | 120 | 40.83 | 26.73 | 1.38 | 22.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill4 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill4 trailbe1r | MNQ | 15m | full | 121 | 31.40 | 19.78 | 1.35 | 13.99 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill4 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill4 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 22.57 | 1.35 | 17.17 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill5 trailnone | MNQ | 15m | full | 120 | 40.83 | 26.73 | 1.38 | 22.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill5 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill5 trailbe1r | MNQ | 15m | full | 121 | 31.40 | 19.78 | 1.35 | 13.99 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill5 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill5 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 22.57 | 1.35 | 17.17 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill6 trailnone | MNQ | 15m | full | 120 | 40.83 | 26.73 | 1.38 | 22.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill6 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill6 trailbe1r | MNQ | 15m | full | 121 | 31.40 | 19.78 | 1.35 | 13.99 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill6 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill6 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 22.57 | 1.35 | 17.17 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill8 trailnone | MNQ | 15m | full | 120 | 40.83 | 26.73 | 1.38 | 22.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill8 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill8 trailbe1r | MNQ | 15m | full | 121 | 31.40 | 19.78 | 1.35 | 13.99 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.29 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.0 fill8 trailtrail05_at15 | MNQ | 15m | full | 121 | 46.28 | 22.57 | 1.35 | 17.17 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill3 trailnone | MNQ | 15m | full | 118 | 38.14 | 21.25 | 1.29 | 15.60 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill3 trailprog | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill3 trailbe1r | MNQ | 15m | full | 121 | 30.58 | 21.49 | 1.38 | 15.96 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill3 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill3 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.67 | 1.30 | 13.76 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill4 trailnone | MNQ | 15m | full | 118 | 38.14 | 21.25 | 1.29 | 15.60 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill4 trailprog | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill4 trailbe1r | MNQ | 15m | full | 121 | 30.58 | 21.49 | 1.38 | 15.96 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill4 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill4 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.67 | 1.30 | 13.76 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill5 trailnone | MNQ | 15m | full | 118 | 38.14 | 21.25 | 1.29 | 15.60 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill5 trailprog | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill5 trailbe1r | MNQ | 15m | full | 121 | 30.58 | 21.49 | 1.38 | 15.96 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill5 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill5 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.67 | 1.30 | 13.76 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill6 trailnone | MNQ | 15m | full | 118 | 38.14 | 21.25 | 1.29 | 15.60 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill6 trailprog | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill6 trailbe1r | MNQ | 15m | full | 121 | 30.58 | 21.49 | 1.38 | 15.96 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill6 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill6 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.67 | 1.30 | 13.76 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill8 trailnone | MNQ | 15m | full | 118 | 38.14 | 21.25 | 1.29 | 15.60 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill8 trailprog | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill8 trailbe1r | MNQ | 15m | full | 121 | 30.58 | 21.49 | 1.38 | 15.96 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 23.99 | 1.43 | 18.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.1 fill8 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.67 | 1.30 | 13.76 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill3 trailnone | MNQ | 15m | full | 118 | 35.59 | 16.16 | 1.21 | 9.56 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill3 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill3 trailbe1r | MNQ | 15m | full | 121 | 28.93 | 20.79 | 1.37 | 15.06 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill3 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill3 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.87 | 1.31 | 13.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill4 trailnone | MNQ | 15m | full | 118 | 35.59 | 16.16 | 1.21 | 9.56 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill4 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill4 trailbe1r | MNQ | 15m | full | 121 | 28.93 | 20.79 | 1.37 | 15.06 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill4 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill4 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.87 | 1.31 | 13.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill5 trailnone | MNQ | 15m | full | 118 | 35.59 | 16.16 | 1.21 | 9.56 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill5 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill5 trailbe1r | MNQ | 15m | full | 121 | 28.93 | 20.79 | 1.37 | 15.06 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill5 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill5 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.87 | 1.31 | 13.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill6 trailnone | MNQ | 15m | full | 118 | 35.59 | 16.16 | 1.21 | 9.56 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill6 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill6 trailbe1r | MNQ | 15m | full | 121 | 28.93 | 20.79 | 1.37 | 15.06 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill6 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill6 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.87 | 1.31 | 13.94 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill8 trailnone | MNQ | 15m | full | 118 | 35.59 | 16.16 | 1.21 | 9.56 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill8 trailprog | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill8 trailbe1r | MNQ | 15m | full | 121 | 28.93 | 20.79 | 1.37 | 15.06 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill8 trailstep_half_r | MNQ | 15m | full | 122 | 40.98 | 22.59 | 1.40 | 17.18 | TESTED |
| mnq_nyam_fine | nyam_fine pair_mid rr2.2 fill8 trailtrail05_at15 | MNQ | 15m | full | 120 | 45.83 | 19.87 | 1.31 | 13.94 | TESTED |
| mnq_fixed_sl25 | fixed_sl25 keep_entry_move_stop | MNQ | 15m | full | 120 | 40.00 | 23.69 | 1.33 | 18.31 | TESTED |
| mnq_fixed_sl25 | fixed_sl25 keep_stop_move_entry | MNQ | 15m | full | 72 | 37.50 | 8.75 | 1.19 | 4.58 | TESTED |
| mnq_fixed_sl_grid | sl_cap=20 keep_entry_move_stop | MNQ | 15m | full | 121 | 39.67 | 22.56 | 1.31 | 16.98 | TESTED |
| mnq_fixed_sl_grid | sl_cap=20 keep_stop_move_entry | MNQ | 15m | full | 63 | 33.33 | -0.24 | 0.99 | -4.07 | TESTED |
| mnq_fixed_sl_grid | sl_cap=22.5 keep_entry_move_stop | MNQ | 15m | full | 120 | 39.17 | 20.66 | 1.28 | 14.76 | TESTED |
| mnq_fixed_sl_grid | sl_cap=22.5 keep_stop_move_entry | MNQ | 15m | full | 69 | 36.23 | 5.74 | 1.13 | 1.60 | TESTED |
| mnq_fixed_sl_grid | sl_cap=25 keep_entry_move_stop | MNQ | 15m | full | 120 | 40.00 | 23.69 | 1.33 | 18.31 | TESTED |
| mnq_fixed_sl_grid | sl_cap=25 keep_stop_move_entry | MNQ | 15m | full | 72 | 37.50 | 8.75 | 1.19 | 4.58 | TESTED |
| mnq_fixed_sl_grid | sl_cap=27.5 keep_entry_move_stop | MNQ | 15m | full | 120 | 41.67 | 29.69 | 1.42 | 25.84 | TESTED |
| mnq_fixed_sl_grid | sl_cap=27.5 keep_stop_move_entry | MNQ | 15m | full | 74 | 39.19 | 12.76 | 1.28 | 8.77 | TESTED |
| mnq_fixed_sl_grid | sl_cap=30 keep_entry_move_stop | MNQ | 15m | full | 120 | 41.67 | 29.70 | 1.42 | 25.85 | TESTED |
| mnq_fixed_sl_grid | sl_cap=30 keep_stop_move_entry | MNQ | 15m | full | 82 | 43.90 | 25.70 | 1.56 | 23.39 | TESTED |
| mnq_narrow_train | narrow cap=27.5 rr=1.9 keep_entry_move_stop | MNQ | 15m | train | 60 | 41.67 | 12.35 | 1.35 | 9.12 | TESTED |
| mnq_narrow_train | narrow cap=27.5 rr=1.9 keep_stop_move_entry | MNQ | 15m | train | 36 | 38.89 | 4.48 | 1.20 | 2.29 | TESTED |
| mnq_narrow_train | narrow cap=27.5 rr=2 keep_entry_move_stop | MNQ | 15m | train | 60 | 41.67 | 14.85 | 1.42 | 11.87 | TESTED |
| mnq_narrow_train | narrow cap=27.5 rr=2 keep_stop_move_entry | MNQ | 15m | train | 36 | 38.89 | 5.88 | 1.27 | 3.72 | TESTED |
| mnq_narrow_train | narrow cap=27.5 rr=2.1 keep_entry_move_stop | MNQ | 15m | train | 60 | 41.67 | 17.35 | 1.50 | 14.68 | TESTED |
| mnq_narrow_train | narrow cap=27.5 rr=2.1 keep_stop_move_entry | MNQ | 15m | train | 36 | 38.89 | 7.28 | 1.33 | 5.16 | TESTED |
| mnq_narrow_train | narrow cap=30 rr=1.9 keep_entry_move_stop | MNQ | 15m | train | 60 | 41.67 | 12.36 | 1.35 | 9.13 | TESTED |
| mnq_narrow_train | narrow cap=30 rr=1.9 keep_stop_move_entry | MNQ | 15m | train | 41 | 41.46 | 8.16 | 1.34 | 5.86 | TESTED |
| mnq_narrow_train | narrow cap=30 rr=2 keep_entry_move_stop | MNQ | 15m | train | 60 | 41.67 | 14.86 | 1.42 | 11.87 | TESTED |
| mnq_narrow_train | narrow cap=30 rr=2 keep_stop_move_entry | MNQ | 15m | train | 41 | 41.46 | 9.86 | 1.41 | 7.65 | TESTED |
| mnq_narrow_train | narrow cap=30 rr=2.1 keep_entry_move_stop | MNQ | 15m | train | 60 | 41.67 | 17.36 | 1.50 | 14.68 | TESTED |
| mnq_narrow_train | narrow cap=30 rr=2.1 keep_stop_move_entry | MNQ | 15m | train | 41 | 41.46 | 11.56 | 1.48 | 9.48 | TESTED |
| mnq_narrow_train | narrow cap=32.5 rr=1.9 keep_entry_move_stop | MNQ | 15m | train | 60 | 45.00 | 18.16 | 1.55 | 15.72 | TESTED |
| mnq_narrow_train | narrow cap=32.5 rr=1.9 keep_stop_move_entry | MNQ | 15m | train | 42 | 42.86 | 10.06 | 1.42 | 7.83 | TESTED |
| mnq_narrow_train | narrow cap=32.5 rr=2 keep_entry_move_stop | MNQ | 15m | train | 60 | 45.00 | 20.86 | 1.63 | 18.87 | TESTED |
| mnq_narrow_train | narrow cap=32.5 rr=2 keep_stop_move_entry | MNQ | 15m | train | 42 | 40.48 | 8.87 | 1.35 | 6.54 | TESTED |
| mnq_narrow_train | narrow cap=32.5 rr=2.1 keep_entry_move_stop | MNQ | 15m | train | 59 | 42.37 | 18.36 | 1.54 | 15.99 | TESTED |
| mnq_narrow_train | narrow cap=32.5 rr=2.1 keep_stop_move_entry | MNQ | 15m | train | 42 | 40.48 | 10.57 | 1.42 | 8.35 | TESTED |
| mnq_narrow_oos | OOS of [narrow cap=32.5 rr=2 keep_entry_move_stop] | MNQ | 15m | test | 60 | 41.67 | 14.84 | 1.42 | 12.17 | TESTED |
| eth_transfer | ETH 5m full_day | ETH | 5m | full | 1162 | 34.85 | 50.30 | 1.07 | 43.97 | TESTED |
| eth_transfer | ETH 5m nyam | ETH | 5m | nyam | 170 | 34.71 | 6.75 | 1.06 | 4.78 | TESTED |
| eth_transfer | ETH 15m full_day | ETH | 15m | full | 703 | 35.56 | 46.02 | 1.10 | 45.41 | TESTED |
| eth_transfer | ETH 15m nyam | ETH | 15m | nyam | 105 | 20.95 | -39.07 | 0.53 | -33.10 | TESTED |
| eth_transfer | ETH 1h full_day | ETH | 1h | full | 376 | 31.91 | -16.18 | 0.94 | -18.72 | TESTED |
| eth_transfer | ETH 1h nyam | ETH | 1h | nyam | 37 | 29.73 | -4.01 | 0.85 | -4.34 | TESTED |
| eth_transfer | ETH 4h full_day | ETH | 4h | full | 201 | 40.30 | 41.95 | 1.35 | 48.30 | TESTED |
| eth_transfer | ETH 4h nyam | ETH | 4h | nyam | 31 | 41.94 | 7.99 | 1.44 | 7.88 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.35 rprev=0 | ETH | 15m | full | 1078 | 37.48 | 51.40 | 1.08 | 48.12 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.35 rprev=1.1 | ETH | 15m | full | 738 | 37.53 | 36.47 | 1.08 | 32.61 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.35 rprev=1.2 | ETH | 15m | full | 666 | 37.84 | 38.63 | 1.09 | 36.60 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.40 rprev=0 | ETH | 15m | full | 1025 | 37.56 | 51.29 | 1.08 | 49.07 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.40 rprev=1.1 | ETH | 15m | full | 703 | 37.70 | 37.93 | 1.09 | 35.14 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.40 rprev=1.2 | ETH | 15m | full | 634 | 37.85 | 37.09 | 1.09 | 35.02 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.45 rprev=0 | ETH | 15m | full | 961 | 37.98 | 59.33 | 1.10 | 62.72 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.45 rprev=1.1 | ETH | 15m | full | 663 | 38.46 | 49.95 | 1.12 | 53.11 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=27.5 body=0.45 rprev=1.2 | ETH | 15m | full | 597 | 38.69 | 48.91 | 1.13 | 52.60 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.35 rprev=0 | ETH | 15m | full | 1078 | 37.48 | 51.40 | 1.08 | 48.12 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.35 rprev=1.1 | ETH | 15m | full | 738 | 37.53 | 36.47 | 1.08 | 32.61 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.35 rprev=1.2 | ETH | 15m | full | 666 | 37.84 | 38.63 | 1.09 | 36.60 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.40 rprev=0 | ETH | 15m | full | 1025 | 37.56 | 51.29 | 1.08 | 49.07 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.40 rprev=1.1 | ETH | 15m | full | 703 | 37.70 | 37.93 | 1.09 | 35.14 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.40 rprev=1.2 | ETH | 15m | full | 634 | 37.85 | 37.09 | 1.09 | 35.02 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.45 rprev=0 | ETH | 15m | full | 961 | 37.98 | 59.33 | 1.10 | 62.72 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.45 rprev=1.1 | ETH | 15m | full | 663 | 38.46 | 49.95 | 1.12 | 53.11 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=30 body=0.45 rprev=1.2 | ETH | 15m | full | 597 | 38.69 | 48.91 | 1.13 | 52.60 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.35 rprev=0 | ETH | 15m | full | 1078 | 37.48 | 51.40 | 1.08 | 48.12 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.35 rprev=1.1 | ETH | 15m | full | 738 | 37.53 | 36.47 | 1.08 | 32.61 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.35 rprev=1.2 | ETH | 15m | full | 666 | 37.84 | 38.63 | 1.09 | 36.60 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.40 rprev=0 | ETH | 15m | full | 1025 | 37.56 | 51.29 | 1.08 | 49.07 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.40 rprev=1.1 | ETH | 15m | full | 703 | 37.70 | 37.93 | 1.09 | 35.15 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.40 rprev=1.2 | ETH | 15m | full | 634 | 37.85 | 37.09 | 1.09 | 35.02 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.45 rprev=0 | ETH | 15m | full | 961 | 37.98 | 59.33 | 1.10 | 62.72 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.45 rprev=1.1 | ETH | 15m | full | 663 | 38.46 | 49.95 | 1.12 | 53.11 | TESTED |
| eth_refine | ETH 15m rr=1.8 sl=32.5 body=0.45 rprev=1.2 | ETH | 15m | full | 597 | 38.69 | 48.91 | 1.13 | 52.60 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.35 rprev=0 | ETH | 15m | full | 1078 | 34.97 | 51.34 | 1.07 | 46.30 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.35 rprev=1.1 | ETH | 15m | full | 738 | 35.37 | 43.96 | 1.09 | 41.80 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.35 rprev=1.2 | ETH | 15m | full | 666 | 35.59 | 44.12 | 1.10 | 43.24 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.40 rprev=0 | ETH | 15m | full | 1025 | 35.22 | 56.40 | 1.08 | 55.16 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.40 rprev=1.1 | ETH | 15m | full | 703 | 35.56 | 46.02 | 1.10 | 45.41 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.40 rprev=1.2 | ETH | 15m | full | 634 | 35.65 | 43.18 | 1.11 | 42.47 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.45 rprev=0 | ETH | 15m | full | 961 | 35.59 | 63.45 | 1.10 | 67.77 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.45 rprev=1.1 | ETH | 15m | full | 663 | 36.20 | 56.03 | 1.13 | 61.52 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=27.5 body=0.45 rprev=1.2 | ETH | 15m | full | 597 | 36.35 | 53.20 | 1.14 | 58.20 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.35 rprev=0 | ETH | 15m | full | 1078 | 34.97 | 51.34 | 1.07 | 46.30 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.35 rprev=1.1 | ETH | 15m | full | 738 | 35.37 | 43.96 | 1.09 | 41.80 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.35 rprev=1.2 | ETH | 15m | full | 666 | 35.59 | 44.12 | 1.10 | 43.24 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.40 rprev=0 | ETH | 15m | full | 1025 | 35.22 | 56.40 | 1.08 | 55.16 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.40 rprev=1.1 | ETH | 15m | full | 703 | 35.56 | 46.02 | 1.10 | 45.41 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.40 rprev=1.2 | ETH | 15m | full | 634 | 35.65 | 43.18 | 1.11 | 42.47 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.45 rprev=0 | ETH | 15m | full | 961 | 35.59 | 63.45 | 1.10 | 67.78 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.45 rprev=1.1 | ETH | 15m | full | 663 | 36.20 | 56.03 | 1.13 | 61.52 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=30 body=0.45 rprev=1.2 | ETH | 15m | full | 597 | 36.35 | 53.20 | 1.14 | 58.20 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.35 rprev=0 | ETH | 15m | full | 1078 | 34.97 | 51.34 | 1.07 | 46.30 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.35 rprev=1.1 | ETH | 15m | full | 738 | 35.37 | 43.96 | 1.09 | 41.80 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.35 rprev=1.2 | ETH | 15m | full | 666 | 35.59 | 44.12 | 1.10 | 43.24 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.40 rprev=0 | ETH | 15m | full | 1025 | 35.22 | 56.40 | 1.08 | 55.16 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.40 rprev=1.1 | ETH | 15m | full | 703 | 35.56 | 46.02 | 1.10 | 45.41 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.40 rprev=1.2 | ETH | 15m | full | 634 | 35.65 | 43.18 | 1.11 | 42.47 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.45 rprev=0 | ETH | 15m | full | 961 | 35.59 | 63.45 | 1.10 | 67.78 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.45 rprev=1.1 | ETH | 15m | full | 663 | 36.20 | 56.04 | 1.13 | 61.52 | TESTED |
| eth_refine | ETH 15m rr=2.0 sl=32.5 body=0.45 rprev=1.2 | ETH | 15m | full | 597 | 36.35 | 53.20 | 1.14 | 58.20 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.35 rprev=0 | ETH | 15m | full | 1076 | 32.90 | 55.33 | 1.08 | 50.40 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.35 rprev=1.1 | ETH | 15m | full | 737 | 32.97 | 39.64 | 1.08 | 34.71 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.35 rprev=1.2 | ETH | 15m | full | 665 | 32.93 | 35.00 | 1.08 | 29.79 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.40 rprev=0 | ETH | 15m | full | 1023 | 33.04 | 57.19 | 1.08 | 54.59 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.40 rprev=1.1 | ETH | 15m | full | 702 | 33.05 | 39.50 | 1.08 | 35.18 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.40 rprev=1.2 | ETH | 15m | full | 633 | 32.86 | 31.86 | 1.07 | 26.34 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.45 rprev=0 | ETH | 15m | full | 960 | 33.23 | 59.43 | 1.09 | 59.39 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.45 rprev=1.1 | ETH | 15m | full | 662 | 33.53 | 47.52 | 1.11 | 47.25 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=27.5 body=0.45 rprev=1.2 | ETH | 15m | full | 596 | 33.39 | 40.08 | 1.10 | 37.84 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.35 rprev=0 | ETH | 15m | full | 1074 | 32.87 | 54.13 | 1.08 | 48.64 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.35 rprev=1.1 | ETH | 15m | full | 736 | 32.88 | 37.44 | 1.08 | 31.79 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.35 rprev=1.2 | ETH | 15m | full | 665 | 32.78 | 31.80 | 1.07 | 25.70 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.40 rprev=0 | ETH | 15m | full | 1021 | 33.01 | 55.99 | 1.08 | 52.78 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.40 rprev=1.1 | ETH | 15m | full | 701 | 32.95 | 37.30 | 1.08 | 32.26 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.40 rprev=1.2 | ETH | 15m | full | 633 | 32.70 | 28.66 | 1.07 | 22.36 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.45 rprev=0 | ETH | 15m | full | 958 | 33.19 | 58.23 | 1.09 | 57.53 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.45 rprev=1.1 | ETH | 15m | full | 661 | 33.43 | 45.32 | 1.10 | 44.07 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=30 body=0.45 rprev=1.2 | ETH | 15m | full | 596 | 33.22 | 36.88 | 1.09 | 33.50 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.35 rprev=0 | ETH | 15m | full | 1074 | 32.87 | 54.13 | 1.08 | 48.64 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.35 rprev=1.1 | ETH | 15m | full | 736 | 32.88 | 37.44 | 1.08 | 31.79 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.35 rprev=1.2 | ETH | 15m | full | 665 | 32.78 | 31.80 | 1.07 | 25.70 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.40 rprev=0 | ETH | 15m | full | 1021 | 33.01 | 55.99 | 1.08 | 52.78 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.40 rprev=1.1 | ETH | 15m | full | 701 | 32.95 | 37.30 | 1.08 | 32.26 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.40 rprev=1.2 | ETH | 15m | full | 633 | 32.70 | 28.66 | 1.07 | 22.36 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.45 rprev=0 | ETH | 15m | full | 958 | 33.19 | 58.23 | 1.09 | 57.53 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.45 rprev=1.1 | ETH | 15m | full | 661 | 33.43 | 45.32 | 1.10 | 44.07 | TESTED |
| eth_refine | ETH 15m rr=2.2 sl=32.5 body=0.45 rprev=1.2 | ETH | 15m | full | 596 | 33.22 | 36.88 | 1.09 | 33.50 | TESTED |
| eth_winner_split | ETH 15m winner train | ETH | 15m | train | 534 | 33.52 | 1.68 | 1.00 | -4.69 | TESTED |
| eth_winner_split | ETH 15m winner test | ETH | 15m | test | 427 | 38.17 | 61.76 | 1.23 | 76.10 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.35 rprev=0 | ETH | 4h | full | 309 | 39.16 | 29.68 | 1.16 | 30.01 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.35 rprev=1.1 | ETH | 4h | full | 222 | 40.99 | 32.73 | 1.25 | 35.30 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.35 rprev=1.2 | ETH | 4h | full | 197 | 40.10 | 24.14 | 1.20 | 24.52 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.40 rprev=0 | ETH | 4h | full | 283 | 40.28 | 36.09 | 1.21 | 38.99 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.40 rprev=1.1 | ETH | 4h | full | 203 | 42.36 | 37.74 | 1.32 | 42.52 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.40 rprev=1.2 | ETH | 4h | full | 179 | 41.34 | 28.16 | 1.27 | 29.85 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.45 rprev=0 | ETH | 4h | full | 266 | 41.35 | 41.90 | 1.27 | 47.58 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.45 rprev=1.1 | ETH | 4h | full | 190 | 43.16 | 39.54 | 1.37 | 45.32 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=27.5 body=0.45 rprev=1.2 | ETH | 4h | full | 166 | 42.17 | 29.96 | 1.31 | 32.40 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.35 rprev=0 | ETH | 4h | full | 308 | 38.96 | 27.88 | 1.15 | 27.70 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.35 rprev=1.1 | ETH | 4h | full | 221 | 40.72 | 30.93 | 1.24 | 32.90 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.35 rprev=1.2 | ETH | 4h | full | 196 | 39.80 | 22.34 | 1.19 | 22.31 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.40 rprev=0 | ETH | 4h | full | 283 | 40.28 | 36.09 | 1.21 | 38.99 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.40 rprev=1.1 | ETH | 4h | full | 203 | 42.36 | 37.74 | 1.32 | 42.52 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.40 rprev=1.2 | ETH | 4h | full | 179 | 41.34 | 28.16 | 1.27 | 29.85 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.45 rprev=0 | ETH | 4h | full | 266 | 41.35 | 41.90 | 1.27 | 47.58 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.45 rprev=1.1 | ETH | 4h | full | 190 | 43.16 | 39.54 | 1.37 | 45.32 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=30 body=0.45 rprev=1.2 | ETH | 4h | full | 166 | 42.17 | 29.96 | 1.31 | 32.40 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.35 rprev=0 | ETH | 4h | full | 308 | 38.96 | 27.88 | 1.15 | 27.70 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.35 rprev=1.1 | ETH | 4h | full | 221 | 40.72 | 30.93 | 1.24 | 32.90 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.35 rprev=1.2 | ETH | 4h | full | 196 | 39.80 | 22.35 | 1.19 | 22.31 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.40 rprev=0 | ETH | 4h | full | 283 | 40.28 | 36.09 | 1.21 | 39.00 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.40 rprev=1.1 | ETH | 4h | full | 203 | 42.36 | 37.74 | 1.32 | 42.52 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.40 rprev=1.2 | ETH | 4h | full | 179 | 41.34 | 28.16 | 1.27 | 29.85 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.45 rprev=0 | ETH | 4h | full | 266 | 41.35 | 41.90 | 1.27 | 47.58 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.45 rprev=1.1 | ETH | 4h | full | 190 | 43.16 | 39.54 | 1.37 | 45.32 | TESTED |
| eth_refine | ETH 4h rr=1.8 sl=32.5 body=0.45 rprev=1.2 | ETH | 4h | full | 166 | 42.17 | 29.96 | 1.31 | 32.40 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.35 rprev=0 | ETH | 4h | full | 306 | 36.27 | 26.88 | 1.14 | 26.05 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.35 rprev=1.1 | ETH | 4h | full | 219 | 38.36 | 32.94 | 1.24 | 35.28 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.35 rprev=1.2 | ETH | 4h | full | 194 | 37.63 | 24.95 | 1.21 | 25.28 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.40 rprev=0 | ETH | 4h | full | 281 | 37.37 | 33.90 | 1.19 | 35.59 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.40 rprev=1.1 | ETH | 4h | full | 201 | 39.80 | 38.95 | 1.32 | 43.93 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.40 rprev=1.2 | ETH | 4h | full | 177 | 38.98 | 29.96 | 1.28 | 31.96 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.45 rprev=0 | ETH | 4h | full | 264 | 38.26 | 38.90 | 1.24 | 42.84 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.45 rprev=1.1 | ETH | 4h | full | 188 | 40.43 | 39.95 | 1.36 | 45.61 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=27.5 body=0.45 rprev=1.2 | ETH | 4h | full | 164 | 39.63 | 30.96 | 1.31 | 33.50 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.35 rprev=0 | ETH | 4h | full | 306 | 37.25 | 35.88 | 1.19 | 37.88 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.35 rprev=1.1 | ETH | 4h | full | 219 | 39.27 | 38.94 | 1.29 | 43.61 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.35 rprev=1.2 | ETH | 4h | full | 194 | 38.66 | 30.95 | 1.26 | 32.99 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.40 rprev=0 | ETH | 4h | full | 282 | 38.30 | 41.90 | 1.24 | 46.82 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.40 rprev=1.1 | ETH | 4h | full | 202 | 40.59 | 43.95 | 1.37 | 51.27 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.40 rprev=1.2 | ETH | 4h | full | 178 | 39.89 | 34.96 | 1.33 | 38.68 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.45 rprev=0 | ETH | 4h | full | 265 | 39.25 | 46.90 | 1.29 | 54.67 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.45 rprev=1.1 | ETH | 4h | full | 189 | 41.27 | 44.95 | 1.40 | 53.03 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=30 body=0.45 rprev=1.2 | ETH | 4h | full | 165 | 40.61 | 35.96 | 1.37 | 40.30 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.35 rprev=0 | ETH | 4h | full | 305 | 37.05 | 33.88 | 1.18 | 35.17 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.35 rprev=1.1 | ETH | 4h | full | 218 | 38.99 | 36.94 | 1.28 | 40.79 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.35 rprev=1.2 | ETH | 4h | full | 194 | 38.14 | 27.95 | 1.23 | 29.08 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.40 rprev=0 | ETH | 4h | full | 281 | 38.08 | 39.90 | 1.23 | 43.94 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.40 rprev=1.1 | ETH | 4h | full | 201 | 40.30 | 41.95 | 1.35 | 48.30 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.40 rprev=1.2 | ETH | 4h | full | 178 | 39.33 | 31.96 | 1.30 | 34.60 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.45 rprev=0 | ETH | 4h | full | 264 | 39.02 | 44.90 | 1.28 | 51.64 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.45 rprev=1.1 | ETH | 4h | full | 188 | 40.96 | 42.95 | 1.39 | 50.03 | TESTED |
| eth_refine | ETH 4h rr=2.0 sl=32.5 body=0.45 rprev=1.2 | ETH | 4h | full | 165 | 40.00 | 32.96 | 1.33 | 36.17 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.35 rprev=0 | ETH | 4h | full | 302 | 35.43 | 40.29 | 1.21 | 43.65 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.35 rprev=1.1 | ETH | 4h | full | 215 | 37.67 | 44.14 | 1.33 | 50.93 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.35 rprev=1.2 | ETH | 4h | full | 192 | 36.46 | 31.95 | 1.26 | 34.06 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.40 rprev=0 | ETH | 4h | full | 278 | 36.33 | 45.10 | 1.25 | 51.18 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.40 rprev=1.1 | ETH | 4h | full | 198 | 38.89 | 48.35 | 1.40 | 57.74 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.40 rprev=1.2 | ETH | 4h | full | 176 | 37.50 | 35.16 | 1.32 | 38.70 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.45 rprev=0 | ETH | 4h | full | 261 | 37.16 | 49.31 | 1.30 | 58.02 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.45 rprev=1.1 | ETH | 4h | full | 185 | 39.46 | 48.55 | 1.43 | 58.33 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=27.5 body=0.45 rprev=1.2 | ETH | 4h | full | 163 | 38.04 | 35.36 | 1.35 | 39.22 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.35 rprev=0 | ETH | 4h | full | 303 | 35.97 | 45.69 | 1.24 | 51.56 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.35 rprev=1.1 | ETH | 4h | full | 216 | 37.96 | 46.34 | 1.35 | 54.25 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.35 rprev=1.2 | ETH | 4h | full | 193 | 36.79 | 34.15 | 1.28 | 37.01 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.40 rprev=0 | ETH | 4h | full | 279 | 36.92 | 50.50 | 1.29 | 59.50 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.40 rprev=1.1 | ETH | 4h | full | 199 | 39.20 | 50.55 | 1.42 | 61.21 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.40 rprev=1.2 | ETH | 4h | full | 177 | 37.85 | 37.36 | 1.34 | 41.75 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.45 rprev=0 | ETH | 4h | full | 262 | 37.79 | 54.71 | 1.34 | 66.72 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.45 rprev=1.1 | ETH | 4h | full | 186 | 39.78 | 50.75 | 1.45 | 61.81 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=30 body=0.45 rprev=1.2 | ETH | 4h | full | 164 | 38.41 | 37.56 | 1.37 | 42.28 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.35 rprev=0 | ETH | 4h | full | 302 | 35.43 | 40.29 | 1.21 | 43.64 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.35 rprev=1.1 | ETH | 4h | full | 215 | 37.21 | 40.94 | 1.30 | 46.20 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.35 rprev=1.2 | ETH | 4h | full | 192 | 35.94 | 28.75 | 1.23 | 29.85 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.40 rprev=0 | ETH | 4h | full | 278 | 36.33 | 45.10 | 1.25 | 51.17 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.40 rprev=1.1 | ETH | 4h | full | 198 | 38.38 | 45.15 | 1.37 | 52.79 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.40 rprev=1.2 | ETH | 4h | full | 176 | 36.93 | 31.96 | 1.29 | 34.35 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.45 rprev=0 | ETH | 4h | full | 261 | 37.16 | 49.31 | 1.30 | 58.01 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.45 rprev=1.1 | ETH | 4h | full | 185 | 38.92 | 45.35 | 1.40 | 53.36 | TESTED |
| eth_refine | ETH 4h rr=2.2 sl=32.5 body=0.45 rprev=1.2 | ETH | 4h | full | 163 | 37.42 | 32.16 | 1.32 | 34.85 | TESTED |
| eth_winner_split | ETH 4h winner train | ETH | 4h | train | 146 | 38.36 | 33.12 | 1.37 | 36.45 | TESTED |
| eth_winner_split | ETH 4h winner test | ETH | 4h | test | 116 | 37.07 | 21.58 | 1.30 | 22.10 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W1 | 208 | 33.17 | -1.58 | 0.99 | -3.97 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W2 | 201 | 29.35 | -24.69 | 0.83 | -23.69 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W3 | 229 | 37.99 | 31.93 | 1.22 | 33.84 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W4 | 157 | 40.76 | 34.86 | 1.37 | 38.92 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W5 | 167 | 37.72 | 21.92 | 1.21 | 21.99 | TESTED |
| eth_rolling_oos_verdict | ETH 15m base summary | ETH | 15m | 5w | 0 | 0 | 3 | 3 | 0 | KILL |
| eth_rolling_oos | wf | ETH | 15m | W1 | 208 | 33.17 | -1.58 | 0.99 | -3.97 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W2 | 201 | 29.35 | -24.69 | 0.83 | -23.69 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W3 | 229 | 37.99 | 31.93 | 1.22 | 33.84 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W4 | 157 | 40.76 | 34.86 | 1.37 | 38.92 | TESTED |
| eth_rolling_oos | wf | ETH | 15m | W5 | 167 | 37.72 | 21.92 | 1.21 | 21.99 | TESTED |
| eth_rolling_oos_verdict | ETH 15m ema50/200 summary | ETH | 15m | 5w | 0 | 0 | 3 | 3 | 0 | KILL |
| eth_rolling_oos | wf | ETH | 4h | W1 | 53 | 37.74 | 10.97 | 1.33 | 10.78 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W2 | 51 | 29.41 | -3.04 | 0.92 | -3.61 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W3 | 67 | 44.78 | 28.99 | 1.78 | 32.27 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W4 | 43 | 34.88 | 4.99 | 1.18 | 4.50 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W5 | 48 | 39.58 | 12.79 | 1.44 | 12.86 | TESTED |
| eth_rolling_oos_verdict | ETH 4h base summary | ETH | 4h | 5w | 0 | 0 | 4 | 3 | 0 | KILL |
| eth_rolling_oos | wf | ETH | 4h | W1 | 53 | 37.74 | 10.97 | 1.33 | 10.78 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W2 | 51 | 29.41 | -3.04 | 0.92 | -3.61 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W3 | 67 | 44.78 | 28.99 | 1.78 | 32.27 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W4 | 43 | 34.88 | 4.99 | 1.18 | 4.50 | TESTED |
| eth_rolling_oos | wf | ETH | 4h | W5 | 48 | 39.58 | 12.79 | 1.44 | 12.86 | TESTED |
| eth_rolling_oos_verdict | ETH 4h ema50/200 summary | ETH | 4h | 5w | 0 | 0 | 4 | 3 | 0 | KILL |
