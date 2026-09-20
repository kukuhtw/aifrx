use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng as ArgonOsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::{DateTime, Duration, Utc};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use totp_rs::{Algorithm, Builder, Totp};
use uuid::Uuid;

use crate::{crypto, error::AppError, state::AppState};

const SESSION_TTL_HOURS: i64 = 8;
const SESSION_COOKIE: &str = "admin_session";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdminRole {
    Owner,
    SecurityOperator,
    OpsAdmin,
    SupportAgent,
    Auditor,
}

impl AdminRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "OWNER",
            Self::SecurityOperator => "SECURITY_OPERATOR",
            Self::OpsAdmin => "OPS_ADMIN",
            Self::SupportAgent => "SUPPORT_AGENT",
            Self::Auditor => "AUDITOR",
        }
    }
}

impl std::str::FromStr for AdminRole {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(match s {
            "OWNER" => Self::Owner,
            "SECURITY_OPERATOR" => Self::SecurityOperator,
            "OPS_ADMIN" => Self::OpsAdmin,
            "SUPPORT_AGENT" => Self::SupportAgent,
            "AUDITOR" => Self::Auditor,
            other => anyhow::bail!("unknown admin role: {other}"),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AdminIdentity {
    pub admin_user_id: Uuid,
    pub username: String,
    pub role: AdminRole,
    pub session_id: Uuid,
}

impl AdminIdentity {
    pub fn require_role(&self, allowed: &[AdminRole]) -> Result<(), AppError> {
        if allowed.contains(&self.role) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

/// Hashes a password with Argon2, used both by the `create-admin` CLI bootstrap
/// tool and the dashboard's admin-creation endpoint.
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    anyhow::ensure!(
        password.len() >= 12,
        "password must be at least 12 characters"
    );
    let salt = SaltString::generate(&mut ArgonOsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))?;
    Ok(hash.to_string())
}

/// Builds a TOTP validator/generator from raw secret bytes.
pub fn build_totp(secret_bytes: Vec<u8>, account_name: &str) -> anyhow::Result<Totp> {
    Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_secret(secret_bytes)
        .with_issuer(Some("AI Forex Admin"))
        .with_account_name(account_name)
        .build()
        .map_err(|error| anyhow::anyhow!("failed to build TOTP: {error}"))
}

pub async fn record_admin_audit(
    db: &PgPool,
    admin_user_id: Uuid,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    reason: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO admin_audit_logs(admin_user_id,action,target_type,target_id,reason) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(admin_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason)
    .execute(db)
    .await?;
    Ok(())
}

async fn create_session(db: &PgPool, admin_user_id: Uuid) -> anyhow::Result<(String, String, DateTime<Utc>)> {
    let mut token_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut token_bytes);
    let raw_token = hex::encode(token_bytes);
    let token_hash = hex::encode(Sha256::digest(raw_token.as_bytes()));

    let mut csrf_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut csrf_bytes);
    let csrf_token = hex::encode(csrf_bytes);

    let expires_at = Utc::now() + Duration::hours(SESSION_TTL_HOURS);
    sqlx::query(
        "INSERT INTO admin_sessions(admin_user_id,token_hash,csrf_token,expires_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(admin_user_id)
    .bind(&token_hash)
    .bind(&csrf_token)
    .bind(expires_at)
    .execute(db)
    .await?;
    Ok((raw_token, csrf_token, expires_at))
}

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').find_map(|part| {
        part.trim()
            .strip_prefix(&format!("{SESSION_COOKIE}="))
            .map(str::to_owned)
    })
}

fn session_cookie(raw_token: &str, max_age_seconds: i64) -> String {
    format!("{SESSION_COOKIE}={raw_token}; HttpOnly; Secure; SameSite=Strict; Path=/admin; Max-Age={max_age_seconds}")
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "administrator authentication required"})),
    )
        .into_response()
}

fn forbidden_csrf() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({"error": "csrf token missing or invalid"})),
    )
        .into_response()
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_id: Uuid,
    csrf_token: String,
    admin_user_id: Uuid,
    username: String,
    role: String,
}

