use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub user_id: Uuid,
    pub account_id: Uuid,
    pub symbol: String,
    pub timeframe: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Analysis {
    pub symbol: String,
    pub timeframe: String,
    pub signal: String,
    pub market_bias: String,
    pub confidence: u8,
    pub entry: Decimal,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub risk_reward_ratio: Option<Decimal>,
    pub reasoning_summary: Vec<String>,
    pub risk_notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIntent {
    pub user_id: Uuid,
    pub account_id: Uuid,
    pub analysis_id: Option<Uuid>,
    pub symbol: String,
    pub side: Side,
    pub volume: Decimal,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmIntent {
    pub user_id: Uuid,
    pub idempotency_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResult {
    pub ticket: i64,
    pub executed_price: Decimal,
    pub status: String,
}
