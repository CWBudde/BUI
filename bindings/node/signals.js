// Signals: BUI's hooks for JavaScript (the Rust crate's Signal / Memo /
// Effect, bindings/rust/blenderui/src/hooks.rs).
//
//   const count = signal(0)
//   const doubled = memo(() => count.value * 2)   // recomputed when count changes
//   effect(() => console.log(doubled.value))      // re-runs when doubled changes
//   label.textContent = doubled                   // bound: only this text node updates
//   count.value++
//
// Bound to an element (`el.setAttribute('class', sig)`, `el.style = sig`,
// `text.data = sig`, or a signal as a React prop or child), a write updates
// that one node: no component re-renders, no tree rebuilt, almost no garbage.
//
// Push-pull, glitch-free: a write marks direct readers dirty and indirect
// ones "check"; memos recompute lazily and stop propagation when their value
// is unchanged; effects run once per batch, after their sources settle. A
// memo nobody observes holds no subscriptions (it is garbage once dropped)
// and revalidates on read.
//
// Transitions (`startTransition`) mark the effects their writes trigger as
// low priority: they run after urgent work, in 5 ms slices.
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

const CLEAN = 0, CHECK = 1, DIRTY = 2
const NONE = 0, URGENT = 1, DEFERRED = 2

let observer = null // the memo / effect running now (collects what it reads)
let owner = null // the effect running now (owns effects created inside it)
let batchDepth = 0
let flushing = false
let writes = 0 // bumped by every signal write: unobserved memos revalidate against it
const pending = []

let transition = null // the transition whose writes are being made now
const deferred = [] // effects and tasks of transitions, run in slices
let deferredAt = 0
let deferredScheduled = false
const waiting = new Set() // transitions waiting for the deferred queue

function track (source) {
  if (observer !== null && !observer._deps.has(source)) {
    observer._deps.add(source)
    if (observer._attached) source._addSub(observer)
  }
}

/** A value that re-runs what reads it when it changes (Rust's `Signal`). */
class Signal {
  constructor (value, options) {
    this._value = value
    this._subs = new Set()
    this._equals = options?.equals ?? Object.is
  }

  /** Read (and subscribe the running memo / effect). */
  get value () { track(this); return this._value }
  set value (v) { this.set(v) }
  get () { return this.value }
  /** Read without subscribing. */
  peek () { return this._value }

  set (v) {
    if (this._equals(this._value, v)) return
    this._value = v
    writes++
    for (const s of this._subs) s._mark(DIRTY)
    flush()
  }

  /** `count.update(n => n + 1)` */
  update (fn) { this.set(fn(this._value)) }
  /** Rust's `with`: `fn(value)`, subscribing. */
  with (fn) { return fn(this.value) }
  /** Call `fn(value)` now and on every change; returns the unsubscribe function. */
  subscribe (fn) { return effect(() => { fn(this.value) }) }
  toString () { return String(this.value) }
  toJSON () { return this.value }

  _addSub (s) { this._subs.add(s) }
  _removeSub (s) { this._subs.delete(s) }
}

class Computation {
  constructor (fn) {
    this._fn = fn
    this._deps = new Set()
    this._state = DIRTY
    this._attached = false
  }

  _unsubscribe () {
    if (this._attached) for (const d of this._deps) d._removeSub(this)
    this._deps.clear()
  }

  _track () {
    this._unsubscribe()
    const prev = observer
    observer = this
    try {
      return this._fn()
    } finally {
      observer = prev
    }
  }

  /** Bring up to date: re-run only if a source really changed. */
  _refresh () {
    if (this._state === CHECK) {
      for (const d of this._deps) {
        if (d instanceof Memo) {
          d._refresh()
          if (this._state === DIRTY) break
        }
      }
    }
    if (this._state === DIRTY) this._run()
    this._state = CLEAN
  }
}

/** A derived value, recomputed only when what it read changed (Rust's `Memo`). */
class Memo extends Computation {
  constructor (fn, options) {
    super(fn)
    this._subs = new Set()
    this._value = undefined
    this._init = false
    this._seen = -1 // `writes` when last known up to date while detached
    this._equals = options?.equals ?? Object.is
  }

