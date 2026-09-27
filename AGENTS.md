# AGENTS.md

Guidance for AI coding agents working in this repo. Humans: see `README.md` for onboarding; agents: this file is your entrypoint.

## 1. What this is

A fullstack template: **Svelte 5 (runes) + Vite 8 + TailwindCSS 4 + PWA** frontend (`frontend/`, Bun-managed) + **Rust + Axum 0.8 + sqlx** JSON API (`backend/`, Cargo-managed, SQLite by default). No SvelteKit, no external router. Deploys to **Vercel as two services** (`vercel.json`: static frontend + container backend).

- Frontend entry: `frontend/index.html` → `frontend/src/main.ts` → `frontend/src/App.svelte`
- Routing: hand-rolled history-API SPA in `frontend/src/lib/router.ts` (`home | post <slug> | not-found`)
- Content: build-time `.md → HTML` via `frontend/plugins/md.ts`, listed in `frontend/src/lib/posts.ts`, rendered in `frontend/src/routes/Post.svelte`
- Backend: `build_router(state)` in `backend/src/lib.rs` (`GET /api/health`, `POST /api/echo`, notes CRUD); `AnyPool` over `DATABASE_URL` in `backend/src/db.rs` (SQLite default, Postgres when set); binary in `backend/src/main.rs` (migrations + listen on `$PORT`)
- Backend calls: `apiUrl()` in `frontend/src/lib/api.ts` + `AsyncState`/`fetchJson` in `frontend/src/lib/async.ts`
- Styling: design tokens + component classes in `frontend/src/app.css` (see `DESIGN.md`)
- PWA: manifest + service worker in `frontend/vite.config.ts`, update UI in `frontend/src/PwaUpdate.svelte`
- CI: `.github/workflows/ci.yml` (frontend check/test/build + backend fmt/clippy/test); deploy: Vercel Git integration

## 2. You are free to change anything

This template is **fully flexible**. Restructure folders, delete the demo page, swap styling, add dependencies, replace the router or the markdown pipeline — all normal. Treat everything below as current conventions to follow **only while the subsystem they describe still exists**. If you replace a subsystem, update or delete the docs that describe it (`DESIGN.md`, `knowledge/`).

## 3. Commands (Bun for frontend, Cargo for backend)

```bash
cd frontend
bun install       # install frontend dependencies
bun run dev       # start the dev server (proxies /api → 127.0.0.1:3000)
bun run check     # svelte-check type/diagnostics — must pass before build
bun run test      # unit tests with bun test — must pass before build
bun run build     # production build into frontend/dist/
bun run preview   # preview the production build

cd ../backend
cargo run         # start the Axum API on :3000 (PORT env overrides)
cargo fmt         # format (CI runs fmt --check)
cargo clippy -- -D warnings  # lint — must pass
cargo test        # integration tests (axum-test) — must pass

# Local fullstack via Docker (rebuild frontend/dist/ first):
docker compose up --build   # frontend :8080 + backend :3000
```

Bun only for the frontend (test runner is `bun test`, `frontend/tests/*.test.ts`). Cargo only for the backend (`backend/tests/*.rs`).

## 4. Code map

