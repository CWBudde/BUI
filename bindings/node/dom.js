// The DOM-like tree: nodes, elements, events and a window-less `document`.
//
// A node lives in a window (`ownerWindow`), where each change becomes an op
// sent to the native UI thread (index.js), or is detached: made by
// `document` (or `cloneNode`) and kept in JavaScript until it is inserted
// into a window's tree, which then creates it and its subtree natively in
// one go. Siblings are a linked list, as in the DOM: insertBefore, remove
// and nextSibling are O(1).
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

const { isSignal, effect, unowned, memo } = require('./signals')

// Keep in sync with bindings/rust/blenderui-node/src/lib.rs.
const OP = { CREATE: 0, TEXT: 1, SET_ATTR: 2, REMOVE_ATTR: 3, APPEND: 4, INSERT: 5, REMOVE: 6, SET_TEXT: 7, LISTEN: 8, FREE: 9 }
const EVENT_BITS = {
  click: 1, mousedown: 2, mouseup: 4, mousemove: 8, mouseenter: 16, mouseleave: 16,
  wheel: 32, keydown: 64, keyup: 128, change: 256, input: 512,
}
const BUTTONS = ['left', 'middle', 'right']
// DOM: these don't bubble
const NO_BUBBLE = new Set(['mouseenter', 'mouseleave'])
const ELEMENT_NODE = 1, TEXT_NODE = 3, COMMENT_NODE = 8, FRAGMENT_NODE = 11
const XHTML = 'http://www.w3.org/1999/xhtml'

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
  get cancelBubble () { return this.propagationStopped }
  set cancelBubble (v) { if (v) this.propagationStopped = true }
  /** The target and its ancestors, innermost first. */
  composedPath () {
    const path = []
    for (let n = this.target; n !== null; n = n.parentNode) path.push(n)
    return path
  }
}

/** Nodes (and strings, as text) for before / after / append / prepend / replaceWith. */
function toNode (parent, n) {
  return typeof n === 'string' ? parent._createText(n) : n
}

class Node {
  constructor () {
    this.ownerWindow = null
    this.id = -1
    this.parentNode = null
    this._first = null
    this._last = null
    this._prev = null
    this._next = null
    this._bindings = null
    this._nextParent = null
    // made by `document`: freed natively when removed and left without a
    // parent at the end of the tick (DOM frameworks drop nodes, never free them)
    this._auto = false
  }

  // accessors on the prototype, as the DOM has them (Svelte reads their getters)
  get firstChild () { return this._first }
  get lastChild () { return this._last }
  get nextSibling () { return this._next }
  get previousSibling () { return this._prev }

  /** The children, as an array (a copy). */
  get childNodes () {
    const a = []
    for (let c = this._first; c !== null; c = c._next) a.push(c)
    return a
  }

  get ownerDocument () { return document }
  get parentElement () { return this.parentNode }
  /** In a window's tree (under its body). */
  get isConnected () {
    let n = this
    while (n.parentNode !== null) n = n.parentNode
    return n.id === 0 && n.ownerWindow !== null
  }

  hasChildNodes () { return this._first !== null }
  contains (other) {
    for (let n = other; n !== null; n = n.parentNode) if (n === this) return true
    return false
  }

  appendChild (child) { return this.insertBefore(child, null) }

  insertBefore (child, before) {
    if (before == null) before = null
    else if (before.parentNode !== this) throw new Error('blenderui: insertBefore: the reference node is not a child of this node')
    if (child.nodeType === FRAGMENT_NODE) { // a fragment inserts its children
      for (let c = child._first; c !== null; c = child._first) this.insertBefore(c, before)
      return child
    }
    if (child === before) return child
    this._adopt(child)
    child._detach()
    this._link(child, before)
    const w = this.ownerWindow
    if (w !== null) {
      if (before === null) w._op(OP.APPEND, this.id, child.id)
      else w._op(OP.INSERT, this.id, child.id, before.id)
    }
    return child
  }

