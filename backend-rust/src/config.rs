use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub mt5_bridge_url: String,
    pub mt5_bridge_api_key: String,
    pub encryption_key: [u8; 32],
    pub trading_mode: TradingMode,
    pub live_trading_enabled: bool,
    pub market_data_max_age_seconds: i64,
    pub admin_username: String,
    pub admin_password_hash: Option<String>,
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
            admin_username: env("ADMIN_USERNAME", "admin"),
            admin_password_hash: std::env::var("ADMIN_PASSWORD_HASH_B64")
                .ok()
                .filter(|v| !v.is_empty())
                .map(|value| STANDARD.decode(value))
                .transpose()
                .context("ADMIN_PASSWORD_HASH_B64 must be valid base64")?
                .map(String::from_utf8)
                .transpose()
                .context("ADMIN_PASSWORD_HASH_B64 must contain a UTF-8 Argon2 hash")?,
        })
    }
}
fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}
pub fn bind_addr() -> String {
    env("BIND_ADDR", "0.0.0.0:8080")
}
