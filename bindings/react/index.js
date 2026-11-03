// @blender-ui/react: a React renderer for BUI (react-reconciler 0.34 / React 19.3).
//
//   const { render } = require('@blender-ui/react')
//   render(<div className="p-4 bg-[#4772b3]" onClick={...}>Hello</div>,
//          { title: 'App', width: 400, height: 300 })
//
// Host components are HTML elements: React mutates @blender-ui/native's
// retained tree, which sends only the changes of each commit to the native
// UI thread. Events arrive as BUI events (onClick, onChange, onKeyDown ...).
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

const React = require('react')
const Reconciler = require('react-reconciler')
const { ConcurrentRoot, DefaultEventPriority, DiscreteEventPriority, NoEventPriority } =
  require('react-reconciler/constants')
const native = require('@blender-ui/native')
const { App, TextNode, Signal, Memo, isSignal } = native
const hooks = require('./hooks')

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
const NO_PROPS = {}

function eventName (el, prop) {
  if (prop === 'onChange') {
    // text fields report every edit as `input`; checkboxes, selects: `change`
    const type = el.getAttribute('type')
    if (TEXT_INPUTS.has(el.localName) || (el.localName === 'input' &&
        !['checkbox', 'radio', 'range', 'color', 'file'].includes(type))) return 'input'
  }
  return EVENTS[prop]
}

/** Listen with `fn` (or stop, when it isn't a function) for React prop `k`.
 *  The DOM event is remembered with it: onChange's depends on `type`. */
function listen (el, k, fn) {
  const cur = el._react[k]
  const type = typeof fn === 'function' ? eventName(el, k) : null
  if (cur !== undefined && cur.fn === fn && cur.type === type) return
  if (cur !== undefined) el.removeEventListener(cur.type, cur.fn)
  if (type === null) delete el._react[k]
  else {
    el.addEventListener(type, fn)
    el._react[k] = { type, fn }
  }
}

function isText (children) {
  return typeof children === 'string' || typeof children === 'number' || typeof children === 'bigint'
}

/** Apply the difference between two prop objects to an element. Hot: React
 *  calls it for every element of a re-rendered list, mostly with equal props. */
