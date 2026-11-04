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
      'https://github.com/ppmpreetham/BUI with `npm run build` in bindings/node (needs Rust, CMake and a C compiler).',
    { cause: err })
  }
}
const signals = require('./signals')
const { isSignal, effect, unowned, memo } = signals

// Keep in sync with bindings/rust/blenderui-node/src/lib.rs.
const OP = { CREATE: 0, TEXT: 1, SET_ATTR: 2, REMOVE_ATTR: 3, APPEND: 4, INSERT: 5, REMOVE: 6, SET_TEXT: 7, LISTEN: 8, FREE: 9 }
const EVENT_BITS = {
  click: 1, mousedown: 2, mouseup: 4, mousemove: 8, mouseenter: 16, mouseleave: 16,
  wheel: 32, keydown: 64, keyup: 128, change: 256, input: 512,
}
const BUTTONS = ['left', 'middle', 'right']
// DOM: these don't bubble
const NO_BUBBLE = new Set(['mouseenter', 'mouseleave'])

/** The event object handlers receive (a subset of the DOM's). */
class BuiEvent {
  constructor (raw, target) {
    this.type = raw.kind
    this.bubbles = !NO_BUBBLE.has(raw.kind)
    // the deepest element listening for this type (BUI reports no deeper target)
    this.target = target
    this.currentTarget = target
    this.clientX = this.x = raw.x
    this.clientY = this.y = raw.y
    this.button = raw.button
    this.detail = raw.clickCount
    this.deltaY = -raw.delta * 100 // DOM: positive scrolls down
    this.key = raw.key ?? undefined
    this.char = raw.keyChar ?? undefined
    this.shiftKey = raw.shift
    this.ctrlKey = raw.ctrl
    this.altKey = raw.alt
    this.metaKey = false
    this.timeStamp = performance.now()
    this.defaultPrevented = false
    this.propagationStopped = false
    this.immediatePropagationStopped = false
  }

  /** Events reach JavaScript after BUI handled them: this marks the event only. */
  preventDefault () { this.defaultPrevented = true }
  stopPropagation () { this.propagationStopped = true }
  stopImmediatePropagation () { this.propagationStopped = this.immediatePropagationStopped = true }
}

class Node {
  constructor (win, id) {
    this.ownerWindow = win
    this.id = id
    this.parentNode = null
    this._bindings = null
    this.childNodes = []
  }

  get firstChild () { return this.childNodes[0] ?? null }
  get lastChild () { return this.childNodes[this.childNodes.length - 1] ?? null }
  get nextSibling () {
    const p = this.parentNode
    return p ? p.childNodes[p.childNodes.indexOf(this) + 1] ?? null : null
  }

  appendChild (child) {
    this._adopt(child)
    child._detach()
    this.childNodes.push(child)
    child.parentNode = this
    this.ownerWindow._op(OP.APPEND, this.id, child.id)
    return child
  }

  insertBefore (child, before) {
    if (!before) return this.appendChild(child)
    if (before.parentNode !== this) throw new Error('blenderui: insertBefore: the reference node is not a child of this node')
    if (child === before) return child
    this._adopt(child)
    child._detach()
    this.childNodes.splice(this.childNodes.indexOf(before), 0, child)
    child.parentNode = this
    this.ownerWindow._op(OP.INSERT, this.id, child.id, before.id)
    return child
  }

  removeChild (child) {
    const i = this.childNodes.indexOf(child)
    if (i >= 0) {
      this.childNodes.splice(i, 1)
      child.parentNode = null
      this.ownerWindow._op(OP.REMOVE, this.id, child.id)
    }
    return child
  }

  remove () { this.parentNode?.removeChild(this) }

  _detach () {
    const p = this.parentNode
    if (p) {
      p.childNodes.splice(p.childNodes.indexOf(this), 1)
      this.parentNode = null
      if (p !== this._nextParent) this.ownerWindow._op(OP.REMOVE, p.id, this.id)
    }
  }

  /** Checks before `child` moves under this node (as the DOM does). */
  _adopt (child) {
    if (child.ownerWindow !== this.ownerWindow) throw new Error('blenderui: the node belongs to another window')
    for (let n = this; n; n = n.parentNode) {
      if (n === child) throw new Error('blenderui: a node cannot be inserted into itself or its descendants')
    }
    child._nextParent = this // same parent: APPEND / INSERT reorders it natively
  }

  /** Keep `key` (an attribute, or the text) following `source`: each change
   *  updates this node alone. */
  _bind (key, source, apply) {
    this._unbind(key)
    // unowned: a binding made inside an effect lives as long as the node, not that effect's run
    ;(this._bindings ??= new Map()).set(key, unowned(() => effect(() => { apply(source.value) })))
  }