pub async fn require_admin_session(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(raw_token) = extract_session_cookie(request.headers()) else {
        return unauthorized();
    };
    let token_hash = hex::encode(Sha256::digest(raw_token.as_bytes()));

    let row = sqlx::query_as::<_, SessionRow>(
        r#"
        SELECT s.id AS session_id, s.csrf_token, s.admin_user_id, u.username, u.role::text AS role
        FROM admin_sessions s
        JOIN admin_users u ON u.id = s.admin_user_id
        WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now() AND u.is_active
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await;

    let Ok(Some(row)) = row else {
        return unauthorized();
    };

    if request.method() != Method::GET {
        let header_token = request
            .headers()
            .get("X-Admin-Csrf-Token")
            .and_then(|v| v.to_str().ok());
        if header_token != Some(row.csrf_token.as_str()) {
            return forbidden_csrf();
        }
    }

    let Ok(role) = row.role.parse::<AdminRole>() else {
        return unauthorized();
    };

    request.extensions_mut().insert(AdminIdentity {
        admin_user_id: row.admin_user_id,
        username: row.username,
        role,
        session_id: row.session_id,
    });
    next.run(request).await
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
    totp_code: String,
}

#[derive(sqlx::FromRow)]
struct AdminAuthRow {
    id: Uuid,
    password_hash: String,
    role: String,
    totp_secret_encrypted: String,
    totp_secret_nonce: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let row = sqlx::query_as::<_, AdminAuthRow>(
        "SELECT id, password_hash, role::text AS role, totp_secret_encrypted, totp_secret_nonce FROM admin_users WHERE username=$1 AND is_active",
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        tracing::warn!(username = %body.username, "admin login rejected: unknown or inactive username");
        return Err(AppError::Forbidden);
    };

    let password_ok = PasswordHash::new(&row.password_hash).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(body.password.as_bytes(), &parsed)
            .is_ok()
    });
    if !password_ok {
        tracing::warn!(username = %body.username, "admin login rejected: bad password");
        return Err(AppError::Forbidden);
    }

    let secret_bytes = crypto::decrypt(
        &state.config.encryption_key,
        &row.totp_secret_encrypted,
        &row.totp_secret_nonce,
    )
    .map_err(AppError::Internal)?;
    let totp = build_totp(secret_bytes, &body.username).map_err(AppError::Internal)?;
    if totp.check_current(&body.totp_code).is_none() {
        tracing::warn!(username = %body.username, "admin login rejected: bad totp code");
        return Err(AppError::Forbidden);
    }

    let role: AdminRole = row.role.parse().map_err(AppError::Internal)?;
    let (raw_token, csrf_token, expires_at) = create_session(&state.db, row.id)
        .await
        .map_err(AppError::Internal)?;
    record_admin_audit(&state.db, row.id, "ADMIN_LOGIN", "admin_user", Some(row.id), None).await?;

    let cookie = session_cookie(&raw_token, SESSION_TTL_HOURS * 3600);
    let mut response = (
        StatusCode::OK,
        Json(json!({
            "username": body.username,
            "role": role.as_str(),
            "csrf_token": csrf_token,
            "expires_at": expires_at,
        })),
    )
        .into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| AppError::Internal(e.into()))?,
    );
    Ok(response)
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
) -> Result<Response, AppError> {
    sqlx::query("UPDATE admin_sessions SET revoked_at = now() WHERE id = $1")
        .bind(identity.session_id)
        .execute(&state.db)
        .await?;
    let mut response = (StatusCode::OK, Json(json!({"ok": true}))).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_static("admin_session=; HttpOnly; Secure; SameSite=Strict; Path=/admin; Max-Age=0"),
    );
    Ok(response)
}

pub async fn session_info(
    State(state): State<Arc<AppState>>,
    Extension(identity): Extension<AdminIdentity>,
) -> Result<Json<serde_json::Value>, AppError> {
    let csrf_token: String = sqlx::query_scalar("SELECT csrf_token FROM admin_sessions WHERE id = $1")
        .bind(identity.session_id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(json!({
        "username": identity.username,
        "role": identity.role.as_str(),
        "csrf_token": csrf_token,
    })))
}
