use std::sync::Arc;

use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use axum::{
    extract::{Query, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

pub async fn require_admin(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let authenticated = state
        .config
        .admin_password_hash
        .as_deref()
        .and_then(|hash| credentials(request.headers()).map(|credentials| (hash, credentials)))
        .is_some_and(|(hash, (username, password))| {
            username == state.config.admin_username
                && PasswordHash::new(hash).is_ok_and(|parsed| {
                    Argon2::default()
                        .verify_password(password.as_bytes(), &parsed)
                        .is_ok()
                })
        });

    if authenticated {
        return next.run(request).await;
    }

    let status = if state.config.admin_password_hash.is_some() {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let message = if status == StatusCode::UNAUTHORIZED {
        "Administrator authentication required"
    } else {
        "Admin dashboard is not configured"
    };
    let mut response = (status, message).into_response();
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"AI Forex Admin\", charset=\"UTF-8\""),
    );
    response
}

fn credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let encoded = headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Basic ")?;
    let decoded = String::from_utf8(STANDARD.decode(encoded).ok()?).ok()?;
    let (username, password) = decoded.split_once(':')?;
    Some((username.to_owned(), password.to_owned()))
}

pub async fn dashboard() -> impl IntoResponse {
    let headers = [
        (header::CACHE_CONTROL, "no-store"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'",
        ),
    ];
    (headers, Html(include_str!("../admin/index.html")))
}

