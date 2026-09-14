use crate::{
    config::Config,
    error::AppError,
    models::{Analysis, Quote},
};
use reqwest::Client;
use serde_json::json;

#[derive(Clone)]
pub struct AiClient {
    http: Client,
    key: Option<String>,
    model: String,
}
impl AiClient {
    pub fn new(c: &Config) -> anyhow::Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
            key: c.openai_api_key.clone(),
            model: c.openai_model.clone(),
        })
    }
    pub async fn analyze(&self, quote: &Quote, timeframe: &str) -> Result<Analysis, AppError> {
        let Some(key) = &self.key else {
            return Ok(mock_analysis(quote, timeframe));
        };
        let schema = json!({"type":"object","properties":{"symbol":{"type":"string"},"timeframe":{"type":"string"},"signal":{"type":"string","enum":["BUY","SELL","WAIT"]},"market_bias":{"type":"string"},"confidence":{"type":"integer","minimum":0,"maximum":100},"entry":{"type":"number"},"stop_loss":{"type":["number","null"]},"take_profit":{"type":["number","null"]},"risk_reward_ratio":{"type":["number","null"]},"reasoning_summary":{"type":"array","items":{"type":"string"}},"risk_notes":{"type":"array","items":{"type":"string"}}},"required":["symbol","timeframe","signal","market_bias","confidence","entry","stop_loss","take_profit","risk_reward_ratio","reasoning_summary","risk_notes"],"additionalProperties":false});
        let body = json!({"model":self.model,"input":[{"role":"system","content":"You are an AI Forex Market Analysis Assistant. Use only supplied market data. Never execute trades, guarantee profit, or invent prices. Prefer WAIT when unclear."},{"role":"user","content":serde_json::to_string(&json!({"symbol":quote.symbol,"bid":quote.bid,"ask":quote.ask,"timestamp":quote.timestamp,"timeframe":timeframe})).unwrap_or_default()}],"text":{"format":{"type":"json_schema","name":"forex_analysis","strict":true,"schema":schema}}});
        let value: serde_json::Value = self
            .http
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|_| AppError::Unavailable)?
            .error_for_status()
            .map_err(|_| AppError::Unavailable)?
            .json()
            .await
            .map_err(|_| AppError::Unavailable)?;
        let text = value
            .pointer("/output/0/content/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation("AI returned no structured analysis".into()))?;
        let analysis: Analysis = serde_json::from_str(text)
            .map_err(|_| AppError::Validation("AI returned invalid analysis".into()))?;
        validate_analysis(&analysis, quote, timeframe)?;
        Ok(analysis)
    }
}
fn validate_analysis(a: &Analysis, q: &Quote, tf: &str) -> Result<(), AppError> {
    if a.symbol != q.symbol
        || a.timeframe != tf
        || a.confidence > 100
        || !matches!(a.signal.as_str(), "BUY" | "SELL" | "WAIT")
    {
        return Err(AppError::Validation(
            "untrusted AI output failed validation".into(),
        ));
    }
    Ok(())
}
fn mock_analysis(q: &Quote, tf: &str) -> Analysis {
    Analysis {
        symbol: q.symbol.clone(),
        timeframe: tf.into(),
        signal: "WAIT".into(),
        market_bias: "NEUTRAL".into(),
        confidence: 50,
        entry: (q.bid + q.ask) / rust_decimal::Decimal::TWO,
        stop_loss: None,
        take_profit: None,
        risk_reward_ratio: None,
        reasoning_summary: vec!["Mock mode has insufficient indicator data".into()],
        risk_notes: vec!["Demo analysis; no order is implied".into()],
    }
}
