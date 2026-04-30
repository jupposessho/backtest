use std::fs::File;
use std::io::Write;

use chrono::Duration;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

use backtest::{
    candle_stick_loader::CandleStickLoader,
    execute,
    model::{
        backtest_result::BacktestResult,
        candle_stick::CandleStick,
        fee_config::FeeConfig,
        position_direction::PositionDirection,
        trade::Trade,
        trade_result::TradeResult,
    },
    strategies::ttrades_fractal_mtf::{FractalMTFConfig, TTradesFractalMTF},
    to_new_york_time,
};

const OUTPUT_HTML: &str = "fractal_mtf_trades.html";
const OUTPUT_CSV: &str = "fractal_mtf_trades.csv";
const OUTPUT_PINE: &str = "fractal_mtf_trades.pine";

fn load_btc_4h() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_4h.json"))
}

fn load_btc_5m() -> Vec<CandleStick> {
    CandleStickLoader::load_binance(include_str!("../../assets/binance_BTCUSDT_5m.json"))
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  Fractal MTF Visualization (BTCUSDT 4h/5m, 3R, Binance Standard Fees)        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let htf = load_btc_4h();
    let ltf = load_btc_5m();

    println!("Loaded data:");
    println!("  • 4h candles: {}", htf.len());
    println!("  • 5m candles: {}\n", ltf.len());

    let config = FractalMTFConfig {
        rr_target: Decimal::from_f32(3.0).unwrap(),
        fee_config: FeeConfig::binance_standard(),
        htf_name: "4h",
        ltf_name: "5m",
    };

    let model = TTradesFractalMTF {
        htf_data: htf.clone(),
        ltf_data: ltf.clone(),
        config: config.clone(),
    };

    let BacktestResult { trades, .. } = execute(model);

    println!("Backtest completed:");
    println!("  • Total trades: {}", trades.len());
    println!("  • Filtering to most recent 60 days...\n");

    if trades.is_empty() {
        println!("No trades were generated. Nothing to visualize.");
        return;
    }

    let latest_close = trades.iter().map(|t| t.close_time).max().unwrap();
    let lookback_seconds = Duration::days(60).num_seconds();
    let cutoff = latest_close - lookback_seconds;

    let recent_trades: Vec<Trade> = trades
        .into_iter()
        .filter(|t| t.close_time >= cutoff)
        .collect();

    if recent_trades.is_empty() {
        println!("No trades in the most recent 60-day window. Nothing to visualize.");
        return;
    }

    let stats = compute_stats(&recent_trades, config.rr_target);

    println!("Recent trade summary (last 60 days):");
    println!("  • Trades: {}", stats.trades);
    println!("  • Winners: {}", stats.winners);
    println!("  • Win rate: {:.2}%", stats.win_rate);
    println!("  • Net PnL: {:.2} USD (from 1000 USD risk-based model)", stats.net_pnl);
    println!("  • Max Drawdown: {:.2}%\n", stats.max_drawdown);

    let recent_candles = extract_recent_candles(&ltf, cutoff);

    if let Err(err) = write_csv(&recent_trades) {
        println!("Failed to write CSV: {err}");
    } else {
        println!("✓ CSV written to {}", OUTPUT_CSV);
    }

    if let Err(err) = write_pine_script(&recent_trades) {
        println!("Failed to write Pine script: {err}");
    } else {
        println!("✓ Pine script written to {}", OUTPUT_PINE);
    }

    if let Err(err) = write_html(&recent_candles, &recent_trades, &stats) {
        println!("Failed to write HTML chart: {err}");
    } else {
        println!("✓ HTML chart written to {}", OUTPUT_HTML);
    }

    println!("\nVisualization complete.");
}

struct EquityStats {
    trades: usize,
    winners: usize,
    win_rate: f64,
    net_pnl: f64,
    max_drawdown: f64,
}

fn compute_stats(trades: &[Trade], rr_target: Decimal) -> EquityStats {
    let start_balance = Decimal::from(1000);
    let risk_pct = Decimal::from_f32(0.01).unwrap();

    let mut balance = start_balance;
    let mut peak = start_balance;
    let mut max_dd = Decimal::ZERO;

    let mut winners = 0usize;

    for trade in trades {
        let risk_amount = balance * risk_pct;
        let entry_price = trade.entry.0;
        let sl_price = trade.sl.0;
        let tp_price = trade.tp.0;

        let risk_distance = match trade.direction {
            PositionDirection::Long => entry_price - sl_price,
            PositionDirection::Short => sl_price - entry_price,
        };

        if risk_distance <= Decimal::ZERO {
            continue;
        }

        let position_size = risk_amount / risk_distance;

        let exit_price = match trade.result {
            TradeResult::Winner => tp_price,
            TradeResult::Expense => sl_price,
            TradeResult::BreakEven => entry_price,
        };

        let price_move = match trade.direction {
            PositionDirection::Long => exit_price - entry_price,
            PositionDirection::Short => entry_price - exit_price,
        };

        let gross_pnl = price_move * position_size;

        // Approximate commissions: maker on entry, taker on exit
        let fee_config = FeeConfig::binance_standard();
        let entry_fee = fee_config.maker_fee(entry_price) * position_size;
        let exit_fee = fee_config.taker_fee(exit_price) * position_size;

        let net_pnl = gross_pnl - entry_fee - exit_fee;
        balance += net_pnl;

        if balance > peak {
            peak = balance;
        } else {
            let dd = (peak - balance) / peak * Decimal::from(100);
            if dd > max_dd {
                max_dd = dd;
            }
        }

        if trade.result == TradeResult::Winner {
            winners += 1;
        }
    }

    let net_pnl = (balance - start_balance).to_f64().unwrap_or(0.0);

    EquityStats {
        trades: trades.len(),
        winners,
        win_rate: if trades.is_empty() {
            0.0
        } else {
            (winners as f64 / trades.len() as f64) * 100.0
        },
        net_pnl,
        max_drawdown: max_dd.to_f64().unwrap_or(0.0),
    }
}

