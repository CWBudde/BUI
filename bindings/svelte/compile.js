// Svelte components for BUI: compiled for the client, with DOM-built
// templates (`fragments: 'tree'`: createElement and setAttribute, no HTML
// parser), and Svelte's browser build (the `browser` export condition). One
// place for both, used by the Node.js hooks and the Bun plugin.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import { compile, compileModule } from 'svelte/compiler'
import fs from 'node:fs'
import path from 'node:path'

/** Components (`.svelte`) and rune modules (`.svelte.js`). */
export const SVELTE_FILE = /\.svelte(\.js)?$/
/** Packages whose browser build a BUI app needs (Svelte's runtime, and the
 *  `esm-env` flags it reads). */
export const BROWSER_PACKAGE = /^(svelte|esm-env)(\/.*)?$/

const CONDITIONS = ['browser', 'import', 'default']
let warnedCss = false

/** The client JavaScript of a `.svelte` / `.svelte.js` file. */
export function compileSvelte (source, filename, dev = process.env.NODE_ENV === 'development') {
  if (!filename.endsWith('.svelte')) return compileModule(source, { filename, generate: 'client', dev }).js.code
  const r = compile(source, { filename, generate: 'client', fragments: 'tree', css: 'external', dev })
  if (r.css && !warnedCss) {
    warnedCss = true
    console.warn(`@blender-ui/svelte: ${path.basename(filename)}: <style> blocks aren't applied; style with class (Tailwind) and style`)
  }
  return r.js.code
}

/** The file `specifier` (`svelte`, `svelte/store` ...) names in its package's
 *  browser build, resolved from `fromDir`. */
export function resolveBrowser (specifier, fromDir) {
  const slash = specifier.indexOf('/')
  const name = slash < 0 ? specifier : specifier.slice(0, slash)
  const sub = slash < 0 ? '.' : '.' + specifier.slice(slash)
  for (let dir = fromDir; ; dir = path.dirname(dir)) {
    const root = path.join(dir, 'node_modules', name)
    if (fs.existsSync(path.join(root, 'package.json'))) {
      const target = pick(JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).exports?.[sub])
      if (target === null) throw new Error(`@blender-ui/svelte: ${specifier}: no browser export`)
      return path.join(root, target)
    }
    if (path.dirname(dir) === dir) throw new Error(`@blender-ui/svelte: cannot find ${name} from ${fromDir}`)
  }
}

/** Files of those packages: Bun's runtime resolves them without plugins. */
export const BROWSER_PACKAGE_FILE = /[\\/]node_modules[\\/](svelte|esm-env)[\\/]/
const redirects = new Map() // package root -> Map(default file -> browser file)

/** Module source re-exporting the browser build of `file` when `file` is a
 *  package's non-browser entry (`svelte/src/index-server.js` ...), or null. */
export function browserRedirect (file) {
  const m = /^(.*[\\/]node_modules[\\/](?:svelte|esm-env))[\\/]/.exec(file)
  if (m === null) return null
  let map = redirects.get(m[1])
  if (map === undefined) {
    map = new Map()
    const exports = JSON.parse(fs.readFileSync(path.join(m[1], 'package.json'), 'utf8')).exports ?? {}
    for (const entry of Object.values(exports)) {
      const browser = pick(entry)
      const fallback = typeof entry === 'object' && entry !== null ? pickFrom(entry, ['default']) : null
      if (browser && fallback && browser !== fallback) map.set(path.join(m[1], fallback), path.join(m[1], browser))
    }
    redirects.set(m[1], map)
  }
  const target = map.get(path.normalize(file))
  if (target === undefined) return null
  const to = JSON.stringify(target)
  return `export * from ${to}\n${/\bexport default\b/.test(fs.readFileSync(target, 'utf8')) ? `export { default } from ${to}\n` : ''}`
}

function pick (entry) { return pickFrom(entry, CONDITIONS) }

function pickFrom (entry, conditions) {
  if (typeof entry === 'string' || entry == null) return entry ?? null
  for (const c of conditions) if (c in entry) return pickFrom(entry[c], conditions)
  return null
}
