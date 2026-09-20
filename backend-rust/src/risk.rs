use crate::{
    config::{Config, TradingMode},
    error::AppError,
    models::Side,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

pub struct RiskInput<'a> {
    pub account_type: &'a str,
    pub permission_mode: &'a str,
    pub account_active: bool,
    pub account_verified: bool,
    pub user_trading_enabled: bool,
    pub user_live_enabled: bool,
    pub volume: Decimal,
    pub max_lot: Decimal,
    pub side: &'a Side,
    pub price: Decimal,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub quote_time: DateTime<Utc>,
    pub open_positions: i64,
    pub max_open_positions: i32,
    pub trades_today: i64,
    pub max_trades_per_day: i32,
}
pub fn validate(config: &Config, i: &RiskInput<'_>) -> Result<(), AppError> {
    if !i.account_active || !i.account_verified {
        return invalid("account is inactive or unverified");
    }
    if i.permission_mode != "TRADING_ENABLED" || !i.user_trading_enabled {
        return invalid("trading is disabled");
    }
    if i.account_type == "LIVE"
        && (!config.live_trading_enabled
            || !i.user_live_enabled
            || config.trading_mode != TradingMode::Live)
    {
        return invalid("live trading is disabled");
    }
    if i.volume <= Decimal::ZERO || i.volume > i.max_lot {
        return invalid("lot size exceeds allowed range");
    }
    if i.open_positions >= i.max_open_positions.into() {
        return invalid("maximum open positions reached");
    }
    if i.trades_today >= i.max_trades_per_day.into() {
        return invalid("daily trade limit reached");
    }
    if (Utc::now() - i.quote_time).num_seconds() > config.market_data_max_age_seconds {
        return invalid("market price is stale");
    }
    match i.side {
        Side::Buy => {
            if i.stop_loss.is_some_and(|x| x >= i.price)
                || i.take_profit.is_some_and(|x| x <= i.price)
            {
                return invalid("invalid stop loss or take profit for BUY");
            }
        }
        Side::Sell => {
            if i.stop_loss.is_some_and(|x| x <= i.price)
                || i.take_profit.is_some_and(|x| x >= i.price)
            {
                return invalid("invalid stop loss or take profit for SELL");
            }
        }
    }
    Ok(())
}
fn invalid<T>(s: &str) -> Result<T, AppError> {
    Err(AppError::Validation(s.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TradingMode;
    fn cfg() -> Config {
        Config {
            database_url: "x".into(),
            telegram_bot_token: None,
            openai_api_key: None,
            openai_model: "x".into(),
            mt5_bridge_url: "x".into(),
            mt5_bridge_api_key: "x".into(),
            encryption_key: [0; 32],
            trading_mode: TradingMode::Demo,
            live_trading_enabled: false,
            market_data_max_age_seconds: 30,
        }
    }
    fn valid<'a>() -> RiskInput<'a> {
        RiskInput {
            account_type: "DEMO",
            permission_mode: "TRADING_ENABLED",
            account_active: true,
            account_verified: true,
            user_trading_enabled: true,
            user_live_enabled: false,
            volume: Decimal::new(1, 2),
            max_lot: Decimal::new(10, 2),
            side: &Side::Buy,
            price: Decimal::new(11752, 4),
            stop_loss: Some(Decimal::new(11710, 4)),
            take_profit: Some(Decimal::new(11810, 4)),
            quote_time: Utc::now(),
            open_positions: 0,
            max_open_positions: 5,
            trades_today: 0,
            max_trades_per_day: 20,
        }
    }
    #[test]
    fn accepts_safe_demo() {
        assert!(validate(&cfg(), &valid()).is_ok())
    }
    #[test]
    fn rejects_live_by_default() {
        let mut v = valid();
        v.account_type = "LIVE";
        assert!(validate(&cfg(), &v).is_err())
    }
    #[test]
    fn rejects_invalid_buy_sl() {
        let mut v = valid();
        v.stop_loss = Some(v.price);
        assert!(validate(&cfg(), &v).is_err())
    }
}
