use crate::{
    error::AppError,
    models::{AnalysisRequest, ConfirmIntent, CreateIntent},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AddMt5Account {
    broker: String,
    login: String,
    password: String,
    server: String,
    account_type: String,
}

pub async fn add_mt5_account(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<AddMt5Account>,
) -> Result<Json<Value>, AppError> {
    let token = s.config.telegram_bot_token.as_deref().ok_or(AppError::Forbidden)?;
    let init_data = headers.get("x-telegram-init-data").and_then(|v| v.to_str().ok()).ok_or(AppError::Forbidden)?;
    let telegram_id = verified_telegram_id(token, init_data)?;
    let broker = input.broker.trim();
    let server = input.server.trim();
    let login = input.login.trim();
    if broker.is_empty() || broker.len() > 120 || server.is_empty() || server.len() > 120
        || login.is_empty() || login.len() > 30 || !login.bytes().all(|b| b.is_ascii_digit())
        || input.password.is_empty() || input.password.len() > 256
        || !matches!(input.account_type.as_str(), "DEMO" | "LIVE")
    {
        return Err(AppError::Validation("invalid MT5 account details".into()));
    }
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE telegram_user_id=$1 AND status='ACTIVE'")
        .bind(telegram_id).fetch_optional(&s.db).await?.ok_or(AppError::Forbidden)?;
    let (ciphertext, nonce) = crate::crypto::encrypt(&s.config.encryption_key, input.password.as_bytes())
        .map_err(AppError::Internal)?;
    let id = Uuid::new_v4();
    let mut tx = s.db.begin().await?;
    let inserted = sqlx::query("INSERT INTO mt5_accounts(id,user_id,broker_name,server,login,encrypted_password,password_nonce,account_type,permission_mode,is_verified) VALUES($1,$2,$3,$4,$5,$6,$7,$8::account_type,'READ_ONLY',false) ON CONFLICT(user_id,server,login) DO NOTHING")
        .bind(id).bind(user_id).bind(broker).bind(server).bind(login).bind(ciphertext).bind(nonce)
        .bind(&input.account_type).execute(&mut *tx).await?;
    if inserted.rows_affected() == 0 {
        return Err(AppError::Validation("account already registered".into()));
    }
    sqlx::query("INSERT INTO audit_logs(user_id,account_id,event_type,entity_type,entity_id,metadata) VALUES($1,$2,'MT5_ACCOUNT_ADDED','mt5_account',$2,'{}')")
        .bind(user_id).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":id,"broker":broker,"server":server,"account_type":input.account_type,"login_last4":login.chars().rev().take(4).collect::<String>().chars().rev().collect::<String>(),"is_verified":false,"permission_mode":"READ_ONLY"})))
}

pub async fn verify_mt5_account(
    State(s): State<Arc<AppState>>,
    Path(account_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let token = s.config.telegram_bot_token.as_deref().ok_or(AppError::Forbidden)?;
    let init_data = headers.get("x-telegram-init-data").and_then(|v| v.to_str().ok()).ok_or(AppError::Forbidden)?;
    let telegram_id = verified_telegram_id(token, init_data)?;
    let row: (Uuid, String, String, String, String) = sqlx::query_as(
        "SELECT a.user_id,a.login,a.server,a.encrypted_password,a.password_nonce FROM mt5_accounts a JOIN users u ON u.id=a.user_id WHERE a.id=$1 AND a.is_active AND u.telegram_user_id=$2 AND u.status='ACTIVE' AND a.server<>'MOCK'"
    ).bind(account_id).bind(telegram_id).fetch_optional(&s.db).await?.ok_or(AppError::Forbidden)?;
    let login: i64 = row.1.parse().map_err(|_| AppError::Validation("invalid MT5 login".into()))?;
    let password = crate::crypto::decrypt(&s.config.encryption_key, &row.3, &row.4)
        .map_err(AppError::Internal)?;
    let password = String::from_utf8(password).map_err(|_| AppError::Unavailable)?;
    let result = s.mt5.verify(account_id, login, &password, &row.2).await?;
    let actual_type = result.get("account_type").and_then(Value::as_str).ok_or(AppError::Unavailable)?;
    let actual_login = result.get("login").and_then(Value::as_i64).ok_or(AppError::Unavailable)?;
    let actual_server = result.get("server").and_then(Value::as_str).ok_or(AppError::Unavailable)?;
    if actual_login != login || actual_server != row.2 || !matches!(actual_type, "DEMO" | "LIVE") {
        return Err(AppError::Validation("broker account identity mismatch".into()));
    }
    let broker = result.get("broker").and_then(Value::as_str).unwrap_or("MT5 broker");
    let mut tx = s.db.begin().await?;
    let changed = sqlx::query("UPDATE mt5_accounts SET broker_name=$1,account_type=$2::account_type,is_verified=true,permission_mode='READ_ONLY',updated_at=now() WHERE id=$3 AND user_id=$4 AND is_active")
        .bind(broker).bind(actual_type).bind(account_id).bind(row.0).execute(&mut *tx).await?;
    if changed.rows_affected() == 0 { return Err(AppError::Forbidden); }
    sqlx::query("INSERT INTO audit_logs(user_id,account_id,event_type,entity_type,entity_id,metadata) VALUES($1,$2,'MT5_ACCOUNT_VERIFIED','mt5_account',$2,'{}')")
        .bind(row.0).bind(account_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"id":account_id,"broker":broker,"server":actual_server,"account_type":actual_type,"is_verified":true,"permission_mode":"READ_ONLY"})))
}

fn verified_telegram_id(token: &str, init_data: &str) -> Result<i64, AppError> {
    if init_data.len() > 8192 { return Err(AppError::Forbidden); }
    let mut fields: Vec<(String, String)> = url::form_urlencoded::parse(init_data.as_bytes())
        .map(|(k,v)| (k.into_owned(),v.into_owned())).collect();
    let hash_pos = fields.iter().position(|(k,_)| k == "hash").ok_or(AppError::Forbidden)?;
    let hash = fields.remove(hash_pos).1;
    if fields.iter().any(|(k,_)| k == "hash") { return Err(AppError::Forbidden); }
    fields.sort_by(|a,b| a.0.cmp(&b.0));
    if fields.windows(2).any(|pair| pair[0].0 == pair[1].0) { return Err(AppError::Forbidden); }
    let data = fields.iter().map(|(k,v)| format!("{k}={v}")).collect::<Vec<_>>().join("\n");
    let mut secret = Hmac::<Sha256>::new_from_slice(b"WebAppData").map_err(|_| AppError::Forbidden)?;
    secret.update(token.as_bytes());
    let mut mac = Hmac::<Sha256>::new_from_slice(&secret.finalize().into_bytes()).map_err(|_| AppError::Forbidden)?;
    mac.update(data.as_bytes());
    let signature = hex::decode(hash).map_err(|_| AppError::Forbidden)?;
    mac.verify_slice(&signature).map_err(|_| AppError::Forbidden)?;
    let auth_date: i64 = fields.iter().find(|(k,_)| k == "auth_date").and_then(|(_,v)| v.parse().ok()).ok_or(AppError::Forbidden)?;
    if (Utc::now().timestamp() - auth_date).abs() > 3600 { return Err(AppError::Forbidden); }
    let user: Value = serde_json::from_str(fields.iter().find(|(k,_)| k == "user").map(|(_,v)| v.as_str()).ok_or(AppError::Forbidden)?).map_err(|_| AppError::Forbidden)?;
    user.get("id").and_then(Value::as_i64).ok_or(AppError::Forbidden)
}

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
