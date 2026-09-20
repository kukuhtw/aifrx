mod admin;
mod ai;
mod config;
mod crypto;
mod error;
mod models;
mod mt5;
mod risk;
mod routes;
mod state;
mod telegram;
mod trading;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use config::Config;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use std::sync::Arc;
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let config = Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let state = Arc::new(AppState::new(config, pool)?);
    if state.config.telegram_bot_token.is_some() {
        let telegram_state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = telegram::run(telegram_state).await {
                tracing::error!(%error, "telegram bot stopped");
            }
        });
    } else {
        tracing::info!("TELEGRAM_BOT_TOKEN is empty; Telegram bot disabled");
    }
    let admin_routes = Router::new()
        .route("/", get(admin::dashboard))
        .route("/api/overview", get(admin::overview))
        .route("/api/users", get(admin::users))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            admin::require_admin,
        ));
    let app = Router::new()
        .nest("/admin", admin_routes)
        .route("/health", get(routes::health))
        .route("/api/v1/analyses", post(routes::analyze))
        .route("/api/v1/trade-intents", post(routes::create_intent))
        .route(
            "/api/v1/trade-intents/{id}/confirm",
            post(routes::confirm_intent),
        )
        .route("/api/v1/trading/stop", post(routes::stop_trading))
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&config::bind_addr()).await?;
    tracing::info!(address = %listener.local_addr()?, "backend listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}
