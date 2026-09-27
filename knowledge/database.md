# Database (sqlx: SQLite by default, Postgres when you need it)

The backend persists with **[sqlx](https://github.com/launchbadge/sqlx)** through one `sqlx::AnyPool` (`backend/src/db.rs`) so **the same binary talks to SQLite locally and Postgres in production**. The worked example is a `notes` CRUD (`backend/src/routes/notes.rs`, table `backend/migrations/0001_notes.sql`).

Rule of thumb: build features against SQLite; switch `DATABASE_URL` to Postgres when you need concurrent writers, a managed backup story, or Vercel persistence (container disks are ephemeral — SQLite created on Vercel vanishes on scale-in).

## 1. How it works

| Piece | Where | Notes |
|---|---|---|
| Pool | `db.rs` (`AnyPool`, `connect_pool()`) | `DATABASE_URL`, defaulting to local SQLite. `DATABASE_URL=none` → `None` → notes routes answer `503`, health/echo keep working |
| Migrations | `migrations/*.sql`, embedded via `sqlx::migrate!("./migrations")` | Applied once at startup (`run_migrations` in `main.rs`). Both Dockerfiles `COPY migrations/` |
| State | `AppState { pool: Option<AnyPool> }` in `lib.rs` | `build_router(state)` takes it explicitly — tests inject pools without env |
| Rows | `notes.rs` (`query` / `query_as` with tuples) | Runtime-checked SQL (works on both drivers). No `query!` macros — compile-time checking can't span two drivers |
| Errors | `ApiError::from(sqlx::Error)` → `500 { "error": "database error" }` | Driver detail is `tracing::error!`-logged, never sent to clients |

Dependencies (`backend/Cargo.toml`): `sqlx` with `runtime-tokio, any, sqlite, postgres, migrate, tls-rustls`, plus `uuid` (v7 ids) and `chrono` (timestamps). `tls-rustls` is pure Rust — the slim runtime images stay openssl-free even for `postgres://` URLs.

## 2. The portability contract (why one SQL runs on both)

Three deliberate choices keep every query portable — violate any of them and the dual-driver story breaks:

1. **`?` placeholders everywhere.** The `Any` driver rebinds `?` per backend (`$1` on Postgres, `?` on SQLite). Never write `$1` in this codebase.
2. **`TEXT` ids generated in Rust** (`Uuid::now_v7().to_string()`). UUID v7 is time-ordered, so `ORDER BY created_at, id` stays stable. This avoids the `AUTOINCREMENT` vs `SERIAL`/`GENERATED … AS IDENTITY` divergence entirely — no `RETURNING` clause needed since the id is known before `INSERT`.
3. **RFC3339 `TEXT` timestamps** (`Utc::now().to_rfc3339_opts(Secs, true)`). Lexicographic order == chronological order on both backends, so `ORDER BY created_at DESC` needs no date functions. The cost: range queries compare strings (fine at this scale; switch to native timestamps if you outgrow it — that forks the SQL).

Migration files must stay in the common subset too: `TEXT/INTEGER` columns, `PRIMARY KEY`, `NOT NULL`. No `SERIAL`, no `AUTOINCREMENT`, no `ON CONFLICT` (branch in Rust if you need upserts), no stored procedures.

## 3. API contract (notes)

```
POST   /api/notes      { "body": "…" } → 200 Note
GET    /api/notes                          → 200 Note[] (newest-first, max 100)
GET    /api/notes/{id}                     → 200 Note | 404 { "error": "note not found" }
DELETE /api/notes/{id}                     → 200 Note (the deleted row) | 404
Note = { "id": "<uuid-v7>", "body": "…", "created_at": "<rfc3339>", "updated_at": "<rfc3339>" }
```

- `body` is trimmed; empty → `422`, over 2000 chars → `422`.
- Notes routes with the DB explicitly disabled (`DATABASE_URL=none`) → `503 { "error": "database is not configured" }` (deliberate: distinguishes "no DB attached" from "DB broken", which is a `500`).
- Unknown `/api/*` paths still return Axum's plain-text `404`.

Frontend calls go through `apiUrl()` as usual (`frontend-patterns.md` §4); e.g. `fetchJson<Note[]>(apiUrl("/api/notes"))` for the list, POST with `Content-Type: application/json` for creates.

## 4. Run locally (SQLite — zero setup)

```bash
cd backend
cargo run              # no env needed: defaults to ./dev.db, runs migrations, listens :3000
curl localhost:3000/api/notes   # [] — then POST to add notes
```

- `mode=rwc` = create the file if missing. The file + journals (`dev.db*`) are gitignored. `cargo run` with no env at all already does this — `.env` is only needed to override (Postgres URL, origins, log level).
- Point at another file with `DATABASE_URL=sqlite:///absolute/path/app.db?mode=rwc`.
- Opt out entirely with `DATABASE_URL=none` (pool `None`, notes → `503`).
- Never commit a `.db` file; never put the SQLite file on a shared/network volume in production.

Docker: `docker compose up --build` sets `DATABASE_URL=sqlite://./dev.db?mode=rwc` with `WORKDIR /data` — the file lives inside the container (ephemeral, correct for previews). `backend/.env` is now optional (`env_file.required: false`); explicit `environment:` values win when both exist.

## 5. Switch to Postgres (local + Vercel)

**Local parity** — uncomment the `db` service block in `docker-compose.yml` (postgres:16-alpine + `pgdata` volume), then run the backend against it:

```bash
docker compose up db -d
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/app cargo run
```

Same migrations, same queries, same tests — nothing else changes. That sameness is the point of the portability contract (§2).

**Vercel** — attach Postgres from the Marketplace (Neon/Supabase) and set the backend service env:

```
DATABASE_URL=<pooled connection string>   # use the pooler (PgBouncer) URL when offered
```

- Migrations run automatically at container boot (`run_migrations` in `main.rs`) — no separate migrate step.
- Keep `max_connections` small (default `5` in `connect_pool`): each scaled-out instance opens its own pool against the shared database. Add `acquire_timeout` if cold starts contend.
- The pooler URL matters: direct Postgres URLs + many short-lived Fluid instances = connection exhaustion. Prefer the `*_POOLED`/`-pooler` host when your provider offers one.
- SQLite on Vercel is a non-starter (ephemeral disk) — either set `DATABASE_URL` to Postgres or set `DATABASE_URL=none` and accept `503` on notes.

## 6. Tests

| Layer | How | Where |
|---|---|---|
| Env parsing | `parse_database_url` / `parse_allowlist` unit tests (pure, no env) | `src/db.rs`, `src/lib.rs` |
| CRUD contract | `axum-test` against **in-memory SQLite** (`sqlite::memory:`, `max_connections(1)` — memory DBs are per-connection) with migrations applied | `tests/api.rs` (`memory_pool()`) |
| Ordering/validation | `notes_list_is_newest_first` (1s sleeps — timestamps have 1s resolution), empty/overlong `422`s, no-DB `503`s | `tests/api.rs` |
| Postgres parity | Same CRUD roundtrip, `DELETE FROM notes` isolation | `tests/api.rs` (`notes_crud_roundtrip_postgres`) — **runs only when `TEST_DATABASE_URL` is set**, otherwise prints `skipping` and passes |

Run the full matrix locally:

```bash
TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/app cargo test
```

CI (`.github/workflows/ci.yml`) provides `TEST_DATABASE_URL` via a `postgres:16-alpine` service container, so Postgres parity runs on every push/PR.

## 7. Add a table (recipe)

1. New migration `backend/migrations/0002_<name>.sql` in the common subset (§2). Never edit an applied migration — add a new file.
2. New module `backend/src/routes/<name>.rs`: `State<AppState>` extractor, `state.pool()` → `503` when `None`, portable queries, `?` operator into `ApiError` for DB failures.
3. Register `pub mod` + `.route("/api/…", …)` in `lib.rs` (keep the `/api` prefix — Vercel forwards the original path).
4. Extend `tests/api.rs`: happy path + validation + 404/503 against `memory_pool()`. If the query uses anything driver-specific, also cover it in the Postgres parity test.
5. `cargo fmt && cargo clippy -- -D warnings && cargo test` (plus `TEST_DATABASE_URL=… cargo test` when touching SQL).
