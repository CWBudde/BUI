// `import '@blender-ui/svelte/register'` (or `node --import` / `bun --preload`
// it): `.svelte` files import as components, and `svelte` resolves to its
// browser build, bundled once (runtime.js). Node.js through module hooks,
// Bun through its plugin.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { SVELTE_FILE, BROWSER_PACKAGE, SELF, selfFile, compileSvelte, resolveBrowser } from './compile.js'
import { runtimePlugin } from './plugin.js'
import { runtimeBundle } from './runtime.js'

const bundle = runtimeBundle() // Svelte's runtime as a few modules (null: its source files)

if (typeof Bun !== 'undefined') {
  Bun.plugin(runtimePlugin(bundle))
} else {
  const { registerHooks } = await import('node:module')
  if (typeof registerHooks !== 'function') throw new Error('@blender-ui/svelte: needs Node.js 22.15 or newer (module.registerHooks), or Bun')
  registerHooks({
    resolve (specifier, context, next) {
      if (SELF.test(specifier)) return { url: pathToFileURL(selfFile(specifier)).href, format: 'module', shortCircuit: true }
      if (!BROWSER_PACKAGE.test(specifier)) return next(specifier, context)
      const from = context.parentURL?.startsWith('file:') ? path.dirname(fileURLToPath(context.parentURL)) : process.cwd()
      const file = resolveBrowser(specifier, from)
      return { url: pathToFileURL(bundle?.get(file) ?? file).href, format: 'module', shortCircuit: true }
    },
    load (url, context, next) {
      if (!url.startsWith('file:') || !SVELTE_FILE.test(url)) return next(url, context)
      const file = fileURLToPath(url)
      return { format: 'module', source: compileSvelte(fs.readFileSync(file, 'utf8'), file), shortCircuit: true }
    },
  })
}