| Path | Role |
|---|---|
| `vercel.json` | Services (`frontend` static-build, `backend` container) + `/api/*` rewrites |
| `frontend/index.html` | Vite entry, mounts `#app`, PWA icon links, `theme-color` |
| `frontend/src/main.ts` | Imports Geist fonts + `app.css`, mounts `App` |
| `frontend/src/App.svelte` | Root route switch; demo landing page (counter, posts list, backend health, deploy guide) |
| `frontend/src/lib/router.ts` | `Route`, `withBase`, `pathWithoutBase`, `navigate`, `currentRoute`, `handleLinkClick`, `parseQuery`, `parseHash` |
| `frontend/src/lib/api.ts` | `API_URL`, `apiUrl(path)` — backend URLs honoring `VITE_API_URL` |
| `frontend/src/lib/base.ts` | `resolveBase({ baseOverride })`, `normalizeBaseOverride` (Vercel root default `/`) |
| `frontend/src/lib/posts.ts` | `import.meta.glob` over `src/content/*.md`, draft filter, date-desc sort |
| `frontend/src/lib/storage.ts` | Never-throw `localStorage` helpers (`StorageLike`, `memoryStorage`, `readStored`, …) |
| `frontend/src/lib/local-store.svelte.ts` | Runes `localStore(key, initial)` factory (check/build-covered, not unit-tested) |
| `frontend/src/lib/async.ts` | `AsyncState` machine + `fetchJson` + `toErrorMessage` |
| `frontend/src/lib/form.ts` | Pure validators (`required`, `emailField`, `minLength`, `validateAll`) |
| `frontend/src/routes/Post.svelte` | Renders `{@html post.html}` inside `article.prose` |
| `frontend/src/content/*.md` | Markdown posts with frontmatter |
| `frontend/plugins/md.ts` | Vite plugin: frontmatter + GFM render + heading ids + asset rewrite + `404.html` |
| `frontend/plugins/seo.ts` | Build-time SEO: head injection (canonical/`og:url`/RSS) + `sitemap.xml`/`rss.xml`/`robots.txt` |
| `frontend/src/Seo.svelte` | Per-route `<title>`/description/canonical/OG via `<svelte:head>` |
| `frontend/src/md.d.ts` | `PostMetadata` / `PostModule` types + `*.md` module declaration |
| `frontend/src/app.css` | Tailwind import, `@theme` tokens, `@layer components` classes |
| `frontend/vite.config.ts` | `mdPlugin`, `seoPlugin`, Svelte, Tailwind, `VitePWA`; root `base` + `/api` dev proxy + `SITE_URL` derivation |
| `frontend/Dockerfile.vercel` + `nginx.conf` | Static-file container for local preview / non-Vercel hosts (Vercel itself uses static-build) |
| `frontend/svelte.config.js` | `vitePreprocess()` |
| `frontend/tsconfig.json` | Strict: `noUncheckedIndexedAccess`, `noUnusedLocals/Parameters`, `verbatimModuleSyntax`, `erasableSyntaxOnly` |
| `backend/src/lib.rs` | `AppState { pool }`, `build_router(state)` factory + CORS layer + `port()` |
| `backend/src/db.rs` | `AnyPool` over `DATABASE_URL`, `connect_pool()`, `run_migrations()` (embedded `./migrations`) |
| `backend/migrations/` | `0001_notes.sql` (portable subset — see `knowledge/database.md`) |
| `backend/src/main.rs` | `#[tokio::main]` binary: pool → migrations → bind `$PORT`, graceful SIGTERM shutdown |
| `backend/src/error.rs` | `ApiError` → `{ error }` JSON envelope (+ `From<sqlx::Error>`) |
| `backend/src/routes/` | `health.rs` (`GET /api/health`), `echo.rs` (`POST /api/echo`), `notes.rs` (notes CRUD) |
| `backend/Dockerfile` | Local dev image (curl + `HEALTHCHECK`) |
| `backend/Dockerfile.vercel` | Vercel container Function image (slim, listens on `$PORT`) |
| `docker-compose.yml` | Local fullstack: backend `:3000` + frontend `:8080` |

## 5. Invariants (while these subsystems exist)