  removeChild (child) {
    if (child.parentNode !== this) return child
    this._unlink(child)
    const w = this.ownerWindow
    if (w !== null) w._op(OP.REMOVE, this.id, child.id)
    if (child._auto && child.ownerWindow !== null) child.ownerWindow._orphan(child)
    return child
  }

  replaceChild (child, old) {
    this.insertBefore(child, old)
    return this.removeChild(old)
  }

  remove () { this.parentNode?.removeChild(this) }

  // ChildNode / ParentNode (strings become text nodes)
  before (...nodes) {
    const p = this.parentNode
    if (p !== null) for (const n of nodes) p.insertBefore(toNode(p, n), this)
  }

  after (...nodes) {
    const p = this.parentNode
    if (p === null) return
    const next = this._next
    for (const n of nodes) p.insertBefore(toNode(p, n), next)
  }

  append (...nodes) { for (const n of nodes) this.insertBefore(toNode(this, n), null) }
  prepend (...nodes) {
    const first = this._first
    for (const n of nodes) this.insertBefore(toNode(this, n), first)
  }

  replaceWith (...nodes) {
    const p = this.parentNode
    if (p === null) return
    const next = this._next
    p.removeChild(this)
    for (const n of nodes) p.insertBefore(toNode(p, n), next)
  }

  /** A detached copy (with its subtree when `deep`); listeners aren't copied. */
  cloneNode (deep = false) {
    const copy = this._clone()
    copy._auto = true
    if (deep) for (let c = this._first; c !== null; c = c._next) copy.appendChild(c.cloneNode(true))
    return copy
  }

  _link (child, before) {
    const prev = before === null ? this._last : before._prev
    child.parentNode = this
    child._prev = prev
    child._next = before
    if (prev === null) this._first = child
    else prev._next = child
    if (before === null) this._last = child
    else before._prev = child
  }

  _unlink (child) {
    const prev = child._prev
    const next = child._next
    if (prev === null) this._first = next
    else prev._next = next
    if (next === null) this._last = prev
    else next._prev = prev
    child.parentNode = child._prev = child._next = null
  }

  _detach () {
    const p = this.parentNode
    if (p === null) return
    p._unlink(this)
    const w = p.ownerWindow
    // same parent: APPEND / INSERT reorders it natively
    if (w !== null && p !== this._nextParent) w._op(OP.REMOVE, p.id, this.id)
  }

  /** Checks before `child` moves under this node (as the DOM does); a
   *  detached child joins this node's window. */
  _adopt (child) {
    for (let n = this; n !== null; n = n.parentNode) {
      if (n === child) throw new Error('blenderui: a node cannot be inserted into itself or its descendants')
    }
    const w = this.ownerWindow
    if (child.ownerWindow !== w) {
      if (child.ownerWindow === null) child._attach(w)
      else if (w !== null) throw new Error('blenderui: the node belongs to another window')
      // else: a window's node parked in a detached node (a fragment): allowed
    }
    child._nextParent = this
  }

  /** Create this detached node and its subtree in `win`. */
  _attach (win) {
    this.ownerWindow = win
    this.id = win._next++
    win._nodes.set(this.id, this)
    this._create(win)
    for (let c = this._first; c !== null; c = c._next) {
      if (c.ownerWindow === null) c._attach(win)
      else if (c.ownerWindow !== win) throw new Error('blenderui: the node belongs to another window')
      win._op(OP.APPEND, this.id, c.id)
    }
  }

  /** A text node in this node's window (or detached). */
  _createText (text) {
    const t = new TextNode('')
    t._auto = this._auto
    t.data = text
    return t
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
    for (let c = this._first; c !== null; c = c._next) c.destroy()
    if (this.ownerWindow !== null) this.ownerWindow._free(this)
    else this._unbindAll()
  }

  /** The text of the subtree (comments excluded). */
  get textContent () {
    let s = ''
    for (let c = this._first; c !== null; c = c._next) if (c.nodeType !== COMMENT_NODE) s += c.textContent
    return s
  }
}

class TextNode extends Node {
  constructor (text) {
    super()
    this._text = text
  }

