use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use totp_rs::Secret;
use uuid::Uuid;

use crate::{
    admin_auth::{self, AdminIdentity, AdminRole},
    crypto,
    error::AppError,
    state::AppState,
};

fn validate_reason(reason: &str) -> Result<(), AppError> {
    let trimmed = reason.trim();
    if trimmed.len() < 5 || trimmed.len() > 500 {
        return Err(AppError::Validation(
            "reason must be between 5 and 500 characters".into(),
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct ReasonBody {
    reason: String,
}

#[derive(Deserialize)]
pub struct CreditBody {
    amount_minor: i64,
    reason: String,
}

// ---- User status mutations (OWNER, OPS_ADMIN, SUPPORT_AGENT) ----

pub async fn suspend_user(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<ReasonBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner, AdminRole::OpsAdmin, AdminRole::SupportAgent])?;
    validate_reason(&body.reason)?;
    let changed = sqlx::query("UPDATE users SET status='SUSPENDED', updated_at=now() WHERE id=$1 AND status <> 'SUSPENDED'")
        .bind(user_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "USER_SUSPENDED",
        "user",
        Some(user_id),
        Some(&body.reason),
    )
    .await?;
    Ok(Json(json!({"user_id": user_id, "status": "SUSPENDED"})))
}

pub async fn reactivate_user(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<ReasonBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner, AdminRole::OpsAdmin, AdminRole::SupportAgent])?;
    validate_reason(&body.reason)?;
    let changed = sqlx::query("UPDATE users SET status='ACTIVE', updated_at=now() WHERE id=$1 AND status <> 'ACTIVE'")
        .bind(user_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "USER_REACTIVATED",
        "user",
        Some(user_id),
        Some(&body.reason),
    )
    .await?;
    Ok(Json(json!({"user_id": user_id, "status": "ACTIVE"})))
}

// ---- Billing mutations (OWNER, OPS_ADMIN) — local/audit-trailed only; no
// payment-provider integration exists yet, see docs/admin-dashboard.md ----

pub async fn cancel_subscription(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(subscription_id): Path<Uuid>,
    Json(body): Json<ReasonBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner, AdminRole::OpsAdmin])?;
    validate_reason(&body.reason)?;
    let changed = sqlx::query(
        "UPDATE subscriptions SET status='CANCEL_AT_PERIOD_END', cancel_at_period_end=true, cancelled_at=now(), updated_at=now() \
         WHERE id=$1 AND status IN ('TRIALING','PENDING_PAYMENT','ACTIVE','PAST_DUE')",
    )
    .bind(subscription_id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "SUBSCRIPTION_CANCEL_SCHEDULED",
        "subscription",
        Some(subscription_id),
        Some(&body.reason),
    )
    .await?;
    Ok(Json(json!({"subscription_id": subscription_id, "status": "CANCEL_AT_PERIOD_END"})))
}

pub async fn grant_credit(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<CreditBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner, AdminRole::OpsAdmin])?;
    validate_reason(&body.reason)?;
    if body.amount_minor <= 0 {
        return Err(AppError::Validation("amount_minor must be positive".into()));
    }
    let user_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    if !user_exists {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "INSERT INTO account_credits(user_id,amount_minor,reason,granted_by) VALUES ($1,$2,$3,$4)",
    )
    .bind(user_id)
    .bind(body.amount_minor)
    .bind(&body.reason)
    .bind(identity.admin_user_id)
    .execute(&state.db)
    .await?;
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "PROMOTIONAL_CREDIT_GRANTED",
        "user",
        Some(user_id),
        Some(&body.reason),
    )
    .await?;
    Ok(Json(json!({"user_id": user_id, "amount_minor": body.amount_minor})))
}

pub async fn refund_invoice(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(invoice_id): Path<Uuid>,
    Json(body): Json<ReasonBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner, AdminRole::OpsAdmin])?;
    validate_reason(&body.reason)?;
    let changed = sqlx::query("UPDATE invoices SET status='REFUND_PENDING', updated_at=now() WHERE id=$1 AND status='PAID'")
        .bind(invoice_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "INVOICE_REFUND_INITIATED",
        "invoice",
        Some(invoice_id),
        Some(&body.reason),
    )
    .await?;
    Ok(Json(json!({"invoice_id": invoice_id, "status": "REFUND_PENDING"})))
}

pub async fn confirm_refund(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(invoice_id): Path<Uuid>,
    Json(body): Json<ReasonBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner, AdminRole::OpsAdmin])?;
    validate_reason(&body.reason)?;
    let changed = sqlx::query("UPDATE invoices SET status='REFUNDED', updated_at=now() WHERE id=$1 AND status='REFUND_PENDING'")
        .bind(invoice_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "INVOICE_REFUND_CONFIRMED",
        "invoice",
        Some(invoice_id),
        Some(&body.reason),
    )
    .await?;
    Ok(Json(json!({"invoice_id": invoice_id, "status": "REFUNDED"})))
}

// ---- Admin audit log (OWNER, SECURITY_OPERATOR, AUDITOR) ----

