//! Shared DB layer: one `sqlx::AnyPool` for SQLite (default) and Postgres.
//!
//! Design rules (see `knowledge/database.md`):
//! - Connection string comes from `DATABASE_URL`, defaulting to a local
//!   SQLite file. Unset/blank means SQLite locally, and "no database"
//!   on Vercel (ephemeral disk — never SQLite there). Set
//!   `DATABASE_URL=none` to explicitly disable the DB anywhere.
//! - Portable SQL everywhere: `?` placeholders, `TEXT` ids generated in Rust
//!   (UUID v7), RFC3339 `TEXT` timestamps. No `AUTOINCREMENT`, no
//!   `SERIAL`, no `RETURNING`, no `$1`.
//! - Schema is embedded at compile time (`sqlx::migrate!("./migrations")`)
//!   and applied once at startup via [`run_migrations`].

use sqlx::{
    any::{install_default_drivers, AnyPoolOptions},
    AnyPool,
};
use std::sync::OnceLock;

/// Register the `Any` drivers once per process. Required before any
/// `AnyPool::connect` — without it every connect fails with
/// "no driver found for URL scheme".
fn drivers_installed() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(install_default_drivers);
}

/// Default connection string: local SQLite file, created on first connect
/// (`mode=rwc`). Used whenever `DATABASE_URL` is unset or blank.
pub const DEFAULT_DATABASE_URL: &str = "sqlite://./dev.db?mode=rwc";

/// Explicit opt-out: `DATABASE_URL=none` disables the DB anywhere
/// (pool `None`, notes routes answer `503`).
pub const DATABASE_URL_DISABLED: &str = "none";

/// Resolve the effective connection string.
///
/// - `None`/blank → [`DEFAULT_DATABASE_URL`] (SQLite by default).
/// - `"none"` (case-insensitive, trimmed) → `None` (DB disabled).
/// - anything else → used as-is (SQLite or `postgres://…`).
pub fn database_url() -> Option<String> {
    parse_database_url(std::env::var("DATABASE_URL").ok().as_deref())
}

/// Pure core of [`database_url`]: trim, apply the default/disabled rules.
/// No env access, so unit tests can pin the rule without touching process env.
pub fn parse_database_url(raw: Option<&str>) -> Option<String> {
    let trimmed = raw.unwrap_or("").trim();
    if trimmed.is_empty() {
        return Some(DEFAULT_DATABASE_URL.to_string());
    }
    if trimmed.eq_ignore_ascii_case(DATABASE_URL_DISABLED) {
        return None;
    }
    Some(trimmed.to_string())
}

/// Open the pool, defaulting to local SQLite. Returns `None` only when
/// the DB is explicitly disabled (`DATABASE_URL=none`).
///
/// SQLite note: `sqlite://./dev.db?mode=rwc` creates the file on first
/// connect (`mode=rwc`).
pub async fn connect_pool() -> Result<Option<AnyPool>, sqlx::Error> {
    // `Any` needs its drivers registered once per process before connecting.
    drivers_installed();
    let Some(url) = database_url() else {
        return Ok(None);
    };
    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    Ok(Some(pool))
}

/// Apply embedded `./migrations` once. No-op when `pool` is `None`.
pub async fn run_migrations(pool: Option<&AnyPool>) -> Result<(), sqlx::migrate::MigrateError> {
    if let Some(pool) = pool {
        sqlx::migrate!("./migrations").run(pool).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_database_url, DEFAULT_DATABASE_URL};

    #[test]
    fn sqlite_is_the_default_db() {
        // Unset/blank → local SQLite file, never "no database".
        assert_eq!(
            parse_database_url(None),
            Some(DEFAULT_DATABASE_URL.to_string())
        );
        assert_eq!(
            parse_database_url(Some("   ")),
            Some(DEFAULT_DATABASE_URL.to_string())
        );
        // Explicit opt-out disables the DB.
        assert_eq!(parse_database_url(Some("none")), None);
        assert_eq!(parse_database_url(Some("  NONE  ")), None);
        // Anything else passes through trimmed (SQLite or Postgres).
        assert_eq!(
            parse_database_url(Some("  postgres://db/app  ")),
            Some("postgres://db/app".to_string())
        );
    }
}
