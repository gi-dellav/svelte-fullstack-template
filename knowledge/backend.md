# Backend (Axum + sqlx)

JSON API in `backend/`: **Axum 0.8 + Tokio**, same binary for local Docker and Vercel container Functions. Persistence via **sqlx `AnyPool`** (SQLite by default, Postgres when `DATABASE_URL` says so) — full story in `database.md`. Auth is docs-only (`auth.md`) — no auth code ships here.

## 1. Layout

| Path | Role |
|---|---|
| `backend/Cargo.toml` | `axum` (json), `tokio` (full), `tower-http` (cors, trace), `serde/serde_json`, `tracing(-subscriber)`, `sqlx` (any/sqlite/postgres/migrate/tls-rustls), `uuid` (v7), `chrono`; dev: `axum-test` |
| `backend/src/lib.rs` | `AppState { pool }`, `build_router(state)` factory + `cors_layer()` + `parse_allowlist()` + `port()` |
| `backend/src/db.rs` | `database_url()` / `parse_database_url()`, `connect_pool()`, `run_migrations()` (embedded `./migrations`) |
| `backend/migrations/` | `0001_notes.sql` (portable subset — see `database.md` §2); add tables as new files, never edit applied ones |
| `backend/src/main.rs` | `#[tokio::main]`: connect pool → run migrations → bind `0.0.0.0:$PORT`, graceful SIGTERM/Ctrl-C shutdown |
| `backend/src/error.rs` | `ApiError` → `(StatusCode, Json<{ error }>)`; constructors `unprocessable()` / `not_found()` / `service_unavailable()` / `internal()`; `From<sqlx::Error>` → `500` (logs detail, hides it from clients) |
| `backend/src/routes/health.rs` | `GET /api/health` → `200 { "status": "ok" }` (works without a DB) |
| `backend/src/routes/echo.rs` | `POST /api/echo { message }` → `200 { message }` / `422 { error }` (works without a DB) |
| `backend/src/routes/notes.rs` | `POST/GET /api/notes`, `GET/DELETE /api/notes/{id}` → `Note` JSON / `422` / `404` / `503` (no DB) |
| `backend/tests/api.rs` | `axum-test`: health/echo/404 (no DB), notes `503`s, notes CRUD + ordering + validation on in-memory SQLite, Postgres parity behind `TEST_DATABASE_URL` |
| `backend/Dockerfile` | Local dev image: build (+ `migrations/` copy) → `debian:bookworm-slim` + `curl`, `WORKDIR /data`, `HEALTHCHECK /api/health` |
| `backend/Dockerfile.vercel` | Vercel Function image: same build, slim runtime, **no** healthcheck, listens on `$PORT` (never SQLite here — ephemeral disk) |
| `backend/.env.example` | `PORT=3000`, `RUST_LOG=info`, `DATABASE_URL=sqlite://./dev.db?mode=rwc`, `ALLOWED_ORIGINS=` (blank = permissive dev) |

## 2. API contract

```
GET  /api/health → 200 { "status": "ok" }
POST /api/echo   → 200 { "message": "<trimmed>" }
                    422 { "error": "message must not be empty" }
                    422 { "error": "message must be at most 280 characters" }
POST /api/notes  → 200 Note | 422 | 503 (see database.md §3 for the full notes contract)
```

- Echo trims surrounding whitespace; length is counted in `char`s (not bytes).
- Unknown `/api/*` paths return Axum's default `404` (plain text) — the frontend router never sees them because `/api/*` is routed to the backend before the SPA fallback.
- Error envelope is always `{ "error": "<message>" }` — construct via `ApiError`.

## 3. Add a route

Three mechanical steps; do all three or the route 404s in one runtime:

1. Add `backend/src/routes/<name>.rs` with an `async fn` handler taking `State<AppState>` and returning `impl IntoResponse` or `Result<Json<T>, ApiError>`. DB-backed routes: `state.pool()` → `503` when `None`; portable SQL (`?`, TEXT ids, RFC3339 TEXT — see `database.md` §2).
2. Declare `pub mod <name>;` in `backend/src/routes/mod.rs` and register it in `build_router()` in `backend/src/lib.rs`:
   ```rust
   Router::new()
       .route("/api/health", get(routes::health::health))
       .route("/api/things", post(routes::things::create))
   ```
