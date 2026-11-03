// Signal twins of React's hooks: `useSignal<Name>` for each React `use<Name>`
// that holds or reacts to state. Same arguments, same return shape, same
// timing, but the state is a signal: pass it as a prop or child and a change
// updates that node without re-rendering the component.
//
// Built only on React's public hooks (useState, useRef, useMemo, the effect
// hooks, startTransition), so they follow React's own semantics (StrictMode,
// concurrent rendering) as React evolves. test/hooks.test.js runs each twin
// beside its React original, and fails when React adds a hook that has
// neither a twin nor an entry in NO_TWIN.
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

const React = require('react')
const S = require('@blenderui/native')
const { signal, memo, effect, batch, untrack, isSignal } = S

/** React hooks without a twin, and why (the coverage test reads this). */
const NO_TWIN = {
  use: 'suspends rendering: inherently a render-time read',
  useContext: 'pass a signal as the context value: consumers bind to it without re-rendering',
  useDebugValue: 'no state',
  useId: 'no state',
  useImperativeHandle: 'no state (works with useSignalRef)',
}

// The latest value, for callbacks that outlive the render that made them.
function useLatest (value) {
  const ref = React.useRef(value)
  ref.current = value
  return ref
}

/** A component-owned signal, created once (lazily when `init` is a function). */
function useOnce (create) {
  return React.useState(create)[0]
}

/** Rust's `use_signal`: a signal owned by this component. */
function useSignal (initial) {
  return useOnce(() => signal(typeof initial === 'function' ? initial() : initial))
}

// ---- state -----------------------------------------------------------------

/** `useState` twin: `[signal, setState]`; `setState(v | prev => next)`. */
function useSignalState (initialState) {
  const s = useSignal(initialState)
  const set = React.useCallback((v) => s.set(typeof v === 'function' ? v(s.peek()) : v), [s])
  return [s, set]
}

/** `useReducer` twin: `[signal, dispatch]`, with the latest render's reducer. */
function useSignalReducer (reducer, initialArg, init) {
  const s = useOnce(() => signal(init !== undefined ? init(initialArg) : initialArg))
  const latest = useLatest(reducer)
  const dispatch = React.useCallback((action) => s.set(latest.current(s.peek(), action)), [s])
  return [s, dispatch]
}

/** `useRef` twin: `.current` is a signal, so memos and effects see the ref change
 *  (an element mounting, for instance). The ref is itself a signal. */
class SignalRef extends S.Signal {
  constructor (value) {
    super(value)
    // an own property: React (dev) rejects ref objects without one
    Object.defineProperty(this, 'current', {
      get () { return this.value },
      set (v) { this.set(v) },
      enumerable: true,
    })
  }
}

function useSignalRef (initialValue) {
  return useOnce(() => new SignalRef(initialValue))
}

// ---- derived values ----------------------------------------------------------

/** `useMemo` twin: a memo, recomputed when the signals it reads change, and
 *  replaced (like useMemo's value) when `deps` change; no deps: every render. */
function useSignalMemo (fn, deps) {
  const latest = useLatest(fn)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return React.useMemo(() => memo(() => latest.current()), deps)
}

/** Rust's `Memo::new`: `useSignalMemo` with `deps = []` by default. */
function useComputed (fn, deps = []) {
  return useSignalMemo(fn, deps)
}

/** `useCallback` twin: the callback runs batched (its writes make one update)
 *  and untracked (calling it in an effect doesn't subscribe to what it reads). */
function useSignalCallback (fn, deps) {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return React.useCallback((...args) => batch(() => untrack(() => fn(...args))), deps)
}

// ---- effects -----------------------------------------------------------------

function effectHook (reactHook) {
  return function (fn, deps) {
    const latest = useLatest(fn)
    // React's timing and deps; in between, re-runs when the signals it read change
    reactHook(() => effect(() => latest.current()), deps)
  }
}

/** `useEffect` twin; also re-runs when the signals it read change. */
const useSignalEffect = effectHook(React.useEffect)
/** `useLayoutEffect` twin. */
const useSignalLayoutEffect = effectHook(React.useLayoutEffect)
/** `useInsertionEffect` twin. */
const useSignalInsertionEffect = effectHook(React.useInsertionEffect)

// ---- external stores ---------------------------------------------------------

/** `useSyncExternalStore` twin: a signal following the store, without re-renders. */
function useSignalSyncExternalStore (subscribe, getSnapshot) {
  const s = useOnce(() => signal(getSnapshot()))
  const latest = useLatest(getSnapshot)
  React.useLayoutEffect(() => {
    const check = () => s.set(latest.current())
    check() // changed between render and subscribe
    return subscribe(check)
  }, [subscribe, s])
  React.useLayoutEffect(() => { s.set(latest.current()) }) // a new getSnapshot
  return s
}