#[derive(sqlx::FromRow)]
struct OverviewRow {
    total_users: i64,
    active_users: i64,
    total_accounts: i64,
    verified_accounts: i64,
    analyses_today: i64,
    trade_intents_today: i64,
    executed_today: i64,
    failed_today: i64,
    active_subscriptions: i64,
    trial_subscriptions: i64,
    past_due_subscriptions: i64,
    pending_subscriptions: i64,
    paid_revenue_month_idr: i64,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct AuditItem {
    id: Uuid,
    event_type: String,
    entity_type: String,
    user_label: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct OverviewResponse {
    total_users: i64,
    active_users: i64,
    total_accounts: i64,
    verified_accounts: i64,
    analyses_today: i64,
    trade_intents_today: i64,
    executed_today: i64,
    failed_today: i64,
    active_subscriptions: i64,
    trial_subscriptions: i64,
    past_due_subscriptions: i64,
    pending_subscriptions: i64,
    paid_revenue_month_idr: i64,
    database_status: &'static str,
    mt5_bridge_status: &'static str,
    live_trading_enabled: bool,
    recent_audit: Vec<AuditItem>,
    generated_at: DateTime<Utc>,
}

pub async fn overview(
    State(state): State<Arc<AppState>>,
) -> Result<Json<OverviewResponse>, AppError> {
    let row: OverviewRow = sqlx::query_as(
        r#"
        SELECT
          (SELECT count(*) FROM users) AS total_users,
          (SELECT count(*) FROM users WHERE status = 'ACTIVE') AS active_users,
          (SELECT count(*) FROM mt5_accounts) AS total_accounts,
          (SELECT count(*) FROM mt5_accounts WHERE is_verified) AS verified_accounts,
          (SELECT count(*) FROM ai_analyses WHERE created_at >= CURRENT_DATE) AS analyses_today,
          (SELECT count(*) FROM trade_intents WHERE created_at >= CURRENT_DATE) AS trade_intents_today,
          (SELECT count(*) FROM orders WHERE status = 'EXECUTED' AND created_at >= CURRENT_DATE) AS executed_today,
          (SELECT count(*) FROM orders WHERE status = 'FAILED' AND created_at >= CURRENT_DATE) AS failed_today,
          (SELECT count(*) FROM subscriptions WHERE status IN ('ACTIVE', 'CANCEL_AT_PERIOD_END') AND current_period_end > now()) AS active_subscriptions,
          (SELECT count(*) FROM subscriptions WHERE status = 'TRIALING' AND current_period_end > now()) AS trial_subscriptions,
          (SELECT count(*) FROM subscriptions WHERE status = 'PAST_DUE') AS past_due_subscriptions,
          (SELECT count(*) FROM subscriptions WHERE status = 'PENDING_PAYMENT') AS pending_subscriptions,
          (SELECT COALESCE(sum(total_minor), 0)::bigint FROM invoices WHERE status = 'PAID' AND paid_at >= date_trunc('month', now())) AS paid_revenue_month_idr
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    let recent_audit = sqlx::query_as::<_, AuditItem>(
        r#"
        SELECT a.id, a.event_type, a.entity_type,
               COALESCE(NULLIF(u.telegram_username, ''), 'Telegram ' || u.telegram_user_id::text) AS user_label,
               a.created_at
        FROM audit_logs a
        JOIN users u ON u.id = a.user_id
        ORDER BY a.created_at DESC
        LIMIT 8
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let mt5_bridge_ok = state.mt5.health().await;
    Ok(Json(OverviewResponse {
        total_users: row.total_users,
        active_users: row.active_users,
        total_accounts: row.total_accounts,
        verified_accounts: row.verified_accounts,
        analyses_today: row.analyses_today,
        trade_intents_today: row.trade_intents_today,
        executed_today: row.executed_today,
        failed_today: row.failed_today,
        active_subscriptions: row.active_subscriptions,
        trial_subscriptions: row.trial_subscriptions,
        past_due_subscriptions: row.past_due_subscriptions,
        pending_subscriptions: row.pending_subscriptions,
        paid_revenue_month_idr: row.paid_revenue_month_idr,
        database_status: "ok",
        mt5_bridge_status: if mt5_bridge_ok { "ok" } else { "degraded" },
        live_trading_enabled: state.config.live_trading_enabled,
        recent_audit,
        generated_at: Utc::now(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    #[serde(default)]
    search: String,
    #[serde(default)]
    subscription: String,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(sqlx::FromRow, Serialize)]
pub struct AdminUserRow {
    id: Uuid,
    telegram_user_id: i64,
    telegram_username: Option<String>,
    first_name: Option<String>,
    user_status: String,
    account_count: i64,
    verified_account_count: i64,
    trading_enabled: bool,
    plan_name: Option<String>,
    subscription_status: String,
    current_period_end: Option<DateTime<Utc>>,
    latest_invoice_status: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UsersResponse {
    items: Vec<AdminUserRow>,
    limit: i64,
    offset: i64,
    returned: usize,
}

pub async fn users(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UserQuery>,
) -> Result<Json<UsersResponse>, AppError> {
    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);
    let search = query.search.trim();
    let subscription = query.subscription.trim().to_uppercase();
    let items = sqlx::query_as::<_, AdminUserRow>(
        r#"
        SELECT
          u.id, u.telegram_user_id, u.telegram_username, u.first_name,
          u.status AS user_status,
          (SELECT count(*) FROM mt5_accounts a WHERE a.user_id = u.id) AS account_count,
          (SELECT count(*) FROM mt5_accounts a WHERE a.user_id = u.id AND a.is_verified) AS verified_account_count,
          COALESCE((SELECT bool_or(r.trading_enabled) FROM risk_profiles r WHERE r.user_id = u.id), false) AS trading_enabled,
          current_subscription.plan_name,
          COALESCE(current_subscription.status, 'UNPAID') AS subscription_status,
          current_subscription.current_period_end,
          latest_invoice.status AS latest_invoice_status,
          u.created_at
        FROM users u
        LEFT JOIN LATERAL (
          SELECT p.name AS plan_name, s.status::text AS status, s.current_period_end, s.id
          FROM subscriptions s
          JOIN pricing_plans p ON p.id = s.plan_id
          WHERE s.user_id = u.id
          ORDER BY s.created_at DESC
          LIMIT 1
        ) current_subscription ON true
        LEFT JOIN LATERAL (
          SELECT i.status::text AS status
          FROM invoices i
          WHERE i.subscription_id = current_subscription.id
          ORDER BY i.created_at DESC
          LIMIT 1
        ) latest_invoice ON true
        WHERE ($1 = '' OR COALESCE(u.telegram_username, '') ILIKE '%' || $1 || '%'
                        OR COALESCE(u.first_name, '') ILIKE '%' || $1 || '%'
                        OR u.telegram_user_id::text LIKE '%' || $1 || '%')
          AND ($2 = '' OR COALESCE(current_subscription.status, 'UNPAID') = $2)
        ORDER BY u.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(search)
    .bind(subscription)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;
    let returned = items.len();
    Ok(Json(UsersResponse {
        items,
        limit,
        offset,
        returned,
    }))
}
