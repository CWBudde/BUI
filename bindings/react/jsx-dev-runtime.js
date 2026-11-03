// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'
const runtime = require('react/jsx-dev-runtime')
const { track } = require('./hooks')

exports.Fragment = runtime.Fragment
// undefined in React's production build, as in React
exports.jsxDEV = runtime.jsxDEV && ((type, props, key, isStatic, source, self) =>
  runtime.jsxDEV(track(type), props, key, isStatic, source, self))
