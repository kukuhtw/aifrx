use crate::{ai::AiClient, config::Config, mt5::Mt5Client};
use sqlx::PgPool;

pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub ai: AiClient,
    pub mt5: Mt5Client,
}
impl AppState {
    pub fn new(config: Config, db: PgPool) -> anyhow::Result<Self> {
        Ok(Self {
            ai: AiClient::new(&config)?,
            mt5: Mt5Client::new(&config)?,
            config,
            db,
        })
    }
}
