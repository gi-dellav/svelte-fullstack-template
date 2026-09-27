//! Axum backend entrypoint.
//!
//! Listens on `$PORT` (Vercel injects it; default `3000` locally) and serves
//! the JSON API under `/api/*`. The same binary runs in plain Docker and in
//! the Vercel container (`backend/Dockerfile.vercel`) — Fluid compute just
//! calls it with a different `PORT`.
//!
//! Persistence defaults to SQLite: without `DATABASE_URL` the pool opens
//! `./dev.db` (created on first connect) and embedded migrations run once at
//! startup (see `knowledge/database.md`). Set `DATABASE_URL=postgres://…`
//! for Postgres, or `DATABASE_URL=none` to run without a database (notes
//! routes answer `503`, everything else works).

use backend::{
    build_router,
    db::{connect_pool, run_migrations},
    port, AppState,
};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let pool = connect_pool()
        .await
        .expect("failed to connect to DATABASE_URL");
    if pool.is_some() {
        tracing::info!("database pool connected; running migrations");
    } else {
        tracing::info!("DATABASE_URL=none; running without a database (notes routes return 503)");
    }
    run_migrations(pool.as_ref())
        .await
        .expect("failed to run migrations");

    let port = port();
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("failed to bind TCP listener");
    tracing::info!("listening on port {port}");
    axum::serve(listener, build_router(AppState::new(pool)))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

/// Graceful shutdown: SIGTERM (Vercel scale-in gives 30s grace) or Ctrl-C.
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut term = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = term.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
    tracing::info!("shutdown signal received");
}
