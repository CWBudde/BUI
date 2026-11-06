// Builds the native addon (bindings/rust/blenderui-node) and the QuickJS
// runtimes (bindings/rust/blenderui-qjs, `bui-runtime`) and copies them here
// as blenderui.<platform>-<arch>.node, bui-runtime.<platform>-<arch>[.exe]
// (the full runtime) and bui-runtime-<name>.<platform>-<arch>[.exe] for each
// smaller runtime features.json lists (`bui build` links the smallest one an
// app needs). `--debug` for a debug build; `--only full` for one runtime.
'use strict'
const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')
const { table } = require('../features.js')

const debug = process.argv.includes('--debug')
const only = process.argv.includes('--only') ? process.argv[process.argv.indexOf('--only') + 1] : null
const rust = path.resolve(__dirname, '../../rust')
// shipped binaries: the `dist` profile (release + fat LTO, one codegen unit:
// smaller and faster; see bindings/rust/Cargo.toml)
const profile = debug ? [] : ['--profile', 'dist']
const target = path.join(rust, 'target', debug ? 'debug' : 'dist')
const tag = `${process.platform}-${process.arch}`
const exe = process.platform === 'win32' ? '.exe' : ''

function cargo (args) {
  execFileSync('cargo', ['build', ...args, ...profile], { cwd: rust, stdio: 'inherit' })
}

function copy (from, to) {
  const dest = path.join(__dirname, '..', to)
  fs.copyFileSync(path.join(target, from), dest)
  console.log(`wrote ${path.relative(process.cwd(), dest)} (${(fs.statSync(dest).size / 1048576).toFixed(2)} MB)`)
}

if (!only) {
  // the addon alone: built with the runtimes, cargo would unify their features
  cargo(['-p', 'blenderui-node'])
  const lib = { win32: 'blenderui_node.dll', darwin: 'libblenderui_node.dylib' }[process.platform] ?? 'libblenderui_node.so'
  copy(lib, `blenderui.${tag}.node`)
}
for (const rt of table().runtimes) {
  if (only && rt.name !== only) continue
  // the kit features of this runtime only (blenderui forwards them to the C kit)
  const feats = ['blenderui/slug', ...rt.features.map((f) => `blenderui/${f}`)]
  cargo(['-p', 'blenderui-qjs', '--no-default-features', '--features', feats.join(',')])
  copy(`bui-runtime${exe}`, rt.name === 'full' ? `bui-runtime.${tag}${exe}` : `bui-runtime-${rt.name}.${tag}${exe}`)
}