  get nodeType () { return TEXT_NODE }
  get nodeName () { return '#text' }
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
      const w = this.ownerWindow
      if (w !== null) w._op(OP.SET_TEXT, this.id, w._s(t))
    }
  }

  _create (win) { win._op(OP.TEXT, this.id, win._s(this._text)) }
  _clone () { return new TextNode(this._text) }

  get nodeValue () { return this._text }
  set nodeValue (t) { this.data = t }
  get textContent () { return this._text }
  set textContent (t) { this.data = t }
}

/** A comment: an anchor for DOM frameworks; natively an empty text node,
 *  which takes no space. */
class Comment extends Node {
  constructor (data) {
    super()
    this.data = data
  }

  get nodeType () { return COMMENT_NODE }
  get nodeName () { return '#comment' }
  get nodeValue () { return this.data }
  set nodeValue (v) { this.data = String(v) }
  get textContent () { return this.data }
  set textContent (v) { this.data = String(v) }
  _create (win) { win._op(OP.TEXT, this.id, win._s('')) }
  _clone () { return new Comment(this.data) }
}

/** Nodes to insert together; inserting it moves its children. Never in a tree. */
class DocumentFragment extends Node {
  get nodeType () { return FRAGMENT_NODE }
  get nodeName () { return '#document-fragment' }
  _create () { throw new Error('blenderui: a fragment cannot be inserted itself') }
  _clone () { return new DocumentFragment() }
}

/** `el.style`: CSS text, edited a declaration at a time (`setProperty`). */
class Style {
  constructor (el) { this._el = el }

  get cssText () { return this._el._attrs.get('style') ?? '' }
  set cssText (v) {
    if (v == null || v === '') this._el.removeAttribute('style')
    else this._el.setAttribute('style', v)
  }

  _declarations () {
    const m = new Map()
    for (const d of this.cssText.split(';')) {
      const i = d.indexOf(':')
      if (i > 0) m.set(d.slice(0, i).trim(), d.slice(i + 1).trim())
    }
    return m
  }

  _write (m) {
    let s = ''
    for (const [k, v] of m) s += `${k}: ${v}; `
    this.cssText = s
  }

  getPropertyValue (name) { return this._declarations().get(name) ?? '' }
  setProperty (name, value, priority) {
    if (value == null || value === '') return this.removeProperty(name)
    const m = this._declarations()
    m.set(name, priority ? `${value} !important` : String(value))
    this._write(m)
  }

  removeProperty (name) {
    const m = this._declarations()
    const old = m.get(name)
    if (old === undefined) return ''
    m.delete(name)
    this._write(m)
    return old
  }

  toString () { return this.cssText }
}

/** `el.classList`, over the `class` attribute. */
class ClassList {
  constructor (el) { this._el = el }
  _list () { return (this._el._attrs.get('class') ?? '').split(/\s+/).filter(Boolean) }
  _write (list) { this._el.setAttribute('class', list.join(' ')) }
  get value () { return this._el._attrs.get('class') ?? '' }
  get length () { return this._list().length }
  contains (name) { return this._list().includes(name) }
  add (...names) {
    const list = this._list()
    for (const n of names) if (!list.includes(n)) list.push(n)
    this._write(list)
  }

  remove (...names) { this._write(this._list().filter((n) => !names.includes(n))) }
  toggle (name, force) {
    const on = force ?? !this.contains(name)
    if (on) this.add(name)
    else this.remove(name)
    return on
  }

  toString () { return this.value }
}

class Element extends Node {
  constructor (tag) {
    super()
    this.tagName = tag.toUpperCase()
    this.localName = tag
    this._attrs = new Map()
    this._listeners = new Map()
    this._bits = 0
    this._style = null
    // form controls: the latest value from input / change events
    this._value = undefined
    this._checked = undefined
    this.valueAsNumber = undefined
  }

  get nodeType () { return ELEMENT_NODE }
  get nodeName () { return this.tagName }
  get namespaceURI () { return XHTML }
  get children () { return this.childNodes.filter((c) => c.nodeType === ELEMENT_NODE) }

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

