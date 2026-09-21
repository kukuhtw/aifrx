use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub telegram_bot_token: Option<String>,
    pub telegram_bot_username: Option<String>,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub mt5_bridge_url: String,
    pub mt5_bridge_api_key: String,
    pub encryption_key: [u8; 32],
    pub trading_mode: TradingMode,
    pub live_trading_enabled: bool,
    pub market_data_max_age_seconds: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TradingMode {
    Mock,
    Demo,
    Live,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let mode = match env("TRADING_MODE", "DEMO").to_uppercase().as_str() {
            "MOCK" => TradingMode::Mock,
            "DEMO" => TradingMode::Demo,
            "LIVE" => TradingMode::Live,
            other => bail!("invalid TRADING_MODE: {other}"),
        };
        let live = env("LIVE_TRADING_ENABLED", "false").parse::<bool>()?;
        if mode == TradingMode::Live && !live {
            bail!("TRADING_MODE=LIVE requires LIVE_TRADING_ENABLED=true");
        }
        let decoded = STANDARD.decode(
            std::env::var("ENCRYPTION_KEY")
                .context("ENCRYPTION_KEY is required (base64 32 bytes)")?,
        )?;
        let encryption_key: [u8; 32] = decoded
            .try_into()
            .map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY must decode to 32 bytes"))?;
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            telegram_bot_token: std::env::var("TELEGRAM_BOT_TOKEN")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            telegram_bot_username: std::env::var("TELEGRAM_BOT_USERNAME")
                .ok()
                .map(|v| v.trim().trim_start_matches('@').to_string())
                .filter(|v| v.len() >= 5 && v.len() <= 32 && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')),
            openai_api_key: std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|v| !v.is_empty()),
            openai_model: env("OPENAI_MODEL", "gpt-5-mini"),
            mt5_bridge_url: env("MT5_BRIDGE_URL", "http://mt5-bridge:8000"),
            mt5_bridge_api_key: std::env::var("MT5_BRIDGE_API_KEY")
                .context("MT5_BRIDGE_API_KEY is required")?,
            encryption_key,
            trading_mode: mode,
            live_trading_enabled: live,
            market_data_max_age_seconds: env("MARKET_DATA_MAX_AGE_SECONDS", "30").parse()?,
        })
    }
}
fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}
pub fn bind_addr() -> String {
    env("BIND_ADDR", "0.0.0.0:8080")
}
