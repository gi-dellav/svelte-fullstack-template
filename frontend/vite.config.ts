import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { VitePWA } from "vite-plugin-pwa";
import { mdPlugin } from "./plugins/md.js";
import { resolveSiteRoot, seoPlugin } from "./plugins/seo.js";
import { resolveBase } from "./src/lib/base.js";

export type { BaseOptions } from "./src/lib/base.js";
export { resolveBase } from "./src/lib/base.js";

export default defineConfig(() => {
  // Vercel serves the frontend from the domain root, so the default base is
  // `/`. Only override with BASE_PATH when embedding under a sub-path
  // (e.g. BASE_PATH=/my-sub-path/ bun run build).
  // Treat an empty BASE_PATH as "not set".
  const baseOverride = process.env["BASE_PATH"]?.trim() || undefined;
  const base = resolveBase({ baseOverride });
  // Canonical root for sitemap.xml / rss.xml / canonical links. `SITE_URL`
  // (e.g. `https://example.com/`) wins; otherwise fall back to the build
  // base. On Vercel, set SITE_URL to the production URL so the sitemap
  // carries absolute URLs.
  const siteRoot = resolveSiteRoot({
    siteUrlOverride: process.env["SITE_URL"]?.trim() || undefined,
    base,
  });

  return {
    base,
    server: {
      // Local fullstack dev: `PORT=3000 cargo run` in backend/, then
      // `bun run dev` here — `/api/*` calls hit Axum without CORS issues.
      proxy: {
        "/api": "http://127.0.0.1:3000",
      },
    },
    plugins: [
      mdPlugin(),
      seoPlugin({ siteRoot }),
      svelte(),
      tailwindcss(),
      VitePWA({
        registerType: "autoUpdate",
        includeAssets: ["favicon.svg", "apple-touch-icon.png"],
        manifest: {
          name: "Svelte Fullstack Template",
          short_name: "Fullstack",
          description: "A fullstack Svelte + Axum template on Vercel.",
          theme_color: "#0f172a",
          background_color: "#0f172a",
          display: "standalone",
          start_url: base,
          scope: base,
          icons: [
            {
              src: "pwa-192x192.png",
              sizes: "192x192",
              type: "image/png",
            },
            {
              src: "pwa-512x512.png",
              sizes: "512x512",
              type: "image/png",
            },
            {
              src: "pwa-maskable-512x512.png",
              sizes: "512x512",
              type: "image/png",
              purpose: "maskable",
            },
          ],
        },
        workbox: {
          globPatterns: ["**/*.{js,css,html,svg,png,ico,woff2,xml,txt}"],
          cleanupOutdatedCaches: true,
        },
        devOptions: {
          enabled: false,
        },
      }),
    ],
  };
});
