use crate::{
    error::AppError,
    models::{AnalysisRequest, ConfirmIntent, CreateIntent},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub async fn health(State(s): State<Arc<AppState>>) -> Json<Value> {
    let db = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&s.db)
        .await
        .is_ok();
    let mt5 = s.mt5.health().await;
    Json(
        json!({"status":if db{"ok"}else{"degraded"},"database":if db{"ok"}else{"error"},"mt5_bridge":if mt5{"ok"}else{"error"}}),
    )
}
pub async fn analyze(
    State(s): State<Arc<AppState>>,
    Json(i): Json<AnalysisRequest>,
) -> Result<Json<Value>, AppError> {
    validate_symbol(&i.symbol)?;
    validate_tf(&i.timeframe)?;
    owned(&s, i.user_id, i.account_id).await?;
    let q = s.mt5.quote(i.account_id, &i.symbol).await?;
    if (Utc::now() - q.timestamp).num_seconds() > s.config.market_data_max_age_seconds {
        return Err(AppError::Validation("market data is stale".into()));
    }
    let a = s.ai.analyze(&q, &i.timeframe).await?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ai_analyses(id,user_id,account_id,symbol,timeframe,market_data,analysis) VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(id).bind(i.user_id).bind(i.account_id).bind(&i.symbol).bind(&i.timeframe)
        .bind(serde_json::to_value(&q).unwrap_or_default()).bind(serde_json::to_value(&a).unwrap_or_default()).execute(&s.db).await?;
    Ok(Json(
        json!({"id":id,"analysis":a,"disclaimer":"AI-assisted analysis may be incorrect and does not guarantee profit."}),
    ))
}
pub async fn create_intent(
    State(s): State<Arc<AppState>>,
    Json(i): Json<CreateIntent>,
) -> Result<Json<Value>, AppError> {
    validate_symbol(&i.symbol)?;
    owned(&s, i.user_id, i.account_id).await?;
    if i.volume <= rust_decimal::Decimal::ZERO {
        return Err(AppError::Validation("volume must be positive".into()));
    }
    let id = Uuid::new_v4();
    let q = s.mt5.quote(i.account_id, &i.symbol).await?;
    let entry = match i.side {
        crate::models::Side::Buy => q.ask,
        crate::models::Side::Sell => q.bid,
    };
    let side = match i.side {
        crate::models::Side::Buy => "BUY",
        crate::models::Side::Sell => "SELL",
    };
    sqlx::query("INSERT INTO trade_intents(id,user_id,account_id,analysis_id,symbol,side,volume,entry_price,stop_loss,take_profit,status,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'PENDING_CONFIRMATION',now()+interval '5 minutes')")
        .bind(id).bind(i.user_id).bind(i.account_id).bind(i.analysis_id).bind(&i.symbol).bind(side).bind(i.volume).bind(entry).bind(i.stop_loss).bind(i.take_profit).execute(&s.db).await?;
    Ok(Json(
        json!({"trade_intent_id":id,"status":"PENDING_CONFIRMATION","message":"No order has been placed. Explicit confirmation is required."}),
    ))
}
pub async fn confirm_intent(
    State(s): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(i): Json<ConfirmIntent>,
) -> Result<Json<Value>, AppError> {
    Ok(Json(
        json!({"order":crate::trading::confirm(&s,id,i).await?,"verification":"Verify the ticket directly in MetaTrader 5."}),
    ))
}
pub async fn stop_trading(
    State(s): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let user = body
        .get("user_id")
        .and_then(Value::as_str)
        .and_then(|x| Uuid::parse_str(x).ok())
        .ok_or_else(|| AppError::Validation("user_id required".into()))?;
    sqlx::query("UPDATE risk_profiles SET trading_enabled=false,updated_at=now() WHERE user_id=$1")
        .bind(user)
        .execute(&s.db)
        .await?;
    sqlx::query("INSERT INTO audit_logs(user_id,event_type,entity_type,entity_id,metadata) VALUES($1,'KILL_SWITCH_ACTIVATED','user',$1,'{}')").bind(user).execute(&s.db).await?;
    Ok(Json(
        json!({"trading_enabled":false,"closing_positions_allowed":true}),
    ))
}
async fn owned(s: &AppState, user: Uuid, account: Uuid) -> Result<(), AppError> {
    let ok = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM mt5_accounts WHERE id=$1 AND user_id=$2)",
    )
    .bind(account)
    .bind(user)
    .fetch_one(&s.db)
    .await?;
    if !ok {
        return Err(AppError::Forbidden);
    }
    Ok(())
}
fn validate_symbol(v: &str) -> Result<(), AppError> {
    if v.len() > 12
        || v.is_empty()
        || !v
            .chars()
            .all(|x| x.is_ascii_uppercase() || x.is_ascii_digit() || x == '.')
    {
        return Err(AppError::Validation("invalid symbol".into()));
    }
    Ok(())
}
fn validate_tf(v: &str) -> Result<(), AppError> {
    if !matches!(v, "M1" | "M5" | "M15" | "M30" | "H1" | "H4" | "D1") {
        return Err(AppError::Validation("unsupported timeframe".into()));
    }
    Ok(())
}
