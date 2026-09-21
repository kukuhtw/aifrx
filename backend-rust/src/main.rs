mod admin;
mod admin_actions;
mod admin_auth;
mod ai;
mod config;
mod crypto;
mod error;
mod landing;
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
    let admin_api_routes = Router::new()
        .route("/overview", get(admin::overview))
        .route("/users", get(admin::users))
        .route("/users/{id}/suspend", post(admin_actions::suspend_user))
        .route("/users/{id}/reactivate", post(admin_actions::reactivate_user))
        .route("/users/{id}/credit", post(admin_actions::grant_credit))
        .route(
            "/subscriptions/{id}/cancel-at-period-end",
            post(admin_actions::cancel_subscription),
        )
        .route("/invoices/{id}/refund", post(admin_actions::refund_invoice))
        .route(
            "/invoices/{id}/confirm-refund",
            post(admin_actions::confirm_refund),
        )
        .route("/audit", get(admin_actions::list_admin_audit))
        .route(
            "/admins",
            get(admin_actions::list_admins).post(admin_actions::create_admin_user),
        )
        .route("/admins/{id}/role", post(admin_actions::change_admin_role))
        .route(
            "/admins/{id}/deactivate",
            post(admin_actions::deactivate_admin),
        )
        .route(
            "/admins/{id}/reactivate",
            post(admin_actions::reactivate_admin),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            admin_auth::require_admin_session,
        ));
    let admin_routes = Router::new()
        .route("/", get(admin::dashboard))
        .route("/auth/login", post(admin_auth::login))
        .route(
            "/auth/logout",
            post(admin_auth::logout).route_layer(middleware::from_fn_with_state(
                state.clone(),
                admin_auth::require_admin_session,
            )),
        )
        .route(
            "/auth/session",
            get(admin_auth::session_info).route_layer(middleware::from_fn_with_state(
                state.clone(),
                admin_auth::require_admin_session,
            )),
        )
        .nest("/api", admin_api_routes);
    let app = Router::new()
        .route("/", get(landing::page))
        .nest("/admin", admin_routes)
        .route("/health", get(routes::health))
        .route("/api/v1/analyses", post(routes::analyze))
        .route("/api/v1/mt5/accounts", post(routes::add_mt5_account))
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