#[derive(sqlx::FromRow, Serialize)]
pub struct AdminAuditItem {
    id: Uuid,
    admin_username: Option<String>,
    action: String,
    target_type: String,
    target_id: Option<Uuid>,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}

pub async fn list_admin_audit(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner, AdminRole::SecurityOperator, AdminRole::Auditor])?;
    let items = sqlx::query_as::<_, AdminAuditItem>(
        r#"
        SELECT a.id, u.username AS admin_username, a.action, a.target_type, a.target_id, a.reason, a.created_at
        FROM admin_audit_logs a
        LEFT JOIN admin_users u ON u.id = a.admin_user_id
        ORDER BY a.created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({"items": items})))
}

// ---- Admin user management (OWNER only) ----

#[derive(Deserialize)]
pub struct CreateAdminBody {
    username: String,
    temporary_password: String,
    role: AdminRole,
}

#[derive(Deserialize)]
pub struct RoleBody {
    role: AdminRole,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct AdminUserItem {
    id: Uuid,
    username: String,
    role: String,
    is_active: bool,
    created_at: DateTime<Utc>,
}

pub async fn list_admins(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner])?;
    let items = sqlx::query_as::<_, AdminUserItem>(
        "SELECT id, username, role::text AS role, is_active, created_at FROM admin_users ORDER BY created_at",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({"items": items})))
}

pub async fn create_admin_user(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Json(body): Json<CreateAdminBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner])?;
    let username = body.username.trim();
    if username.is_empty() || username.len() > 64 {
        return Err(AppError::Validation("username must be 1-64 characters".into()));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_users WHERE username=$1)")
        .bind(username)
        .fetch_one(&state.db)
        .await?;
    if exists {
        return Err(AppError::Validation("username already exists".into()));
    }
    let password_hash = admin_auth::hash_password(&body.temporary_password)
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let secret = Secret::generate();
    let (totp_secret_encrypted, totp_secret_nonce) =
        crypto::encrypt(&state.config.encryption_key, secret.as_bytes()).map_err(AppError::Internal)?;
    let totp = admin_auth::build_totp(secret.as_bytes().to_vec(), username).map_err(AppError::Internal)?;
    let provisioning_url = totp
        .to_url()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO admin_users(username,password_hash,role,totp_secret_encrypted,totp_secret_nonce,created_by) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
    )
    .bind(username)
    .bind(&password_hash)
    .bind(body.role.as_str())
    .bind(&totp_secret_encrypted)
    .bind(&totp_secret_nonce)
    .bind(identity.admin_user_id)
    .fetch_one(&state.db)
    .await?;

    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "ADMIN_USER_CREATED",
        "admin_user",
        Some(id),
        Some(&format!("role={}", body.role.as_str())),
    )
    .await?;

    Ok(Json(json!({
        "id": id,
        "username": username,
        "role": body.role.as_str(),
        "totp_secret_base32": secret.to_base32(),
        "totp_provisioning_url": provisioning_url,
        "note": "Save the TOTP secret now — it is shown only once and cannot be retrieved again.",
    })))
}

pub async fn change_admin_role(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(admin_id): Path<Uuid>,
    Json(body): Json<RoleBody>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner])?;
    if admin_id == identity.admin_user_id {
        return Err(AppError::Validation("cannot change your own role".into()));
    }
    let changed = sqlx::query("UPDATE admin_users SET role=$2, updated_at=now() WHERE id=$1")
        .bind(admin_id)
        .bind(body.role.as_str())
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "ADMIN_USER_ROLE_CHANGED",
        "admin_user",
        Some(admin_id),
        Some(&format!("new_role={}", body.role.as_str())),
    )
    .await?;
    Ok(Json(json!({"id": admin_id, "role": body.role.as_str()})))
}

pub async fn deactivate_admin(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(admin_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner])?;
    if admin_id == identity.admin_user_id {
        return Err(AppError::Validation("cannot deactivate your own account".into()));
    }
    let changed = sqlx::query("UPDATE admin_users SET is_active=false, updated_at=now() WHERE id=$1")
        .bind(admin_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query("UPDATE admin_sessions SET revoked_at=now() WHERE admin_user_id=$1 AND revoked_at IS NULL")
        .bind(admin_id)
        .execute(&state.db)
        .await?;
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "ADMIN_USER_DEACTIVATED",
        "admin_user",
        Some(admin_id),
        None,
    )
    .await?;
    Ok(Json(json!({"id": admin_id, "is_active": false})))
}

pub async fn reactivate_admin(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
    Path(admin_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    identity.require_role(&[AdminRole::Owner])?;
    let changed = sqlx::query("UPDATE admin_users SET is_active=true, updated_at=now() WHERE id=$1")
        .bind(admin_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    admin_auth::record_admin_audit(
        &state.db,
        identity.admin_user_id,
        "ADMIN_USER_REACTIVATED",
        "admin_user",
        Some(admin_id),
        None,
    )
    .await?;
    Ok(Json(json!({"id": admin_id, "is_active": true})))
}