/** Re-render this component when `source` changes; returns its value. */
function useSignalValue (source) {
  const subscribe = React.useCallback((onChange) => {
    let first = true
    return effect(() => {
      source.value // subscribe
      if (first) first = false
      else onChange()
    })
  }, [source])
  return React.useSyncExternalStore(subscribe, () => source.peek())
}

// ---- transitions and actions -------------------------------------------------

// Run `action` as a BUI transition and a React transition at once: signal
// writes defer their effects, React state updates render at transition priority.
function transition (action) {
  return S.startTransition(() => {
    let result
    React.startTransition(() => {
      result = action()
      return result
    })
    return result
  })
}

/** `useTransition` twin: `[isPending signal, startTransition(action)]`. `action`
 *  may be async; isPending stays true until it settles and its deferred
 *  updates have run. */
function useSignalTransition () {
  const isPending = useOnce(() => signal(false))
  const start = React.useCallback((action) => {
    isPending._count = (isPending._count ?? 0) + 1
    S.urgent(() => isPending.set(true))
    transition(action).then(() => {
      if (--isPending._count === 0) S.urgent(() => isPending.set(false))
    })
  }, [isPending])
  return [isPending, start]
}

/** `useDeferredValue` twin: a signal that follows `value` (a signal or a plain
 *  value) at transition priority, after urgent updates. */
function useSignalDeferredValue (value, initialValue) {
  const read = () => (isSignal(value) ? value.peek() : value)
  const out = useOnce(() => signal(initialValue !== undefined ? initialValue : read()))
  const latest = useLatest(value)
  React.useEffect(() => {
    let scheduled = false
    const push = () => {
      if (scheduled) return
      scheduled = true
      S.scheduleDeferred(() => {
        scheduled = false
        const v = latest.current
        out.set(isSignal(v) ? v.peek() : v)
      })
    }
    if (isSignal(value)) return effect(() => { value.value; push() })
    push()
  }, [value, out])
  return out
}

/** `useActionState` twin: `[state signal, dispatch, isPending signal]`.
 *  Dispatches queue and run in order, each with the previous state; an action
 *  may be async. A thrown error goes to the nearest error boundary, and the
 *  queued actions are dropped, as in React. */
function useSignalActionState (action, initialState) {
  const state = useOnce(() => signal(initialState))
  const isPending = useOnce(() => signal(false))
  const latest = useLatest(action)
  const queue = useOnce(() => ({ items: [], running: false }))
  const [, setError] = React.useState(null)

  const runNext = React.useCallback(() => {
    const payload = queue.items.shift()
    queue.running = true
    let result
    transition(() => {
      result = latest.current(state.peek(), payload)
      return result
    })
    Promise.resolve(result).then((next) => {
      const more = queue.items.length > 0
      batch(() => {
        state.set(next)
        if (!more) isPending.set(false)
      })
      if (more) runNext()
      else queue.running = false
    }, (err) => {
      queue.items.length = 0
      queue.running = false
      isPending.set(false)
      setError(() => { throw err })
    })
  }, [state, isPending, queue])

  const dispatch = React.useCallback((payload) => {
    queue.items.push(payload)
    if (!queue.running) {
      S.urgent(() => isPending.set(true))
      runNext()
    }
  }, [queue, isPending, runNext])

  return [state, dispatch, isPending]
}

/** `useOptimistic` twin: `[optimistic signal, addOptimistic]`. Updates added
 *  during a transition or action show at once and are dropped when it ends;
 *  `passthrough` (a signal or a value) shows through otherwise. */
function useSignalOptimistic (passthrough, reducer) {
  const updates = useOnce(() => signal([]))
  const latestReducer = useLatest(reducer)
  const out = React.useMemo(() => memo(() => {
    let v = isSignal(passthrough) ? passthrough.value : passthrough
    for (const u of updates.value) v = latestReducer.current ? latestReducer.current(v, u.action) : u.action
    return v
  }), [passthrough, updates])
  const add = React.useCallback((action) => {
    const entry = { action }
    const remove = () => S.urgent(() => updates.set(updates.peek().filter((e) => e !== entry)))
    S.urgent(() => updates.set([...updates.peek(), entry]))
    if (!S.onTransitionDone(remove)) {
      console.error('An optimistic state update occurred outside a transition or action. ' +
        'To fix, move the update to an action, or wrap it in startTransition.')
      setTimeout(remove, 0)
    }
  }, [updates])
  return [out, add]
}

module.exports = {
  NO_TWIN,
  SignalRef,
  useSignal,
  useComputed,
  useSignalValue,
  useSignalState,
  useSignalReducer,
  useSignalRef,
  useSignalMemo,
  useSignalCallback,
  useSignalEffect,
  useSignalLayoutEffect,
  useSignalInsertionEffect,
  useSignalSyncExternalStore,
  useSignalTransition,
  useSignalDeferredValue,
  useSignalActionState,
  useSignalOptimistic,
}
