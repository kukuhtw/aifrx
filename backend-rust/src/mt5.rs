use crate::{
    config::Config,
    error::AppError,
    models::{OrderResult, Quote, Side},
};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone)]
pub struct Mt5Client {
    http: Client,
    base: String,
    key: String,
}
#[derive(Serialize)]
struct Order<'a> {
    trade_intent_id: Uuid,
    account_id: Uuid,
    symbol: &'a str,
    side: &'a Side,
    volume: Decimal,
    stop_loss: Option<Decimal>,
    take_profit: Option<Decimal>,
    max_slippage_points: i32,
}
impl Mt5Client {
    pub fn new(c: &Config) -> anyhow::Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
            base: c.mt5_bridge_url.trim_end_matches('/').into(),
            key: c.mt5_bridge_api_key.clone(),
        })
    }
    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{}", self.base, path))
            .header("X-Internal-API-Key", &self.key)
    }
    pub async fn health(&self) -> bool {
        self.req(reqwest::Method::GET, "/health")
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
    }
    pub async fn quote(&self, account: Uuid, symbol: &str) -> Result<Quote, AppError> {
        self.req(
            reqwest::Method::GET,
            &format!("/accounts/{account}/quote/{symbol}"),
        )
        .send()
        .await
        .map_err(|_| AppError::Unavailable)?
        .error_for_status()
        .map_err(|_| AppError::Unavailable)?
        .json()
        .await
        .map_err(|_| AppError::Unavailable)
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn order(
        &self,
        intent: Uuid,
        account: Uuid,
        symbol: &str,
        side: &Side,
        volume: Decimal,
        sl: Option<Decimal>,
        tp: Option<Decimal>,
        slip: i32,
    ) -> Result<OrderResult, AppError> {
        self.req(
            reqwest::Method::POST,
            &format!("/accounts/{account}/orders"),
        )
        .json(&Order {
            trade_intent_id: intent,
            account_id: account,
            symbol,
            side,
            volume,
            stop_loss: sl,
            take_profit: tp,
            max_slippage_points: slip,
        })
        .send()
        .await
        .map_err(|_| AppError::Unavailable)?
        .error_for_status()
        .map_err(|_| AppError::Unavailable)?
        .json()
        .await
        .map_err(|_| AppError::Unavailable)
    }
}
