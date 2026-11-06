// What an app needs from the kit, for `bui build`: the feature table
// (features.json at the repository root, copied next to this file when the
// package is packed) and the prebuilt runtimes it lists (base, media, full).
//
// An app pays only for what it uses: `bui build` links the smallest runtime
// that has every feature the app needs. Explicit beats detected:
//   1. `--features png,video` on the command line, or
//   2. `"bui": { "features": ["png", "video"] }` in the app's package.json, else
//   3. detection in the bundle: <video> elements, media file extensions in
//      string literals ("intro.mp4", 'logo.webp') and file dialogs. Paths built
//      at run time ("img." + ext) can't be seen, and the GPU renderer is
//      never detected (every bundle's library code names its option): list
//      those features explicitly.
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'
const fs = require('node:fs')
const path = require('node:path')

/** The table: next to this file in the package, else the repository's. */
function table () {
  for (const p of [path.join(__dirname, 'features.json'), path.join(__dirname, '../../features.json')]) {
    if (fs.existsSync(p)) return JSON.parse(fs.readFileSync(p, 'utf8'))
  }
  throw new Error('features.json not found')
}

const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

/** Features the bundled `js` uses, with the reason for each. */
function detect (js, t = table()) {
  const found = new Map()
  for (const f of t.features) {
    if (f.always) found.set(f.name, 'always')
    for (const ext of f.ext || []) {
      // a string literal ending in the extension (optionally ?query / #hash)
      const re = new RegExp(`\\.${escape(ext)}(?:[?#][^"'\`]*)?["'\`)]`, 'i')
      if (re.test(js)) { found.set(f.name, `.${ext} file`); break }
    }
    for (const tag of f.tags || []) {
      if (new RegExp(`["'\`]${escape(tag)}["'\`]`).test(js)) found.set(f.name, `<${tag}>`)
    }
    for (const m of f.markers || []) {
      if (new RegExp(`\\b${escape(m)}\\b\\s*[:(=]`).test(js)) found.set(f.name, `${m}`)
    }
  }
  // what a feature needs comes with it
  for (const f of t.features) {
    if (found.has(f.name)) for (const r of f.requires || []) if (!found.has(r)) found.set(r, `needed by ${f.name}`)
  }
  return found
}

/** The package.json `bui.features` of the app at `entry` (nearest one up), or null. */
function configured (entry) {
  for (let dir = path.dirname(path.resolve(entry)); ; dir = path.dirname(dir)) {
    const p = path.join(dir, 'package.json')
    if (fs.existsSync(p)) {
      const feats = JSON.parse(fs.readFileSync(p, 'utf8')).bui?.features
      return Array.isArray(feats) ? feats : null
    }
    if (path.dirname(dir) === dir) return null
  }
}

/** The smallest runtime with every needed feature (runtimes are listed smallest first). */
function pickRuntime (needed, t = table()) {
  const known = new Set(t.features.map((f) => f.name))
  needed = new Set(needed instanceof Map ? needed.keys() : needed)
  for (const n of needed) if (!known.has(n)) throw new Error(`unknown feature "${n}" (features.json: ${[...known].join(', ')})`)
  const rt = t.runtimes.find((r) => [...needed].every((n) => r.features.includes(n)))
  if (!rt) {
    const full = t.runtimes[t.runtimes.length - 1]
    const missing = [...needed].filter((n) => !full.features.includes(n))
    throw new Error(`no prebuilt runtime has ${missing.join(', ')}: build one with cargo (see docs: binary size)`)
  }
  return rt
}

module.exports = { table, detect, configured, pickRuntime }
