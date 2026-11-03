// React's hooks, by React's names, with signals for state.
//
// `useState`, `useReducer`, `useMemo` ... take React's arguments, run at
// React's moments and return React's shapes, but the state they hold is a
// signal. Pass it as a prop or child (`<span>{count}</span>`) and a change
// updates that one node: the component doesn't re-render. Read it while
// rendering (`count.value`, `count + 1`, a condition) and the component
// re-renders when it changes, exactly as with React's own hooks.
//
// The second part needs to know which component is rendering: components
// created through this package's JSX runtime (`jsxImportSource:
// "@blender-ui/react"`) or `createElement` are wrapped with `track()`.
//
// Built only on React's public hooks (useState, useReducer, useRef, useMemo,
// the effect hooks, startTransition), so React's semantics (StrictMode,
// concurrent rendering) come from React itself. test/hooks.test.js runs each
// hook beside React's original and fails when React adds a hook that has
// neither a signal version here nor an entry in NO_TWIN.
//
// SPDX-License-Identifier: GPL-2.0-or-later
'use strict'

const React = require('react')
const S = require('@blender-ui/native')
const { signal, memo, effect, batch, untrack, isSignal, Tracker, Ref } = S

/** React hooks re-exported unchanged, and why (the coverage test reads this). */
const NO_TWIN = {
  use: 'suspends rendering: inherently a render-time read',
  useContext: 'pass a signal as the context value: consumers bind to it without re-rendering',
  useDebugValue: 'no state',
  useId: 'no state',
  useImperativeHandle: 'no state (works with useRef)',
}

// The latest value, for callbacks that outlive the render that made them.
function useLatest (value) {
  const ref = React.useRef(value)
  ref.current = value
  return ref
}

/** A component-owned value, created once. */
function useOnce (create) {
  return React.useState(create)[0]
}

// ---- render tracking -----------------------------------------------------------

const bump = (n) => n + 1

/** Re-render this component when a signal its render read changes. */
function useRenderTracker () {
  const [, force] = React.useReducer(bump, 0)
  const tracker = useOnce(() => new Tracker(() => force()))
  const reads = React.useRef(null)
  // after each commit: follow what that render read
  React.useLayoutEffect(() => { tracker.commit(reads.current) })
  // mount / unmount (StrictMode re-mounts: pause, then resume)
  React.useLayoutEffect(() => {
    tracker.resume()
    return () => tracker.pause()
  }, [tracker])
  return { tracker, reads }
}

const tracked = new WeakMap()
const MEMO = Symbol.for('react.memo')
const FORWARD_REF = Symbol.for('react.forward_ref')

/** `type` with render tracking: function components, `memo` and `forwardRef`. */
function track (type) {
  if (type == null) return type
  if (typeof type === 'function') {
    if (type.prototype?.isReactComponent) return type // class components: no hooks
  } else if (typeof type !== 'object' || (type.$$typeof !== MEMO && type.$$typeof !== FORWARD_REF)) {
    return type // host components, fragments, providers, lazy ...
  }
  let t = tracked.get(type)
  if (t !== undefined) return t
  if (typeof type === 'function') {
    t = function (props, secondArg) {
      const { tracker, reads } = useRenderTracker()
      const [result, r] = tracker.run(() => type(props, secondArg))
      reads.current = r
      return result
    }
    Object.defineProperty(t, 'name', { value: type.name })
    t.displayName = type.displayName || type.name
  } else if (type.$$typeof === MEMO) {
    t = React.memo(track(type.type), type.compare)
    t.displayName = type.displayName
  } else {
    t = React.forwardRef(track(type.render))
    t.displayName = type.displayName
  }
  tracked.set(type, t)
  tracked.set(t, t)
  return t
}

/** React's `createElement`, with render tracking for components. */
function createElement (type, props, ...children) {
  return React.createElement(track(type), props, ...children)
}

// ---- state -----------------------------------------------------------------

/** Rust's `use_signal`: a signal owned by this component. */
function useSignal (initial) {
  return useOnce(() => signal(typeof initial === 'function' ? initial() : initial))
}

/** `[signal, setState]`; `setState(v | prev => next)`; the same value: no update. */
function useState (initialState) {
  const s = useSignal(initialState)
  const set = React.useCallback((v) => s.set(typeof v === 'function' ? v(s.peek()) : v), [s])
  return [s, set]
}

/** `[signal, dispatch]`, with the latest render's reducer (as React does). */
function useReducer (reducer, initialArg, init) {
  const s = useOnce(() => signal(init !== undefined ? init(initialArg) : initialArg))
  const latest = useLatest(reducer)
  const dispatch = React.useCallback((action) => s.set(latest.current(s.peek(), action)), [s])
  return [s, dispatch]
}

/** `{ current }`, observable by effects and memos (an element mounting re-runs
 *  them); reading it while rendering doesn't subscribe, as with React's refs. */
function useRef (initialValue) {
  return useOnce(() => new Ref(initialValue))
}

// ---- derived values ----------------------------------------------------------

/** A memo: recomputed when the signals it reads change, and replaced (like
 *  useMemo's value) when `deps` change; without deps, every render. */
function useMemo (fn, deps) {
  const latest = useLatest(fn)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return React.useMemo(() => memo(() => latest.current()), deps)
}

/** Rust's `Memo::new`: `useMemo` with `deps = []` by default. */
function useComputed (fn, deps = []) {
  return useMemo(fn, deps)
}

/** React's `useCallback`; calls are batched (their writes make one update) and
 *  untracked (calling it in an effect doesn't subscribe to what it reads). */
function useCallback (fn, deps) {
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return React.useCallback((...args) => batch(() => untrack(() => fn(...args))), deps)
}

