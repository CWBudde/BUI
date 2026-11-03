// Builds the native addon (bindings/rust/blenderui-node) and copies it here
// as blenderui.<platform>-<arch>.node. `--debug` for a debug build.
'use strict'
const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')

const debug = process.argv.includes('--debug')
const rust = path.resolve(__dirname, '../../rust')
const args = ['build', '-p', 'blenderui-node', ...(debug ? [] : ['--release'])]
execFileSync('cargo', args, { cwd: rust, stdio: 'inherit' })
const lib = { win32: 'blenderui_node.dll', darwin: 'libblenderui_node.dylib' }[process.platform] ?? 'libblenderui_node.so'
const from = path.join(rust, 'target', debug ? 'debug' : 'release', lib)
const to = path.join(__dirname, '..', `blenderui.${process.platform}-${process.arch}.node`)
fs.copyFileSync(from, to)
console.log(`wrote ${path.relative(process.cwd(), to)}`)
