use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
}

/// `GET /api/health` — liveness probe (Docker `HEALTHCHECK`, Vercel, uptime).
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(Health { status: "ok" }))
}
