use axum::{
    http::header,
    response::{Html, IntoResponse},
};

pub async fn page() -> impl IntoResponse {
    (
        [
            (header::CACHE_CONTROL, "public, max-age=300"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::X_FRAME_OPTIONS, "DENY"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
            ),
        ],
        Html(include_str!("../landing/index.html")),
    )
}
