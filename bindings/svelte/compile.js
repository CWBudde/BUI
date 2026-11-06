// Svelte components for BUI: compiled for the client, with DOM-built
// templates (`fragments: 'tree'`: createElement and setAttribute, no HTML
// parser), and Svelte's browser build (the `browser` export condition). One
// place for both, used by the Node.js hooks and the Bun plugin.
//
// Compiled files are cached (node_modules/.cache/@blender-ui/svelte, by
// content, Svelte version and mode): an app starts without loading Svelte's
// compiler unless a component changed.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import crypto from 'node:crypto'
import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const HERE = path.dirname(fileURLToPath(import.meta.url))
const VERSION = require('svelte/package.json').version
const CACHE = path.join(process.cwd(), 'node_modules', '.cache', '@blender-ui', 'svelte')
let compiler = null // svelte/compiler (its CommonJS build: one file), on the first cache miss

/** Components (`.svelte`) and rune modules (`.svelte.js`). */
export const SVELTE_FILE = /\.svelte(\.js)?$/
/** Packages whose browser build a BUI app needs (Svelte's runtime, and the
 *  `esm-env` flags it reads). */
export const BROWSER_PACKAGE = /^(svelte|esm-env)(\/.*)?$/
/** This package, as compiled components import it: always this copy (one
 *  event patch, one stylesheet registry), installed next to them or not. */
export const SELF = /^@blender-ui\/svelte$/
export const SELF_FILE = path.join(HERE, 'index.js')

const CONDITIONS = ['browser', 'import', 'default']

/** The client JavaScript of a `.svelte` / `.svelte.js` file. */
export function compileSvelte (source, filename, dev = process.env.NODE_ENV === 'development') {
  const key = crypto.createHash('sha1').update(`${VERSION}\0${dev}\0${filename}\0${source}`).digest('hex')
  const cached = path.join(CACHE, `${key}.js`)
  try {
    return fs.readFileSync(cached, 'utf8')
  } catch {
    const code = compileNow(source, filename, dev)
    try {
      fs.mkdirSync(CACHE, { recursive: true })
      fs.writeFileSync(cached, code)
    } catch {} // a read-only place: no cache
    return code
  }
}

function compileNow (source, filename, dev) {
  compiler ??= require('svelte/compiler')
  if (!filename.endsWith('.svelte')) return compiler.compileModule(source, { filename, generate: 'client', dev }).js.code
  const r = compiler.compile(source, { filename, generate: 'client', fragments: 'tree', css: 'external', dev })
  if (r.css === null) return r.js.code
  // the scoped stylesheet: added to each window once (index.js), not per instance
  const key = crypto.createHash('sha1').update(filename).digest('hex').slice(0, 12)
  return `${r.js.code}\nimport { addStyle as __buiAddStyle } from '@blender-ui/svelte'\n` +
    `__buiAddStyle(${JSON.stringify(key)}, ${JSON.stringify(r.css.code)})\n`
}

/** The file `specifier` (`svelte`, `svelte/store` ...) names in its package's
 *  browser build, resolved from `fromDir`. */
export function resolveBrowser (specifier, fromDir) {
  const key = `${fromDir}\0${specifier}`
  let file = resolved.get(key)
  if (file === undefined) resolved.set(key, (file = findBrowser(specifier, fromDir)))
  return file
}

const resolved = new Map() // fromDir + specifier -> file
const exportsByRoot = new Map() // package root -> its package.json's exports

function packageExports (root) {
  let e = exportsByRoot.get(root)
  if (e === undefined) exportsByRoot.set(root, (e = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).exports ?? {}))
  return e
}

function findBrowser (specifier, fromDir) {
  const slash = specifier.indexOf('/')
  const name = slash < 0 ? specifier : specifier.slice(0, slash)
  const sub = slash < 0 ? '.' : '.' + specifier.slice(slash)
  for (let dir = fromDir; ; dir = path.dirname(dir)) {
    const root = path.join(dir, 'node_modules', name)
    if (fs.existsSync(path.join(root, 'package.json'))) {
      const target = pick(packageExports(root)[sub])
      if (target === null) throw new Error(`@blender-ui/svelte: ${specifier}: no browser export`)
      return path.join(root, target)
    }
    if (path.dirname(dir) === dir) {
      if (fromDir !== HERE) return findBrowser(specifier, HERE) // not installed there: as this package sees it
      throw new Error(`@blender-ui/svelte: cannot find ${name}`)
    }
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
    for (const entry of Object.values(packageExports(m[1]))) {
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
