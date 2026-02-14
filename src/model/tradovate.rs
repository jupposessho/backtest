use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct AuthRequest<'a> {
    pub name: &'a str,
    pub password: &'a str,
    #[serde(rename = "appId")]
    pub app_id: &'a str,
    #[serde(rename = "appVersion")]
    pub app_version: &'a str,
    pub cid: u32,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "userId")]
    pub user_id: u32,
    pub accounts: Vec<Account>,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    #[serde(rename = "accountId")]
    pub account_id: u32,
}

#[derive(Debug, Serialize)]
pub struct OrderRequest {
    #[serde(rename = "accountId")]
    pub account_id: u32,
    pub action: String,
    pub symbol: String,
    #[serde(rename = "orderQty")]
    pub order_qty: i32,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
}