3. Add `axum-test` cases in `backend/tests/api.rs` (happy path + validation + 404/503 against `memory_pool()`; Postgres-sensitive SQL also in the parity test), then:
   ```bash
   cd backend && cargo fmt && cargo clippy -- -D warnings && cargo test
   ```

Keep handlers thin: validate input → query → return JSON. New error kind = new `ApiError` constructor, not inline status tuples. New table = new migration file first (`database.md` §7).

## 4. Config (env)

| Var | Default | Meaning |
|---|---|---|
| `PORT` | `3000` | Listen port. Vercel injects its own (default `80`); local default matches the Vite proxy. |
| `RUST_LOG` | `info` | `tracing_subscriber` `EnvFilter` (e.g. `debug`, `backend=debug,tower_http=warn`). |
| `DATABASE_URL` | _(unset → SQLite `./dev.db`)_ | Local SQLite by default; `postgres://…` for Postgres (see `database.md` §5); `none` to disable the DB (pool `None` → notes answer `503`). |
| `ALLOWED_ORIGINS` | _(unset)_ | Comma-separated browser origins, e.g. `https://app.vercel.app,https://example.com`. Unset/blank = permissive dev CORS. |
| `FRONTEND_URL` | _(unset)_ | Single-origin alias for `ALLOWED_ORIGINS` (convenience for one-domain deploys). |

Rules: never `expect()` on env in `lib.rs` — defaults, not panics (the one exception: auth stores must fail startup loudly when unconfigured — see `auth.md` §2). `port()` tolerates blank/garbage by falling back to `3000`. Production **must** set `ALLOWED_ORIGINS` (or `FRONTEND_URL`) to the Vercel URL — the permissive default is dev-only.

## 5. CORS model

- Dev (`ALLOWED_ORIGINS` unset): `CorsLayer::very_permissive()` — Vite (`:5173`), `docker-compose` frontend (`:8080`), and `curl` all work with zero config. CORS methods include `DELETE` for the notes routes.
- Prod (`ALLOWED_ORIGINS` set): strict allowlist, `GET/POST/DELETE/OPTIONS` + `Content-Type`/`Accept` headers only. Origins that fail `HeaderValue::from_str` are skipped.
- Same-origin production (Vercel rewrites `/api/*` → backend service) rarely triggers CORS at all — the allowlist is defense for direct backend hits and previews.

## 6. Run locally

```bash
cd backend
cp .env.example .env   # optional; defaults already work (SQLite ./dev.db)
cargo run              # → creates dev.db, runs migrations, listening on port 3000
curl localhost:3000/api/health   # {"status":"ok"}
curl -X POST localhost:3000/api/echo \
  -H 'Content-Type: application/json' -d '{"message":"hi"}'
curl -X POST localhost:3000/api/notes \
  -H 'Content-Type: application/json' -d '{"body":"first note"}'
```

Fullstack (Vite proxy): in another terminal `cd frontend && bun install && bun run dev` → `http://localhost:5173/api/health` hits Axum.

Docker:

```bash
docker build -f backend/Dockerfile -t fullstack-backend ./backend
docker run -p 3000:3000 fullstack-backend
# or everything at once (needs frontend/dist built for the nginx image):
cd frontend && bun run build && cd .. && docker compose up --build
```

Note: `docker compose` builds the frontend nginx image from `frontend/Dockerfile.vercel`, which `COPY`s `dist/` — build the frontend first. `backend/.env` is optional (`env_file.required: false`); the compose `environment:` SQLite default applies when no `.env` exists.

## 7. Vercel notes

- Vercel builds `backend/Dockerfile.vercel` into a container Function on Fluid compute: autoscaled, Active CPU billing, scale-to-zero after ~5 min idle (30s on previews). On scale-in the container gets `SIGTERM` + 30s grace — `main.rs` shuts down gracefully.
- The service receives the **original path** (`/api/health`, not `/health`) — keep the `/api` prefix in `build_router()` routes.
- Stateless only: no in-memory sessions, no disk writes (see `vercel-deploy.md` §5). The SQLite file **must not** be relied on under Vercel — set `DATABASE_URL` to Postgres or `none`. Persistent state needs a backing service (see `database.md` §5).
- Logs go to `stdout`/`stderr` via `tracing` and appear in Vercel Observability. DB errors log the driver detail server-side but return the generic `500 { "error": "database error" }`.
