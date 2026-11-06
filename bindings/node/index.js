// @blender-ui/native: Blender's UI from JavaScript.
//
// A DOM-like retained tree (createElement, setAttribute, appendChild,
// addEventListener) rendered by BUI in native windows: HTML elements, CSS,
// Tailwind classes. BUI runs on its own thread; each window collects the
// tree changes of a tick and sends them in one call (an Int32Array of ops
// and a string table), so only what changed crosses into Rust. Events come
// back asynchronously.
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

// The native layer: the QuickJS runtime's built-in one (bui-runtime, apps
// made by `bui build`), or the napi addon (Node.js, Bun).
const native = globalThis.__BUI_NATIVE__ ?? loadAddon()

function loadAddon () {
  const env = process.env.BUI_NODE_ADDON
  try {
    // a literal path where a prebuilt binary exists: `bun build --compile` (and
    // other bundlers) can then embed the addon in the executable
    if (!env && process.platform === 'win32' && process.arch === 'x64') return require('./blenderui.win32-x64.node')
    return require(env || require('node:path').join(__dirname, `blenderui.${process.platform}-${process.arch}.node`))
  } catch (err) {
    throw new Error(`@blender-ui/native: no native addon for ${process.platform}-${process.arch}. ` +
      'Prebuilt binaries are for Windows x64; elsewhere, build it from a clone of ' +
      'https://github.com/ppmpreetham/BUI with `npm run build` in bindings/node (needs Rust, CMake and a C compiler), ' +
      'then use that package, or point BUI_NODE_ADDON at the built blenderui.<platform>-<arch>.node.',
    { cause: err })
  }
}
const signals = require('./signals')
const dom = require('./dom')
const { OP, HTMLElement, TextNode, Comment, DocumentFragment, createElement } = dom

class Window {
  constructor (app, id, options = {}) {
    this.app = app
    this.id = id
    this.closed = false
    this._next = 1
    this._nodes = new Map()
    this._ops = []
    this._strings = []
    this._stringIds = new Map()
    this._scheduled = false
    this._onClose = new Set()
    this._frameCallbacks = new Map()
    this._nextFrameId = 1
    this._framePending = false
    this._inFrame = false
    this._orphans = []
    /** The root element: node 0. */
    this.body = new HTMLElement('body')
    this.body.ownerWindow = this
    this.body.id = 0
    this._nodes.set(0, this.body)
    app.native.openWindow(id, {
      title: options.title, width: options.width, height: options.height,
      resizable: options.resizable, gpu: options.gpu,
    })
  }

  /** An element. Capitalized tags are Blender's widgets (`Button`, `NumberSlider` ...). */
  createElement (tag) { return createElement(this, tag) }

  /** A text node; `text` may be a signal (the text then follows it). */
  createTextNode (text) {
    const t = new TextNode('')
    t.data = text
    t._attach(this)
    return t
  }

  createComment (data = '') {
    const c = new Comment(String(data))
    c._attach(this)
    return c
  }

  createDocumentFragment () { return new DocumentFragment() }

  /** Changes are sent once per tick; `flush()` sends them now. */
  flush () {
    this._scheduled = false
    if (this._orphans.length > 0) this._sweep()
    if (this.closed) {
      this._ops = []
      this._strings = []
      this._stringIds.clear()
      return
    }
    if (!this._ops.length) return
    const ops = Int32Array.from(this._ops)
    const strings = this._strings
    this._ops = []
    this._strings = []
    this._stringIds.clear()
    // while animating, changes render on the next vsync (one rebuild per frame)
    this.app.native.commit(this.id, ops, strings, this._inFrame || this._framePending)
  }

  /**
   * Run `cb(timestamp)` before the next frame renders, paced by the display
   * (like the browser's). Changes made in it render in that frame.
   */
  requestAnimationFrame (cb) {
    const id = this._nextFrameId++
    this._frameCallbacks.set(id, cb)
    if (!this._framePending) {
      this._framePending = true
      this.flush()
      this.app.native.requestFrame(this.id)
    }
    return id
  }

  cancelAnimationFrame (id) { this._frameCallbacks.delete(id) }

