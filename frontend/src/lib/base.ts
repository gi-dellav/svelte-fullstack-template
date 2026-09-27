export interface BaseOptions {
  baseOverride: string | undefined;
}

/**
 * Resolve the Vite `base`.
 *
 * Vercel serves the frontend from the domain root, so the default is `/`.
 * Only `BASE_PATH` overrides it (e.g. `BASE_PATH=/my-sub-path/` when
 * embedding under a sub-path). No `process.env` access — pass the trimmed
 * `BASE_PATH` value (or `undefined`) in.
 */
export function resolveBase({ baseOverride }: BaseOptions): string {
  return normalizeBaseOverride(baseOverride) ?? "/";
}

/** Normalize a user-supplied base: tolerate missing/extra slashes, reject empties. */
export function normalizeBaseOverride(baseOverride: string | undefined): string | undefined {
  const trimmed = baseOverride?.trim() ?? "";
  if (trimmed === "") return undefined;
  const withLeading = trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
  return withLeading.endsWith("/") ? withLeading : `${withLeading}/`;
}