  get value () {
    track(this)
    this._refresh()
    return this._value
  }

  get () { return this.value }
  peek () { this._refresh(); return this._value }
  with (fn) { return fn(this.value) }
  subscribe (fn) { return effect(() => { fn(this.value) }) }
  toString () { return String(this.value) }
  toJSON () { return this.value }

  _refresh () {
    if (this._attached) return super._refresh()
    // nobody observes this memo: revalidate against the global write count
    if (this._seen !== writes || !this._init) this._run()
    this._seen = writes
    this._state = CLEAN
  }

  _run () {
    const v = this._track()
    if (!this._init || !this._equals(this._value, v)) {
      this._init = true
      this._value = v
      for (const s of this._subs) s._state = DIRTY
    }
  }

  _mark (state) {
    if (this._state >= state) return
    const was = this._state
    this._state = state
    if (was === CLEAN) for (const s of this._subs) s._mark(CHECK)
  }

  _addSub (s) {
    this._subs.add(s)
    if (!this._attached) {
      // first observer: subscribe to the sources (stale if anything was written since)
      this._attached = true
      if (this._seen !== writes) this._state = DIRTY
      for (const d of this._deps) d._addSub(this)
    }
  }

  _removeSub (s) {
    this._subs.delete(s)
    if (this._subs.size === 0 && this._attached) {
      // last observer gone: let go of the sources, so this memo can be collected
      for (const d of this._deps) d._removeSub(this)
      this._attached = false
      this._seen = this._state === CLEAN ? writes : -1
    }
  }
}

/** Runs `fn` now and again when what it read changes (Rust's `Effect`). */
class Effect extends Computation {
  constructor (fn) {
    super(fn)
    this._attached = true
    this._cleanup = undefined
    this._children = null
    this._disposed = false
    this._queued = NONE
    if (owner !== null) (owner._children ??= []).push(this)
    this._run()
    this._state = CLEAN
  }

  _run () {
    this._clean()
    const prevOwner = owner
    owner = this
    try {
      const r = this._track()
      if (typeof r === 'function') this._cleanup = r
    } finally {
      owner = prevOwner
    }
  }

  _clean () {
    if (this._children !== null) {
      for (const c of this._children) c.dispose()
      this._children = null
    }
    if (this._cleanup !== undefined) {
      const c = this._cleanup
      this._cleanup = undefined
      untrack(c)
    }
  }

  _mark (state) {
    if (state > this._state) this._state = state
    if (transition === null) {
      if (this._queued !== URGENT) {
        this._queued = URGENT
        pending.push(this)
      }
    } else if (this._queued === NONE) {
      this._queued = DEFERRED
      defer(this)
    }
  }

  dispose () {
    if (this._disposed) return
    this._disposed = true
    this._unsubscribe()
    this._clean()
  }
}

function runEffect (e) {
  e._queued = NONE
  if (e._disposed) return
  try {
    e._refresh()
  } catch (err) {
    e._state = CLEAN
    reportError(err)
  }
}

function flush () {
  if (batchDepth > 0 || flushing) return
  flushing = true
  try {
    for (let i = 0; i < pending.length; i++) {
      if (i > 100000) throw new Error('blenderui: effects keep re-triggering each other')
      if (pending[i]._queued === URGENT) runEffect(pending[i])
    }
  } finally {
    pending.length = 0
    flushing = false
  }
}

function reportError (err) {
  // like a browser: one effect's error doesn't stop the others
  queueMicrotask(() => { throw err })
}

// ---- transitions -----------------------------------------------------------

class Transition {
  constructor () {
    this._open = 1 // the synchronous part
    this._done = []
    this._queued = false
  }

  _hold () { this._open++ }
  _release () {
    if (--this._open === 0) for (const f of this._done.splice(0)) f()
  }
}

const now = typeof performance !== 'undefined' ? () => performance.now() : () => Date.now()
const macrotask = typeof setImmediate === 'function' ? setImmediate : (f) => setTimeout(f, 0)

