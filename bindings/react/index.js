// @blenderui/react: a React renderer for BUI (react-reconciler 0.31 / React 19).
//
//   const { render } = require('@blenderui/react')
//   render(<div className="p-4 bg-[#4772b3]" onClick={...}>Hello</div>,
//          { title: 'App', width: 400, height: 300 })
//
// Host components are HTML elements: React mutates @blenderui/native's
// retained tree, which sends only the changes of each commit to the native
// UI thread. Events arrive as BUI events (onClick, onChange, onKeyDown ...).
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

const React = require('react')
const Reconciler = require('react-reconciler')
const { ConcurrentRoot, DefaultEventPriority, DiscreteEventPriority, NoEventPriority } =
  require('react-reconciler/constants')
const native = require('@blenderui/native')
const { App, TextNode, Signal, Memo, signal, memo, effect, isSignal } = native

// A signal rendered as a child (`<span>{count}</span>`) becomes this host
// type: a text node bound to the signal, updated without re-rendering.
const SIGNAL_TEXT = '#bui-signal-text'
for (const C of [Signal, Memo]) {
  Object.defineProperties(C.prototype, {
    $$typeof: { value: Symbol.for('react.transitional.element'), configurable: true },
    type: { value: SIGNAL_TEXT, configurable: true },
    key: { value: null, configurable: true },
    ref: { value: null, configurable: true },
    _owner: { value: null, configurable: true },
    // React (dev) marks list children validated here: a fresh object each time
    _store: { get () { return { validated: 1 } }, configurable: true },
    props: { get () { return (this._reactProps ??= { signal: this }) }, configurable: true },
  })
}

// React prop -> BUI event; onChange is per keystroke on text fields, as in React DOM.
const EVENTS = {
  onClick: 'click', onMouseDown: 'mousedown', onMouseUp: 'mouseup', onMouseMove: 'mousemove',
  onMouseEnter: 'mouseenter', onMouseLeave: 'mouseleave', onWheel: 'wheel',
  onKeyDown: 'keydown', onKeyUp: 'keyup', onInput: 'input', onChange: 'change',
}
const TEXT_INPUTS = new Set(['textarea'])
const ATTR_ALIASES = { className: 'class', htmlFor: 'for', tabIndex: 'tabindex' }
const SKIP = new Set(['children', 'key', 'ref', 'dangerouslySetInnerHTML', 'suppressHydrationWarning',
  'suppressContentEditableWarning'])

function eventName (el, prop) {
  if (prop === 'onChange') {
    // text fields report every edit as `input`; checkboxes, selects: `change`
    const type = el.getAttribute('type')
    if (TEXT_INPUTS.has(el.localName) || (el.localName === 'input' &&
        !['checkbox', 'radio', 'range', 'color', 'file'].includes(type))) return 'input'
  }
  return EVENTS[prop]
}

function isText (children) {
  return typeof children === 'string' || typeof children === 'number' || typeof children === 'bigint'
}

/** Apply the difference between two prop objects to an element. */
function applyProps (el, prev, next) {
  for (const k in prev) {
    if (SKIP.has(k) || k in next) continue
    if (k in EVENTS) {
      el.removeEventListener(eventName(el, k), el._react[k])
      delete el._react[k]
    } else {
      el.removeAttribute(ATTR_ALIASES[k] ?? k)
    }
  }
  for (const k in next) {
    const v = next[k]
    if (SKIP.has(k) || prev[k] === v) continue
    if (k in EVENTS) {
      const type = eventName(el, k)
      if (el._react[k]) el.removeEventListener(type, el._react[k])
      if (typeof v === 'function') el.addEventListener(type, (el._react[k] = v))
      else delete el._react[k]
    } else if (k === 'style') {
      if (v == null) el.removeAttribute('style')
      else el.style = v // a string, an object or a signal of either
    } else if (v == null || v === false) {
      el.removeAttribute(ATTR_ALIASES[k] ?? k)
    } else if (isSignal(v)) {
      el.setAttribute(ATTR_ALIASES[k] ?? k, v) // bound: updates without a re-render
    } else {
      el.setAttribute(ATTR_ALIASES[k] ?? k, v === true ? '' : v)
    }
  }
  if (isText(next.children) && next.children !== prev.children) el.textContent = String(next.children)
  else if (isText(prev.children) && !isText(next.children)) el.textContent = ''
}

let updatePriority = NoEventPriority
const ROOT_CONTEXT = {}

