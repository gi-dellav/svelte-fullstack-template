# knowledge/ — deep-dive guides

Docs-only directory for agents and humans. It is **never imported by `frontend/src/`/`frontend/plugins/`, never referenced in `frontend/vite.config.ts`, and never built into `frontend/dist/`**. If you replace the subsystem a file describes, update or delete that file.

## Index

| File | Read when… |
|---|---|
| `architecture.md` | Touching routing, build pipeline, base path, or PWA. Start here for structural changes. |
| `backend.md` | Touching the Axum API (`backend/`): routes, CORS, env, Docker. |
| `database.md` | Adding tables, touching `sqlx`/`migrations`, switching SQLite ↔ Postgres, seeding auth stores. |
| `auth.md` | Adding login/sessions/JWT, protecting routes, or evaluating providers. Docs-only cookbook — no auth code ships. |
| `vercel-deploy.md` | Deploying to Vercel (services, env vars, previews, custom domains). |
| `content-authoring.md` | Adding/editing markdown posts, frontmatter, or content assets. |
| `deployment.md` | Changing CI, local Docker preview, `BASE_PATH`, PWA manifest — non-Vercel deploys. |
| `frontend-patterns.md` | Adding a route, lazy import, persisted state, async fetch, backend call, form, or query/hash. Recipe book with tested primitives. |

Shorter context: `AGENTS.md` (agent entrypoint), `DESIGN.md` (design system = current `app.css`).
