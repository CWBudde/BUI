// `bui`'s bundler, run on Bun (`bun bundle.js <options as JSON>`): the app and
// its packages in one script for the QuickJS runtime, or one Bun executable.
//
// Framework bindings plug in here: a package in the `@blender-ui` scope with
// `"bui": { "bunPlugin": "./plugin.js" }` in its package.json (a Bun plugin
// as the default export) is used for every app that can import it. That's
// how `.svelte` files compile; a future binding adds its own the same way.
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'
const fs = require('node:fs')
const path = require('node:path')
const { pathToFileURL } = require('node:url')

/** The Bun plugins of the `@blender-ui/*` packages resolvable from `dir`. */
function frameworkPlugins (dir) {
  const found = new Map()
  for (;; dir = path.dirname(dir)) {
    const scope = path.join(dir, 'node_modules', '@blender-ui')
    for (const name of fs.existsSync(scope) ? fs.readdirSync(scope) : []) {
      const pkg = path.join(scope, name, 'package.json')
      if (found.has(name) || !fs.existsSync(pkg)) continue
      const plugin = JSON.parse(fs.readFileSync(pkg, 'utf8')).bui?.bunPlugin
      if (plugin) found.set(name, path.join(scope, name, plugin))
    }
    if (path.dirname(dir) === dir) break
  }
  return Promise.all([...found.values()].map(async (file) => (await import(pathToFileURL(file).href)).default))
}

async function main ({ entry, out, bun, dev, console: keepConsole }) {
  const define = { 'process.env.NODE_ENV': JSON.stringify(dev ? 'development' : 'production') }
  const plugins = await frameworkPlugins(path.dirname(entry))
  const r = await Bun.build(bun
    ? {
        entrypoints: [entry], minify: true, define, plugins,
        compile: { outfile: out, windows: { hideConsole: !keepConsole } },
      }
    : {
        // an ES module importing the entry: Bun's IIFE output runs a CommonJS
        // entry only when something imports it
        entrypoints: [writeWrapper(entry, out)], target: 'browser', format: 'iife', minify: true, define, plugins,
        external: ['node:*', '*.node'],
      })
  if (!r.success) {
    for (const log of r.logs) console.error(log)
    process.exit(1)
  }
  if (!bun) fs.writeFileSync(out, await r.outputs[0].text())
}

function writeWrapper (entry, out) {
  const wrapper = path.join(path.dirname(out), 'entry.mjs')
  fs.writeFileSync(wrapper, `import ${JSON.stringify(path.resolve(entry).split(path.sep).join('/'))}\n`)
  return wrapper
}

main(JSON.parse(process.argv[2])).catch((err) => {
  console.error(err)
  process.exit(1)
})
