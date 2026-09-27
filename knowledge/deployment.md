# Deployment

Primary target: **Vercel** (Docker services) — see `vercel-deploy.md`. This file covers CI, the local Docker preview, and non-Vercel static hosts.

## 1. CI (`.github/workflows/ci.yml`)

Every push to `main` + every PR:

1. `oven-sh/setup-bun` → `bun install --frozen-lockfile` (workdir `frontend/`)
2. `bun run check` (svelte-check) → `bun run test` (`bun test`) → `bun run build`
3. `rust-toolchain` (stable) → `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` (workdir `backend/`, Postgres service container provides `TEST_DATABASE_URL` for the parity test)

No config needed on forks. Vercel's own Git integration builds Preview/Production deploys separately on push.

## 2. Local Docker preview

```bash
cd frontend && bun run build && cd ..   # nginx image COPYs dist/
docker compose up --build
# frontend → http://localhost:8080, backend → http://localhost:3000/api/health
```

Services: `backend` (`backend/Dockerfile`, `:3000`, healthchecked, SQLite `DATABASE_URL` default; `backend/.env` optional) + `frontend` (`frontend/Dockerfile.vercel` + `nginx.conf`, `:8080`, waits for healthy backend). Uncomment the `db` block for local Postgres parity (see `database.md` §5). Native alternative without Docker: `cargo run` in `backend/` + `bun run dev` in `frontend/` (Vite proxies `/api` → `:3000`).

## 3. Base path overrides

`frontend/vite.config.ts` defaults `base` to `/` (Vercel serves from the domain root). Override only when embedding under a sub-path:

```bash
BASE_PATH=/my-sub-path/ bun run build   # (run inside frontend/)
```

PWA manifest `start_url`/`scope` follow the same `base`.

## 4. Non-Vercel hosts

Frontend-only hosts (Netlify, Cloudflare Pages, S3, Nginx, …):

```bash
cd frontend && bun run build    # outputs to frontend/dist/
```

- Upload `frontend/dist/` as-is. `dist/404.html` (SPA fallback copy of `index.html`) is harmless — plain static servers ignore it; SPA-style hosts can reuse it as the fallback route.
- Set the canonical root for absolute sitemap/RSS URLs:
  ```bash
  SITE_URL=https://example.com/ bun run build
  ```
  Without `SITE_URL`, builds emit path-rooted sitemap/RSS links. Combine with `BASE_PATH` when serving under a sub-path.
- Fullstack elsewhere: run the backend image (`backend/Dockerfile`) on any container host (Fly.io, Render, Railway, …), set `VITE_API_URL=https://<backend-host>` at frontend build time and `ALLOWED_ORIGINS=https://<frontend-host>` at backend runtime.

## 5. PWA + deploy interaction

The service worker + manifest are generated at build time with `start_url`/`scope` = build `base`. Sub-path moves therefore require a rebuild with the matching `BASE_PATH`, or "Add to Home Screen" scope breaks. Workbox precaches `**/*.{js,css,html,svg,png,ico,woff2,xml,txt}` including the in-band `404.html`, `sitemap.xml`, `rss.xml` and `robots.txt`; `PwaUpdate.svelte` prompts on new versions (`registerType: "autoUpdate"`).