fn extract_recent_candles(candles: &[CandleStick], cutoff: i64) -> Vec<CandleStick> {
    let mut start_index = 0;
    for (idx, candle) in candles.iter().enumerate() {
        if candle.open_time >= cutoff {
            start_index = idx.saturating_sub(12 * 24); // add one extra day for context
            break;
        }
    }
    candles[start_index..].to_vec()
}

fn write_csv(trades: &[Trade]) -> std::io::Result<()> {
    let mut file = File::create(OUTPUT_CSV)?;
    writeln!(
        file,
        "Entry Time,Exit Time,Direction,Result,Entry,SL,TP,RR,Duration Minutes"
    )?;

    for trade in trades {
        let entry_time = to_new_york_time(trade.open_time).format("%Y-%m-%d %H:%M:%S");
        let exit_time = to_new_york_time(trade.close_time).format("%Y-%m-%d %H:%M:%S");
        let direction = match trade.direction {
            PositionDirection::Long => "LONG",
            PositionDirection::Short => "SHORT",
        };
        let result = match trade.result {
            TradeResult::Winner => "WIN",
            TradeResult::Expense => "LOSS",
            TradeResult::BreakEven => "BE",
        };
        let rr = trade.rr().0.to_f64().unwrap_or(0.0);
        let duration_minutes = (trade.close_time - trade.open_time) as f64 / 60_000.0;

        writeln!(
            file,
            "{},{},{},{},{:.2},{:.2},{:.2},{:.2},{:.2}",
            entry_time,
            exit_time,
            direction,
            result,
            trade.entry.0.to_f64().unwrap_or(0.0),
            trade.sl.0.to_f64().unwrap_or(0.0),
            trade.tp.0.to_f64().unwrap_or(0.0),
            rr,
            duration_minutes
        )?;
    }

    Ok(())
}

fn write_pine_script(trades: &[Trade]) -> std::io::Result<()> {
    let mut file = File::create(OUTPUT_PINE)?;

    writeln!(file, "//@version=5")?;
    writeln!(
        file,
        "indicator(\"BTC 4h/5m Fractal MTF - Last 60d\", overlay=true, max_boxes_count=500, max_labels_count=500)"
    )?;

    let limited = trades.iter().take(500);

    for (idx, trade) in limited.enumerate() {
        let entry_ms = trade.open_time * 1000;
        let exit_ms = trade.close_time * 1000;

        let entry_price = trade.entry.0.to_f64().unwrap_or(0.0);
        let sl = trade.sl.0.to_f64().unwrap_or(0.0);
        let tp = trade.tp.0.to_f64().unwrap_or(0.0);

        let (bottom, top) = if matches!(trade.direction, PositionDirection::Long) {
            (sl, tp)
        } else {
            (tp, sl)
        };

        let color = match trade.result {
            TradeResult::Winner => "color.new(color.lime, 70)",
            TradeResult::Expense => "color.new(color.red, 70)",
            TradeResult::BreakEven => "color.new(color.gray, 70)",
        };

        writeln!(
            file,
            "var box bx{idx} = box.new({entry_ms}, {bottom}, {exit_ms}, {top}, xloc=xloc.bar_time, bgcolor={color});"
        )?;
        writeln!(
            file,
            "var line ln{idx} = line.new({entry_ms}, {entry_price}, {exit_ms}, {entry_price}, xloc=xloc.bar_time, color=color.new(color.yellow, 0));"
        )?;
        writeln!(
            file,
            "var label lb{idx} = label.new({entry_ms}, {entry_price}, \"{} {}\", xloc=xloc.bar_time, style=label.style_label_left, color=color.black, textcolor=color.white);",
            if matches!(trade.direction, PositionDirection::Long) { "L" } else { "S" },
            match trade.result {
                TradeResult::Winner => "✓",
                TradeResult::Expense => "✗",
                TradeResult::BreakEven => "=",
            }
        )?;
    }

    Ok(())
}

