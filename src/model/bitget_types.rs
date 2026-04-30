use serde::{Deserialize, Serialize};

/// ===== REST: Common Response Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetRestResponse<T> {
    pub code: String,
    pub msg: String,
    #[serde(rename = "requestTime")]
    pub request_time: Option<i64>,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetRestEmptyResponse {
    pub code: String,
    pub msg: String,
    #[serde(rename = "requestTime")]
    pub request_time: Option<i64>,
}

/// ===== REST: Change Leverage =====
/// POST /api/v2/mix/account/set-leverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetSetLeverageRequest {
    pub symbol: String,
    #[serde(rename = "productType")]
    pub product_type: String,
    #[serde(rename = "marginCoin")]
    pub margin_coin: String,
    #[serde(rename = "leverage", skip_serializing_if = "Option::is_none")]
    pub leverage: Option<String>,
    #[serde(rename = "longLeverage", skip_serializing_if = "Option::is_none")]
    pub long_leverage: Option<String>,
    #[serde(rename = "shortLeverage", skip_serializing_if = "Option::is_none")]
    pub short_leverage: Option<String>,
    #[serde(rename = "holdSide", skip_serializing_if = "Option::is_none")]
    pub hold_side: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetSetLeverageResponse {
    pub symbol: Option<String>,
    #[serde(rename = "marginCoin")]
    pub margin_coin: Option<String>,
    #[serde(rename = "longLeverage")]
    pub long_leverage: Option<String>,
    #[serde(rename = "shortLeverage")]
    pub short_leverage: Option<String>,
    #[serde(rename = "crossMarginLeverage")]
    pub cross_margin_leverage: Option<String>,
    #[serde(rename = "marginMode")]
    pub margin_mode: Option<String>,
}

/// ===== REST: Candles =====
/// GET /api/v2/mix/market/candles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetCandleQuery {
    pub symbol: String,
    #[serde(rename = "productType")]
    pub product_type: String,
    pub granularity: String,
    #[serde(rename = "startTime", skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(rename = "kLineType", skip_serializing_if = "Option::is_none")]
    pub kline_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
}

/// Bitget REST candle row:
/// [ timestamp(ms), open, high, low, close, baseVol, quoteVol ]
pub type BitgetRestCandleRow = Vec<String>;

/// ===== REST: Contracts =====
/// GET /api/v2/mix/market/contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetContractInfo {
    pub symbol: Option<String>,
    #[serde(rename = "productType")]
    pub product_type: Option<String>,
    #[serde(rename = "minTradeNum")]
    pub min_trade_num: Option<String>,
    #[serde(rename = "sizeMultiplier")]
    pub size_multiplier: Option<String>,
    #[serde(rename = "priceEndStep")]
    pub price_end_step: Option<String>,
    #[serde(rename = "tickSize")]
    pub tick_size: Option<String>,
    #[serde(rename = "minTradeAmount")]
    pub min_trade_amount: Option<String>,
    #[serde(rename = "maxLever")]
    pub max_lever: Option<String>,
    #[serde(rename = "minLever")]
    pub min_lever: Option<String>,
}

/// ===== REST: Place Order =====
/// POST /api/v2/mix/order/place-order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetPlaceOrderRequest {
    pub symbol: String,
    #[serde(rename = "productType")]
    pub product_type: String,
    #[serde(rename = "marginMode")]
    pub margin_mode: String,
    #[serde(rename = "marginCoin")]
    pub margin_coin: String,
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    pub side: String,
    #[serde(rename = "tradeSide", skip_serializing_if = "Option::is_none")]
    pub trade_side: Option<String>,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<String>,
    #[serde(rename = "clientOid", skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
    #[serde(rename = "reduceOnly", skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<String>,
    #[serde(rename = "presetTakeProfitPrice", skip_serializing_if = "Option::is_none")]
    pub preset_take_profit_price: Option<String>,
    #[serde(rename = "presetStopLossPrice", skip_serializing_if = "Option::is_none")]
    pub preset_stop_loss_price: Option<String>,
    #[serde(rename = "presetTakeProfitExecutePrice", skip_serializing_if = "Option::is_none")]
    pub preset_take_profit_execute_price: Option<String>,
    #[serde(rename = "presetStopLossExecutePrice", skip_serializing_if = "Option::is_none")]
    pub preset_stop_loss_execute_price: Option<String>,
    #[serde(rename = "stpMode", skip_serializing_if = "Option::is_none")]
    pub stp_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetPlaceOrderResponse {
    #[serde(rename = "clientOid")]
    pub client_oid: Option<String>,
    #[serde(rename = "orderId")]
    pub order_id: Option<String>,
}

