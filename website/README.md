# BUI website

The documentation site for [BUI](https://github.com/ppmpreetham/BUI): an
[Astro](https://astro.build) static site with MDX content, Tailwind CSS 4 and
a little vanilla JavaScript (tabs, search, theme toggle, the landing-page
demos). No framework runtime ships to the browser.

**Writing docs?** Read [WRITING.md](WRITING.md): voice, page template,
components, and the list of planned pages to claim.

## Develop

Astro 7 needs **Node.js 22.23 or newer** (or Bun).

```sh
cd website
pnpm install        # or: npm install
pnpm dev            # http://localhost:4321
```

With an older Node, run Astro on Bun instead: `bun --bun run dev`.

| Command | What it does |
| --- | --- |
| `pnpm dev` | Dev server with live reload |
| `pnpm build` | Static site in `dist/` |
| `pnpm preview` | Serve `dist/` locally |
| `pnpm check` | Type-check components, pages and content frontmatter |

## Deploy

`pnpm build` writes a plain static site to `dist/`. Upload it to any static
host (GitHub Pages, Cloudflare Pages, Netlify, an S3 bucket …). Set the
address at build time:

```sh
SITE_URL=https://example.org pnpm build                 # served at the root
SITE_URL=https://user.github.io BASE_PATH=/BUI pnpm build   # served under /BUI/
```

`SITE_URL` is used for canonical links, Open Graph images and the sitemap
(default: `https://bui.preetham.top`, see `astro.config.mjs`). Every internal
link goes through `href()` in `src/lib/docs.ts`, so a sub-path deploy works.
The host should serve `404.html` for unknown paths (most do by default).

### bui.preetham.top

The production site is `https://bui.preetham.top`, which is already the
default `SITE_URL`, so a plain `pnpm build` produces the right canonical
links, Open Graph URLs, `sitemap-index.xml` and `robots.txt`. The output is a
static `dist/` folder: any static host works. Settings for the two common
ones:

| Setting | Cloudflare Pages | Netlify |
| --- | --- | --- |
| Root / base directory | `website` | `website` |
| Build command | `pnpm install --frozen-lockfile && pnpm build` | same |
| Output / publish directory | `dist` | `dist` |
| Node.js | `NODE_VERSION=22.23.4` or newer (`engines` in `package.json`) | same |
| Package manager | pnpm (from `pnpm-lock.yaml`; both hosts detect it) | same |
| Environment | none needed; set `SITE_URL` only for a preview domain | same |

With an older Node on the host, build with Bun instead (`bun install && bun --bun run build`; tested with Bun 1.4).

Then point the `bui.preetham.top` DNS record at the host (a CNAME to the
Pages / Netlify site) and enable HTTPS there. No redirects or headers are
required: `/docs` redirects to the introduction through a generated HTML page,
and unknown paths get `404.html`.

## How it's put together

```
website/
  astro.config.mjs        site URL, integrations (MDX, sitemap, Expressive Code), Tailwind
  ec.config.mjs           code block themes and colors (Expressive Code)
  WRITING.md              the writers' guide
  public/                 favicon, OG image, screenshots (images/ copied from ../docs/images)
  src/
    config/nav.ts         ★ the docs sections, their order, the language list, top links
    config/site.ts        name, description, repository links
    content.config.ts     the docs schema (frontmatter fields)
    content/docs/**.mdx   ★ the pages: one file = one page
    components/docs/      MDX components (Callout, LangTabs, Steps, Figure, StatusTable …)
    components/site/      header, sidebar, table of contents, search palette, footer, icons
    components/landing/   the landing page sections and demos
    layouts/              Base (head, theme) and DocsLayout (sidebar, TOC, prev/next)
    lib/docs.ts           ordering, prev/next, links, last-updated dates
    lib/string-search.ts  Blender's fuzzy search, ported (docs search, landing demo)
    lib/shots.ts          finds light/dark screenshot pairs in public/images/shots/
    data/bench.ts         every benchmark number (charts and tables read it)
    components/charts/    BenchChart, BarChart, GroupedBarChart, LineChart, BenchTable
    pages/                index (landing), docs/[...slug], search.json, 404
    styles/global.css     design tokens, prose styles, the hatch
```

- **Sidebar, breadcrumbs, previous/next, search and "Soon" badges** are
  generated from the content collection. Adding a page is adding one `.mdx`
  file.
- **Search** is a static index (`/search.json`) built with the site and
  searched in the browser with a port of Blender's menu search
  (`src/lib/string-search.ts`): <kbd>Ctrl</kbd>+<kbd>K</kbd>, <kbd>Cmd</kbd>+<kbd>K</kbd>
  or <kbd>/</kbd>.
- **Last updated** comes from the page's last git commit (or the file's
  modification time before it's committed).
- **Theme**: light and dark follow the system until the reader picks one with
  the toggle (stored in `localStorage` as `bui:theme`).

## Design tokens

Defined once in `src/styles/global.css` and exposed to Tailwind (`bg-surface`,
`text-muted`, `border-border`, `text-link`, `bg-primary` …).

| Token | Light | Dark | Use |
| --- | --- | --- | --- |
| `--primary` | `#4772b3` | `#4772b3` | Blender's selection blue: active nav, buttons, BUI's bars in charts |
| `--primary-soft` | `#e3ebf6` | `#253041` | Quiet blue surface: selected search result |
| `--accent` | `#c94a00` | `#ed5700` | Blender's active orange; sparingly |
| `--link` | `#2f5896` | `#8cb0ea` | Link text (contrast-safe blue) |
| `--background` | `#fbfbfa` | `#1b1b1b` | Page |
| `--surface` / `--surface-2` | `#f2f2f0` / `#e8e8e5` | `#222222` / `#2b2b2b` | Panels, table heads, inline code |
| `--foreground` / `--muted` | `#161616` / `#545452` | `#e8e8e8` / `#a6a6a6` | Text / secondary text |
| `--border` / `--border-strong` / `--border-soft` | `#d3d3cf` / `#b4b4b0` / `#e5e5e1` | `#383838` / `#4c4c4c` / `#2d2d2d` | Structure lines / emphasis / row dividers |
| `--hatch` | blue 24% | blue 42% | The 135° hatch dividers (`.hatch`) |
| `--stage` | `#eceeef` | `#141414` | Behind screenshots |
| `--code-bg` | `#f6f6f4` | `#171717` | Code blocks |
| `--chart-*` | greys and blues | same, darker | Charts (`src/styles/charts.css`) |
| `--status-*` | stable, preview, experimental, soon, warn | lighter in dark | Status icons and callouts |
| `--bl-*` | Blender's default theme (`region #303030`, `widget #545454` …) | same | Demos that imitate Blender widgets |

Icons are [Phosphor](https://phosphoricons.com) (MIT), inlined at build time by
`src/components/site/Icon.astro` from `@phosphor-icons/core`. No emoji in the UI.

Motion: `[data-reveal]` elements and charts fade or grow in when scrolled
into view (Base.astro), only with JavaScript and without
`prefers-reduced-motion`. <kbd>Ctrl</kbd>+<kbd>Z</kbd> outside a text field
goes back a page.

Type: Inter Variable (Blender's UI font) for text, JetBrains Mono Variable for
code, both self-hosted through Fontsource. Corners are square; only the
Blender-widget demos use Blender's 4 px rounding.