fn write_html(candles: &[CandleStick], trades: &[Trade], stats: &EquityStats) -> std::io::Result<()> {
    let mut file = File::create(OUTPUT_HTML)?;

    let candle_data = candles
        .iter()
        .map(|c| {
            format!(
                "[{}, {:.2}, {:.2}, {:.2}, {:.2}]",
                c.open_time,
                c.open.0.to_f64().unwrap_or(0.0),
                c.high.0.to_f64().unwrap_or(0.0),
                c.low.0.to_f64().unwrap_or(0.0),
                c.close.0.to_f64().unwrap_or(0.0)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let trade_markers = trades
        .iter()
        .map(|t| {
            format!(
                "{{ time: {}, price: {:.2}, direction: \"{}\", result: \"{}\", sl: {:.2}, tp: {:.2} }}",
                t.open_time,
                t.entry.0.to_f64().unwrap_or(0.0),
                if matches!(t.direction, PositionDirection::Long) {
                    "LONG"
                } else {
                    "SHORT"
                },
                match t.result {
                    TradeResult::Winner => "WIN",
                    TradeResult::Expense => "LOSS",
                    TradeResult::BreakEven => "BE",
                },
                t.sl.0.to_f64().unwrap_or(0.0),
                t.tp.0.to_f64().unwrap_or(0.0)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <title>BTCUSDT 4h/5m Fractal MTF - Last 60 Days</title>
    <script src="https://cdn.jsdelivr.net/npm/lightweight-charts@4.1.0/dist/lightweight-charts.standalone.production.js"></script>
    <style>
        body {{
            font-family: "Segoe UI", Tahoma, sans-serif;
            background: #11161f;
            color: #d0d4dc;
            margin: 0;
            padding: 20px;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
        }}
        .chart {{
            height: 700px;
        }}
        .stats {{
            display: flex;
            gap: 20px;
            margin-bottom: 20px;
            flex-wrap: wrap;
        }}
        .stat {{
            background: #1b222d;
            padding: 16px;
            border-radius: 8px;
            min-width: 160px;
        }}
        .stat .label {{
            font-size: 12px;
            color: #888ea8;
        }}
        .stat .value {{
            font-size: 24px;
            font-weight: 600;
            color: #4dabf7;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>BTCUSDT 4h / 5m Fractal MTF (3R) — Recent Trades</h1>
        <div class="stats">
            <div class="stat">
                <div class="label">Trades (60d)</div>
                <div class="value">{}</div>
            </div>
            <div class="stat">
                <div class="label">Win Rate</div>
                <div class="value">{:.2}%</div>
            </div>
            <div class="stat">
                <div class="label">Winners</div>
                <div class="value">{}</div>
            </div>
            <div class="stat">
                <div class="label">Net PnL (USD)</div>
                <div class="value">{:.2}</div>
            </div>
            <div class="stat">
                <div class="label">Max Drawdown</div>
                <div class="value">{:.2}%</div>
            </div>
        </div>
        <div id="chart" class="chart"></div>
    </div>

    <script>
        const candleData = [{}];
        const tradeMarkers = [{}];

        const chart = LightweightCharts.createChart(document.getElementById('chart'), {{
            layout: {{
                background: {{ type: 'Solid', color: '#11161f' }},
                textColor: '#d0d4dc',
            }},
            rightPriceScale: {{
                borderColor: 'rgba(197, 203, 206, 0.4)',
            }},
            timeScale: {{
                borderColor: 'rgba(197, 203, 206, 0.4)',
                timeVisible: true,
                secondsVisible: false,
            }},
            grid: {{
                vertLines: {{
                    color: 'rgba(197, 203, 206, 0.1)',
                }},
                horzLines: {{
                    color: 'rgba(197, 203, 206, 0.1)',
                }},
            }},
        }});

        const candleSeries = chart.addCandlestickSeries({{
            upColor: '#26a69a',
            downColor: '#ef5350',
            borderDownColor: '#ef5350',
            borderUpColor: '#26a69a',
            wickDownColor: '#ef5350',
            wickUpColor: '#26a69a',
        }});

        candleSeries.setData(
            candleData.map(([time, open, high, low, close]) => {{
                return {{
                    time: Math.floor(time / 1000),
                    open,
                    high,
                    low,
                    close,
                }};
            }})
        );

        const markerData = tradeMarkers.map((marker) => {{
            return {{
                time: Math.floor(marker.time / 1000),
                position: marker.direction === 'LONG' ? 'belowBar' : 'aboveBar',
                color: marker.result === 'WIN' ? '#4caf50' : marker.result === 'LOSS' ? '#ef5350' : '#ffa726',
                shape: marker.direction === 'LONG' ? 'arrowUp' : 'arrowDown',
                text: `${{marker.direction}} ${{
                    marker.result === 'WIN' ? '✓' : marker.result === 'LOSS' ? '✗' : '='
                }}`,
            }};
        }});

        candleSeries.setMarkers(markerData);

        chart.timeScale().fitContent();
    </script>
</body>
</html>
"#,
        stats.trades,
        stats.win_rate,
        stats.winners,
        stats.net_pnl,
        stats.max_drawdown,
        candle_data,
        trade_markers
    );

    file.write_all(html.as_bytes())
}