/// ===== REST: Orders (History/Pending/Details) =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetOrderInfo {
    #[serde(rename = "orderId")]
    pub order_id: Option<String>,
    #[serde(rename = "clientOid")]
    pub client_oid: Option<String>,
    pub symbol: Option<String>,
    #[serde(rename = "productType")]
    pub product_type: Option<String>,
    pub side: Option<String>,
    #[serde(rename = "orderType")]
    pub order_type: Option<String>,
    pub size: Option<String>,
    pub price: Option<String>,
    #[serde(rename = "filledSize")]
    pub filled_size: Option<String>,
    #[serde(rename = "avgFillPrice")]
    pub avg_fill_price: Option<String>,
    #[serde(rename = "priceAvg")]
    pub price_avg: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "baseVolume")]
    pub base_volume: Option<String>,
    #[serde(rename = "quoteVolume")]
    pub quote_volume: Option<String>,
    pub fee: Option<String>,
    #[serde(rename = "totalProfits")]
    pub total_profits: Option<String>,
    #[serde(rename = "posSide")]
    pub pos_side: Option<String>,
    #[serde(rename = "marginCoin")]
    pub margin_coin: Option<String>,
    pub leverage: Option<String>,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: Option<String>,
    #[serde(rename = "marginMode")]
    pub margin_mode: Option<String>,
    #[serde(rename = "tradeSide")]
    pub trade_side: Option<String>,
    #[serde(rename = "posMode")]
    pub pos_mode: Option<String>,
    #[serde(rename = "enterPointSource")]
    pub enter_point_source: Option<String>,
    #[serde(rename = "orderSource")]
    pub order_source: Option<String>,
    #[serde(rename = "presetStopSurplusPrice")]
    pub preset_stop_surplus_price: Option<String>,
    #[serde(rename = "presetStopLossPrice")]
    pub preset_stop_loss_price: Option<String>,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "updateTime")]
    pub update_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetOrdersHistoryData {
    #[serde(rename = "entrustedList")]
    pub entrusted_list: Vec<BitgetOrderInfo>,
    #[serde(rename = "endId")]
    pub end_id: Option<String>,
}

/// ===== REST: Positions =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetPosition {
    pub symbol: Option<String>,
    #[serde(rename = "productType")]
    pub product_type: Option<String>,
    #[serde(rename = "marginMode")]
    pub margin_mode: Option<String>,
    #[serde(rename = "holdSide")]
    pub hold_side: Option<String>,
    pub side: Option<String>,
    pub size: Option<String>,
    #[serde(rename = "total")]
    pub total: Option<String>,
    pub available: Option<String>,
    pub locked: Option<String>,
    #[serde(rename = "openPriceAvg")]
    pub open_price_avg: Option<String>,
    #[serde(rename = "avgEntryPrice")]
    pub avg_entry_price: Option<String>,
    #[serde(rename = "markPrice")]
    pub mark_price: Option<String>,
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Option<String>,
    #[serde(rename = "marginCoin")]
    pub margin_coin: Option<String>,
    #[serde(rename = "leverage")]
    pub leverage: Option<String>,
    #[serde(rename = "liquidationPrice")]
    pub liquidation_price: Option<String>,
    #[serde(rename = "marginRatio")]
    pub margin_ratio: Option<String>,
    #[serde(rename = "takeProfit")]
    pub take_profit: Option<String>,
    #[serde(rename = "stopLoss")]
    pub stop_loss: Option<String>,
    #[serde(rename = "takeProfitId")]
    pub take_profit_id: Option<String>,
    #[serde(rename = "stopLossId")]
    pub stop_loss_id: Option<String>,
}

/// ===== REST: Account =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetAccountInfo {
    #[serde(rename = "marginCoin")]
    pub margin_coin: Option<String>,
    pub locked: Option<String>,
    pub available: Option<String>,
    #[serde(rename = "accountEquity")]
    pub account_equity: Option<String>,
    #[serde(rename = "usdtEquity")]
    pub usdt_equity: Option<String>,
    #[serde(rename = "posMode")]
    pub pos_mode: Option<String>,
    #[serde(rename = "marginMode")]
    pub margin_mode: Option<String>,
}

/// ===== WebSocket: Public Candlesticks =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetWsSubscribe {
    pub op: String,
    pub args: Vec<BitgetWsChannelArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetWsChannelArg {
    #[serde(rename = "instType")]
    pub inst_type: String,
    pub channel: String,
    #[serde(rename = "instId")]
    pub inst_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetWsEvent {
    pub event: Option<String>,
    pub arg: Option<BitgetWsChannelArg>,
    pub code: Option<String>,
    pub msg: Option<String>,
}

/// WebSocket candlestick push:
/// data: [[ts, open, high, low, close, baseVol, quoteVol, usdtVol], ...]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitgetWsCandlePush {
    pub action: Option<String>,
    pub arg: BitgetWsChannelArg,
    pub data: Vec<BitgetWsCandleRow>,
    pub ts: Option<i64>,
}

pub type BitgetWsCandleRow = Vec<String>;
