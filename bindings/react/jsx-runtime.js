// `"jsxImportSource": "@blender-ui/react"`: React's JSX runtime. Components
// get render tracking (they re-render when a signal their render read
// changes); the types let every host-element prop take a signal.
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'
const runtime = require('react/jsx-runtime')
const { track } = require('./hooks')

exports.Fragment = runtime.Fragment
exports.jsx = (type, props, key) => runtime.jsx(track(type), props, key)
exports.jsxs = (type, props, key) => runtime.jsxs(track(type), props, key)