  setAttributeNS (ns, name, value) { this.setAttribute(name, value) }

  _setAttr (name, value) {
    value = String(value)
    if (this._attrs.get(name) === value) return
    this._attrs.set(name, value)
    const w = this.ownerWindow
    if (w !== null) w._op(OP.SET_ATTR, this.id, w._s(name), w._s(value))
  }

  getAttribute (name) { return this._attrs.get(name) ?? null }
  hasAttribute (name) { return this._attrs.has(name) }
  getAttributeNames () { return [...this._attrs.keys()] }
  get attributes () { return [...this._attrs].map(([name, value]) => ({ name, value })) }
  toggleAttribute (name, force) {
    const on = force ?? !this.hasAttribute(name)
    if (on) this.setAttribute(name, '')
    else this.removeAttribute(name)
    return on
  }

  removeAttribute (name) {
    this._unbind(name)
    this._removeAttr(name)
  }

  _removeAttr (name) {
    if (this._attrs.delete(name)) {
      const w = this.ownerWindow
      if (w !== null) w._op(OP.REMOVE_ATTR, this.id, w._s(name))
    }
  }

  get className () { return this._attrs.get('class') ?? '' }
  set className (v) { this.setAttribute('class', v) }
  get classList () { return new ClassList(this) }
  /** The inline style (`cssText`, `setProperty` ...). */
  get style () { return (this._style ??= new Style(this)) }
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
    for (let c = this._first; c !== null; c = this._first) {
      this.removeChild(c)
      if (!c._auto) c.destroy() // a document's nodes: freed at the end of the tick, unless reinserted
    }
    if (t !== '' && t != null) this.appendChild(this._createText(isSignal(t) ? t : String(t)))
  }

  /** Any event type; those BUI reports (`EVENT_BITS`) arrive from the UI thread. */
  addEventListener (type, fn) {
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
    for (const [type, set] of this._listeners) if (set.size) bits |= EVENT_BITS[type] ?? 0
    if (bits !== this._bits) {
      this._bits = bits
      const w = this.ownerWindow
      if (w !== null) w._op(OP.LISTEN, this.id, bits)
    }
  }

  _create (win) {
    win._op(OP.CREATE, this.id, win._s(this.localName))
    for (const [name, value] of this._attrs) win._op(OP.SET_ATTR, this.id, win._s(name), win._s(value))
    if (this._bits !== 0) win._op(OP.LISTEN, this.id, this._bits)
  }

  _clone () {
    const el = new HTMLElement(this.localName)
    for (const [name, value] of this._attrs) el._attrs.set(name, value)
    return el
  }

  _dispatch (raw) {
    if (raw.kind === 'input' || raw.kind === 'change') {
      this._value = raw.value
      this._checked = raw.checked
      this.valueAsNumber = raw.number
    }
    this._propagate(new BuiEvent(raw, this))
  }

  /** Dispatch a DOM event (`new CustomEvent('introstart')` ...) through this
   *  element's listeners and, if it bubbles, its ancestors'. */
  dispatchEvent (event) {
    const own = { value: this, writable: true, configurable: true }
    Object.defineProperties(event, { target: own, currentTarget: own })
    this._propagate(event)
    return !event.defaultPrevented
  }

  _propagate (e) {
    for (let n = this; n !== null; n = n.parentNode) {
      const set = n._listeners?.get(e.type)
      if (set && set.size) {
        e.currentTarget = n
        for (const fn of [...set]) {
          try { fn.call(n, e) } catch (err) { queueMicrotask(() => { throw err }) }
          if (e.immediatePropagationStopped) break
        }
      }
      if (e.cancelBubble || !e.bubbles) break
    }
    e.currentTarget = null
  }

  /** Web Animations aren't run: the animation finishes at once (as React's
   *  `<ViewTransition>` commits without animating). */
  animate () {
    const a = { playState: 'finished', currentTime: 0, onfinish: null, cancel () {}, finish () {}, pause () {}, play () {} }
    a.finished = Promise.resolve(a)
    queueMicrotask(() => a.onfinish?.())
    return a
  }

  /** Layout box `{ x, y, width, height }` in logical pixels (or null). */
  getBoundingClientRect () {
    const w = this.ownerWindow
    if (w === null) return Promise.resolve(null)
    w.flush() // the UI thread must see the tree first
    return w.app.native.bounds(w.id, this.id)
  }
}

