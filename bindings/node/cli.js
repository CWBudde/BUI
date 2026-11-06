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
//   --dev              React's development build (default: production)
//
// Bundling uses Bun (https://bun.sh), which must be installed.
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { spawnSync } = require('node:child_process')

const MAGIC = Buffer.from('BUIAPP01')
const TAG = `${process.platform}-${process.arch}`
const EXE = process.platform === 'win32' ? '.exe' : ''

function fail (msg) {
  console.error(`bui: ${msg}`)
  process.exit(1)
}

function parse (argv) {
  const o = { cmd: argv[0], entry: null, out: null, bun: false, console: false, dev: false, rest: [] }
  for (let i = 1; i < argv.length; i++) {
    const a = argv[i]
    if (o.cmd === 'run' && o.entry !== null) o.rest.push(a)
    else if (a === '-o' || a === '--out') o.out = argv[++i]
    else if (a === '--bun') o.bun = true
    else if (a === '--console') o.console = true
    else if (a === '--dev') o.dev = true
    else if (a.startsWith('-')) fail(`unknown option ${a}`)
    else o.entry = a
  }
  return o
}

/** The QuickJS runtime shipped in this package (or the workspace build). */
function runtimePath () {
  const shipped = path.join(__dirname, `bui-runtime.${TAG}${EXE}`)
  if (fs.existsSync(shipped)) return shipped
  for (const profile of ['dist', 'release']) { // npm run build, or a plain cargo build
    const built = path.join(__dirname, '../rust/target', profile, `bui-runtime${EXE}`)
    if (fs.existsSync(built)) return built
  }
  fail(`no bui-runtime for ${TAG}: prebuilt for Windows x64; run \`npm run build\` in bindings/node to build it`)
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

/** One script for QuickJS: the app and its packages, React in production mode. */
function bundle (entry, dev) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bui-'))
  // an ES module importing the entry: Bun's IIFE output runs a CommonJS
  // entry only when something imports it
  const wrapper = path.join(dir, 'entry.mjs')
  fs.writeFileSync(wrapper, `import ${JSON.stringify(path.resolve(entry).split(path.sep).join('/'))}\n`)
  const out = path.join(dir, 'app.js')
  bun(['build', wrapper, '--target=browser', '--format=iife', '--minify',
    `--define=process.env.NODE_ENV="${dev ? 'development' : 'production'}"`,
    '--external=node:*', '--external=*.node', `--outfile=${out}`], { quiet: true })
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
    const args = ['build', '--compile', '--minify', o.entry, `--outfile=${out}`,
      `--define=process.env.NODE_ENV="${o.dev ? 'development' : 'production'}"`]
    if (process.platform === 'win32' && !o.console) args.push('--windows-hide-console')
    bun(args)
  } else {
    const js = bundle(o.entry, o.dev)
    const runtime = fs.readFileSync(runtimePath())
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
  console.log(fs.readFileSync(__filename, 'utf8').split('\n').slice(1, 14).map((l) => l.replace(/^\/\/ ?/, '')).join('\n'))
  process.exit(o.cmd ? 1 : 0)
}
if (!fs.existsSync(o.entry)) fail(`no such file: ${o.entry}`)
if (o.cmd === 'build') build(o)
else run(o)
