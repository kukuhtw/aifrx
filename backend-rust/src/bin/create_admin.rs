//! Standalone bootstrap tool for the first (or any) admin_users row.
//! Self-contained like hash_admin_password.rs — it does not depend on the
//! main binary crate, only on shared external crates, so it needs no lib target.

use std::io::{self, Write};

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{rand_core::OsRng as ArgonOsRng, PasswordHasher, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use sqlx::postgres::PgPoolOptions;
use totp_rs::{Algorithm, Builder, Secret};

const VALID_ROLES: &[&str] = &[
    "OWNER",
    "SECURITY_OPERATOR",
    "OPS_ADMIN",
    "SUPPORT_AGENT",
    "AUDITOR",
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;
    let encryption_key: [u8; 32] = STANDARD
        .decode(std::env::var("ENCRYPTION_KEY").map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY is required (base64 32 bytes)"))?)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY must decode to 32 bytes"))?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let username = prompt("Admin username: ")?;
    anyhow::ensure!(
        !username.is_empty() && username.len() <= 64,
        "username must be 1-64 characters"
    );

    let role = prompt(&format!("Role [{}]: ", VALID_ROLES.join(", ")))?.to_uppercase();
    anyhow::ensure!(VALID_ROLES.contains(&role.as_str()), "unknown role: {role}");

    eprint!("Temporary password (min 12 characters): ");
    io::stderr().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim_end_matches(['\r', '\n']);
    anyhow::ensure!(password.len() >= 12, "password must be at least 12 characters");
    let salt = SaltString::generate(&mut ArgonOsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))?
        .to_string();

    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_users WHERE username=$1)")
        .bind(&username)
        .fetch_one(&pool)
        .await?;
    anyhow::ensure!(!exists, "an admin user named '{username}' already exists");

    let secret = Secret::generate();
    let (totp_secret_encrypted, totp_secret_nonce) = encrypt(&encryption_key, secret.as_bytes())?;
    let totp = Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_secret(secret.as_bytes().to_vec())
        .with_issuer(Some("AI Forex Admin"))
        .with_account_name(username.as_str())
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build TOTP: {e}"))?;
    let provisioning_url = totp
        .to_url()
        .map_err(|e| anyhow::anyhow!("failed to build otpauth URL: {e}"))?;

    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO admin_users(username,password_hash,role,totp_secret_encrypted,totp_secret_nonce) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(&username)
    .bind(&password_hash)
    .bind(&role)
    .bind(&totp_secret_encrypted)
    .bind(&totp_secret_nonce)
    .fetch_one(&pool)
    .await?;

    println!("Created admin user {username} ({id}) with role {role}");
    println!();
    println!("TOTP setup — add this to an authenticator app now, it will not be shown again:");
    println!("  Secret (manual entry): {}", secret.to_base32());
    println!("  Provisioning URL:      {provisioning_url}");
    Ok(())
}

fn prompt(label: &str) -> anyhow::Result<String> {
    eprint!("{label}");
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_owned())
}

fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> anyhow::Result<(String, String)> {
    let cipher = Aes256Gcm::new_from_slice(key)?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| anyhow::anyhow!("encryption failed"))?;
    Ok((STANDARD.encode(encrypted), STANDARD.encode(nonce)))
}
