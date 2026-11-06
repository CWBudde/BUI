// `import '@blender-ui/svelte/register'` (or `node --import` / `bun --preload`
// it): `.svelte` files import as components, and `svelte` resolves to its
// browser build. Node.js through module hooks, Bun through its plugin.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { SVELTE_FILE, BROWSER_PACKAGE, SELF, SELF_FILE, compileSvelte, resolveBrowser } from './compile.js'
import plugin from './plugin.js'

if (typeof Bun !== 'undefined') {
  Bun.plugin(plugin)
} else {
  const { registerHooks } = await import('node:module')
  if (typeof registerHooks !== 'function') throw new Error('@blender-ui/svelte: needs Node.js 22.15 or newer (module.registerHooks), or Bun')
  registerHooks({
    resolve (specifier, context, next) {
      if (SELF.test(specifier)) return { url: pathToFileURL(SELF_FILE).href, format: 'module', shortCircuit: true }
      if (!BROWSER_PACKAGE.test(specifier)) return next(specifier, context)
      const from = context.parentURL?.startsWith('file:') ? path.dirname(fileURLToPath(context.parentURL)) : process.cwd()
      return { url: pathToFileURL(resolveBrowser(specifier, from)).href, format: 'module', shortCircuit: true }
    },
    load (url, context, next) {
      if (!url.startsWith('file:') || !SVELTE_FILE.test(url)) return next(url, context)
      const file = fileURLToPath(url)
      return { format: 'module', source: compileSvelte(fs.readFileSync(file, 'utf8'), file), shortCircuit: true }
    },
  })
}
