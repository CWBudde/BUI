// Svelte's client runtime for Node.js and Bun, bundled once. Svelte ships it
// as about 90 source modules; loading them one by one is most of a Svelte
// app's start on Node.js. On the first run this bundles them with Bun (one
// module per export of the package, sharing chunks, so state stays shared)
// into the cache, keyed by Svelte's version, and register.js serves those
// files from then on. Without Bun or a cache, the source modules load as
// before. (`bui build` bundles the whole app itself.)
//
// SPDX-License-Identifier: GPL-2.0-or-later
import fs from 'node:fs'
import path from 'node:path'
import { CACHE, VERSION, hash, packageExports, pick, resolveBrowser } from './compile.js'

/** Map(browser file of Svelte's -> its bundled module), or null. */
export function runtimeBundle () {
  if (CACHE === null || process.env.BUI_SVELTE_BUNDLE === '0') return null
  const entry = resolveBrowser('svelte', process.cwd())
  const root = entry.slice(0, entry.lastIndexOf(`${path.sep}src${path.sep}`))
  const id = hash(root)
  const dir = path.join(CACHE, `runtime-${VERSION}-${id}`)
  try {
    return new Map(JSON.parse(fs.readFileSync(path.join(dir, 'manifest.json'), 'utf8')))
  } catch {}
  const entries = []
  for (const [sub, exp] of Object.entries(packageExports(root))) {
    const file = pick(exp)
    // the client's modules: not the compiler, the server renderer or types
    if (file === null || !file.endsWith('.js') || sub === './compiler' || sub.includes('server')) continue
    entries.push(path.join(root, file))
  }
  const tmp = `${dir}.${process.pid}`
  if (!bundle(entries, root, tmp)) {
    fs.rmSync(tmp, { recursive: true, force: true })
    return null
  }
  const map = entries.map((f) => [f, path.join(dir, path.relative(root, f))])
  fs.writeFileSync(path.join(tmp, 'manifest.json'), JSON.stringify(map))
  try {
    fs.renameSync(tmp, dir)
  } catch {
    fs.rmSync(tmp, { recursive: true, force: true }) // another process made it first
  }
  return new Map(map)
}

const BUILD = `const o = JSON.parse(process.env.BUI_SVELTE_BUNDLE_OPTIONS)
const r = await Bun.build({ entrypoints: o.entries, root: o.root, outdir: o.out, splitting: true, format: 'esm',
  target: 'browser', minify: true, naming: { entry: '[dir]/[name].[ext]', chunk: '_chunks/[name]-[hash].[ext]' } })
if (!r.success) { for (const l of r.logs) console.error(l); process.exit(1) }`

function bundle (entries, root, out) {
  const { spawnSync } = process.getBuiltinModule('node:child_process') // only on the first run
  const bun = typeof Bun !== 'undefined'
    ? process.execPath
    : spawnSync('bun -e "process.stdout.write(process.execPath)"', { encoding: 'utf8', shell: true }).stdout?.trim()
  if (!bun) return false
  const r = spawnSync(bun, ['-e', BUILD], {
    stdio: 'pipe',
    env: { ...process.env, BUI_SVELTE_BUNDLE_OPTIONS: JSON.stringify({ entries, root, out }) },
  })
  return r.status === 0 && entries.every((f) => fs.existsSync(path.join(out, path.relative(root, f))))
}