/** React's `useEffectEvent`: a stable function that calls the latest render's
 *  `fn`; calls are batched and untracked (an effect calling it doesn't
 *  subscribe to the signals `fn` reads). */
function useEffectEvent (fn) {
  const latest = useLatest(fn)
  return React.useCallback((...args) => batch(() => untrack(() => latest.current(...args))), [latest])
}

// ---- effects -----------------------------------------------------------------

function effectHook (reactHook) {
  return function (fn, deps) {
    const latest = useLatest(fn)
    // React's timing and deps; in between, re-runs when the signals it read change
    reactHook(() => effect(() => latest.current()), deps)
  }
}

/** React's `useEffect`, also re-run when the signals it read change. */
const useEffect = effectHook(React.useEffect)
/** React's `useLayoutEffect`, also re-run when the signals it read change. */
const useLayoutEffect = effectHook(React.useLayoutEffect)
/** React's `useInsertionEffect`, also re-run when the signals it read change. */
const useInsertionEffect = effectHook(React.useInsertionEffect)

// ---- external stores ---------------------------------------------------------

/** A signal following the store. */
function useSyncExternalStore (subscribe, getSnapshot) {
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

/** Re-render this component when `source` changes; returns its value (for
 *  components rendered without this package's JSX runtime). */
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
// React tracks the action's promise (its optimistic updates last until it
// settles) but never sees its rejection: each hook sends errors where React's does.
const quiet = () => {}
function transition (action) {
  return S.startTransition(() => {
    let result
    let failure = null
    React.startTransition(() => {
      try {
        result = action()
      } catch (error) {
        failure = { error } // React would report it globally and swallow it
        return
      }
      return result && typeof result.then === 'function' ? result.then(quiet, quiet) : undefined
    })
    if (failure !== null) throw failure.error
    return result
  })
}

/** React's `startTransition`, for signal writes too. Returns a promise that
 *  settles when the action and the updates it caused are done. */
function startTransition (action) {
  return transition(action)
}

/** A setter that rethrows `error` during render: React's way to reach the
 *  nearest error boundary from an async callback. */
function useThrowToBoundary () {
  const [, set] = React.useState(null)
  return React.useCallback((error) => set(() => { throw error }), [])
}

/** `[isPending signal, startTransition(action)]`. `action` may be async;
 *  isPending stays true until it settles and its deferred updates have run.
 *  An error goes to the nearest error boundary. */
function useTransition () {
  const isPending = useOnce(() => signal(false))
  const fail = useThrowToBoundary()
  const start = React.useCallback((action) => {
    isPending._count = (isPending._count ?? 0) + 1
    S.urgent(() => isPending.set(true))
    const settle = () => {
      if (--isPending._count === 0) S.urgent(() => isPending.set(false))
    }
    let done
    try {
      done = transition(action)
    } catch (error) {
      settle()
      return fail(error)
    }
    done.then(settle, (error) => { settle(); fail(error) })
  }, [isPending, fail])
  return [isPending, start]
}

/** A signal following `value` (a signal or a value) at transition priority,
 *  after urgent updates; `initialValue` first, if given. */
function useDeferredValue (value, initialValue) {
  const read = () => (isSignal(value) ? value.peek() : value)
  const out = useOnce(() => signal(initialValue !== undefined ? initialValue : read()))
  const latest = useLatest(value)
  React.useEffect(() => {
    let scheduled = false
    let live = true
    const push = () => {
      if (scheduled) return
      scheduled = true
      S.scheduleDeferred(() => {
        scheduled = false
        if (!live) return
        const v = latest.current
        out.set(isSignal(v) ? v.peek() : v)
      })
    }
    if (isSignal(value)) {
      const stop = effect(() => { value.value; push() })
      return () => { live = false; stop() }
    }
    push()
    return () => { live = false }
  }, [value, out])
  return out
}

/** `[state signal, dispatch, isPending signal]`. Dispatches queue and run in
 *  order, each with the previous state; an action may be async. A thrown
 *  error goes to the nearest error boundary and drops the queue, as in React. */
function useActionState (action, initialState) {
  const state = useOnce(() => signal(initialState))
  const isPending = useOnce(() => signal(false))
  const latest = useLatest(action)
  const queue = useOnce(() => ({ items: [], running: false }))
  const fail = useThrowToBoundary()

  const runNext = React.useCallback(() => {
    const payload = queue.items.shift()
    queue.running = true
    let result
    try {
      transition(() => {
        result = latest.current(state.peek(), payload)
        return result
      }).catch(quiet) // the error is handled below, from `result`
    } catch (error) {
      result = Promise.reject(error)
    }
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
      fail(err)
    })
  }, [state, isPending, queue, fail])

  const dispatch = React.useCallback((payload) => {
    queue.items.push(payload)
    if (!queue.running) {
      S.urgent(() => isPending.set(true))
      runNext()
    }
  }, [queue, isPending, runNext])

  return [state, dispatch, isPending]
}

/** `[optimistic signal, addOptimistic]`. Updates added during a transition or
 *  action show at once and are dropped when it ends; `passthrough` (a signal
 *  or a value) shows through otherwise. */
function useOptimistic (passthrough, reducer) {
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
  track,
  createElement,
  startTransition,
  useSignal,
  useComputed,
  useSignalValue,
  useState,
  useReducer,
  useRef,
  useMemo,
  useCallback,
  useEffectEvent,
  useEffect,
  useLayoutEffect,
  useInsertionEffect,
  useSyncExternalStore,
  useTransition,
  useDeferredValue,
  useActionState,
  useOptimistic,
}
