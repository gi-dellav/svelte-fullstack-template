/**
 * Backend API base URL.
 *
 * - Local dev: empty → same-origin, so `/api/*` hits the Vite proxy
 *   (`frontend/vite.config.ts` forwards to Axum on `:3000`).
 * - Production/split deploys: set `VITE_API_URL=https://<backend-host>`
 *   (Vercel project env). Same-origin when the backend serves `/api/*`
 *   behind the same domain via `vercel.json` rewrites.
 */
export const API_URL: string = (import.meta.env["VITE_API_URL"] as string | undefined)?.replace(
  /\/$/,
  "",
) ?? "";

/** Build an API URL honoring `VITE_API_URL`. `path` must start with `/`. */
export function apiUrl(path: string): string {
  return `${API_URL}${path}`;
}