  _unbind (key) {
    const dispose = this._bindings?.get(key)
    if (dispose) {
      dispose()
      this._bindings.delete(key)
    }
  }

  _unbindAll () {
    if (this._bindings) {
      for (const dispose of this._bindings.values()) dispose()
      this._bindings = null
    }
  }

  /** Free this node and its subtree on the native side (after removing it). */
  destroy () {
    for (const c of this.childNodes) c.destroy()
    this.ownerWindow._free(this)
  }

  get textContent () { return this.childNodes.map(c => c.textContent).join('') }
}

class TextNode extends Node {
  constructor (win, id, text) {
    super(win, id)
    this._text = text
  }

  get nodeType () { return 3 }
  get data () { return this._text }
  /** A string, or a signal (the text then follows it). */
  set data (t) {
    if (isSignal(t)) return this._bind('data', t, (v) => this._setText(v))
    this._unbind('data')
    this._setText(t)
  }

  _setText (t) {
    t = t == null ? '' : String(t)
    if (t !== this._text) {
      this._text = t
      this.ownerWindow._op(OP.SET_TEXT, this.id, this.ownerWindow._s(t))
    }
  }

  get nodeValue () { return this._text }
  set nodeValue (t) { this.data = t }
  get textContent () { return this._text }
  set textContent (t) { this.data = t }
}

class Element extends Node {
  constructor (win, id, tag) {
    super(win, id)
    this.tagName = tag.toUpperCase()
    this.localName = tag
    this._attrs = new Map()
    this._listeners = new Map()
    this._bits = 0
    // form controls: the latest value from input / change events
    this.value = undefined
    this.checked = undefined
  }

  get nodeType () { return 1 }

  /** A value, or a signal: the attribute then follows it (null / false removes it). */
  setAttribute (name, value) {
    if (isSignal(value)) {
      return this._bind(name, value, (v) => {
        if (v == null || v === false) this._removeAttr(name)
        else this._setAttr(name, name === 'style' && typeof v === 'object' ? styleText(v) : v === true ? '' : v)
      })
    }
    this._unbind(name)
    this._setAttr(name, value)
  }

  _setAttr (name, value) {
    value = String(value)
    if (this._attrs.get(name) === value) return
    this._attrs.set(name, value)
    const w = this.ownerWindow
    w._op(OP.SET_ATTR, this.id, w._s(name), w._s(value))
  }

  getAttribute (name) { return this._attrs.get(name) ?? null }
  hasAttribute (name) { return this._attrs.has(name) }

  removeAttribute (name) {
    this._unbind(name)
    this._removeAttr(name)
  }

  _removeAttr (name) {
    if (this._attrs.delete(name)) this.ownerWindow._op(OP.REMOVE_ATTR, this.id, this.ownerWindow._s(name))
  }

  get className () { return this._attrs.get('class') ?? '' }
  set className (v) { this.setAttribute('class', v) }
  get style () { return this._attrs.get('style') ?? '' }
  /** A CSS declaration string, or an object (`{ backgroundColor: 'red', width: 40 }`). */
  set style (v) {
    if (v == null) return this.removeAttribute('style')
    if (typeof v === 'string' || isSignal(v)) return this.setAttribute('style', v)
    // an object: plain values, or some signals (`{ color: colorSignal }`), then bound
    for (const k in v) if (isSignal(v[k])) return this.setAttribute('style', memo(() => styleText(v)))
    this.setAttribute('style', styleText(v))
  }
  get textContent () { return super.textContent }
  set textContent (t) {
    for (const c of [...this.childNodes]) { this.removeChild(c); c.destroy() }
    if (t !== '' && t != null) this.appendChild(this.ownerWindow.createTextNode(isSignal(t) ? t : String(t)))
  }

  addEventListener (type, fn) {
    if (!(type in EVENT_BITS)) throw new Error(`blenderui: unsupported event "${type}"`)
    let set = this._listeners.get(type)
    if (!set) this._listeners.set(type, (set = new Set()))
    set.add(fn)
    this._listen()
  }

  removeEventListener (type, fn) {
    this._listeners.get(type)?.delete(fn)
    this._listen()
  }

  _listen () {
    let bits = 0
    for (const [type, set] of this._listeners) if (set.size) bits |= EVENT_BITS[type]
    if (bits !== this._bits) {
      this._bits = bits
      this.ownerWindow._op(OP.LISTEN, this.id, bits)
    }
  }

