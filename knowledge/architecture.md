# Architecture

Stack: **Svelte 5 (runes) + Vite 8 + TailwindCSS 4 (Vite plugin) + `vite-plugin-pwa` + Bun** for the frontend (`frontend/`), **Rust + Axum 0.8 + Tokio** for the API (`backend/`). Deploy target: **Vercel Services with Docker** (`vercel.json` + `*/Dockerfile.vercel`) — see `vercel-deploy.md`.

## 1. Repo layout

```
vercel.json              # services (frontend/backend) + top-level rewrites
frontend/                # Svelte SPA: owns index.html, vite.config.ts, src/, plugins/, public/, tests/
  src/main.ts → App.svelte   # entry + route switch (home | post <slug> | not-found)
  src/lib/api.ts             # apiUrl() — VITE_API_URL-aware backend URLs
  Dockerfile.vercel + nginx.conf  # static-file container (local preview, non-Vercel hosts;
                                  # Vercel itself serves the static build, no container)
  dist/                      # `bun run build` output (served by Vercel static-build)
backend/                 # Axum crate: build_router(state) in src/lib.rs, binary in src/main.rs
  src/db.rs              # AnyPool over DATABASE_URL (SQLite default, Postgres when set)
  migrations/            # embedded SQL (0001_notes.sql), applied once at startup
  src/routes/health.rs   # GET /api/health → { status: "ok" }
  src/routes/echo.rs     # POST /api/echo { message } → { message }
  src/routes/notes.rs    # notes CRUD → Note JSON / 422 / 404 / 503 (no DB)
  Dockerfile             # local dev image (HEALTHCHECK, curl)
  Dockerfile.vercel      # Vercel Function image (slim, listens on $PORT)
docker-compose.yml       # local fullstack: backend :3000 + frontend :8080
```

Frontend and backend share nothing at build time — only the `/api/*` URL
contract and the `vercel.json` rewrite table.

## 2. Entry & route switch

`frontend/index.html` → `frontend/src/main.ts` (imports Geist Fontsource fonts + `app.css`, mounts `App` on `#app`) → `frontend/src/App.svelte`:

- `route = $state<Route>({ name: "home" })`; `activePost = $derived(...)` via `getPost(slug)`.
- `onMount` wires `popstate` (recompute route + scroll to top) and a document-level `click` listener delegating to `handleLinkClick`. Cleanup removes both.
- Three states: `home` (demo landing + backend health check), `post` (renders `frontend/src/routes/Post.svelte` or an inline 404 block), `not-found` (shows `route.path`).
- Every state mounts `<Seo/>` (per-route `<title>`/description/canonical/OG via `<svelte:head>`, `noindex` on 404s).
- `<PwaUpdate/>` is always mounted (toast UI for SW updates; `onDestroy` clears the auto-dismiss timer).

## 3. Router (`frontend/src/lib/router.ts`)

`Route = { home } | { post, slug } | { not-found, path }`.

- `basePath()` reads `import.meta.env.BASE_URL` (trailing slash stripped).
- `pathWithoutBase(pathname)` strips the configured base so matching always runs on `/…`.
- `parseRoute` matches `/` → home, `/post/<slug>` (trailing slash tolerated, `decodeURIComponent` on slug) → post, else not-found. Query/hash are ignored by matching; parse them per-route with `currentQuery()` / `currentHash()` (pure cores `parseQuery` / `parseHash`, see `frontend-patterns.md` §6).
- `withBase(path)` prefixes `BASE_URL` for every internal `href`. `navigate(path)` pushes `withBase`d URL and dispatches `popstate`.
- `handleLinkClick(event)` keeps navigation client-side: ignores non-left-click, modifier keys, `target="_blank"`, `download`, `rel="external"`, `#`/`mailto:`/absolute-scheme hrefs, and cross-origin URLs. Same-path navigations only scroll to `url.hash` if present. Everything else is intercepted, `preventDefault`ed, and pushed via history API.

**Rules:** build internal links with `withBase()`, navigate with `navigate()`/`handleLinkClick()`, match with `pathWithoutBase()`. Never hardcode `/`-rooted hrefs in app code. External links: `target="_blank" rel="noreferrer"`.

Adding a route: full 6-step recipe in `frontend-patterns.md` §1 (extend `Route`, `parseRoute` branch, `App.svelte` arm + `<Seo/>`, `withBase()` link, router test, `check`/`test`/`build`).

## 4. Backend (`backend/`, Axum + sqlx)

One shared router factory, two runtimes:

- `src/lib.rs` → `build_router(state)` wires `/api/health` + `/api/echo` + `/api/notes*`, `TraceLayer`, and the CORS layer. `port()` reads `$PORT` (default `3000`). `AppState { pool: Option<AnyPool> }` carries the DB handle.
- `src/db.rs` → `AnyPool` over `DATABASE_URL` (SQLite default, Postgres when set), embedded `./migrations` via `run_migrations`. See `database.md`.
- `src/routes/notes.rs` → portable notes CRUD (`?` placeholders, TEXT UUID-v7 ids, RFC3339 TEXT timestamps). `503` when no DB is configured.
- `src/main.rs` → `#[tokio::main]`, connects the pool, runs migrations once, binds `0.0.0.0:$PORT`, graceful shutdown on SIGTERM (Vercel scale-in gives 30s) / Ctrl-C.
- No `vercel_runtime` adapter: Vercel runs the **same binary** from `backend/Dockerfile.vercel` as a container Function — it only needs to listen on `$PORT` and speak HTTP.
- Local Docker uses `backend/Dockerfile` (adds `curl` + `HEALTHCHECK`, `WORKDIR /data` for the SQLite file); Vercel uses `backend/Dockerfile.vercel` (slimmer, no healthcheck — Vercel drives health via traffic; never SQLite there, disk is ephemeral).
- CORS: no `ALLOWED_ORIGINS`/`FRONTEND_URL` = permissive dev mode (Vite proxy avoids CORS anyway); set `ALLOWED_ORIGINS=https://<project>.vercel.app` in production. See `backend.md`.
- Auth: no auth code ships — cookbook only (`auth.md`: cookie sessions vs JWT vs providers).

## 5. Build pipeline (`frontend/vite.config.ts`, `frontend/plugins/md.ts`, `frontend/plugins/seo.ts`)

Plugin order: `mdPlugin()` (`enforce: "pre"`) → `seoPlugin()` → `svelte()` → `tailwindcss()` → `VitePWA()`.

1. `mdPlugin.transform` compiles each `frontend/src/content/*.md` at build time (frontmatter via `gray-matter`, GFM via `marked`, heading ids via `github-slugger`); see `content-authoring.md`.
2. `seoPlugin.transformIndexHtml` injects canonical / `og:url` / RSS links into `index.html`; `seoPlugin.generateBundle` emits `sitemap.xml`, `rss.xml`, `robots.txt` in-band (so Workbox precaches them).
3. Svelte compiles runes components (`vitePreprocess` per `svelte.config.js`).
4. Tailwind 4 scans Svelte + generated HTML; component classes come from `@layer components` in `frontend/src/app.css`.
5. `VitePWA` emits the service worker + manifest, precaching `**/*.{js,css,html,svg,png,ico,woff2,xml,txt}` with `cleanupOutdatedCaches`.
6. `mdPlugin.closeBundle` copies `dist/index.html` → `dist/404.html` **in-band** (so Workbox precaches the fallback). Vercel production routing uses the `/(.*) → frontend` rewrite + nginx `try_files … /index.html` — the `404.html` copy is harmless and keeps plain-static hosts working.

Dev proxy: `server.proxy["/api"] → http://127.0.0.1:3000`, so `bun run dev` + `cargo run` behave like production same-origin routing without CORS.

## 6. Base path (domain root on Vercel)

Vercel serves the frontend from the domain root, so `base` defaults to `/`:

- `BASE_PATH` env (trimmed; empty = unset, missing slashes auto-added) wins when set — only needed when embedding under a sub-path.
- Pure core lives in `frontend/src/lib/base.ts` (`resolveBase({ baseOverride })`, `normalizeBaseOverride`); `frontend/tests/base.test.ts` pins the behavior.

The canonical site root (for `sitemap.xml`, `rss.xml`, canonical links) follows `SITE_URL` when set, else the build `base` (set `SITE_URL=https://<project>.vercel.app/` in production for absolute sitemap URLs).

`PwaUpdate` needs no base handling; manifest `start_url`/`scope` are set to `base`. Content asset URLs use the `__BASE__` placeholder replaced at runtime with `import.meta.env.BASE_URL` (see `content-authoring.md`).

## 7. PWA

- `VitePWA({ registerType: "autoUpdate", includeAssets: ["favicon.svg", "apple-touch-icon.png"], manifest: { name, short_name, description, theme_color/background_color #0f172a, display standalone, start_url/scope = base, icons: 192/512/maskable } })`. Icons live in `frontend/public/`.
- `frontend/src/PwaUpdate.svelte`: `registerSW` from `virtual:pwa-register`; `needRefresh` → "A new version is available." + Reload toast; `offlineReady` → "Ready to work offline." + auto-dismiss after 4s. Styled with `.toast*` classes, `fly` transition.
- Keep `theme_color`, `index.html` `theme-color` meta, and app chrome in sync when re-skinning.

## 8. Strictness

Frontend `tsconfig.json`: `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, `noUnusedLocals/Parameters`, `verbatimModuleSyntax` (+ `erasableSyntaxOnly`), `noEmit`. `bun run check` (`svelte-check`) must pass before `bun run build`. Use `import type` for types; narrow `unknown` explicitly.

Backend: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` must pass. Handlers return `Result<Json<T>, ApiError>`; error shape is always `{ "error": "<message>" }`.
