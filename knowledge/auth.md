# Authentication (cookbook — no auth code ships in this template)

Deliberately docs-only: auth is the easiest code to get subtly wrong and the hardest to keep current, so the template carries **zero** auth dependencies and **zero** auth routes. This guide gives the decision tree plus copy-pasteable recipes that build on `database.md` (pooled `AnyPool`) and `frontend-patterns.md` (`apiUrl`, `AsyncState`). Promote a recipe to `backend/src/` only after it has served a real project — see §7 for when to stop hand-rolling entirely.

## 1. Which approach? (decide first)

| Situation | Use | Why |
|---|---|---|
| Same-origin SPA on Vercel (`vercel.json` rewrites `/api/*` → backend) | **Cookie sessions** (§2) | Browser sends the cookie automatically; XSS can't read `HttpOnly`; CSRF is manageable same-origin; revocation = delete row |
| SPA + mobile app / third-party clients | **JWT access + rotating refresh** (§3) | No cookies on native clients; short-lived access limits blast radius; refresh rotation detects theft |
| Social login, MFA, magic links, email flows | **External provider** (§7) | Don't hand-roll: Clerk / Auth.js / Keycloak / Supabase Auth |

Default for this template's shape (same domain, Svelte SPA): **cookie sessions**. Only reach for JWT when a non-browser client forces you.

## 2. Cookie sessions (recommended recipe)

**Crates:** `tower-sessions` + `async-sqlx-session` (backs the session store with the existing `AnyPool` — same SQLite/Postgres portability as `database.md`), `argon2` (password hashing). **No new migrations framework:** one portable migration:

```sql
-- backend/migrations/0002_sessions.sql (common subset: TEXT keys only)
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    data BLOB NOT NULL,          -- session payload; BLOB works on both drivers
    expiry_time TEXT NOT NULL    -- RFC3339, like notes.timestamps
);
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL, -- argon2id PHC string, never the password
    created_at TEXT NOT NULL
);
```

