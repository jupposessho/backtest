// extern crate backtest;

// use backtest::model::tradovate::{AuthRequest, AuthResponse};
// // use futures_util::{SinkExt, StreamExt};
// use reqwest::Client;
// // use serde::{Deserialize, Serialize};
// // use std::time::Duration;
// // use tokio_tungstenite::{connect_async, tungstenite::Message};
// // use url::Url;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let username = "your_username";
//     let password = "your_password";
//     let app_id = "MyRustBot";
//     let app_version = "1.0";

//     // let symbol = "ESU5"; // Replace with active contract
//     // let buy_below = 5300.00;
//     // let sell_above = 5400.00;

//     let auth = AuthRequest {
//         name: username,
//         password,
//         app_id,
//         app_version,
//         cid: 1,
//     };

//     let client = Client::new();

//     // 🔐 Authenticate
//     let raw_response = client
//         .post("https://demo-api.tradovate.com/v1/auth/accesstokenrequest")
//         .json(&auth)
//         .send();

//     match raw_response.await {
//         Ok(response) => {
//             println!("{}", response.text().await?);
//             // let res = response.json::<AuthResponse>().await?;

//             // let account_id = res.accounts[0].account_id;
//             // println!("✅ Authenticated as {}", res.user_id);
//             // println!("✅ Account ID: {}", account_id);
//         }
//         Err(e) => {
//             println!("Error: {}", e);
//         }
//     }
//     // println!("{}", raw_response.text().await?);

//     Ok(())
// }
