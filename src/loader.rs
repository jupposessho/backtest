use anyhow::Result;
use backtest::model::binance_klines_item::BinanceKlinesItem;
use clap::{Arg, Command};
use dialoguer::Confirm;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("Binance CLI")
        .version("1.0")
        .author("Your Name <youremail@example.com>")
        .about("Fetches candlestick data from exchanges")
        .arg(
            Arg::new("start-time")
                .short('s')
                .long("start-time")
                .value_parser(clap::value_parser!(u64))
                .required(true)
                .help("Start time in milliseconds since Unix epoch"),
        )
        .arg(
            Arg::new("symbol")
                .short('y')
                .long("symbol")
                .value_parser(clap::value_parser!(String))
                .required(true)
                .help("Trading pair symbol (e.g., BTCUSDT)"),
        )
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .value_parser([
                    "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d",
                    "3d", "1w", "1M",
                ])
                .required(true)
                .help("Candlestick time frame (e.g., 1m, 5m, 1h, 1d)"),
        )
        .get_matches();

    let mut start_time = *matches
        .get_one::<u64>("start-time")
        .expect("start-time is a required argument");
    let symbol = matches
        .get_one::<String>("symbol")
        .expect("symbol is a required argument");
    let interval = matches
        .get_one::<String>("interval")
        .expect("interval is a required argument");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    // Ensure the assets directory exists
    let assets_dir = Path::new("assets");
    if !assets_dir.exists() {
        fs::create_dir(assets_dir)?;
    }

    // File path
    let file_path = assets_dir.join(format!("{}_{}.json", symbol, interval));

    // Check if file exists and prompt for overwrite
    if file_path.exists() {
        if !Confirm::new()
            .with_prompt(format!(
                "{} already exists. Do you want to overwrite it?",
                file_path.display()
            ))
            .default(false)
            .interact()?
        {
            println!("Operation cancelled. File not overwritten.");
            return Ok(());
        }
    }
    let mut all_klines: Vec<BinanceKlinesItem> = Vec::new();

    loop {
        let url = format!(
            "https://api.binance.com/api/v1/klines?symbol={}&interval={}&startTime={}&limit=1500",
            symbol, interval, start_time
        );

        let response = client.get(&url).send().await?;

        let status = response.status();
        let text = response.text().await?;

        if status.is_success() {
            let klines: Vec<BinanceKlinesItem> =
                serde_json::from_str(&text).unwrap_or_else(|err| {
                    println!("Failed to parse response: {}", err);
                    Vec::new()
                });

            if klines.is_empty() {
                println!("No more data available.");
                break;
            }

            for kline in klines {
                println!("{:?}", kline);
                start_time = kline.close_time;
                all_klines.push(kline);
            }
        } else {
            println!("Error: {}", status);
            println!("Response body: {}", text);
            break;
        }
    }

    // Serialize all klines to JSON and save to file
    let json_data = serde_json::to_string_pretty(&all_klines)?;
    fs::write(file_path, json_data)?;

    println!("Data saved to assets/{}_{}.json", symbol, interval);

    Ok(())
}

// BTC/ETH: 1502942400000 - 2014-09-05T17:00:00Z
