use axum_test::TestServer;
use backend::{build_router, AppState};
use serde_json::{json, Value};
use sqlx::{
    any::{install_default_drivers, AnyPoolOptions},
    AnyPool,
};

/// In-memory SQLite pool with migrations applied. Single connection:
/// `:memory:` databases are per-connection, so `max_connections(1)` keeps
/// all queries on the same database.
async fn memory_pool() -> AnyPool {
    install_default_drivers();
    let pool = AnyPoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("failed to connect to in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    pool
}

fn server(pool: Option<AnyPool>) -> TestServer {
    TestServer::new(build_router(AppState::new(pool))).expect("failed to build test server")
}

#[tokio::test]
async fn health_returns_ok_without_db() {
    let response = server(None).get("/api/health").await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn echo_reflects_message() {
    let response = server(None)
        .post("/api/echo")
        .json(&json!({ "message": " hello " }))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body, json!({ "message": "hello" }));
}

#[tokio::test]
async fn echo_rejects_empty_message() {
    let response = server(None)
        .post("/api/echo")
        .json(&json!({ "message": "   " }))
        .await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    assert_eq!(body, json!({ "error": "message must not be empty" }));
}

#[tokio::test]
async fn echo_rejects_overlong_message() {
    let long = "x".repeat(281);
    let response = server(None)
        .post("/api/echo")
        .json(&json!({ "message": long }))
        .await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    assert!(body["error"].as_str().unwrap_or("").contains("at most 280"));
}

#[tokio::test]
async fn unknown_api_route_is_404() {
    let response = server(None).get("/api/nope").await;
    response.assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn notes_routes_return_503_without_db() {
    let server = server(None);
    server
        .post("/api/notes")
        .json(&json!({ "body": "hi" }))
        .await
        .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    server
        .get("/api/notes")
        .await
        .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    server
        .get("/api/notes/whatever")
        .await
        .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn notes_crud_roundtrip() {
    let server = server(Some(memory_pool().await));

    // Create (trims the body).
    let response = server
        .post("/api/notes")
        .json(&json!({ "body": "  first note  " }))
        .await;
    response.assert_status_ok();
    let created: Value = response.json();
    assert_eq!(created["body"], json!("first note"));
    let id = created["id"].as_str().expect("note has an id").to_string();

    // List contains it.
    let response = server.get("/api/notes").await;
    response.assert_status_ok();
    let listed: Value = response.json();
    assert_eq!(listed.as_array().map(Vec::len), Some(1));
    assert_eq!(listed[0]["id"], json!(id));

    // Get by id.
    let response = server.get(&format!("/api/notes/{id}")).await;
    response.assert_status_ok();
    let fetched: Value = response.json();
    assert_eq!(fetched, created);

    // Delete returns the deleted note, then it is gone.
    let response = server.delete(&format!("/api/notes/{id}")).await;
    response.assert_status_ok();
    let deleted: Value = response.json();
    assert_eq!(deleted["id"], json!(id));
    server
        .get(&format!("/api/notes/{id}"))
        .await
        .assert_status(axum::http::StatusCode::NOT_FOUND);
    server
        .delete(&format!("/api/notes/{id}"))
        .await
        .assert_status(axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn notes_list_is_newest_first() {
    let server = server(Some(memory_pool().await));
    for body in ["older", "newer"] {
        // Timestamps have 1s resolution; sleep keeps creation order stable.
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        server
            .post("/api/notes")
            .json(&json!({ "body": body }))
            .await
            .assert_status_ok();
    }
    let response = server.get("/api/notes").await;
    response.assert_status_ok();
    let listed: Value = response.json();
    assert_eq!(listed[0]["body"], json!("newer"));
    assert_eq!(listed[1]["body"], json!("older"));
}

#[tokio::test]
async fn notes_reject_empty_and_overlong_bodies() {
    let server = server(Some(memory_pool().await));
    let response = server
        .post("/api/notes")
        .json(&json!({ "body": "   " }))
        .await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);

    let response = server
        .post("/api/notes")
        .json(&json!({ "body": "x".repeat(2001) }))
        .await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

/// Postgres parity: same contract against a real Postgres. Requires
/// `TEST_DATABASE_URL=postgres://…`; ignored otherwise (see
/// knowledge/database.md §6). CI provides it via a service container.
#[tokio::test]
async fn notes_crud_roundtrip_postgres() {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_default();
    if url.trim().is_empty() {
        eprintln!("skipping: TEST_DATABASE_URL is not set");
        return;
    }
    install_default_drivers();
    let pool = AnyPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("failed to connect to TEST_DATABASE_URL");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    // Isolate from other runs; Postgres has no per-test memory DB.
    sqlx::query("DELETE FROM notes")
        .execute(&pool)
        .await
        .expect("cleanup failed");

    let server = server(Some(pool));
    let response = server
        .post("/api/notes")
        .json(&json!({ "body": "pg note" }))
        .await;
    response.assert_status_ok();
    let created: Value = response.json();
    let id = created["id"].as_str().expect("note has an id").to_string();
    server
        .get(&format!("/api/notes/{id}"))
        .await
        .assert_status_ok();
    server
        .delete(&format!("/api/notes/{id}"))
        .await
        .assert_status_ok();
    server
        .get(&format!("/api/notes/{id}"))
        .await
        .assert_status(axum::http::StatusCode::NOT_FOUND);
}