function applyProps (el, prev, next) {
  if (prev !== NO_PROPS) {
    for (const k in prev) {
      if (k in next || SKIP.has(k)) continue
      if (k in EVENTS) {
        listen(el, k, null)
      } else {
        el.removeAttribute(ATTR_ALIASES[k] ?? k)
      }
    }
  }
  let events = false
  for (const k in next) {
    const v = next[k]
    if (prev[k] === v || SKIP.has(k)) continue
    if (k in EVENTS) {
      events = true // after the attributes: onChange's event depends on `type`
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
  if (events || (next.onChange !== undefined && prev.type !== next.type)) {
    for (const k in next) if (k in EVENTS) listen(el, k, next[k])
  }
  // text children; text -> elements is React's resetTextContent (before it
  // inserts them), so nothing to clear here
  const c = next.children
  if (c !== prev.children && isText(c)) el.textContent = String(c)
}

// Nodes a render created. React may throw a render away (an interrupted or
// suspended concurrent render); its nodes are then never attached, and are
// freed after the next commit.
function created (container, node) {
  container.created.push(node)
  return node
}

function sweep (container) {
  const body = container.window.body
  for (const node of container.created) {
    let top = node
    while (top.parentNode !== null) top = top.parentNode
    if (top !== body && container.window._nodes.has(top.id)) top.destroy()
  }
  container.created.length = 0
}

let updatePriority = NoEventPriority
const ROOT_CONTEXT = {}

const host = {
  rendererPackageName: '@blender-ui/react',
  rendererVersion: require('./package.json').version,
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
  resetAfterCommit (container) {
    if (container.created.length > 0) sweep(container)
    container.window.flush() // one native call per commit
  },
  preparePortalMount () {},

  createInstance (type, props, container) {
    if (type === SIGNAL_TEXT) return created(container, container.window.createTextNode(props.signal))
    const el = container.window.createElement(type)
    el._react = {}
    applyProps(el, NO_PROPS, props)
    return created(container, el)
  },
  createTextInstance: (text, container) => created(container, container.window.createTextNode(text)),
  appendInitialChild: (parent, child) => parent.appendChild(child),
  finalizeInitialChildren: () => false,
  shouldSetTextContent: (type, props) => isText(props.children),

  appendChild: (parent, child) => parent.appendChild(child),
  appendChildToContainer: (container, child) => container.window.body.appendChild(child),
  insertBefore: (parent, child, before) => parent.insertBefore(child, before),
  insertInContainerBefore: (container, child, before) => container.window.body.insertBefore(child, before),
  removeChild (parent, child) {
    parent.removeChild(child)
    child.destroy()
  },
  removeChildFromContainer (container, child) {
    container.window.body.removeChild(child)
    child.destroy()
  },
  clearContainer (container) {
    for (const c of [...container.window.body.childNodes]) container.window.body.removeChild(c)
  },
  // the subtree: text set through textContent has no fiber of its own (freeing twice is a no-op)
  detachDeletedInstance: (node) => node.destroy(),

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

  // Commits that wait on resources: none here.
  maySuspendCommitOnUpdate: () => false,
  maySuspendCommitInSyncRender: () => false,
  getSuspendedCommitReason: () => null,

  // View transitions (<ViewTransition>): not animated here. As React DOM does
  // without the browser API: run the commit's phases now and report none.
  startViewTransition (suspendedState, container, types, mutation, layout, afterMutation, spawnedWork) {
    mutation()
    layout()
    spawnedWork()
    return null
  },
  startGestureTransition: () => null,
  stopViewTransition () {},
  suspendOnActiveViewTransition () {},
  addViewTransitionFinishedListener (transition, callback) { callback() },
  createViewTransitionInstance: (name) => ({ name, group: null, imagePair: null, old: null, new: null }),
  applyViewTransitionName () {},
  restoreViewTransitionName () {},
  cancelViewTransitionName () {},
  cancelRootViewTransitionName () {},
  restoreRootViewTransitionName () {},
  cloneRootViewTransitionContainer: (container) => container,
  removeRootViewTransitionClone () {},
  measureInstance: () => null,
  measureClonedInstance: () => null,
  wasInstanceInViewport: () => true,
  hasInstanceChanged: () => false,
  hasInstanceAffectedParent: () => false,
  getCurrentGestureOffset: () => 0,

  // <Fragment ref>: the fragment's fiber, nothing more (no DOM-like API yet).
  createFragmentInstance: (fiber) => ({ _fragmentFiber: fiber }),
  updateFragmentInstanceFiber (fiber, instance) { instance._fragmentFiber = fiber },
  commitNewChildToFragmentInstance () {},
  deleteChildFromFragmentInstance () {},
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
  return (err) => console.error(`@blender-ui/react: ${kind}`, err)
}

// The root component gets render tracking even when created with React's
// own createElement (components below need this package's JSX runtime or
// createElement).
function trackRoot (element) {
  if (!React.isValidElement(element)) return element
  const type = hooks.track(element.type)
  if (type === element.type) return element
  return React.createElement(type, element.key != null ? { ...element.props, key: element.key } : element.props)
}

/** A React root rendering into `window`'s body. */
function createRoot (window) {
  const container = { window, created: [] }
  const root = reconciler.createContainer(container, ConcurrentRoot, null, false, null, 'bui',
    report('uncaught error'), report('caught error'), report('recoverable error'), null)
  window.on('close', () => reconciler.updateContainer(null, root, null, null))
  return {
    render (element) { reconciler.updateContainer(trackRoot(element), root, null, null) },
    unmount () { reconciler.updateContainer(null, root, null, null) },
  }
}

/**
 * Open a window and render `element` into it.
 * @param {React.ReactNode} element
 * @param {{ title?: string, width?: number, height?: number, resizable?: boolean,
 *           gpu?: boolean, slug?: boolean, headless?: boolean, app?: App }} [options]
 * @returns {{ app: App, window: import('@blender-ui/native').Window, root: ReturnType<typeof createRoot> }}
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
  const flushSync = reconciler.flushSyncFromReconciler ?? reconciler.flushSync
  return typeof flushSync === 'function' ? flushSync(fn) : fn()
}

// React's hooks by React's names: those that hold or react to state are the
// signal versions (hooks.js), the others are React's own.
const reactHooks = {}
for (const k of Object.keys(React)) if (/^use([A-Z]|$)/.test(k)) reactHooks[k] = React[k]

module.exports = {
  ...reactHooks,
  ...hooks,
  render,
  createRoot,
  act,
  reconciler,
  signal: native.signal,
  computed: native.memo, // `memo` stays React's (components)
  effect: native.effect,
  batch: native.batch,
  untrack: native.untrack,
  createSignal: native.createSignal,
  ref: native.ref,
  isSignal,
  Signal,
  Memo,
}
