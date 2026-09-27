# Deploy on Vercel (Docker services)

This template deploys as **one Vercel project, two services** — a static frontend and an Axum backend container — routed by `vercel.json`. The backend image builds from `backend/Dockerfile.vercel` and runs on Fluid compute (autoscaled, Active CPU billing, scale-to-zero); the frontend is a plain static build (no Docker involved on Vercel).

References: [Dockerfile on Vercel](https://vercel.com/blog/dockerfile-on-vercel), [Running Docker on Vercel](https://vercel.com/kb/guide/docker), [Container Images](https://vercel.com/docs/functions/container-images), [Services](https://vercel.com/docs/services), [Services routing](https://vercel.com/docs/services/routing).

## 1. How routing works

Root `vercel.json` (the only deployment config — no dashboard routing needed):

```json
{
  "services": {
    "frontend": {
      "root": "frontend/",
      "installCommand": "bun install --frozen-lockfile",
      "buildCommand": "bun run build",
      "outputDirectory": "dist"
    },
    "backend": {
      "root": "backend/",
      "runtime": "container",
      "entrypoint": "Dockerfile.vercel"
    }
  },
  "rewrites": [
    { "source": "/api/(.*)", "destination": { "service": "backend" } },
    { "source": "/(.*)", "destination": { "service": "frontend" } }
  ]
}
```

- `/api/*` → `backend` service (Axum container). The service receives the **original path** (`/api/health`), so routes keep the `/api` prefix.
- Everything else → `frontend` service (static `dist/`). SPA deep links (`/post/<slug>`) resolve via the catch-all rewrite; the local nginx image mirrors it with `try_files … /index.html`.
- Order matters: the `/api` rule must come first. Vercel evaluates rewrites top-down and routing into a service is final (no fallthrough).

## 2. What each service builds

| Service | Build | Runtime |
|---|---|---|
| `frontend` | Bun static build: `bun install --frozen-lockfile` → `bun run build` → `dist/` | Static file serving (no container on Vercel) |
| `backend` | Docker build of `backend/Dockerfile.vercel` (Rust multi-stage → `debian:bookworm-slim` + binary) | Container Function: listens on `$PORT`, HTTP only |

`frontend/Dockerfile.vercel` (nginx + `dist/`) is **not** used by Vercel — the frontend service uses the static build directly. It exists for local `docker compose` previews and Docker-only hosts; keep it in sync with `frontend/nginx.conf` if routing changes.

## 3. Step-by-step: first deploy

1. **Push the repo to GitHub** (default `main` branch).
2. **Vercel → Add New… → Project → Import** the repo. Leave Root Directory empty — `vercel.json` services own their roots.
3. **Environment variables** (Production + Preview; see §4):
   - `SITE_URL=https://<project>.vercel.app/` — absolute sitemap/RSS/canonical URLs. Update after adding a custom domain.
   - `ALLOWED_ORIGINS=https://<project>.vercel.app` (or `FRONTEND_URL=…`) — strict prod CORS for the backend.
   - `DATABASE_URL=<marketplace Postgres URL>` — only when using notes/auth persistence (see `database.md` §5); `none` = notes return `503`.
   - `RUST_LOG=info` — backend log verbosity.
   - No `VITE_API_URL` needed for same-domain deploys (empty = same-origin).
4. **Deploy.** First build compiles the Rust release binary (several minutes — dependencies cache after that). Verify:
   - `https://<project>.vercel.app/` → landing page.
   - `https://<project>.vercel.app/api/health` → `{"status":"ok"}`.
   - `https://<project>.vercel.app/post/hello/` → deep link works (SPA rewrite).
5. **Custom domain:** Project → Settings → Domains → Add. Then update `SITE_URL` + `ALLOWED_ORIGINS` to the apex domain and redeploy.

## 4. Environment variables

| Var | Where | Example | Required? |
|---|---|---|---|
| `SITE_URL` | frontend service | `https://app.example.com/` | Yes (else sitemap links are path-only) |
| `ALLOWED_ORIGINS` or `FRONTEND_URL` | backend service | `https://app.example.com` | Yes in prod (else permissive dev CORS) |
| `DATABASE_URL` | backend service | Marketplace Postgres URL (pooled), or `none` | Only when using notes/auth persistence (see `database.md` §5); `none` = notes return `503` |
| `RUST_LOG` | backend service | `info` | No (defaults to `info`) |
| `VITE_API_URL` | frontend service | _(empty)_ | Only for split-domain backends |
| `PORT` | backend service | _(injected by Vercel)_ | Never set manually |
| `BASE_PATH` | frontend service | _(empty)_ | Only when embedding under a sub-path |

Service-scoped vs shared: `SITE_URL`/`VITE_API_URL` are read at frontend **build** time; `ALLOWED_ORIGINS`/`RUST_LOG` at backend **runtime**. If the dashboard scopes vars per service, put each in its owner; shared scope also works.

## 5. Behavior on Vercel (read before relying on it)

- **Ports:** containers serve HTTP on `$PORT` (Vercel injects it; default `80`). The Axum binary reads it via `port()` — never hardcode `3000` in backend code.
- **Scale-in:** no traffic for ~5 min (prod) / 30s (preview) → `SIGTERM` + 30s grace → terminate. `main.rs` handles it; design handlers to finish in milliseconds.
- **Stateless:** instances hold nothing between requests. No in-memory sessions, no disk writes, no SQLite file. Persistent state needs a backing service (e.g. Postgres from the Vercel Marketplace).
- **Cold starts:** first request after idle (or a fresh deploy) pays the boot cost — the `debian-slim + binary` image is built for fast starts; dependencies stream on demand. Preview envs feel this most.
- **Observability:** `stdout`/`stderr` (our `tracing` logs) broadcast to inflight requests and land in Vercel Observability like any Function.
- **Limits/pricing:** same as Vercel Functions — Active CPU billing (idle I/O wait is not billed). Secure Compute / Static IPs are **not** supported for container images.
- **Previews:** every push gets immutable preview URLs for the whole fullstack (frontend + backend together) — the recommended way to test `/api` changes before merging.

## 6. Local parity (`vercel dev`)

```bash
npm i -g vercel          # once
vercel link              # connect local checkout to the project
vercel dev               # builds both services, routes like production
```

Needs the `docker` CLI + a running daemon (backend builds as a container). No daemon here? Use the native fallback instead: `cargo run` in `backend/` + `bun run dev` in `frontend/` (Vite proxies `/api` → `:3000`) — same URL contract, no Docker needed.

## 7. Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `/api/*` returns the SPA shell | Rewrites out of order / missing `/api` rule | `/api/(.*) → backend` must precede `/(.*) → frontend` in `vercel.json` |
| `/api/health` 404s with Axum 404 | Route registered without `/api` prefix | Backend receives the original path — keep `/api/health` in `build_router()` |
| CORS errors on previews | `ALLOWED_ORIGINS` pins production only | Add preview URLs or `*`-less list per env; same-origin prod calls bypass CORS |
| Sitemap has path-only URLs | `SITE_URL` unset at build time | Set `SITE_URL=https://<domain>/` and redeploy (build-time var) |
| Slow first backend build | Rust release compile from scratch | Expected once; later builds cache layers. Don't downgrade to `debug` for prod |
| Container boot loops | Binary not listening on `$PORT` / crashing on env | Check Runtime Logs; `port()` falls back to `3000` — ensure `Dockerfile.vercel` `CMD` runs `/backend` |
| `dist/` stale in Docker preview | `frontend/dist` not rebuilt before `compose build` | `cd frontend && bun run build` before `docker compose up --build` |
