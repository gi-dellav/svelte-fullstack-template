# svelte-fullstack-template

A fullstack template: **Svelte 5 + Vite + TailwindCSS 4 + PWA** frontend + **Rust + Axum** JSON API, deployed to **Vercel as two Docker services**.

- Frontend: [Svelte 5](https://svelte.dev) (runes), [Vite](https://vite.dev), [TailwindCSS 4](https://tailwindcss.com), [vite-plugin-pwa](https://vite-pwa-org.netlify.app), [Bun](https://bun.sh) — in `frontend/`
- Backend: [Axum](https://github.com/tokio-rs/axum) + Tokio + [sqlx](https://github.com/launchbadge/sqlx) JSON API (`GET /api/health`, `POST /api/echo`, notes CRUD on SQLite by default, Postgres when `DATABASE_URL` says so) — in `backend/`
- Deploy: [Vercel Services](https://vercel.com/docs/services) via `vercel.json` (static frontend + container backend, see `knowledge/vercel-deploy.md`). Auth is a docs-only cookbook (`knowledge/auth.md`) — no auth code ships.

## Getting started

Requires [Bun](https://bun.sh) ≥ 1.1 and [Rust](https://www.rust-lang.org) (stable).

```bash
cd frontend
bun install
bun run dev       # start the dev server (proxies /api → 127.0.0.1:3000)
bun run check     # svelte-check type/diagnostics
bun run test      # unit tests with bun test
bun run build     # production build into frontend/dist/

cd ../backend
cargo run         # start the Axum API on :3000 (SQLite ./dev.db, migrations auto-run)
```

Run both together: `cargo run` in `backend/`, `bun run dev` in `frontend/` — the Vite proxy makes `/api/*` same-origin, exactly like production. Or preview the Docker images: `docker compose up --build` (frontend `:8080`, backend `:3000`; build `frontend/dist/` first).

The landing page (`frontend/src/App.svelte`) includes a backend health check, the posts list, and the Vercel deploy steps in the browser.

## Project structure

```
vercel.json             # services (frontend static-build, backend container) + /api/* rewrites
docker-compose.yml      # local fullstack: backend :3000 + frontend :8080
frontend/
  index.html            # Vite entry (SEO/OG defaults, PWA icons, theme-color)
  vite.config.ts        # Svelte, Tailwind and PWA plugins (+ /api dev proxy, SITE_URL)
  svelte.config.js      # vitePreprocess()
  tsconfig.json         # strict TS (exactOptionalPropertyTypes, noUncheckedIndexedAccess, …)
  Dockerfile.vercel     # nginx static container (local preview / non-Vercel hosts)
  nginx.conf            # SPA fallback + cache headers for the nginx image
  .env.example          # VITE_API_URL (empty = same-origin), SITE_URL
  package.json / bun.lock
  plugins/
    md.ts               # build-time .md → HTML (frontmatter, GFM, heading ids, 404 fallback)
    seo.ts              # build-time SEO (canonical/OG/RSS injection, sitemap, rss, robots)
  src/
    main.ts             # mounts App (Geist fonts + app.css)
    App.svelte          # sample page (counter, backend health, posts, deploy guide) + route switch
    PwaUpdate.svelte    # offline-ready / update prompts
    Seo.svelte          # per-route <title>/description/canonical/OG (Svelte head)
    app.css             # Tailwind import + theme tokens + component classes
    lib/
      router.ts         # hand-rolled history-API SPA (Route, withBase, navigate, …)
      api.ts            # API_URL + apiUrl(path) — backend URLs honoring VITE_API_URL
      base.ts           # base derivation (resolveBase, normalizeBaseOverride)
      posts.ts          # import.meta.glob collection (draft filter, date-desc sort)
      post-utils.ts     # pure post builders (tested without env/glob)
      storage.ts        # never-throw localStorage layer (StorageLike, memoryStorage)
      local-store.svelte.ts # runes localStore(key, initial) factory
      async.ts          # AsyncState machine + fetchJson + toErrorMessage
      form.ts           # pure validators (required, emailField, minLength, validateAll)
    routes/
      Post.svelte       # renders compiled post HTML inside article.prose
    content/
      *.md              # markdown posts with frontmatter (title, date, description, draft)
    md.d.ts             # PostMetadata / PostModule types + *.md declaration
  public/               # favicon, PWA icons (served as-is)
  tests/
    api.test.ts         # apiUrl (same-origin default)
    base.test.ts        # resolveBase / normalizeBaseOverride
    router.test.ts      # stripBase / joinBase / parseRoute
    md.test.ts          # rewriteUrl / rewriteAssetUrls / srcset / mdPlugin transform
    posts.test.ts       # buildPosts / sorting / draft filter
    seo.test.ts         # site root / sitemap / RSS / robots / head injection
    storage.test.ts     # StorageLike / readStored / writeStored / clearStored
    async.test.ts       # AsyncState / toErrorMessage / fetchJson (stubbed fetch)
    form.test.ts        # required / email / minLength / validateAll
backend/
  Cargo.toml            # axum, tokio, tower-http, serde, tracing, sqlx, uuid, chrono; dev: axum-test
  src/
    lib.rs              # AppState { pool }, build_router(state) + CORS layer + port()
    db.rs               # AnyPool over DATABASE_URL, connect_pool(), run_migrations()
    main.rs             # #[tokio::main] binary: pool → migrations → bind $PORT, graceful SIGTERM shutdown
    error.rs            # ApiError → { error } JSON envelope (+ From<sqlx::Error>)
    routes/
      health.rs         # GET /api/health → { status: "ok" }
      echo.rs           # POST /api/echo { message } → { message } / 422
      notes.rs          # notes CRUD → Note JSON / 422 / 404 / 503 (no DB)
  migrations/
    0001_notes.sql      # portable schema (TEXT ids, RFC3339 TEXT timestamps)
  tests/
    api.rs              # axum-test: health/echo/404, notes 503s, SQLite CRUD, Postgres parity via TEST_DATABASE_URL
  Dockerfile            # local dev image (curl + HEALTHCHECK, WORKDIR /data)
  Dockerfile.vercel     # Vercel container Function image (slim, listens on $PORT)
  .env.example          # PORT, RUST_LOG, DATABASE_URL (SQLite default), ALLOWED_ORIGINS / FRONTEND_URL
knowledge/
  architecture.md       # layout, router, backend, build pipeline, base path, PWA
  backend.md            # Axum routes, CORS, env, Docker
  database.md           # sqlx guide: SQLite default, Postgres path, portability contract, recipes
  auth.md               # auth cookbook (sessions vs JWT vs providers) — docs-only, no code ships
  vercel-deploy.md      # Vercel services, env vars, previews, custom domains, troubleshooting
  content-authoring.md  # markdown pipeline, frontmatter, assets
  deployment.md         # CI, local Docker preview, non-Vercel hosts
  frontend-patterns.md  # route/lazy, localStore, async, backend call, form, query recipes
```

## API

```
GET  /api/health → 200 { "status": "ok" }
POST /api/echo   → 200 { "message": "<trimmed>" }
                    422 { "error": "message must not be empty" }
                    422 { "error": "message must be at most 280 characters" }
POST /api/notes      { "body": "…" } → 200 Note
GET  /api/notes                          → 200 Note[] (newest-first)
GET  /api/notes/{id}                     → 200 Note | 404
DELETE /api/notes/{id}                   → 200 Note | 404
(notes routes → 503 { "error": "database is not configured" } only with DATABASE_URL=none)
```

The frontend calls it via `apiUrl()` (`frontend/src/lib/api.ts`) — empty `VITE_API_URL` means same-origin (Vite proxy in dev, `vercel.json` rewrite in prod). The backend service receives the original path, so Axum routes keep the `/api` prefix. Persistence: SQLite file locally (`DATABASE_URL=sqlite://./dev.db?mode=rwc`, the default), Postgres on Vercel — see `knowledge/database.md`. Auth: cookbook only, see `knowledge/auth.md`.

## Continuous integration

`.github/workflows/ci.yml` runs on every push to `main` and on every pull request:

- Frontend (`frontend/`): `oven-sh/setup-bun` → `bun install --frozen-lockfile` → `bun run check` → `bun run test` → `bun run build`
- Backend (`backend/`): Rust stable → `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` (Postgres service container provides `TEST_DATABASE_URL` for the parity test)

Deploy itself is Vercel Git integration (Preview per push, Production on `main`) — see below.

## Deploy to Vercel

One project, two services (`vercel.json`): `frontend` static-build + `backend` container Function. Full guide: `knowledge/vercel-deploy.md`.

1. **Import** the repo in Vercel (Add New → Project → Import). No Root Directory override — services own their roots.
2. **Set env vars** (Production + Preview):
   - `SITE_URL=https://<project>.vercel.app/` (frontend build — absolute sitemap URLs)
   - `ALLOWED_ORIGINS=https://<project>.vercel.app` (backend runtime — strict prod CORS)
   - `DATABASE_URL=<marketplace Postgres URL>` (backend runtime — only when using notes/auth persistence; `none` = notes return `503`)
   - `RUST_LOG=info` (backend runtime — log verbosity)
3. **Push to `main`.** Every push gets Preview URLs for the whole fullstack; production deploys from `main`.

Verify: `/` → landing page, `/api/health` → `{"status":"ok"}`, `/post/hello/` → SPA deep link.

### Local Docker preview

```bash
cd frontend && bun run build && cd ..  # nginx image COPYs dist/
docker compose up --build              # frontend :8080 + backend :3000
```

### Non-Vercel hosts

Frontend-only: upload `frontend/dist/` anywhere static. Fullstack elsewhere: run `backend/Dockerfile` on any container host, set `VITE_API_URL=https://<backend-host>` at frontend build time and `ALLOWED_ORIGINS=https://<frontend-host>` at runtime. Details in `knowledge/deployment.md`.

## PWA

The service worker is generated at build time with `registerType: "autoUpdate"`.
Edit the manifest name, colors and icons in `frontend/vite.config.ts`. Icons live in
`frontend/public/` and are precached automatically.

## Theming

Design tokens (colors, radius, fonts) are defined in `frontend/src/app.css` under
`@theme` — change them there to re-skin the whole app.