  _frame () {
    this._framePending = false
    const cbs = [...this._frameCallbacks.values()]
    this._frameCallbacks.clear()
    this._inFrame = true
    const now = performance.now()
    try {
      for (const cb of cbs) {
        try { cb(now) } catch (err) { queueMicrotask(() => { throw err }) }
      }
    } finally {
      this._inFrame = false
    }
    this.flush()
  }

  /** Render and read the pixels: `{ width, height, data }` (RGBA). */
  async screenshot () {
    this.flush()
    return this.app.native.screenshot(this.id)
  }

  /** Click at (x, y) logical pixels (tests). */
  click (x, y) {
    this.flush()
    this.app.native.simulateClick(this.id, x, y)
  }

  close () {
    this.flush()
    this.app.native.closeWindow(this.id)
    this._closed()
  }

  on (event, fn) {
    if (event !== 'close') throw new Error(`blenderui: unsupported window event "${event}"`)
    this._onClose.add(fn)
    return this
  }

  _closed () {
    if (this.closed) return
    this.closed = true
    for (const fn of this._onClose) {
      try { fn() } catch (err) { queueMicrotask(() => { throw err }) }
    }
    // the native window is gone: free what follows signals or holds listeners
    for (const n of this._nodes.values()) {
      n._unbindAll()
      n._listeners?.clear()
    }
    this._frameCallbacks.clear()
    this.flush()
    this.app._windowClosed(this)
  }

  _op (...args) {
    for (const a of args) this._ops.push(a)
    if (!this._scheduled) {
      this._scheduled = true
      queueMicrotask(() => this.flush())
    }
  }

  _s (str) {
    let i = this._stringIds.get(str)
    if (i === undefined) {
      i = this._strings.push(str) - 1
      this._stringIds.set(str, i)
    }
    return i
  }

  /** A node of `document` was removed: freed at the next flush unless reinserted. */
  _orphan (node) {
    this._orphans.push(node)
    if (!this._scheduled) {
      this._scheduled = true
      queueMicrotask(() => this.flush())
    }
  }

  _sweep () {
    const orphans = this._orphans
    this._orphans = []
    for (const n of orphans) if (n.parentNode === null) n.destroy()
  }

  _free (node) {
    node._unbindAll()
    if (node.id === 0 || !this._nodes.delete(node.id)) return // the body, or freed already
    this._op(OP.FREE, node.id)
  }
}

class App {
  /**
   * @param {{ gpu?: boolean, slug?: boolean, headless?: boolean, theme?: 'dark' | 'light',
   *           quitOnLastWindowClosed?: boolean }} [options]
   */
  constructor (options = {}) {
    this.windows = new Map()
    this._nextWindow = 1
    this.quitOnLastWindowClosed = options.quitOnLastWindowClosed ?? true
    this.native = new native.NativeApp(
      { gpu: options.gpu, slug: options.slug, headless: options.headless, theme: options.theme },
      raw => this._event(raw))
  }

  get headless () { return this.native.headless }

  /** Open a window: `{ title, width, height, resizable, gpu }`. */
  window (options) {
    const w = new Window(this, this._nextWindow++, options)
    this.windows.set(w.id, w)
    return w
  }

  /** GPU text: Slug (glyph outlines) or the glyph atlas. */
  setSlug (on) { this.native.setSlug(!!on) }

  /** Blender's 'dark' (default) or 'light' theme; open windows redraw. */
  setTheme (name) {
    if (!this.native.setTheme(String(name))) throw new RangeError(`unknown theme ${name} (expected 'dark' or 'light')`)
  }

  /** Close every window and end the UI thread (Node may then exit). */
  quit () {
    if (this._quit) return
    this._quit = true
    for (const w of [...this.windows.values()]) {
      w.flush()
      w._closed()
    }
    this.native.quit()
  }

  _event (raw) {
    const win = this.windows.get(raw.window)
    if (!win) return
    if (raw.kind === 'close') {
      win._closed()
      return
    }
    if (raw.kind === 'frame') {
      win._frame()
      return
    }
    win._nodes.get(raw.target)?._dispatch(raw)
  }

  _windowClosed (win) {
    this.windows.delete(win.id)
    if (this.quitOnLastWindowClosed && this.windows.size === 0 && !this._quit) {
      this._quit = true
      this.native.quit()
    }
  }
}

module.exports = { ...dom, App, Window, native, ...signals }
