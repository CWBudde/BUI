// The Bun plugin: `.svelte` files and Svelte's browser build. Bun's runtime
// loads it through `register.js`; `bui build` finds it through this
// package's `"bui": { "bunPlugin" }` field and bundles with it.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import fs from 'node:fs'
import path from 'node:path'
import { SVELTE_FILE, BROWSER_PACKAGE, BROWSER_PACKAGE_FILE, SELF, SELF_FILE, compileSvelte, resolveBrowser, browserRedirect } from './compile.js'

export default {
  name: '@blender-ui/svelte',
  setup (build) {
    build.onResolve({ filter: SELF }, () => ({ path: SELF_FILE }))
    build.onResolve({ filter: BROWSER_PACKAGE }, (args) =>
      ({ path: resolveBrowser(args.path, args.importer ? path.dirname(args.importer) : process.cwd()) }))
    // Bun's runtime resolves bare imports without plugins: its server build
    // of Svelte loads the browser one instead
    build.onLoad({ filter: BROWSER_PACKAGE_FILE }, (args) => {
      return { contents: browserRedirect(args.path) ?? fs.readFileSync(args.path, 'utf8'), loader: 'js' }
    })
    build.onLoad({ filter: SVELTE_FILE }, (args) =>
      ({ contents: compileSvelte(fs.readFileSync(args.path, 'utf8'), args.path), loader: 'js' }))
  },
}
