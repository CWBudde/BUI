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
  /** `count + 1`, `count > 5` read the value (`===` and truthiness don't: use `.value`). */
  valueOf () { return this.value }
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
    if (this._state === DIRTY) {
      this._state = CLEAN // a mark made while running (a write to a source) re-queues it
      this._run()
    } else {
      this._state = CLEAN
    }
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
    this._error = null // { error } when the computation threw: rethrown on read
    this._lowPriority = false // marked by a transition only
    this._equals = options?.equals ?? Object.is
  }

  get value () {
    track(this)
    this._refresh()
    if (this._error !== null) throw this._error.error
    return this._value
  }

  get () { return this.value }
  peek () {
    this._refresh()
    if (this._error !== null) throw this._error.error
    return this._value
  }
  with (fn) { return fn(this.value) }
  subscribe (fn) { return effect(() => { fn(this.value) }) }
  toString () { return String(this.value) }
  /** `count + 1`, `count > 5` read the value (`===` and truthiness don't: use `.value`). */
  valueOf () { return this.value }
  toJSON () { return this.value }

  _refresh () {
    if (this._attached) return super._refresh()
    // nobody observes this memo: revalidate against the global write count
    const at = writes
    this._state = CLEAN
    if (this._seen !== at || !this._init) this._run()
    this._seen = at
  }

  _run () {
    let v
    try {
      v = this._track()
    } catch (error) {
      this._init = true
      this._error = { error }
      for (const s of this._subs) if (s._state === CHECK) s._state = DIRTY // readers waiting on this check
      return
    }
    if (!this._init || this._error !== null || !this._equals(this._value, v)) {
      this._init = true
      this._error = null
      this._value = v
      for (const s of this._subs) if (s._state === CHECK) s._state = DIRTY // readers waiting on this check
    }
  }

  _mark (state) {
    const urgent = transition === null
    const was = this._state
    // already marked: pass it on only to make a transition's mark urgent
    if (was >= state && !(urgent && this._lowPriority)) return
    if (state > was) this._state = state
    const propagate = was === CLEAN || (urgent && this._lowPriority)
    this._lowPriority = !urgent && (was === CLEAN || this._lowPriority)
    if (propagate) for (const s of this._subs) s._mark(CHECK)
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
    this._state = CLEAN
    batchDepth++ // writes made by this run re-run it after, not inside, the run
    try {
      this._run()
    } catch (error) {
      this.dispose() // no half-subscribed effect left behind
      throw error
    } finally {
      batchDepth--
      flush()
    }
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
    } else if (this._queued === DEFERRED) {
      holdTransition() // this transition also waits for the deferred run
    }
  }

  dispose () {
    if (this._disposed) return
    this._disposed = true
    this._unsubscribe()
    this._attached = false // reads after dispose (mid-run) don't subscribe
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
  let i = 0
  try {
    for (; i < pending.length; i++) {
      if (i > 100000) throw new Error('blenderui: effects keep re-triggering each other')
      if (pending[i]._queued === URGENT) runEffect(pending[i])
    }
  } finally {
    for (; i < pending.length; i++) if (pending[i]._queued === URGENT) pending[i]._queued = NONE
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

/** The current transition waits for the deferred queue to drain. */
function holdTransition () {
  if (transition !== null && !transition._queued) {
    transition._queued = true
    transition._hold()
    waiting.add(transition)
  }
}

/** Queue an effect or task of the current transition. */
function defer (item) {
  deferred.push(item)
  holdTransition()
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
 * when `fn` (and its promise, if any) and the deferred work are done; it
 * rejects with the error of an async `fn` (a synchronous throw propagates).
 */
function startTransition (fn) {
  const t = new Transition()
  let failure = null
  const done = new Promise((resolve, reject) => {
    t._done.push(() => (failure ? reject(failure.error) : resolve()))
  })
  const prev = transition
  transition = t
  batchDepth++
  let result
  try {
    result = fn()
  } catch (error) {
    t._release()
    throw error
  } finally {
    batchDepth--
    transition = prev
    flush()
  }
  if (result && typeof result.then === 'function') {
    t._hold()
    result.then(() => t._release(), (error) => { failure = { error }; t._release() })
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

// ---- render tracking (components) ------------------------------------------

/**
 * What a component's render read, and a callback when any of it changes.
 * `run(fn)` collects reads without subscribing (a render may be thrown
 * away); `commit(reads)` subscribes once the render is committed; `pause`
 * / `resume` follow mount and unmount. Built for React components.
 */
class Tracker extends Computation {
  constructor (onChange) {
    super(null)
    this._attached = true
    this._state = CLEAN
    this._queued = NONE
    this._disposed = false
    this._onChange = onChange
    this._pausedAt = 0
  }

  /** Run `fn`, collecting the signals it reads. Returns `[result, reads]`. */
  run (fn) {
    const reads = { _deps: new Set(), _attached: false, _render: true, _at: 0 }
    const prev = observer
    observer = reads
    try {
      const result = fn()
      reads._at = writes // writes made by the render itself don't count
      return [result, reads]
    } finally {
      observer = prev
    }
  }

  /** The render that read `reads` was committed: follow those signals now. */
  commit (reads) {
    if (reads === null) return
    const next = reads._deps
    if (this._attached) {
      for (const d of this._deps) if (!next.has(d)) d._removeSub(this)
      for (const d of next) if (!this._deps.has(d)) d._addSub(this)
    }
    this._deps = next
    this._state = CLEAN
    if (writes !== reads._at && next.size > 0) this._onChange() // changed in between
  }

  pause () {
    if (!this._attached) return
    for (const d of this._deps) d._removeSub(this)
    this._attached = false
    this._pausedAt = writes
  }

  resume () {
    if (this._attached) return
    this._attached = true
    for (const d of this._deps) d._addSub(this)
    if (writes !== this._pausedAt && this._deps.size > 0) this._onChange()
  }

  _run () { this._onChange() }
  dispose () { this.pause(); this._disposed = true }
}
Tracker.prototype._mark = Effect.prototype._mark

const inRender = () => observer !== null && observer._render === true

/** A ref whose `.current` is a signal for effects and memos, but not for
 *  renders (reading a ref while rendering doesn't subscribe, as in React). */
class Ref extends Signal {
  constructor (value) {
    super(value)
    // an own property: React (dev) rejects ref objects without one
    Object.defineProperty(this, 'current', {
      get () { return inRender() ? this._value : this.value },
      set (v) { this.set(v) },
      enumerable: true,
    })
  }
}

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

/** Run `fn` outside any owning effect: effects it creates live until disposed. */
function unowned (fn) {
  const prev = owner
  owner = null
  try {
    return fn()
  } finally {
    owner = prev
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

/** A ref (`{ current }`) whose changes effects and memos can follow. */
function ref (value) { return new Ref(value) }

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
  unowned,
  createSignal,
  isSignal,
  startTransition,
  urgent,
  inTransition,
  onTransitionDone,
  scheduleDeferred,
  Tracker,
  Ref,
  ref,
}