/** Queue an effect or task of the current transition. */
function defer (item) {
  deferred.push(item)
  if (transition !== null && !transition._queued) {
    transition._queued = true
    transition._hold()
    waiting.add(transition)
  }
  if (!deferredScheduled) {
    deferredScheduled = true
    macrotask(runDeferred)
  }
}

function runDeferred () {
  deferredScheduled = false
  const end = now() + 5
  while (deferredAt < deferred.length) {
    const item = deferred[deferredAt++]
    const t = item._transition
    const prev = transition
    transition = t ?? IDLE
    batchDepth++
    try {
      if (typeof item === 'function') {
        try { item() } catch (err) { reportError(err) }
      } else if (item._queued === DEFERRED) {
        runEffect(item)
      }
    } finally {
      batchDepth--
      transition = prev
    }
    flush()
    if (now() >= end && deferredAt < deferred.length) {
      // yield to urgent work (input, frames), then carry on
      deferredScheduled = true
      macrotask(runDeferred)
      return
    }
  }
  deferred.length = 0
  deferredAt = 0
  const done = [...waiting]
  waiting.clear()
  for (const t of done) {
    t._queued = false
    t._release()
  }
}

const IDLE = new Transition()

/**
 * Run `fn` as a transition: what its signal writes trigger (bindings,
 * effects) runs after urgent work, in slices. Like React, only writes made
 * before `fn`'s first `await` belong to it. Returns a promise that settles
 * when `fn` (and its promise, if any) and the deferred work are done.
 */
function startTransition (fn) {
  const t = new Transition()
  const done = new Promise((resolve) => t._done.push(resolve))
  const prev = transition
  transition = t
  batchDepth++
  let result
  try {
    result = fn()
  } finally {
    batchDepth--
    transition = prev
    flush()
  }
  if (result && typeof result.then === 'function') {
    t._hold()
    result.then(() => t._release(), (err) => { reportError(err); t._release() })
  }
  t._release()
  return done
}

/** Inside a transition: call `cb` when it completes. False outside one. */
function onTransitionDone (cb) {
  if (transition === null || transition === IDLE) return false
  transition._done.push(cb) // the transition is open while its body runs
  return true
}

/** Run `fn` at transition priority, later (deferred values). */
function scheduleDeferred (fn) {
  fn._transition = IDLE
  const prev = transition
  transition = IDLE
  try {
    defer(fn)
  } finally {
    transition = prev
  }
}

/** Run `fn` with urgent priority, even inside a transition. */
function urgent (fn) {
  const prev = transition
  transition = null
  try {
    return fn()
  } finally {
    transition = prev
  }
}

const inTransition = () => transition !== null

// ---- API -------------------------------------------------------------------

/** `signal(0)`: a read-write signal (`.value`, `.set`, `.update`). */
function signal (value, options) { return new Signal(value, options) }

/** `memo(() => a.value + b.value)`: a lazily derived, cached signal. */
function memo (fn, options) { return new Memo(fn, options) }

/** `effect(() => ...)`: runs now and on changes; `fn` may return a cleanup. Returns `dispose`. */
function effect (fn) {
  const e = new Effect(fn)
  return () => e.dispose()
}

/** Run `fn`, deferring effects until it returns (several writes, one update). */
function batch (fn) {
  batchDepth++
  try {
    return fn()
  } finally {
    batchDepth--
    flush()
  }
}

/** Run `fn` without subscribing to what it reads. */
function untrack (fn) {
  const prev = observer
  observer = null
  try {
    return fn()
  } finally {
    observer = prev
  }
}

/** Leptos / Solid style: `const [count, setCount] = createSignal(0)`. */
function createSignal (value, options) {
  const s = new Signal(value, options)
  return [() => s.value, (v) => s.set(typeof v === 'function' ? v(s._value) : v)]
}

const isSignal = (x) => x instanceof Signal || x instanceof Memo

module.exports = {
  Signal,
  Memo,
  Effect,
  signal,
  memo,
  computed: memo,
  effect,
  batch,
  untrack,
  createSignal,
  isSignal,
  startTransition,
  urgent,
  inTransition,
  onTransitionDone,
  scheduleDeferred,
}
