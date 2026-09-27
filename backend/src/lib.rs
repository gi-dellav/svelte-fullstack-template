use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use sqlx::AnyPool;
use std::env;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

pub mod db;
mod error;
mod routes;

pub use db::{database_url, parse_database_url};
pub use error::ApiError;
pub use routes::{echo::EchoBody, health::Health, notes::Note};

/// Shared application state. `pool` is `None` only when the DB is
/// explicitly disabled (`DATABASE_URL=none`) — notes routes then answer
/// `503`, everything else works.
#[derive(Clone, Debug)]
pub struct AppState {
    pool: Option<AnyPool>,
}

impl AppState {
    pub fn new(pool: Option<AnyPool>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> Option<&AnyPool> {
        self.pool.as_ref()
    }
}

/// Shared router factory used by the standalone binary (`main.rs`, local dev
/// + plain Docker) and the Vercel container (`Dockerfile.vercel`).
///
/// Takes the state explicitly so tests can inject an in-memory SQLite pool
/// without touching process env.
pub fn build_router(state: AppState) -> Router {
    let cors = cors_layer();

    Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/api/echo", post(routes::echo::echo))
        .route(
            "/api/notes",
            post(routes::notes::create_note).get(routes::notes::list_notes),
        )
        .route(
            "/api/notes/{id}",
            get(routes::notes::get_note).delete(routes::notes::delete_note),
        )
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(cors)
}

/// CORS policy:
/// - no `ALLOWED_ORIGINS` (local dev): permissive, so `bun run dev`
///   (Vite, port 5173) works out of the box;
/// - `ALLOWED_ORIGINS="https://app.vercel.app,https://example.com"`: strict
///   allowlist. `FRONTEND_URL` is accepted as a single-origin alias.
fn cors_layer() -> CorsLayer {
    match allowed_origins() {
        None => CorsLayer::very_permissive(),
        Some(origins) => {
            let mut layer = CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
                .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);
            for origin in origins {
                if let Ok(value) = HeaderValue::from_str(&origin) {
                    layer = layer.allow_origin(value);
                }
            }
            layer
        }
    }
}

/// Split a raw allowlist value on commas, trim, drop empties.
/// `None` = blank input = permissive dev mode.
pub fn parse_allowlist(raw: &str) -> Option<Vec<String>> {
    if raw.trim().is_empty() {
        return None;
    }
    let origins: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();
    if origins.is_empty() {
        None
    } else {
        Some(origins)
    }
}

/// Parse the allowlist from env. `None` = unset/blank = permissive dev mode.
fn allowed_origins() -> Option<Vec<String>> {
    let raw = env::var("ALLOWED_ORIGINS")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| env::var("FRONTEND_URL").ok())
        .filter(|v| !v.trim().is_empty())?;
    parse_allowlist(&raw)
}

/// Port the server listens on. Vercel injects `PORT` (default `80` there);
/// local default is `3000` to match `frontend/vite.config.ts` proxy.
pub fn port() -> u16 {
    env::var("PORT")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(3000)
}

#[cfg(test)]
mod tests {
    use super::parse_allowlist;

    #[test]
    fn blank_allowlist_means_permissive_dev_mode() {
        assert_eq!(parse_allowlist(""), None);
        assert_eq!(parse_allowlist("   "), None);
        assert_eq!(
            parse_allowlist("https://a.example, https://b.example ,,"),
            Some(vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ])
        );
    }
}