**Wiring sketch** (`lib.rs`): build the session store from the pool, fail startup loudly when the pool is `None` (unlike notes, auth without a store is a security hole — `expect` here, don't `503`):

```rust
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};

let session_store = async_sqlx_session::SqliteSessionStore::from_client(pool.clone());
// ... or PostgresSessionStore when on DATABASE_URL=postgres://…
let session_layer = SessionManagerLayer::new(session_store)
    .with_secure(true)              // __Host- cookies only over HTTPS (prod)
    .with_same_site(SameSite::Lax)  // enough same-origin; Strict breaks top-level login redirects
    .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
```

**Routes** (all under `/api/auth/*`, same error envelope):

```
POST /api/auth/signup  { email, password } → 200 { id, email } | 422 (bad input) | 409 (email taken)
POST /api/auth/login   { email, password } → 200 { id, email } (sets session cookie) | 401
POST /api/auth/logout                          → 200 (destroys session, clears cookie)
GET  /api/auth/me                              → 200 { id, email } | 401 (no/expired session)
```

**Handler rules:**

- Hash with `argon2` (`Argon2::default()`, random salt per user via `SaltString::generate`); verify with `PasswordHash` + `verify_password`. Verification takes ~100ms by design — don't "optimize" the params down.
- Compare emails case-insensitively (`LOWER(email) = LOWER(?)` — portable); store the original.
- Login response must not reveal "email exists vs wrong password": both failures → `401 { "error": "invalid credentials" }`. Signup's `409` may reveal existence — acceptable, but rate-limit it.
- `GET /me` reads `session.get::<String>("user_id")`; missing → `401`. Wrap this in an `AuthUser` extractor (`FromRequestParts`) so protected routes take `AuthUser` as a parameter instead of repeating session code.
- Rotate the session id on login (`session.cycle_id().await`) — prevents session fixation.
- CSRF: same-origin cookie POSTs via `fetch` need no token **if** you check `Origin`/`Sec-Fetch-Site` on mutations or keep `SameSite=Lax` (top-level GET navigations can't mutate). Add an explicit `Origin` allow-check when `ALLOWED_ORIGINS` is set.

**Secrets:** `AUTH_SECRET` (or reuse the session-layer key) from env — generate with `openssl rand -base64 32`, store in Vercel backend-service env, never commit. Dev may use a fixed throwaway documented as insecure.

## 3. JWT access + refresh (when §1 forces you)

**Crates:** `jsonwebtoken` (access tokens), `argon2` (passwords as above). Refresh tokens are opaque random strings stored hashed in the DB (same TEXT-id pattern):

```sql
CREATE TABLE refresh_tokens (
    id TEXT PRIMARY KEY,          -- the token's jti; store sha256(token), never raw
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0
);
```

**Rules:**

- Access: 15 min expiry, claims `{ sub: user_id, exp }`, HS256 with `AUTH_SECRET` (or RS256 when multiple services verify). Sent as `Authorization: Bearer <token>`; frontend keeps it **in memory only** (a `$state`, never `localStorage` — XSS steals stored tokens silently).
- Refresh: 30-day opaque token in an `HttpOnly` cookie (web) or secure device storage (native), single-use rotation — each use issues a new pair and revokes the old; reuse of a revoked token revokes the whole family (theft signal).
- Extractor: `FromRequestParts` impl verifying signature + `exp`, mapping failures to `401 { "error": "…" }` — never `500` on a bad token, never log the token.
- Logout = revoke refresh family; access dies within 15 min by expiry.

## 4. Frontend wiring (both approaches)

Cookies (sessions):

```ts
// apiUrl() as usual, but cookies need credentials on cross-origin previews:
fetchJson<User>(apiUrl("/api/auth/me"), { credentials: "include" });
```

Same-origin production sends cookies automatically; set `credentials: "include"` anyway so Vercel preview URLs (also same-origin per deployment) keep working. Login state lives in a shared runes store seeded from `/me`; on `401` redirect to the login route (add it via `frontend-patterns.md` §1).

JWT: keep access token in a module-scope `$state` (cleared on reload → silent refresh via the cookie on boot), attach via `headers: { Authorization: \`Bearer ${token}\` }`. Refresh failures → logged-out state, same redirect.

## 5. Hardening checklist (both)

- [ ] Rate-limit `/login` + `/signup` (`tower_governor` or provider-level): 5–10 attempts/min/IP, `429` with `Retry-After`.
- [ ] `tracing`: log user ids and outcomes, **never** passwords, hashes, tokens, or session payloads.
- [ ] Validation mirrors notes: trim, length-cap, `422` in the `{ error }` envelope. Email format check is UX, not security — the DB `UNIQUE` + `409` is the enforcement.
- [ ] Cookie flags in prod: `Secure`, `HttpOnly`, `SameSite=Lax`, `__Host-` prefix (requires `Secure` + path `/` + no `Domain`).
- [ ] `ALLOWED_ORIGINS` set to the real frontend origin(s) — sessions over permissive CORS leak.
- [ ] Clock skew: JWT `leeway` ~60s; session expiry `OnInactivity` rather than absolute where UX allows.

## 6. Testing auth (follow the template's pyramid)

- Pure: email normalization, password-policy validators → `bun test` / `cargo test` unit tests.
- Contract: `axum-test` + `memory_pool()` (§database.md §6) — signup → login → `/me` 200 → logout → `/me` 401; wrong password → 401 without user enumeration; expired session → 401; JWT rotation reuse → family revoked.
- Postgres parity: same suite behind `TEST_DATABASE_URL` (CI provides it).
- Never assert on real hashes/tokens beyond shape (`startsWith("$argon2id$")`, JWT 3 segments); use fixed low-cost argon2 params **only** in tests.

## 7. When not to hand-roll (graduation criteria)

Promote to an external provider when any of these appear: social/OIDC login, MFA/WebAuthn, password-reset emails, user admin panels, or compliance requirements (SOC2/GDPR deletion workflows). Candidates: Clerk (hosted UI + Svelte SDK), Auth.js (self-hosted sessions), Keycloak (self-hosted OIDC), Supabase Auth (pairs with Supabase Postgres from `database.md` §5). The `/me`-shaped frontend contract survives the migration — only the backend issuer changes.
