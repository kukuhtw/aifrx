use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("access denied")]
    Forbidden,
    #[error("{0}")]
    Validation(String),
    #[error("service temporarily unavailable")]
    Unavailable,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            _ => {
                tracing::error!(error = ?self, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };
        (
            status,
            Json(json!({"error": message, "order_placed": false})),
        )
            .into_response()
    }
}