  _dispatch (raw) {
    if (raw.kind === 'input' || raw.kind === 'change') {
      this.value = raw.value
      this.checked = raw.checked
      this.valueAsNumber = raw.number
    }
    const e = new BuiEvent(raw, this)
    for (let n = this; n !== null; n = n.parentNode) {
      const set = n._listeners?.get(raw.kind)
      if (set && set.size) {
        e.currentTarget = n
        for (const fn of [...set]) {
          try { fn.call(n, e) } catch (err) { queueMicrotask(() => { throw err }) }
          if (e.immediatePropagationStopped) break
        }
      }
      if (e.propagationStopped || !e.bubbles) break
    }
    e.currentTarget = null
  }

  /** Layout box `{ x, y, width, height }` in logical pixels (or null). */
  getBoundingClientRect () {
    this.ownerWindow.flush() // the UI thread must see the tree first
    return this.ownerWindow.app.native.bounds(this.ownerWindow.id, this.id)
  }
}

const UNITLESS = new Set(['opacity', 'z-index', 'flex', 'flex-grow', 'flex-shrink', 'order', 'font-weight',
  'line-height', 'zoom', 'aspect-ratio', 'grid-row', 'grid-column', 'scale'])

/** `{ backgroundColor: 'red', width: 40 }` -> `background-color: red; width: 40px` */
function styleText (o) {
  let s = ''
  for (const k in o) {
    const raw = o[k]
    const v = isSignal(raw) ? raw.value : raw // read: a memo around this follows it
    if (v == null || v === false || v === '') continue
    const prop = k.startsWith('--') ? k : k.replace(/[A-Z]/g, m => '-' + m.toLowerCase())
    s += `${prop}: ${typeof v === 'number' && v !== 0 && !UNITLESS.has(prop) ? v + 'px' : v}; `
  }
  return s
}

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
    /** The root element: node 0. */
    this.body = new Element(this, 0, 'body')
    this._nodes.set(0, this.body)
    app.native.openWindow(id, {
      title: options.title, width: options.width, height: options.height,
      resizable: options.resizable, gpu: options.gpu,
    })
  }

  createElement (tag) {
    // Blender's widgets: capitalized tags are always Blender widgets; their
    // lowercase HTML counterparts look like a browser's (Blender's inside `styled`)
    const blender = BLENDER_COMPONENTS[tag]
    if (blender) {
      const el = this.createElement(blender[0])
      if (blender[1]) el.setAttribute('type', blender[1])
      el.setAttribute('bui-blender', '')
      return el
    }
    const id = this._next++
    const el = new Element(this, id, tag)
    this._nodes.set(id, el)
    this._op(OP.CREATE, id, this._s(tag))
    return el
  }

  /** A text node; `text` may be a signal (the text then follows it). */
  createTextNode (text) {
    const id = this._next++
    const bound = isSignal(text)
    const t = new TextNode(this, id, bound ? String(text.peek() ?? '') : String(text))
    this._nodes.set(id, t)
    this._op(OP.TEXT, id, this._s(t._text))
    if (bound) t.data = text
    return t
  }

  /** Changes are sent once per tick; `flush()` sends them now. */
  flush () {
    this._scheduled = false
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

  _free (node) {
    node._unbindAll()
    if (node.id === 0 || !this._nodes.delete(node.id)) return // the body, or freed already
    this._op(OP.FREE, node.id)
  }
}

class App {
  /**
   * @param {{ gpu?: boolean, slug?: boolean, headless?: boolean,
   *           quitOnLastWindowClosed?: boolean }} [options]
   */
  constructor (options = {}) {
    this.windows = new Map()
    this._nextWindow = 1
    this.quitOnLastWindowClosed = options.quitOnLastWindowClosed ?? true
    this.native = new native.NativeApp(
      { gpu: options.gpu, slug: options.slug, headless: options.headless },
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

/** Capitalized Blender widgets for `createElement`: [HTML tag, input type]. */
const BLENDER_COMPONENTS = {
  Button: ['button'],
  Checkbox: ['input', 'checkbox'],
  Radio: ['input', 'radio'],
  NumberField: ['input', 'number'],
  NumberSlider: ['input', 'range'],
  TextField: ['input', 'text'],
  TextArea: ['textarea'],
  ColorSwatch: ['input', 'color'],
  Dropdown: ['select'],
  ProgressBar: ['progress'],
}

module.exports = {
  BLENDER_COMPONENTS, App, Window, Element, TextNode, Node, BuiEvent, styleText, BUTTONS, native, ...signals }
