use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ApiError, AppState};

/// Max note length (mirrors the echo route's abuse-resistance rule).
pub const MAX_BODY_LEN: usize = 2000;

#[derive(Debug, Serialize, Clone)]
pub struct Note {
    pub id: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNote {
    pub body: String,
}

fn validate_body(raw: &str) -> Result<String, ApiError> {
    let body = raw.trim().to_string();
    if body.is_empty() {
        return Err(ApiError::unprocessable("body must not be empty"));
    }
    if body.chars().count() > MAX_BODY_LEN {
        return Err(ApiError::unprocessable(format!(
            "body must be at most {MAX_BODY_LEN} characters"
        )));
    }
    Ok(body)
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// `query_as` with a tuple keeps the row mapping portable across drivers.
fn row_to_note(row: (String, String, String, String)) -> Note {
    Note {
        id: row.0,
        body: row.1,
        created_at: row.2,
        updated_at: row.3,
    }
}

/// `POST /api/notes` — create a note. DB required; 503 when unconfigured.
pub async fn create_note(
    State(state): State<AppState>,
    Json(input): Json<CreateNote>,
) -> Result<Json<Note>, ApiError> {
    let Some(pool) = state.pool() else {
        return Err(ApiError::service_unavailable("database is not configured"));
    };
    let body = validate_body(&input.body)?;
    let now = now_rfc3339();
    // UUID v7: time-ordered TEXT id — no AUTOINCREMENT/SERIAL divergence.
    let id = Uuid::now_v7().to_string();

    sqlx::query("INSERT INTO notes (id, body, created_at, updated_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(&body)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

    Ok(Json(Note {
        id,
        body,
        created_at: now.clone(),
        updated_at: now,
    }))
}

/// `GET /api/notes` — list newest-first. DB required; 503 when unconfigured.
pub async fn list_notes(State(state): State<AppState>) -> Result<Json<Vec<Note>>, ApiError> {
    let Some(pool) = state.pool() else {
        return Err(ApiError::service_unavailable("database is not configured"));
    };
    // RFC3339 TEXT sorts lexicographically == chronologically — portable
    // ORDER BY on both SQLite and Postgres. Tie-break on id for stability.
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, body, created_at, updated_at FROM notes ORDER BY created_at DESC, id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    Ok(Json(rows.into_iter().map(row_to_note).collect()))
}

/// `GET /api/notes/{id}` — fetch one note. 404 when missing.
pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Note>, ApiError> {
    let Some(pool) = state.pool() else {
        return Err(ApiError::service_unavailable("database is not configured"));
    };
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT id, body, created_at, updated_at FROM notes WHERE id = ?")
            .bind(&id)
            .fetch_optional(pool)
            .await?;
    match row {
        Some(row) => Ok(Json(row_to_note(row))),
        None => Err(ApiError::not_found("note not found")),
    }
}

/// `DELETE /api/notes/{id}` — delete one note. 404 when missing.
pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Note>, ApiError> {
    let Some(pool) = state.pool() else {
        return Err(ApiError::service_unavailable("database is not configured"));
    };
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT id, body, created_at, updated_at FROM notes WHERE id = ?")
            .bind(&id)
            .fetch_optional(pool)
            .await?;
    let Some(row) = row else {
        return Err(ApiError::not_found("note not found"));
    };
    sqlx::query("DELETE FROM notes WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await?;
    Ok(Json(row_to_note(row)))
}
