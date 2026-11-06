// The Bun plugin: `.svelte` files and Svelte's browser build. `bui build`
// finds the default export through this package's `"bui": { "bunPlugin" }`
// field and bundles with it; Bun's runtime loads `runtimePlugin(bundle)`
// through register.js, which also serves runtime.js's bundle.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import fs from 'node:fs'
import path from 'node:path'
import { SVELTE_FILE, BROWSER_PACKAGE, BROWSER_PACKAGE_FILE, SELF, selfFile, compileSvelte, resolveBrowser, browserTarget, reexport } from './compile.js'

/** The plugin; `bundle` (runtime.js's map) serves Svelte's modules bundled. */
export function runtimePlugin (bundle = null) {
  return {
    name: '@blender-ui/svelte',
    setup (build) {
      build.onResolve({ filter: SELF }, (args) => ({ path: selfFile(args.path) }))
      build.onResolve({ filter: BROWSER_PACKAGE }, (args) =>
        ({ path: resolveBrowser(args.path, args.importer ? path.dirname(args.importer) : process.cwd()) }))
      // Bun's runtime resolves bare imports without plugins: a Svelte module
      // it loads re-exports the one it should be (the browser build, bundled)
      build.onLoad({ filter: BROWSER_PACKAGE_FILE }, (args) => {
        const target = browserTarget(args.path)
        const served = bundle?.get(target) ?? target
        return { contents: served === args.path ? fs.readFileSync(args.path, 'utf8') : reexport(served), loader: 'js' }
      })
      build.onLoad({ filter: SVELTE_FILE }, (args) =>
        ({ contents: compileSvelte(fs.readFileSync(args.path, 'utf8'), args.path), loader: 'js' }))
    },
  }
}

export default runtimePlugin()