1. **Base-aware links.** Internal links use `withBase(path)`, navigation uses `navigate()` / `handleLinkClick()`, matching uses `pathWithoutBase()`. Never hardcode `/`-rooted hrefs in app code. External links use `target="_blank" rel="noreferrer"`. (Default base is `/` on Vercel; `BASE_PATH` only for sub-path embeds.)
2. **Backend URLs via `apiUrl()`.** Never hardcode the backend origin; never `withBase()` an `/api/*` path. Empty `VITE_API_URL` = same-origin (Vite proxy in dev, `vercel.json` rewrite in prod).
3. **Backend path contract.** The backend service receives the original path — Axum routes keep the `/api` prefix (`/api/health`, not `/health`).
4. **`$PORT`, not `:3000`, in backend code.** Vercel injects `PORT`; `port()` defaults to `3000` locally. Never hardcode the listen port.
5. **SPA fallback.** `mdPlugin.closeBundle()` copies `dist/index.html` → `dist/404.html` **in-band** so Workbox precaches it. Do not replace with a post-build `cp`. Vercel production fallback is the `/(.*) → frontend` rewrite (+ nginx `try_files` in the Docker preview).
6. **Markdown asset convention.** `/x` → base-prefixed; `./x` or `x` → colocated asset in `frontend/public/content/<slug>/x`. Keep `__BASE__` placeholder flow in `frontend/plugins/md.ts`.
7. **PWA scope.** `start_url`/`scope` follow `base`; icons live in `frontend/public/` and are listed in `includeAssets`. Keep manifest `theme_color` in sync with the app chrome.
8. **Strict check.** `bun run check` must pass before `bun run build`. Respect `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, and `verbatimModuleSyntax` (use `import type` where appropriate). Backend: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` must pass.
9. **Stateless backend.** No in-memory sessions, no disk writes — Vercel scale-in kills idle instances. Persistent state needs a backing service.

## 6. Conventions

- **Styling:** reuse `@layer components` classes from `frontend/src/app.css`; change the look by editing `@theme` tokens, not by inlining ad-hoc Tailwind everywhere. Render markdown HTML inside `prose`. See `DESIGN.md`.
- **Interactive patterns:** `knowledge/frontend-patterns.md` is the recipe book (add route + lazy import, `localStore`, async fetch, backend call, form, query/hash). Use its primitives in `frontend/src/lib/`; keep new validators/state pure and `bun test`-covered.
- **Backend patterns:** `knowledge/backend.md` is the API recipe book (add route = handler + registration + `axum-test` case). Keep handlers thin, errors in the `{ error }` envelope.
- **Database:** `knowledge/database.md` owns persistence (portability contract: `?` placeholders, TEXT UUID-v7 ids, RFC3339 TEXT timestamps; new tables = new migration files). SQLite default, Postgres via `DATABASE_URL`.
- **Auth:** docs-only cookbook (`knowledge/auth.md`) — no auth code ships. Cookie sessions vs JWT vs providers decision tree + recipes.
- **Fonts:** via Fontsource (`@fontsource-variable/geist`, `geist-mono`), wired in `main.ts` and `@theme`.
- **Posts:** `frontend/src/content/<slug>.md` with `title` (required), `date`, `description`, `draft` frontmatter. `draft: true` hides the post in `PROD` builds only.
- **TypeScript:** strict; prefer `import type`, avoid unused locals/params, narrow `unknown` frontmatter explicitly.
- **Rust:** `cargo fmt` clean, no `unwrap()`/`expect()` on env or request input in library code (`main.rs` bind may `expect` — startup failure should be loud).

## 7. Where to read next

- `DESIGN.md` — the design system (documents current `app.css`; agents may expand it).
- `knowledge/README.md` — index of deep-dive guides:
  - `knowledge/architecture.md` — layout, router, build pipeline, base path, PWA
  - `knowledge/backend.md` — Axum routes, CORS, env, Docker
  - `knowledge/database.md` — sqlx persistence, SQLite default, Postgres path, recipes
  - `knowledge/auth.md` — auth cookbook (docs-only): sessions vs JWT vs providers
  - `knowledge/vercel-deploy.md` — Vercel services, env vars, previews, custom domains, troubleshooting
  - `knowledge/content-authoring.md` — markdown pipeline, frontmatter, assets
  - `knowledge/deployment.md` — CI, local Docker preview, non-Vercel hosts
  - `knowledge/frontend-patterns.md` — route/lazy, `localStore`, async, backend call, form, query recipes

`knowledge/` is **docs-only**: it lives at the repo root, is never imported by `frontend/src/`/`frontend/plugins/`, and is never built into `frontend/dist/`. Keep it that way.