const host = {
  rendererPackageName: '@blenderui/react',
  rendererVersion: '0.1.0',
  supportsMutation: true,
  supportsPersistence: false,
  supportsHydration: false,
  supportsMicrotasks: true,
  isPrimaryRenderer: true,
  warnsIfNotActing: false,
  noTimeout: -1,
  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,
  scheduleMicrotask: queueMicrotask,

  getRootHostContext: () => ROOT_CONTEXT, // must not be null (React checks)
  getChildHostContext: (parent) => parent,
  getPublicInstance: (instance) => instance,
  prepareForCommit: () => null,
  resetAfterCommit (container) { container.window.flush() }, // one native call per commit
  preparePortalMount () {},

  createInstance (type, props, container) {
    if (type === SIGNAL_TEXT) return container.window.createTextNode(props.signal)
    const el = container.window.createElement(type)
    el._react = {}
    applyProps(el, {}, props)
    return el
  },
  createTextInstance: (text, container) => container.window.createTextNode(text),
  appendInitialChild: (parent, child) => parent.appendChild(child),
  finalizeInitialChildren: () => false,
  shouldSetTextContent: (type, props) => isText(props.children),

  appendChild: (parent, child) => parent.appendChild(child),
  appendChildToContainer: (container, child) => container.window.body.appendChild(child),
  insertBefore: (parent, child, before) => parent.insertBefore(child, before),
  insertInContainerBefore: (container, child, before) => container.window.body.insertBefore(child, before),
  removeChild: (parent, child) => parent.removeChild(child),
  removeChildFromContainer: (container, child) => container.window.body.removeChild(child),
  clearContainer (container) {
    for (const c of [...container.window.body.childNodes]) container.window.body.removeChild(c)
  },
  detachDeletedInstance: (node) => node.ownerWindow._free(node),

  commitUpdate (el, type, prev, next) {
    if (type === SIGNAL_TEXT) {
      if (prev.signal !== next.signal) el.data = next.signal
    } else {
      applyProps(el, prev, next)
    }
  },
  commitTextUpdate: (text, prev, next) => { text.data = next },
  commitMount () {},
  resetTextContent: (el) => { el.textContent = '' },
  hideInstance (el) {
    if (el instanceof TextNode) el.data = '' // a signal's text: unbound while hidden
    else el.setAttribute('hidden', '')
  },
  unhideInstance (el, props) {
    if (el instanceof TextNode) el.data = props.signal
    else if (!props.hidden) el.removeAttribute('hidden')
  },
  hideTextInstance: (text) => { text._hidden = text.data; text.data = '' },
  unhideTextInstance: (text, value) => { text.data = value },

  setCurrentUpdatePriority: (p) => { updatePriority = p },
  getCurrentUpdatePriority: () => updatePriority,
  resolveUpdatePriority: () => updatePriority || DefaultEventPriority,
  resolveEventType: () => null,
  resolveEventTimeStamp: () => -1.1,
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent () {},
  requestPostPaintCallback () {},
  maySuspendCommit: () => false,
  preloadInstance: () => true,
  startSuspendingCommit () {},
  suspendInstance () {},
  waitForCommitToBeReady: () => null,
  NotPendingTransition: null,
  HostTransitionContext: React.createContext(null),
  resetFormInstance () {},
  getInstanceFromNode: () => null,
  getInstanceFromScope: () => null,
  beforeActiveInstanceBlur () {},
  afterActiveInstanceBlur () {},
  prepareScopeUpdate () {},
  bindToConsole: (method, args) => Function.prototype.bind.apply(console[method], [console, ...args]),
}

const reconciler = Reconciler(host)

// Events: run handlers as discrete updates (React DOM does the same), so a
// click's state change commits right away.
const nativeDispatch = native.Element.prototype._dispatch
native.Element.prototype._dispatch = function (raw) {
  const prev = updatePriority
  updatePriority = DiscreteEventPriority
  try {
    nativeDispatch.call(this, raw)
  } finally {
    updatePriority = prev
  }
}

function report (kind) {
  return (err) => console.error(`@blenderui/react: ${kind}`, err)
}

/** A React root rendering into `window`'s body. */
function createRoot (window) {
  const container = { window }
  const root = reconciler.createContainer(container, ConcurrentRoot, null, false, null, 'bui',
    report('uncaught error'), report('caught error'), report('recoverable error'), null)
  return {
    render (element) { reconciler.updateContainer(element, root, null, null) },
    unmount () { reconciler.updateContainer(null, root, null, null) },
  }
}

/**
 * Open a window and render `element` into it.
 * @param {React.ReactNode} element
 * @param {{ title?: string, width?: number, height?: number, resizable?: boolean,
 *           gpu?: boolean, slug?: boolean, headless?: boolean, app?: App }} [options]
 * @returns {{ app: App, window: import('@blenderui/native').Window, root: ReturnType<typeof createRoot> }}
 */
function render (element, options = {}) {
  const app = options.app ?? new App({ gpu: options.gpu, slug: options.slug, headless: options.headless })
  const window = app.window(options)
  const root = createRoot(window)
  root.render(element)
  return { app, window, root }
}

/** Run `fn` and flush React's work synchronously (tests). */
function act (fn) {
  reconciler.flushSyncFromReconciler?.(fn) ?? reconciler.flushSync?.(fn) ?? fn()
}

// React's hooks, re-exported as they are (whatever this React version has),
// so one import serves both; then BUI's signal twins (hooks.js).
const reactHooks = {}
for (const k of Object.keys(React)) if (/^use([A-Z]|$)/.test(k)) reactHooks[k] = React[k]

module.exports = {
  ...reactHooks,
  ...require('./hooks'),
  render,
  createRoot,
  act,
  reconciler,
  signal,
  memo,
  computed: memo,
  effect,
  batch: native.batch,
  untrack: native.untrack,
  createSignal: native.createSignal,
  startTransition: native.startTransition,
}
