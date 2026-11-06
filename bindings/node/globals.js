// The browser globals DOM-based frameworks read (Svelte's compiled
// components; Solid's or Lit's DOM output the same way): `document`,
// `window`, the node classes for `instanceof`, `requestAnimationFrame`.
// `require('@blender-ui/native/globals')` installs them once; `document`
// makes detached nodes that join a window's tree when inserted into it, so a
// component mounts into any BUI window's `body`.
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

const dom = require('./dom')

const g = globalThis
if (g.document !== dom.document) {
  // never instantiated: `instanceof` checks against them are simply false
  class HTMLMediaElement {}
  class ShadowRoot {}
  const win = {
    document: dom.document,
    addEventListener () {},
    removeEventListener () {},
  }
  const define = (name, value) => { if (g[name] === undefined) g[name] = value }
  g.document = dom.document
  g.window = Object.assign(g.window ?? win, win)
  define('navigator', { userAgent: 'BUI' })
  // the classes `instanceof` checks name; prototypes for frameworks that patch them
  for (const [name, value] of Object.entries({
    Node: dom.Node,
    Element: dom.Element,
    HTMLElement: dom.HTMLElement,
    Text: dom.TextNode,
    Comment: dom.Comment,
    DocumentFragment: dom.DocumentFragment,
    Document: dom.Document,
  })) g[name] = value
  // Node.js and Bun have these; the QuickJS runtime (`bui build`) doesn't
  define('Event', class Event {
    constructor (type, init) {
      this.type = type
      this.bubbles = !!init?.bubbles
      this.cancelBubble = this.defaultPrevented = false
    }

    preventDefault () { this.defaultPrevented = true }
    stopPropagation () { this.cancelBubble = true }
  })
  define('CustomEvent', class CustomEvent extends g.Event {
    constructor (type, init) {
      super(type, init)
      this.detail = init?.detail ?? null
    }
  })
  define('HTMLMediaElement', HTMLMediaElement)
  define('ShadowRoot', ShadowRoot)
  // timing for animations (motion, tweens): a 60 Hz clock; a window's own
  // `requestAnimationFrame` is paced by its display
  define('requestAnimationFrame', (cb) => setTimeout(() => cb(performance.now()), 16))
  define('cancelAnimationFrame', (id) => clearTimeout(id))
  define('getComputedStyle', (el) => el.style)
}

module.exports = { document: dom.document }
