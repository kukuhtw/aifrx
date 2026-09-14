use crate::{
    error::AppError,
    models::{ConfirmIntent, OrderResult},
    risk::{self, RiskInput},
    state::AppState,
};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct LockedIntent {
    user_id: Uuid,
    account_id: Uuid,
    symbol: String,
    side: String,
    volume: Decimal,
    entry_price: Decimal,
    stop_loss: Option<Decimal>,
    take_profit: Option<Decimal>,
    status: String,
    account_type: String,
    permission_mode: String,
    is_active: bool,
    is_verified: bool,
    max_lot_size: Decimal,
    max_open_positions: i32,
    max_trades_per_day: i32,
    max_slippage_points: i32,
    trading_enabled: bool,
    live_trading_enabled: bool,
}

pub async fn confirm(
    state: &AppState,
    id: Uuid,
    input: ConfirmIntent,
) -> Result<OrderResult, AppError> {
    if input.idempotency_key.trim().len() < 16 {
        return Err(AppError::Validation(
            "idempotency key must be at least 16 characters".into(),
        ));
    }
    let mut tx = state.db.begin().await?;
    let locked: LockedIntent = sqlx::query_as(r#"SELECT ti.user_id,ti.account_id,ti.symbol,ti.side,ti.volume,ti.entry_price,ti.stop_loss,ti.take_profit,ti.status,a.account_type::text AS account_type,a.permission_mode::text AS permission_mode,a.is_active,a.is_verified,r.max_lot_size,r.max_open_positions,r.max_trades_per_day,r.max_slippage_points,r.trading_enabled,s.live_trading_enabled FROM trade_intents ti JOIN mt5_accounts a ON a.id=ti.account_id JOIN risk_profiles r ON r.user_id=ti.user_id AND (r.account_id=ti.account_id OR r.account_id IS NULL) JOIN user_settings s ON s.user_id=ti.user_id WHERE ti.id=$1 AND ti.expires_at > now() ORDER BY (r.account_id IS NOT NULL) DESC LIMIT 1 FOR UPDATE OF ti"#)
        .bind(id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    if locked.user_id != input.user_id {
        return Err(AppError::Forbidden);
    }
    if locked.status != "PENDING_CONFIRMATION" {
        return Err(AppError::Validation(
            "trade intent is not pending confirmation".into(),
        ));
    }
    let key_hash = hex::encode(Sha256::digest(input.idempotency_key.as_bytes()));
    let inserted=sqlx::query("INSERT INTO idempotency_keys(user_id,key_hash,trade_intent_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(input.user_id).bind(key_hash).bind(id).execute(&mut *tx).await?.rows_affected();
    if inserted == 0 {
        return Err(AppError::Validation("duplicate confirmation".into()));
    }
    let quote = state.mt5.quote(locked.account_id, &locked.symbol).await?;
    let side = if locked.side == "BUY" {
        crate::models::Side::Buy
    } else {
        crate::models::Side::Sell
    };
    let price = match side {
        crate::models::Side::Buy => quote.ask,
        crate::models::Side::Sell => quote.bid,
    };
    let (open_positions,trades_today):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM positions_snapshot WHERE user_id=$1 AND account_id=$2 AND status='OPEN'),(SELECT count(*) FROM orders WHERE user_id=$1 AND created_at>=CURRENT_DATE)").bind(input.user_id).bind(locked.account_id).fetch_one(&mut *tx).await?;
    risk::validate(
        &state.config,
        &RiskInput {
            account_type: &locked.account_type,
            permission_mode: &locked.permission_mode,
            account_active: locked.is_active,
            account_verified: locked.is_verified,
            user_trading_enabled: locked.trading_enabled,
            user_live_enabled: locked.live_trading_enabled,
            volume: locked.volume,
            max_lot: locked.max_lot_size,
            side: &side,
            price,
            stop_loss: locked.stop_loss,
            take_profit: locked.take_profit,
            quote_time: quote.timestamp,
            open_positions,
            max_open_positions: locked.max_open_positions,
            trades_today,
            max_trades_per_day: locked.max_trades_per_day,
        },
    )?;
    let point = Decimal::new(1, 5);
    let movement = (price - locked.entry_price).abs() / point;
    if movement > Decimal::from(locked.max_slippage_points) {
        return Err(AppError::Validation(
            "price moved beyond your allowed slippage; no order was placed".into(),
        ));
    }
    sqlx::query("UPDATE trade_intents SET status='EXECUTING',updated_at=now() WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let result = state
        .mt5
        .order(
            id,
            locked.account_id,
            &locked.symbol,
            &side,
            locked.volume,
            locked.stop_loss,
            locked.take_profit,
            locked.max_slippage_points,
        )
        .await;
    let mut tx = state.db.begin().await?;
    match result {
        Ok(result) => {
            sqlx::query("UPDATE trade_intents SET status='EXECUTED',updated_at=now() WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO orders(user_id,account_id,trade_intent_id,mt5_ticket,symbol,side,volume,requested_price,executed_price,stop_loss,take_profit,status,executed_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'EXECUTED',now())")
                .bind(input.user_id).bind(locked.account_id).bind(id).bind(result.ticket).bind(&locked.symbol).bind(&locked.side).bind(locked.volume).bind(price).bind(result.executed_price).bind(locked.stop_loss).bind(locked.take_profit).execute(&mut *tx).await?;
            audit(
                &mut tx,
                input.user_id,
                locked.account_id,
                "TRADE_EXECUTED",
                id,
            )
            .await?;
            tx.commit().await?;
            Ok(result)
        }
        Err(e) => {
            sqlx::query("UPDATE trade_intents SET status='FAILED',updated_at=now() WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            audit(
                &mut tx,
                input.user_id,
                locked.account_id,
                "TRADE_FAILED",
                id,
            )
            .await?;
            tx.commit().await?;
            Err(e)
        }
    }
}
pub async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: Uuid,
    account: Uuid,
    event: &str,
    entity: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO audit_logs(user_id,account_id,event_type,entity_type,entity_id,metadata) VALUES($1,$2,$3,'trade_intent',$4,'{}')").bind(user).bind(account).bind(event).bind(entity).execute(&mut **tx).await?;
    Ok(())
}