/** An HTML element: its DOM properties (`disabled`, `value` ...) are setters
 *  here, below Element, where frameworks look for them (as in browsers). */
class HTMLElement extends Element {
  /** Form controls: the latest value from input / change events, else the attribute. */
  get value () { return this._value ?? this._attrs.get('value') ?? (this.localName === 'input' || this.localName === 'textarea' ? '' : undefined) }
  set value (v) {
    this._value = v == null ? '' : String(v)
    this.setAttribute('value', this._value)
  }

  get checked () { return this._checked ?? this._attrs.has('checked') }
  set checked (v) {
    this._checked = !!v
    this.toggleAttribute('checked', this._checked)
  }
}

// DOM properties that reflect a boolean attribute.
for (const [prop, attr] of [['disabled', 'disabled'], ['hidden', 'hidden'], ['selected', 'selected'],
  ['readOnly', 'readonly'], ['required', 'required'], ['multiple', 'multiple'], ['open', 'open'],
  ['indeterminate', 'indeterminate'], ['autofocus', 'autofocus'], ['defaultChecked', 'checked']]) {
  Object.defineProperty(HTMLElement.prototype, prop, {
    get () { return this._attrs.has(attr) },
    set (v) { this.toggleAttribute(attr, !!v) },
    configurable: true,
  })
}
// ... and a string attribute.
// (not `id`: a node's `id` is its native id; use setAttribute('id', ...))
for (const [prop, attr] of [['type', 'type'], ['name', 'name'], ['placeholder', 'placeholder'],
  ['title', 'title'], ['min', 'min'], ['max', 'max'], ['step', 'step'], ['href', 'href'], ['src', 'src'],
  ['defaultValue', 'value'], ['tabIndex', 'tabindex'], ['htmlFor', 'for']]) {
  Object.defineProperty(HTMLElement.prototype, prop, {
    get () { return this._attrs.get(attr) ?? '' },
    set (v) { this.setAttribute(attr, v) },
    configurable: true,
  })
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

/** An element in `win`, or detached (`win` null). Capitalized tags are
 *  Blender's widgets; their lowercase HTML counterparts look like a browser's
 *  (Blender's inside `styled`). */
function createElement (win, tag) {
  const blender = BLENDER_COMPONENTS[tag]
  const el = new HTMLElement(blender ? blender[0] : tag)
  if (blender) {
    if (blender[1]) el._attrs.set('type', blender[1])
    el._attrs.set('bui-blender', '')
  }
  if (win !== null) el._attach(win)
  else el._auto = true
  return el
}

/** The window-less document of DOM frameworks: its nodes are detached until
 *  inserted into a window's tree, and freed when removed and dropped. */
class Document {
  constructor () {
    this.head = createElement(null, 'head')
    this.activeElement = null
    this._listeners = new Map()
  }

  get nodeType () { return 9 }
  createElement (tag) { return createElement(null, tag) }
  createElementNS (ns, tag) { return createElement(null, tag) }
  createTextNode (text) {
    const t = new TextNode('')
    t._auto = true
    t.data = text
    return t
  }

  createComment (data = '') {
    const c = new Comment(String(data))
    c._auto = true
    return c
  }

  createDocumentFragment () { return new DocumentFragment() }
  importNode (node, deep) { return node.cloneNode(deep) }
  // events never reach the document (they bubble up to a window's body)
  addEventListener () {}
  removeEventListener () {}
}

const document = new Document()

module.exports = {
  OP, EVENT_BITS, BUTTONS, BLENDER_COMPONENTS, BuiEvent, Node, TextNode, Comment, DocumentFragment, Element, HTMLElement, Document,
  document, createElement, styleText,
}
