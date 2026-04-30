use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_yaml;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::connect_async;
use uuid::Uuid;

use backtest::model::bitget_types::{
    BitgetAccountInfo, BitgetCandleQuery, BitgetContractInfo, BitgetOrdersHistoryData,
    BitgetPlaceOrderRequest, BitgetPlaceOrderResponse, BitgetPosition, BitgetRestCandleRow,
    BitgetRestResponse, BitgetSetLeverageRequest, BitgetSetLeverageResponse, BitgetWsCandlePush,
    BitgetWsChannelArg, BitgetWsSubscribe,
};
use backtest::model::candle_stick::CandleStick;
use backtest::model::decimal::DecimalVec;
use backtest::model::position_direction::PositionDirection;
use backtest::{BitgetClient, BitgetCredentials};

#[derive(Debug, Deserialize, Clone)]
struct RootConfig {
    bot: BotConfig,
    exchange: ExchangeConfig,
    credentials: CredentialsConfig,
    logging: LoggingConfig,
    alerts: AlertsConfig,
    pnl: PnlConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct BotConfig {
    assets: Vec<String>,
    timeframes: TimeframesConfig,
    rr_target: Decimal,
    risk_pct: Decimal,
    leverage: Decimal,
    margin_mode: String,
    order_type: String,
    max_total_drawdown_pct: Decimal,
    size_step: Option<Decimal>,
    price_step: Option<Decimal>,
}

#[derive(Debug, Deserialize, Clone)]
struct TimeframesConfig {
    htf: String,
    ltf: String,
}

#[derive(Debug, Deserialize, Clone)]
struct ExchangeConfig {
    product_type: String,
    margin_coin: String,
    locale: String,
    demo: bool,
    rest_base_url: String,
    ws_public_url_demo: String,
    ws_public_url_live: String,
}

#[derive(Debug, Deserialize, Clone)]
struct CredentialsConfig {
    api_key: String,
    secret_key: String,
    passphrase: String,
}

#[derive(Debug, Deserialize, Clone)]
struct LoggingConfig {
    enable_csv: bool,
    enable_json: bool,
    log_dir: String,
    include_raw_errors: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct AlertsConfig {
    telegram: TelegramConfig,
    discord: DiscordConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct TelegramConfig {
    enabled: bool,
    bot_token: String,
    chat_id: String,
}

#[derive(Debug, Deserialize, Clone)]
struct DiscordConfig {
    enabled: bool,
    webhook_url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct PnlConfig {
    mode: String,
    poll_interval_sec: u64,
}

#[derive(Clone, Debug)]
struct CandleBuffer {
    interval_ms: i64,
    candles: VecDeque<CandleStick>,
}

impl CandleBuffer {
    fn new(interval_ms: i64) -> Self {
        Self {
            interval_ms,
            candles: VecDeque::new(),
        }
    }

    fn seed(&mut self, mut candles: Vec<CandleStick>, limit: usize) {
        candles.sort_by_key(|c| c.open_time);
        if candles.len() > limit {
            candles = candles.split_off(candles.len() - limit);
        }
        self.candles = VecDeque::from(candles);
    }

    fn update_from_ws_row(&mut self, row: &BitgetRestCandleRow) -> Result<()> {
        if row.len() < 5 {
            return Err(anyhow!("Invalid candle row length"));
        }
        let ts_ms = row[0].parse::<i64>()?;
        let open = DecimalVec(Decimal::from_str(&row[1])?);
        let high = DecimalVec(Decimal::from_str(&row[2])?);
        let low = DecimalVec(Decimal::from_str(&row[3])?);
        let close = DecimalVec(Decimal::from_str(&row[4])?);

        let open_time = ts_ms / 1000;
        let close_time = open_time + (self.interval_ms / 1000);

        if let Some(last) = self.candles.back_mut() {
            if last.open_time == open_time {
                last.open = open;
                last.high = high;
                last.low = low;
                last.close = close;
                last.close_time = close_time;
                return Ok(());
            }
        }

        self.candles.push_back(CandleStick {
            open_time,
            open,
            high,
            low,
            close,
            close_time,
        });

        if self.candles.len() > 220 {
            self.candles.pop_front();
        }

        Ok(())
    }

    fn last(&self) -> Option<CandleStick> {
        self.candles.back().copied()
    }

    fn len(&self) -> usize {
        self.candles.len()
    }

    fn as_vec(&self) -> Vec<CandleStick> {
        self.candles.iter().copied().collect()
    }
}

#[derive(Clone, Debug)]
struct AssetState {
    htf: CandleBuffer,
    ltf: CandleBuffer,
    size_step: Option<Decimal>,
    price_step: Option<Decimal>,
    last_processed_ltf_open_time: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HTFBias {
    Bullish,
    Bearish,
    None,
}

#[derive(Debug, Clone)]
struct HTFSetup {
    bias: HTFBias,
    poi_high: DecimalVec,
    poi_low: DecimalVec,
    swing_high: DecimalVec,
    swing_low: DecimalVec,
}

#[derive(Debug, Clone)]
struct Signal {
    direction: PositionDirection,
    entry: DecimalVec,
    sl: DecimalVec,
    tp: DecimalVec,
}

#[derive(Debug)]
struct CandleUpdate {
    symbol: String,
    channel: String,
    row: BitgetRestCandleRow,
}

#[derive(Debug, Clone)]
struct SignalMeta {
    entry: Decimal,
    sl: Decimal,
    tp: Decimal,
    side: String,
    size: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RiskState {
    peak_equity: Decimal,
    last_equity: Decimal,
    max_drawdown_pct: Decimal,
    halted: bool,
}

#[derive(Debug, Serialize)]
struct TradeLogEntry {
    timestamp: String,
    symbol: String,
    side: String,
    size: String,
    entry: Option<String>,
    sl: Option<String>,
    tp: Option<String>,
    status: String,
    pnl: Option<String>,
    fee: Option<String>,
    rr: Option<String>,
    order_id: Option<String>,
    client_oid: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = env::var("BOT_CONFIG_PATH")
        .unwrap_or_else(|_| "config/bitget_bot.yaml".to_string());
    let cfg = load_config(&config_path)?;

    let api_key = resolve_secret(&cfg.credentials.api_key, "BITGET_API_KEY")?;
    let secret_key = resolve_secret(&cfg.credentials.secret_key, "BITGET_SECRET_KEY")?;
    let passphrase = resolve_secret(&cfg.credentials.passphrase, "BITGET_PASSPHRASE")?;

    let creds = BitgetCredentials {
        api_key,
        secret_key,
        passphrase,
        locale: cfg.exchange.locale.clone(),
        demo: cfg.exchange.demo,
    };

    let bitget = BitgetClient::new(cfg.exchange.rest_base_url.clone(), creds);
    let http = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let ws_public_url = if cfg.exchange.demo {
        cfg.exchange.ws_public_url_demo.clone()
    } else {
        cfg.exchange.ws_public_url_live.clone()
    };

    let logger = Arc::new(TradeLogger::new(&cfg.logging)?);

    let mut states: HashMap<String, AssetState> = HashMap::new();
    let contract_map =
        fetch_contracts(&http, &bitget, &cfg.exchange.product_type, logger.as_ref()).await?;

    for asset in &cfg.bot.assets {
        let htf = fetch_candles(
            &http,
            &bitget,
            asset,
            &cfg.exchange.product_type,
            &cfg.bot.timeframes.htf,
            logger.as_ref(),
        )
        .await?;
        let ltf = fetch_candles(
            &http,
            &bitget,
            asset,
            &cfg.exchange.product_type,
            &cfg.bot.timeframes.ltf,
            logger.as_ref(),
        )
        .await?;

        let (size_step, price_step) = contract_map
            .get(asset)
            .cloned()
            .unwrap_or((cfg.bot.size_step, cfg.bot.price_step));

        set_leverage(
            &http,
            &bitget,
            asset,
            &cfg.exchange.product_type,
            &cfg.exchange.margin_coin,
            cfg.bot.leverage,
            logger.as_ref(),
        )
        .await?;

        let mut htf_buf = CandleBuffer::new(granularity_ms(&cfg.bot.timeframes.htf)?);
        let mut ltf_buf = CandleBuffer::new(granularity_ms(&cfg.bot.timeframes.ltf)?);

        htf_buf.seed(htf, 200);
        ltf_buf.seed(ltf, 200);

        states.insert(
            asset.to_string(),
            AssetState {
                htf: htf_buf,
                ltf: ltf_buf,
                size_step,
                price_step,
                last_processed_ltf_open_time: None,
            },
        );
    }

    let mut risk_state = RiskState {
        peak_equity: Decimal::ZERO,
        last_equity: Decimal::ZERO,
        max_drawdown_pct: cfg.bot.max_total_drawdown_pct,
        halted: false,
    };
    if let Some(saved) = load_risk_state(&cfg.logging.log_dir) {
        risk_state = saved;
    }

    let shared_states = Arc::new(Mutex::new(states));
    let shared_risk = Arc::new(RwLock::new(risk_state));
    let seen_orders = Arc::new(Mutex::new(HashSet::new()));
    if let Some(saved) = load_seen_orders(&cfg.logging.log_dir) {
        let mut guard = seen_orders.lock().await;
        *guard = saved;
    }
    let signal_map = Arc::new(Mutex::new(HashMap::<String, SignalMeta>::new()));

    let alerts = Arc::new(Alerts::new(&cfg.alerts, http.clone()));
    if shared_risk.read().await.halted {
        let _ = logger.log_error(
            "Trading is halted on startup. Awaiting manual resume command.",
            None,
        );
        alerts
            .send("Trading is halted on startup. Awaiting manual resume command.".to_string())
            .await;
    }

    let (tx, mut rx) = mpsc::channel::<CandleUpdate>(2048);
    let ws_url = ws_public_url.clone();
    let tx_clone = tx.clone();
    let assets = cfg.bot.assets.clone();
    let product_type = cfg.exchange.product_type.clone();
    let ltf_channel = channel_from_granularity(&cfg.bot.timeframes.ltf)?;
    let htf_channel = channel_from_granularity(&cfg.bot.timeframes.htf)?;

    tokio::spawn(async move {
        if let Err(err) = run_ws_stream(
            &ws_url,
            &product_type,
            &assets,
            &ltf_channel,
            &htf_channel,
            tx_clone,
        )
        .await
        {
            println!("[ws] error: {err}");
        }
    });

    if cfg.pnl.mode == "exchange" {
        let pnl_cfg = cfg.pnl.clone();
        let bot_cfg = cfg.bot.clone();
        let ex_cfg = cfg.exchange.clone();
        let bitget_clone = bitget.clone();
        let http_clone = http.clone();
        let seen_orders_clone = seen_orders.clone();
        let signal_map_clone = signal_map.clone();
        let logger_clone = logger.clone();
        let alerts_clone = alerts.clone();
        let risk_clone = shared_risk.clone();

        tokio::spawn(async move {
            if let Err(err) = pnl_monitor_loop(
                &http_clone,
                &bitget_clone,
                &bot_cfg,
                &ex_cfg,
                &pnl_cfg,
                &seen_orders_clone,
                &signal_map_clone,
                &logger_clone,
                &alerts_clone,
                &risk_clone,
            )
            .await
            {
                println!("[pnl] error: {err}");
            }
        });
    }

    println!(
        "Bitget Fractal MTF bot started (demo={}, rest={}, ws={})",
        cfg.exchange.demo, cfg.exchange.rest_base_url, ws_public_url
    );

    while let Some(update) = rx.recv().await {
        if !cfg.bot.assets.contains(&update.symbol) {
            continue;
        }
        if shared_risk.read().await.halted {
            if let Ok(true) = try_resume(&cfg.logging.log_dir, &shared_risk, &alerts).await {
                // resumed
            }
        }
        let should_trade = { !shared_risk.read().await.halted };
        if !should_trade {
            continue;
        }

        let mut guard = shared_states.lock().await;
        let Some(state) = guard.get_mut(&update.symbol) else { continue };

        let buffer = if update.channel == ltf_channel {
            &mut state.ltf
        } else if update.channel == htf_channel {
            &mut state.htf
        } else {
            continue;
        };

        if let Err(err) = buffer.update_from_ws_row(&update.row) {
            println!(
                "[{}] candle update failed for {}: {err}",
                update.symbol, update.channel
            );
            continue;
        }

        if update.channel == ltf_channel {
            if should_process_ltf(state, buffer) {
                if let Err(err) = handle_ltf_close(
                    &http,
                    &bitget,
                    &cfg,
                    &update.symbol,
                    state,
                    &signal_map,
                    &alerts,
                    &logger,
                )
                .await
                {
                    println!("[{}] handle_ltf_close error: {err}", update.symbol);
                }
            }
        }
    }

    Ok(())
}

fn load_config(path: &str) -> Result<RootConfig> {
    let content = fs::read_to_string(path)?;
    let cfg: RootConfig = serde_yaml::from_str(&content)?;
    Ok(cfg)
}

fn resolve_secret(config_value: &str, env_key: &str) -> Result<String> {
    if !config_value.trim().is_empty() {
        return Ok(config_value.to_string());
    }
    env::var(env_key).map_err(|_| anyhow!("Missing {}", env_key))
}

fn load_risk_state(log_dir: &str) -> Option<RiskState> {
    let path = Path::new(log_dir).join("risk_state.json");
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn persist_risk_state(log_dir: &str, state: &RiskState) -> Result<()> {
    let path = Path::new(log_dir).join("risk_state.json");
    let content = serde_json::to_string(state)?;
    fs::create_dir_all(log_dir)?;
    fs::write(path, content)?;
    Ok(())
}

fn load_seen_orders(log_dir: &str) -> Option<HashSet<String>> {
    let path = Path::new(log_dir).join("seen_orders.json");
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn persist_seen_orders(log_dir: &str, seen: &HashSet<String>) -> Result<()> {
    let path = Path::new(log_dir).join("seen_orders.json");
    let content = serde_json::to_string(seen)?;
    fs::create_dir_all(log_dir)?;
    fs::write(path, content)?;
    Ok(())
}

async fn try_resume(
    log_dir: &str,
    risk_state: &Arc<RwLock<RiskState>>,
    alerts: &Arc<Alerts>,
) -> Result<bool> {
    let resume_signal = Path::new(log_dir).join("resume.signal");
    let resume_command = Path::new(log_dir).join("resume.command");
    if resume_signal.exists() || resume_command.exists() {
        let _ = fs::remove_file(&resume_signal);
        let _ = fs::remove_file(&resume_command);
        let _ = fs::remove_file(Path::new(log_dir).join("halted.signal"));
        let mut guard = risk_state.write().await;
        guard.halted = false;
        if guard.last_equity > Decimal::ZERO {
            guard.peak_equity = guard.last_equity;
        }
        let _ = persist_risk_state(log_dir, &guard);
        alerts
            .send("Resume command detected. Trading resumed.".to_string())
            .await;
        return Ok(true);
    }
    Ok(false)
}

fn should_process_ltf(state: &mut AssetState, buffer: &CandleBuffer) -> bool {
    let Some(last) = buffer.last() else { return false };
    let open_time = last.open_time;
    if let Some(prev) = state.last_processed_ltf_open_time {
        if prev == open_time {
            return false;
        }
    }

    let now_ms = BitgetClient::timestamp_ms();
    let candle_close_ms = (last.open_time * 1000) + buffer.interval_ms;
    if now_ms < candle_close_ms {
        return false;
    }

    state.last_processed_ltf_open_time = Some(open_time);
    true
}

async fn handle_ltf_close(
    http: &Client,
    bitget: &BitgetClient,
    cfg: &RootConfig,
    symbol: &str,
    state: &mut AssetState,
    signal_map: &Arc<Mutex<HashMap<String, SignalMeta>>>,
    alerts: &Arc<Alerts>,
    logger: &Arc<TradeLogger>,
) -> Result<()> {
    if state.htf.len() < 10 || state.ltf.len() < 10 {
        return Ok(());
    }

    let signal = generate_signal(
        &state.htf.as_vec(),
        &state.ltf.as_vec(),
        cfg.bot.rr_target,
    );
    if let Some(signal) = signal {
        if has_open_position(
            http,
            bitget,
            symbol,
            &cfg.exchange.product_type,
            &cfg.exchange.margin_coin,
            logger.as_ref(),
        )
        .await?
        {
            return Ok(());
        }

        if has_pending_order(
            http,
            bitget,
            symbol,
            &cfg.exchange.product_type,
            logger.as_ref(),
        )
        .await?
        {
            return Ok(());
        }

        let equity = fetch_account_equity(
            http,
            bitget,
            symbol,
            &cfg.exchange.product_type,
            &cfg.exchange.margin_coin,
            logger.as_ref(),
        )
        .await?;
        let risk_amount = equity * cfg.bot.risk_pct;

        let risk_distance = match signal.direction {
            PositionDirection::Long => signal.entry - signal.sl,
            PositionDirection::Short => signal.sl - signal.entry,
        };

        if risk_distance.0 <= Decimal::ZERO {
            return Ok(());
        }

        let size_step = state.size_step.or(cfg.bot.size_step);
        let price_step = state.price_step.or(cfg.bot.price_step);

        let mut size = DecimalVec(risk_amount) / risk_distance;
        if let Some(step) = size_step {
            size = DecimalVec(floor_to_step(size.0, step));
        }

        if size.0 <= Decimal::ZERO {
            return Ok(());
        }

        let entry = signal.entry.0;
        let sl = if let Some(step) = price_step {
            floor_to_step(signal.sl.0, step)
        } else {
            signal.sl.0
        };
        let tp = if let Some(step) = price_step {
            floor_to_step(signal.tp.0, step)
        } else {
            signal.tp.0
        };

        let side = match signal.direction {
            PositionDirection::Long => "buy",
            PositionDirection::Short => "sell",
        };

        let client_oid = Uuid::new_v4().to_string();

        let body = BitgetPlaceOrderRequest {
            symbol: symbol.to_string(),
            product_type: cfg.exchange.product_type.clone(),
            margin_mode: cfg.bot.margin_mode.clone(),
            margin_coin: cfg.exchange.margin_coin.clone(),
            size: format_decimal(size.0),
            price: if cfg.bot.order_type == "limit" {
                Some(format_decimal(entry))
            } else {
                None
            },
            side: side.to_string(),
            trade_side: None,
            order_type: cfg.bot.order_type.clone(),
            force: if cfg.bot.order_type == "limit" {
                Some("gtc".to_string())
            } else {
                None
            },
            client_oid: Some(client_oid.clone()),
            reduce_only: None,
            preset_take_profit_price: Some(format_decimal(tp)),
            preset_stop_loss_price: Some(format_decimal(sl)),
            preset_take_profit_execute_price: None,
            preset_stop_loss_execute_price: None,
            stp_mode: None,
        };

        let (resp, raw): (BitgetRestResponse<BitgetPlaceOrderResponse>, String) =
            signed_post(http, bitget, "/api/v2/mix/order/place-order", &body).await?;

        if resp.code == "00000" {
            let mut map = signal_map.lock().await;
            map.insert(
                client_oid.clone(),
                SignalMeta {
                    entry,
                    sl,
                    tp,
                    side: side.to_string(),
                    size: size.0,
                },
            );

            alerts
                .send(format!(
                    "[{}] Order placed: side={} size={} entry={} sl={} tp={} clientOid={}",
                    symbol, side, size.0, entry, sl, tp, client_oid
                ))
                .await;

            logger.log_trade(TradeLogEntry {
                timestamp: Utc::now().to_rfc3339(),
                symbol: symbol.to_string(),
                side: side.to_string(),
                size: format_decimal(size.0),
                entry: Some(format_decimal(entry)),
                sl: Some(format_decimal(sl)),
                tp: Some(format_decimal(tp)),
                status: "placed".to_string(),
                pnl: None,
                fee: None,
                rr: None,
                order_id: resp.data.order_id.clone(),
                client_oid: resp.data.client_oid.clone(),
            })?;
        } else {
            let msg = format!(
                "[{}] Order rejected: code={} msg={}",
                symbol, resp.code, resp.msg
            );
            log_response_error(
                logger.as_ref(),
                "Order rejected",
                &resp.code,
                &resp.msg,
                &raw,
            );
            alerts.send(msg).await;
        }
    }

    Ok(())
}

fn generate_signal(
    htf: &[CandleStick],
    ltf: &[CandleStick],
    rr_target: Decimal,
) -> Option<Signal> {
    let ltf_index = ltf.len().saturating_sub(1);
    let ltf_candle = ltf.get(ltf_index)?;

    let htf_index = find_htf_index_for_time(htf, ltf_candle.open_time);
    let setup = create_htf_setup(htf, htf_index)?;

    if !is_in_poi(ltf_candle.low, ltf_candle.high, &setup) {
        return None;
    }

    let direction = match setup.bias {
        HTFBias::Bullish => PositionDirection::Long,
        HTFBias::Bearish => PositionDirection::Short,
        HTFBias::None => return None,
    };

    if !detect_ltf_cisd(ltf, ltf_index, direction) {
        return None;
    }

    let ob_level = detect_ltf_order_block(ltf, ltf_index, direction)?;
    let entry = ltf_candle.close;

    let sl = match direction {
        PositionDirection::Long => {
            ob_level - DecimalVec(ob_level.0 * Decimal::from_f32(0.0005).unwrap())
        }
        PositionDirection::Short => {
            ob_level + DecimalVec(ob_level.0 * Decimal::from_f32(0.0005).unwrap())
        }
    };

    let risk = match direction {
        PositionDirection::Long => entry - sl,
        PositionDirection::Short => sl - entry,
    };

    if risk.0 <= Decimal::ZERO {
        return None;
    }

    let tp = match direction {
        PositionDirection::Long => entry + DecimalVec(risk.0 * rr_target),
        PositionDirection::Short => entry - DecimalVec(risk.0 * rr_target),
    };

    Some(Signal {
        direction,
        entry,
        sl,
        tp,
    })
}

fn determine_htf_bias(htf: &[CandleStick], idx: usize) -> HTFBias {
    if idx < 3 || htf.len() < 4 {
        return HTFBias::None;
    }

    let current = htf[idx];
    let prev1 = htf[idx - 1];
    let prev2 = htf[idx - 2];

    let hh = current.high > prev1.high && prev1.high > prev2.high;
    let hl = current.low > prev1.low && prev1.low > prev2.low;

    let lh = current.high < prev1.high && prev1.high < prev2.high;
    let ll = current.low < prev1.low && prev1.low < prev2.low;

    if hh || hl {
        HTFBias::Bullish
    } else if lh || ll {
        HTFBias::Bearish
    } else {
        let bullish_closes = (current.close > current.open) as i32
            + (prev1.close > prev1.open) as i32
            + (prev2.close > prev2.open) as i32;

        if bullish_closes >= 2 {
            HTFBias::Bullish
        } else if bullish_closes <= 1 {
            HTFBias::Bearish
        } else {
            HTFBias::None
        }
    }
}

fn find_swing_high(htf: &[CandleStick], start: usize, end: usize) -> DecimalVec {
    let mut highest = htf[start].high;
    for i in start..=end.min(htf.len() - 1) {
        if htf[i].high > highest {
            highest = htf[i].high;
        }
    }
    highest
}

fn find_swing_low(htf: &[CandleStick], start: usize, end: usize) -> DecimalVec {
    let mut lowest = htf[start].low;
    for i in start..=end.min(htf.len() - 1) {
        if htf[i].low < lowest {
            lowest = htf[i].low;
        }
    }
    lowest
}

fn detect_htf_fvg(htf: &[CandleStick], idx: usize) -> Option<(DecimalVec, DecimalVec)> {
    if idx < 2 {
        return None;
    }

    let candle1 = htf[idx - 2];
    let candle3 = htf[idx];

    if candle3.low > candle1.high {
        return Some((candle3.low, candle1.high));
    }

    if candle3.high < candle1.low {
        return Some((candle1.low, candle3.high));
    }

    None
}

fn create_htf_setup(htf: &[CandleStick], idx: usize) -> Option<HTFSetup> {
    let bias = determine_htf_bias(htf, idx);
    if bias == HTFBias::None {
        return None;
    }

    let lookback = 10.min(idx);
    let start = idx.saturating_sub(lookback);

    let swing_high = find_swing_high(htf, start, idx);
    let swing_low = find_swing_low(htf, start, idx);

    let (poi_high, poi_low) = if let Some((fvg_high, fvg_low)) = detect_htf_fvg(htf, idx) {
        (fvg_high, fvg_low)
    } else {
        match bias {
            HTFBias::Bullish => (
                swing_low,
                swing_low - DecimalVec(swing_low.0 * Decimal::from_f32(0.0025).unwrap()),
            ),
            HTFBias::Bearish => (
                swing_high + DecimalVec(swing_high.0 * Decimal::from_f32(0.0025).unwrap()),
                swing_high,
            ),
            HTFBias::None => return None,
        }
    };

    Some(HTFSetup {
        bias,
        poi_high,
        poi_low,
        swing_high,
        swing_low,
    })
}

fn is_in_poi(low: DecimalVec, high: DecimalVec, setup: &HTFSetup) -> bool {
    high >= setup.poi_low && low <= setup.poi_high
}

fn detect_ltf_cisd(ltf: &[CandleStick], idx: usize, expected_direction: PositionDirection) -> bool {
    if idx < 3 {
        return false;
    }

    let current = ltf[idx];
    let prev1 = ltf[idx - 1];
    let prev2 = ltf[idx - 2];
    let prev3 = ltf[idx - 3];

    match expected_direction {
        PositionDirection::Long => {
            let bearish_count = (prev3.close < prev3.open) as i32
                + (prev2.close < prev2.open) as i32
                + (prev1.close < prev1.open) as i32;

            let current_bullish = current.close > current.open;
            bearish_count >= 2 && current_bullish
        }
        PositionDirection::Short => {
            let bullish_count = (prev3.close > prev3.open) as i32
                + (prev2.close > prev2.open) as i32
                + (prev1.close > prev1.open) as i32;

            let current_bearish = current.close < current.open;
            bullish_count >= 2 && current_bearish
        }
    }
}

fn detect_ltf_order_block(
    ltf: &[CandleStick],
    idx: usize,
    direction: PositionDirection,
) -> Option<DecimalVec> {
    if idx < 5 {
        return None;
    }

    let current = ltf[idx];
    let lookback = 10.min(idx);

    match direction {
        PositionDirection::Long => {
            let mut swing_low = ltf[idx - 1].low;
            for i in (idx.saturating_sub(lookback))..idx {
                if ltf[i].low < swing_low {
                    swing_low = ltf[i].low;
                }
            }
            if current.low < swing_low && current.close > current.open {
                Some(swing_low)
            } else {
                None
            }
        }
        PositionDirection::Short => {
            let mut swing_high = ltf[idx - 1].high;
            for i in (idx.saturating_sub(lookback))..idx {
                if ltf[i].high > swing_high {
                    swing_high = ltf[i].high;
                }
            }
            if current.high > swing_high && current.close < current.open {
                Some(swing_high)
            } else {
                None
            }
        }
    }
}

fn find_htf_index_for_time(htf: &[CandleStick], ltf_time: i64) -> usize {
    let mut idx = 0;
    for (i, candle) in htf.iter().enumerate() {
        if candle.open_time <= ltf_time {
            idx = i;
        } else {
            break;
        }
    }
    idx
}

async fn run_ws_stream(
    url: &str,
    product_type: &str,
    assets: &[String],
    ltf_channel: &str,
    htf_channel: &str,
    tx: mpsc::Sender<CandleUpdate>,
) -> Result<()> {
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    let mut args = Vec::new();
    for asset in assets {
        args.push(BitgetWsChannelArg {
            inst_type: product_type.to_string(),
            channel: ltf_channel.to_string(),
            inst_id: asset.to_string(),
        });
        args.push(BitgetWsChannelArg {
            inst_type: product_type.to_string(),
            channel: htf_channel.to_string(),
            inst_id: asset.to_string(),
        });
    }

    let sub = BitgetWsSubscribe {
        op: "subscribe".to_string(),
        args,
    };

    let sub_text = serde_json::to_string(&sub)?;
    write.send(sub_text.into()).await?;

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if !msg.is_text() {
            continue;
        }

        let text = msg.to_text()?;
        let value: Value = serde_json::from_str(text)?;

        if value.get("event").is_some() {
            continue;
        }

        let push: Result<BitgetWsCandlePush> =
            serde_json::from_value(value.clone()).map_err(|e| e.into());
        if let Ok(push) = push {
            for row in push.data {
                let update = CandleUpdate {
                    symbol: push.arg.inst_id.clone(),
                    channel: push.arg.channel.clone(),
                    row,
                };
                if tx.send(update).await.is_err() {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

async fn fetch_candles(
    http: &Client,
    bitget: &BitgetClient,
    symbol: &str,
    product_type: &str,
    granularity: &str,
    logger: &TradeLogger,
) -> Result<Vec<CandleStick>> {
    let query = BitgetCandleQuery {
        symbol: symbol.to_string(),
        product_type: product_type.to_string(),
        granularity: granularity.to_string(),
        start_time: None,
        end_time: None,
        kline_type: None,
        limit: Some("100".to_string()),
    };

    let qs = serde_urlencoded::to_string(&query)?;
    let url = format!("{}/api/v2/mix/market/candles?{}", bitget.base_url, qs);
    let resp = http.get(url).send().await?.text().await?;
    let parsed: BitgetRestResponse<Vec<BitgetRestCandleRow>> = serde_json::from_str(&resp)?;

    if parsed.code != "00000" {
        log_response_error(
            logger,
            "Bitget candles error",
            &parsed.code,
            &parsed.msg,
            &resp,
        );
        return Err(anyhow!(
            "Bitget candles error: code={} msg={}",
            parsed.code,
            parsed.msg
        ));
    }

    let interval_ms = granularity_ms(granularity)?;
    let mut candles = Vec::new();
    for row in parsed.data {
        if row.len() < 5 {
            continue;
        }
        let ts_ms = row[0].parse::<i64>()?;
        let open = DecimalVec(Decimal::from_str(&row[1])?);
        let high = DecimalVec(Decimal::from_str(&row[2])?);
        let low = DecimalVec(Decimal::from_str(&row[3])?);
        let close = DecimalVec(Decimal::from_str(&row[4])?);

        let open_time = ts_ms / 1000;
        let close_time = open_time + (interval_ms / 1000);

        candles.push(CandleStick {
            open_time,
            open,
            high,
            low,
            close,
            close_time,
        });
    }

    Ok(candles)
}

async fn signed_get<T: DeserializeOwned>(
    http: &Client,
    bitget: &BitgetClient,
    path: &str,
    query: &str,
) -> Result<(T, String)> {
    let headers = bitget.build_headers("GET", path, Some(query), None)?;
    let url = format!("{}{}?{}", bitget.base_url, path, query);
    let resp = http.get(url).headers(headers).send().await?.text().await?;
    let parsed: T = serde_json::from_str(&resp)?;
    Ok((parsed, resp))
}

async fn signed_post<T: DeserializeOwned, B: serde::Serialize>(
    http: &Client,
    bitget: &BitgetClient,
    path: &str,
    body: &B,
) -> Result<(T, String)> {
    let body_str = serde_json::to_string(body)?;
    let headers = bitget.build_headers("POST", path, None, Some(&body_str))?;
    let url = format!("{}{}", bitget.base_url, path);
    let resp = http
        .post(url)
        .headers(headers)
        .body(body_str)
        .send()
        .await?
        .text()
        .await?;
    let parsed: T = serde_json::from_str(&resp)?;
    Ok((parsed, resp))
}

async fn fetch_account_equity(
    http: &Client,
    bitget: &BitgetClient,
    symbol: &str,
    product_type: &str,
    margin_coin: &str,
    logger: &TradeLogger,
) -> Result<Decimal> {
    let query = format!(
        "symbol={}&productType={}&marginCoin={}",
        symbol, product_type, margin_coin
    );
    let (resp, raw): (BitgetRestResponse<BitgetAccountInfo>, String) =
        signed_get(http, bitget, "/api/v2/mix/account/account", &query).await?;

    if resp.code != "00000" {
        log_response_error(logger, "Account query error", &resp.code, &resp.msg, &raw);
        return Err(anyhow!(
            "Account query error: code={} msg={}",
            resp.code,
            resp.msg
        ));
    }

    let equity = resp
        .data
        .usdt_equity
        .as_ref()
        .or(resp.data.account_equity.as_ref())
        .ok_or_else(|| anyhow!("Missing equity fields in account response"))?;

    Ok(Decimal::from_str(equity)?)
}

async fn has_open_position(
    http: &Client,
    bitget: &BitgetClient,
    symbol: &str,
    product_type: &str,
    margin_coin: &str,
    logger: &TradeLogger,
) -> Result<bool> {
    let query = format!(
        "symbol={}&productType={}&marginCoin={}",
        symbol, product_type, margin_coin
    );
    let (resp, raw): (BitgetRestResponse<Vec<BitgetPosition>>, String) =
        signed_get(http, bitget, "/api/v2/mix/position/single-position", &query).await?;

    if resp.code != "00000" {
        log_response_error(logger, "Position query error", &resp.code, &resp.msg, &raw);
        return Err(anyhow!(
            "Position query error: code={} msg={}",
            resp.code,
            resp.msg
        ));
    }

    for pos in resp.data {
        if let Some(total) = pos.total.as_ref().and_then(|v| Decimal::from_str(v).ok()) {
            if total > Decimal::ZERO {
                return Ok(true);
            }
        } else if let Some(size) = pos.size.as_ref().and_then(|v| Decimal::from_str(v).ok()) {
            if size > Decimal::ZERO {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

async fn has_pending_order(
    http: &Client,
    bitget: &BitgetClient,
    symbol: &str,
    product_type: &str,
    logger: &TradeLogger,
) -> Result<bool> {
    let query = format!(
        "productType={}&symbol={}&limit=100",
        product_type, symbol
    );
    let (resp, raw): (BitgetRestResponse<BitgetOrdersHistoryData>, String) =
        signed_get(http, bitget, "/api/v2/mix/order/orders-pending", &query).await?;

    if resp.code != "00000" {
        log_response_error(logger, "Pending orders error", &resp.code, &resp.msg, &raw);
        return Err(anyhow!(
            "Pending orders error: code={} msg={}",
            resp.code,
            resp.msg
        ));
    }

    Ok(!resp.data.entrusted_list.is_empty())
}

async fn fetch_contracts(
    http: &Client,
    bitget: &BitgetClient,
    product_type: &str,
    logger: &TradeLogger,
) -> Result<HashMap<String, (Option<Decimal>, Option<Decimal>)>> {
    let url = format!(
        "{}/api/v2/mix/market/contracts?productType={}",
        bitget.base_url, product_type
    );
    let resp = http.get(url).send().await?.text().await?;
    let parsed: BitgetRestResponse<Vec<BitgetContractInfo>> = serde_json::from_str(&resp)?;

    if parsed.code != "00000" {
        log_response_error(logger, "Contracts query error", &parsed.code, &parsed.msg, &resp);
        return Err(anyhow!(
            "Contracts query error: code={} msg={}",
            parsed.code,
            parsed.msg
        ));
    }

    let mut map = HashMap::new();
    for contract in parsed.data {
        let Some(symbol) = contract.symbol.clone() else { continue };
        let size_step = contract
            .size_multiplier
            .as_ref()
            .and_then(|v| Decimal::from_str(v).ok())
            .or_else(|| {
                contract
                    .min_trade_num
                    .as_ref()
                    .and_then(|v| Decimal::from_str(v).ok())
            });

        let price_step = contract
            .tick_size
            .as_ref()
            .and_then(|v| Decimal::from_str(v).ok())
            .or_else(|| {
                contract
                    .price_end_step
                    .as_ref()
                    .and_then(|v| Decimal::from_str(v).ok())
            });

        map.insert(symbol, (size_step, price_step));
    }

    Ok(map)
}

async fn set_leverage(
    http: &Client,
    bitget: &BitgetClient,
    symbol: &str,
    product_type: &str,
    margin_coin: &str,
    leverage: Decimal,
    logger: &TradeLogger,
) -> Result<()> {
    let body = BitgetSetLeverageRequest {
        symbol: symbol.to_string(),
        product_type: product_type.to_string(),
        margin_coin: margin_coin.to_string(),
        leverage: Some(format_decimal(leverage)),
        long_leverage: None,
        short_leverage: None,
        hold_side: None,
    };

    let (resp, raw): (BitgetRestResponse<BitgetSetLeverageResponse>, String) =
        signed_post(http, bitget, "/api/v2/mix/account/set-leverage", &body).await?;

    if resp.code != "00000" {
        log_response_error(logger, "Set leverage error", &resp.code, &resp.msg, &raw);
        return Err(anyhow!(
            "Set leverage error: code={} msg={}",
            resp.code,
            resp.msg
        ));
    }

    Ok(())
}

async fn pnl_monitor_loop(
    http: &Client,
    bitget: &BitgetClient,
    bot_cfg: &BotConfig,
    ex_cfg: &ExchangeConfig,
    pnl_cfg: &PnlConfig,
    seen_orders: &Arc<Mutex<HashSet<String>>>,
    signal_map: &Arc<Mutex<HashMap<String, SignalMeta>>>,
    logger: &Arc<TradeLogger>,
    alerts: &Arc<Alerts>,
    risk_state: &Arc<RwLock<RiskState>>,
) -> Result<()> {
    loop {
        for symbol in &bot_cfg.assets {
            let equity = fetch_account_equity(
                http,
                bitget,
                symbol,
                &ex_cfg.product_type,
                &ex_cfg.margin_coin,
                logger.as_ref(),
            )
            .await?;
            update_drawdown(
                symbol,
                equity,
                bot_cfg.max_total_drawdown_pct,
                &logger.log_dir,
                risk_state,
                alerts,
            )
            .await;

            let query = format!(
                "productType={}&symbol={}&limit=100",
                ex_cfg.product_type, symbol
            );
            let (resp, raw): (BitgetRestResponse<BitgetOrdersHistoryData>, String) =
                signed_get(http, bitget, "/api/v2/mix/order/orders-history", &query).await?;

            if resp.code != "00000" {
                let msg = format!(
                    "[{}] Order history error: code={} msg={}",
                    symbol, resp.code, resp.msg
                );
                log_response_error(
                    logger.as_ref(),
                    "Order history error",
                    &resp.code,
                    &resp.msg,
                    &raw,
                );
                alerts.send(msg).await;
                continue;
            }

            for order in resp.data.entrusted_list {
                let order_id = order.order_id.clone().unwrap_or_default();
                if order_id.is_empty() {
                    continue;
                }

                let mut seen_guard = seen_orders.lock().await;
                if seen_guard.contains(&order_id) {
                    continue;
                }
                seen_guard.insert(order_id.clone());
                let seen_snapshot = seen_guard.clone();
                drop(seen_guard);
                let _ = persist_seen_orders(&logger.log_dir, &seen_snapshot);

                if order.status.as_deref() != Some("filled") {
                    continue;
                }

                let pnl = order.total_profits.clone();
                let fee = order.fee.clone();

                let client_oid = order.client_oid.clone();
                let meta = if let Some(coid) = client_oid.clone() {
                    let map = signal_map.lock().await;
                    map.get(&coid).cloned()
                } else {
                    None
                };

                let rr = meta.as_ref().and_then(|m| {
                    let risk = (m.entry - m.sl).abs();
                    if risk > Decimal::ZERO {
                        Some(format_decimal((m.tp - m.entry).abs() / risk))
                    } else {
                        None
                    }
                });

                logger.log_trade(TradeLogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    symbol: symbol.clone(),
                    side: order.side.clone().unwrap_or_else(|| "unknown".to_string()),
                    size: order.size.clone().unwrap_or_default(),
                    entry: meta.as_ref().map(|m| format_decimal(m.entry)),
                    sl: meta.as_ref().map(|m| format_decimal(m.sl)),
                    tp: meta.as_ref().map(|m| format_decimal(m.tp)),
                    status: "filled".to_string(),
                    pnl,
                    fee,
                    rr,
                    order_id: order.order_id.clone(),
                    client_oid: client_oid.clone(),
                })?;

                alerts
                    .send(format!(
                        "[{}] Filled order {} pnl={:?} fee={:?}",
                        symbol, order_id, order.total_profits, order.fee
                    ))
                    .await;
            }
        }

        tokio::time::sleep(Duration::from_secs(pnl_cfg.poll_interval_sec)).await;
    }
}

async fn update_drawdown(
    symbol: &str,
    equity: Decimal,
    max_dd_pct: Decimal,
    log_dir: &str,
    risk_state: &Arc<RwLock<RiskState>>,
    alerts: &Arc<Alerts>,
) {
    let mut guard = risk_state.write().await;
    if guard.peak_equity == Decimal::ZERO {
        guard.peak_equity = equity;
        guard.last_equity = equity;
        return;
    }

    if equity > guard.peak_equity {
        guard.peak_equity = equity;
    }

    guard.last_equity = equity;

    let _ = persist_risk_state(log_dir, &guard);

    let drawdown = if guard.peak_equity > Decimal::ZERO {
        (guard.peak_equity - equity) / guard.peak_equity
    } else {
        Decimal::ZERO
    };

    if drawdown >= max_dd_pct && !guard.halted {
        guard.halted = true;
        let _ = persist_risk_state(log_dir, &guard);
        let halt_path = Path::new(log_dir).join("halted.signal");
        let _ = fs::write(
            &halt_path,
            format!(
                "halted=true symbol={} drawdown_pct={:.2} resume=resume.signal|resume.command",
                symbol,
                (drawdown * Decimal::from(100)).to_f64().unwrap_or(0.0)
            ),
        );
        alerts
            .send(format!(
                "[{}] Max drawdown hit: {:.2}%. Trading halted.",
                symbol,
                (drawdown * Decimal::from(100)).to_f64().unwrap_or(0.0)
            ))
            .await;
    }
}

fn granularity_ms(granularity: &str) -> Result<i64> {
    match granularity {
        "1m" => Ok(60_000),
        "3m" => Ok(180_000),
        "5m" => Ok(300_000),
        "15m" => Ok(900_000),
        "30m" => Ok(1_800_000),
        "1H" => Ok(3_600_000),
        "4H" => Ok(14_400_000),
        "6H" => Ok(21_600_000),
        "12H" => Ok(43_200_000),
        "1D" => Ok(86_400_000),
        _ => Err(anyhow!("Unsupported granularity: {}", granularity)),
    }
}

fn channel_from_granularity(granularity: &str) -> Result<String> {
    match granularity {
        "1m" => Ok("candle1m".to_string()),
        "3m" => Ok("candle3m".to_string()),
        "5m" => Ok("candle5m".to_string()),
        "15m" => Ok("candle15m".to_string()),
        "30m" => Ok("candle30m".to_string()),
        "1H" => Ok("candle1H".to_string()),
        "4H" => Ok("candle4H".to_string()),
        "6H" => Ok("candle6H".to_string()),
        "12H" => Ok("candle12H".to_string()),
        "1D" => Ok("candle1D".to_string()),
        _ => Err(anyhow!("Unsupported granularity: {}", granularity)),
    }
}

fn floor_to_step(value: Decimal, step: Decimal) -> Decimal {
    if step <= Decimal::ZERO {
        return value;
    }
    let multiplier = (value / step).floor();
    multiplier * step
}

fn format_decimal(value: Decimal) -> String {
    let f = value.to_f64().unwrap_or(0.0);
    format!("{:.8}", f)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn log_response_error(
    logger: &TradeLogger,
    context: &str,
    code: &str,
    msg: &str,
    raw: &str,
) {
    let message = format!("{context}: code={code} msg={msg}");
    let _ = logger.log_error(&message, Some(raw));
}

struct TradeLogger {
    csv: Option<StdMutex<File>>,
    json: Option<StdMutex<File>>,
    error: Option<StdMutex<File>>,
    log_dir: String,
}

impl TradeLogger {
    fn new(cfg: &LoggingConfig) -> Result<Self> {
        let log_dir = cfg.log_dir.clone();
        if cfg.enable_csv || cfg.enable_json {
            fs::create_dir_all(&log_dir)?;
        }

        let csv = if cfg.enable_csv {
            let path = Path::new(&log_dir).join("trades.csv");
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            Some(StdMutex::new(file))
        } else {
            None
        };

        let json = if cfg.enable_json {
            let path = Path::new(&log_dir).join("trades.jsonl");
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            Some(StdMutex::new(file))
        } else {
            None
        };

        let error = if cfg.include_raw_errors {
            let path = Path::new(&log_dir).join("errors.log");
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            Some(StdMutex::new(file))
        } else {
            None
        };

        Ok(Self {
            csv,
            json,
            error,
            log_dir,
        })
    }

    fn log_trade(&self, entry: TradeLogEntry) -> Result<()> {
        if let Some(csv) = &self.csv {
            let mut file = csv.lock().unwrap();
            let line = format!(
                "{},{},{},{},{},{},{},{},{},{},{},{}\n",
                entry.timestamp,
                entry.symbol,
                entry.side,
                entry.size,
                entry.entry.unwrap_or_default(),
                entry.sl.unwrap_or_default(),
                entry.tp.unwrap_or_default(),
                entry.status,
                entry.pnl.unwrap_or_default(),
                entry.fee.unwrap_or_default(),
                entry.order_id.unwrap_or_default(),
                entry.client_oid.unwrap_or_default()
            );
            file.write_all(line.as_bytes())?;
        }

        if let Some(json) = &self.json {
            let mut file = json.lock().unwrap();
            let line = serde_json::to_string(&entry)?;
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
        }

        Ok(())
    }

    fn log_error(&self, message: &str, raw: Option<&str>) -> Result<()> {
        if let Some(error) = &self.error {
            let mut file = error.lock().unwrap();
            let raw_str = raw.unwrap_or("");
            let line = format!(
                "{} | {} | {}\n",
                Utc::now().to_rfc3339(),
                message,
                raw_str
            );
            file.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

struct Alerts {
    telegram: TelegramConfig,
    discord: DiscordConfig,
    http: Client,
}

impl Alerts {
    fn new(cfg: &AlertsConfig, http: Client) -> Self {
        Self {
            telegram: cfg.telegram.clone(),
            discord: cfg.discord.clone(),
            http,
        }
    }

    async fn send(&self, message: String) {
        println!("{message}");

        if self.discord.enabled && !self.discord.webhook_url.is_empty() {
            let payload = serde_json::json!({ "content": message });
            let _ = self.http.post(&self.discord.webhook_url).json(&payload).send().await;
            return;
        }

        if self.telegram.enabled
            && !self.telegram.bot_token.is_empty()
            && !self.telegram.chat_id.is_empty()
        {
            let url = format!(
                "https://api.telegram.org/bot{}/sendMessage",
                self.telegram.bot_token
            );
            let payload = serde_json::json!({
                "chat_id": self.telegram.chat_id,
                "text": message
            });
            let _ = self.http.post(url).json(&payload).send().await;
        }
    }
}
