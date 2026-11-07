// The prebuilt platforms. Each one is an npm package (@blender-ui/native-<tag>)
// that holds the addon and the bui-runtime files; @blender-ui/native lists them
// as optional dependencies, so npm and Bun install only the one that matches.
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'
const path = require('node:path')

/** One entry per platform package: the npm `os`, `cpu` and `libc` fields. */
const TARGETS = {
  'win32-x64': { os: 'win32', cpu: 'x64' },
  'linux-x64': { os: 'linux', cpu: 'x64', libc: 'glibc' },
  'darwin-arm64': { os: 'darwin', cpu: 'arm64' },
  'darwin-x64': { os: 'darwin', cpu: 'x64' },
}

const TAG = `${process.platform}-${process.arch}`
const EXE = process.platform === 'win32' ? '.exe' : ''

const packageName = (tag = TAG) => `@blender-ui/native-${tag}`

/** The folder of this platform's package, or null when it is not installed. */
function packageDir () {
  try {
    return path.dirname(require.resolve(`${packageName()}/package.json`))
  } catch {
    return null
  }
}

module.exports = { TARGETS, TAG, EXE, packageName, packageDir }
