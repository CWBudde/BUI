#!/usr/bin/env node
// bui: build standalone apps from a JavaScript / JSX entry.
//
//   bui build app.jsx [-o app.exe]   a small executable: BUI + QuickJS (~5 MB)
//   bui build app.jsx --bun          a Bun executable (full Node APIs, ~120 MB)
//   bui run app.jsx [args]           run on the small runtime (no executable)
//
// Options:
//   -o, --out <file>   output file (default: the entry's name, .exe on Windows)
//   --bun              compile with `bun build --compile` instead of QuickJS
//   --console          keep a console window (Windows; default: none, for apps)
//   --dev              development builds (React's, Svelte's dev mode)
//   --features a,b     the kit features the app uses (default: package.json
//                      "bui": {"features": [...]}, else detected in the bundle);
//                      the smallest runtime with them is linked (features.js)
//
// Bundling uses Bun (https://bun.sh), which must be installed.
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const features = require('./features.js')
const platform = require('./platform.js')

const MAGIC = Buffer.from('BUIAPP01')
const { TAG, EXE } = platform

function fail (msg) {
  console.error(`bui: ${msg}`)
  process.exit(1)
}

function parse (argv) {
  const o = { cmd: argv[0], entry: null, out: null, bun: false, console: false, dev: false, features: null, rest: [] }
  for (let i = 1; i < argv.length; i++) {
    const a = argv[i]
    if (o.cmd === 'run' && o.entry !== null) o.rest.push(a)
    else if (a === '-o' || a === '--out') o.out = argv[++i]
    else if (a === '--bun') o.bun = true
    else if (a === '--console') o.console = true
    else if (a === '--dev') o.dev = true
    else if (a === '--features') o.features = argv[++i].split(',').map((s) => s.trim()).filter(Boolean)
    else if (a.startsWith('-')) fail(`unknown option ${a}`)
    else o.entry = a
  }
  return o
}

/** The QuickJS runtime `name` (features.json runtimes; "full" is bui-runtime
 *  itself): `npm run build` here, this platform's package, or the workspace build's full one. */
function runtimePath (name = 'full') {
  const file = name === 'full' ? `bui-runtime.${TAG}${EXE}` : `bui-runtime-${name}.${TAG}${EXE}`
  for (const dir of [__dirname, platform.packageDir()]) {
    if (dir && fs.existsSync(path.join(dir, file))) return path.join(dir, file)
  }
  for (const profile of ['dist', 'release']) { // npm run build, or a plain cargo build
    const built = path.join(__dirname, '../rust/target', profile, `bui-runtime${EXE}`)
    if (name === 'full' && fs.existsSync(built)) return built
  }
  if (name !== 'full') return runtimePath('full') // a smaller one wasn't built: the full one works
  fail(`no bui-runtime for ${TAG}: install ${platform.packageName()} (an optional dependency), ` +
    'or run `npm run build` in bindings/node to build it')
}

/** The runtime an app needs: explicit features, else detected ones. */
function chooseRuntime (o, js) {
  try {
    const explicit = o.features ?? features.configured(o.entry)
    const needed = explicit ? new Set(explicit) : new Set(features.detect(js.toString('latin1')).keys())
    const rt = features.pickRuntime(needed)
    console.log(`bui: runtime "${rt.name}" for ${needed.size ? [...needed].join(', ') : 'no optional features'}${explicit ? '' : ' (detected)'}`)
    return rt.name
  } catch (e) {
    fail(e.message)
  }
}

/** Bun's executable: this process when running on Bun; otherwise asked once
 *  (npm installs it behind a shell shim on Windows), then run without a shell
 *  so arguments keep their quotes. */
let bunPath = null
function findBun () {
  if (bunPath !== null) return bunPath
  if (typeof Bun !== 'undefined') return (bunPath = process.execPath)
  const r = spawnSync('bun', ['-e', 'process.stdout.write(process.execPath)'],
    { encoding: 'utf8', shell: process.platform === 'win32' })
  if (r.status !== 0 || !r.stdout) fail('bundling needs Bun: https://bun.sh')
  return (bunPath = r.stdout.trim())
}

function bun (args, opts = {}) {
  const r = spawnSync(findBun(), args, { stdio: opts.quiet ? 'pipe' : 'inherit', encoding: 'utf8' })
  if (r.status !== 0) fail(`bun ${args[0]} failed${r.stderr ? `:
${r.stderr}` : ''}`)
  return r
}

/** Bundle with Bun (bundle.js: the framework bindings' plugins included). */
function bundleWith (options) {
  bun([path.join(__dirname, 'bundle.js'), JSON.stringify(options)], { quiet: true })
}

/** One script for QuickJS: the app and its packages, React in production mode. */
function bundle (entry, dev) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bui-'))
  const out = path.join(dir, 'app.js')
  bundleWith({ entry: path.resolve(entry), out, dev })
  const js = fs.readFileSync(out)
  fs.rmSync(dir, { recursive: true, force: true })
  return js
}

/** Windows: mark the executable as a windowed app (no console window). */
function setGuiSubsystem (buf) {
  if (buf.readUInt16LE(0) !== 0x5a4d) return
  const pe = buf.readUInt32LE(0x3c)
  if (buf.readUInt32LE(pe) !== 0x4550) return
  buf.writeUInt16LE(2, pe + 24 + 68) // IMAGE_SUBSYSTEM_WINDOWS_GUI
}

function build (o) {
  const name = path.basename(o.entry).replace(/\.[^.]+$/, '')
  const out = path.resolve(o.out ?? `${name}${EXE}`)
  const t0 = Date.now()
  if (o.bun) {
    bundleWith({ entry: path.resolve(o.entry), out, bun: true, dev: o.dev, console: o.console || process.platform !== 'win32' })
  } else {
    const js = bundle(o.entry, o.dev)
    const runtime = fs.readFileSync(runtimePath(chooseRuntime(o, js)))
    if (process.platform === 'win32' && !o.console) setGuiSubsystem(runtime)
    const trailer = Buffer.alloc(16)
    MAGIC.copy(trailer, 0)
    trailer.writeBigUInt64LE(BigInt(js.length), 8)
    fs.writeFileSync(out, Buffer.concat([runtime, js, trailer]))
    if (process.platform !== 'win32') fs.chmodSync(out, 0o755)
  }
  const mb = (fs.statSync(out).size / 1048576).toFixed(1)
  console.log(`bui: built ${path.relative(process.cwd(), out)} (${mb} MB, ${o.bun ? 'Bun' : 'QuickJS'}) in ${Date.now() - t0} ms`)
}

function run (o) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bui-run-'))
  const file = path.join(dir, `${path.basename(o.entry).replace(/\.[^.]+$/, '')}.js`)
  fs.writeFileSync(file, bundle(o.entry, o.dev))
  const r = spawnSync(runtimePath(), [file, ...o.rest], { stdio: 'inherit' })
  fs.rmSync(dir, { recursive: true, force: true })
  process.exit(r.status ?? 1)
}

const o = parse(process.argv.slice(2))
if (!['build', 'run'].includes(o.cmd) || !o.entry) {
  console.log(fs.readFileSync(__filename, 'utf8').split('\n').slice(1, 17).map((l) => l.replace(/^\/\/ ?/, '')).join('\n'))
  process.exit(o.cmd ? 1 : 0)
}
if (!fs.existsSync(o.entry)) fail(`no such file: ${o.entry}`)
if (o.cmd === 'build') build(o)
else run(o)
