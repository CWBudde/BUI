# Notes for agents working on the website

- Writing or editing docs pages: follow [WRITING.md](WRITING.md). Pages are
  `src/content/docs/**.mdx`; sections are defined in `src/config/nav.ts`.
- Check facts against the repository (one level up), not memory. List the
  files you checked in the page's `sources:` frontmatter.
- Only change files inside `website/`.
- `pnpm build` must pass without warnings. Astro 7 needs Node 22.23+; with an
  older Node use `bun --bun run build`.
- Start the dev server in the background (`astro dev --background`; manage it
  with `astro dev stop | status | logs`). Astro's docs: https://docs.astro.build
