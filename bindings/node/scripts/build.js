// Builds the native addon (bindings/rust/blenderui-node) and the QuickJS
// runtime (bindings/rust/blenderui-qjs, `bui-runtime`) and copies them here
// as blenderui.<platform>-<arch>.node and bui-runtime.<platform>-<arch>[.exe].
// `--debug` for a debug build.
'use strict'
const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')

const debug = process.argv.includes('--debug')
const rust = path.resolve(__dirname, '../../rust')
// shipped binaries: the `dist` profile (release + fat LTO, one codegen unit:
// smaller and faster; see bindings/rust/Cargo.toml)
const args = ['build', '-p', 'blenderui-node', '-p', 'blenderui-qjs', ...(debug ? [] : ['--profile', 'dist'])]
execFileSync('cargo', args, { cwd: rust, stdio: 'inherit' })
const target = path.join(rust, 'target', debug ? 'debug' : 'dist')
const tag = `${process.platform}-${process.arch}`
const exe = process.platform === 'win32' ? '.exe' : ''
const lib = { win32: 'blenderui_node.dll', darwin: 'libblenderui_node.dylib' }[process.platform] ?? 'libblenderui_node.so'
for (const [from, to] of [[lib, `blenderui.${tag}.node`], [`bui-runtime${exe}`, `bui-runtime.${tag}${exe}`]]) {
  const dest = path.join(__dirname, '..', to)
  fs.copyFileSync(path.join(target, from), dest)
  console.log(`wrote ${path.relative(process.cwd(), dest)}`)
}
