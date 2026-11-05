// @ts-check
import { defineConfig } from "astro/config";
import mdx from "@astrojs/mdx";
import sitemap from "@astrojs/sitemap";
import expressiveCode from "astro-expressive-code";
import tailwindcss from "@tailwindcss/vite";

// Where the site is deployed. Override at build time:
//   SITE_URL=https://example.org BASE_PATH=/docs pnpm build
const SITE_URL = process.env.SITE_URL ?? "https://bui.preetham.top";
const BASE_PATH = process.env.BASE_PATH ?? "/";

export default defineConfig({
  site: SITE_URL,
  base: BASE_PATH,
  // "ignore": /docs/x and /docs/x/ both work, and unknown paths get our 404
  // page in dev too (with "always", Astro answers a missing slash with its own
  // error page). The build still writes /docs/x/index.html.
  trailingSlash: "ignore",
  redirects: {
    "/docs": "/docs/start/introduction/",
  },
  integrations: [
    expressiveCode(),
    mdx(),
    sitemap(),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
});
