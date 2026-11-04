// Types for @blender-ui/react.
import type { ReactNode, ReactElement, DependencyList, EffectCallback, Dispatch, SetStateAction, ElementType } from 'react'
import type { App, Window, AppOptions, WindowOptions, Signal, Memo, ReadSignal, Ref } from '@blender-ui/native'

export interface Root {
  render (element: ReactNode): void
  unmount (): void
}

/** A React root rendering into `window`'s body (unmounted when the window closes). */
export function createRoot (window: Window): Root

/** Open a window (and an App, unless `app` is given) and render `element` into it. */
export function render (element: ReactNode, options?: WindowOptions & AppOptions & { app?: App }):
  { app: App, window: Window, root: Root }

/** Run `fn` and flush React's work synchronously (tests). */
export function act<R> (fn: () => R): R

/** React's `createElement`, with render tracking for components. */
export const createElement: typeof import('react').createElement
/** A component type with render tracking (re-renders when a signal its render read changes). */
export function track<T extends ElementType> (type: T): T

// React's hooks that hold no state, as React exports them.
export { use, useContext, useDebugValue, useId, useImperativeHandle } from 'react'

// Signals.
export { signal, effect, batch, untrack, createSignal, isSignal, Signal, Memo, ReadSignal, Ref } from '@blender-ui/native'
/** A derived, cached signal (`memo` in @blender-ui/native; here `memo` stays React's). */
export { memo as computed, ref } from '@blender-ui/native'

/** React hooks re-exported unchanged, and why. */
export const NO_TWIN: Record<string, string>

// React's hooks by React's names, with signals for state. Same arguments and
// timing as React's. A signal passed as a prop or child updates that node
// alone; read while rendering (`.value`, `count + 1`), it re-renders the
// component when it changes. Conditions and `===` need `.value`: a signal
// object is always truthy.

/** `[signal, setState]`: `setState(v)` or `setState(prev => next)`. */
export function useState<T> (initialState: T | (() => T)): [Signal<T>, Dispatch<SetStateAction<T>>]
export function useState<T = undefined> (): [Signal<T | undefined>, Dispatch<SetStateAction<T | undefined>>]
/** `[signal, dispatch]`, with the latest render's reducer. */
export function useReducer<S, A> (reducer: (state: S, action: A) => S, initialArg: S): [Signal<S>, Dispatch<A>]
export function useReducer<S, A, I> (reducer: (state: S, action: A) => S, initialArg: I, init: (arg: I) => S): [Signal<S>, Dispatch<A>]
/** `{ current }`, observable by effects and memos; reading it while rendering doesn't subscribe. */
export function useRef<T> (initialValue: T): Ref<T>
export function useRef<T = undefined> (): Ref<T | undefined>
/** A memo following its signals, replaced when `deps` change (without deps: every render). */
export function useMemo<T> (fn: () => T, deps?: DependencyList): Memo<T>
/** React's `useCallback`; calls are batched and untracked. */
export function useCallback<F extends (...args: any[]) => any> (fn: F, deps?: DependencyList): F
/** React's `useEffectEvent`; calls are batched and untracked. */
export function useEffectEvent<F extends (...args: any[]) => any> (fn: F): F
/** React's `useEffect`, also re-run when the signals it read change. */
export function useEffect (effect: EffectCallback, deps?: DependencyList): void
/** React's `useLayoutEffect`, also re-run when the signals it read change. */
export function useLayoutEffect (effect: EffectCallback, deps?: DependencyList): void
/** React's `useInsertionEffect`, also re-run when the signals it read change. */
export function useInsertionEffect (effect: EffectCallback, deps?: DependencyList): void
/** A signal following the store. */
export function useSyncExternalStore<T> (subscribe: (onChange: () => void) => () => void, getSnapshot: () => T,
  getServerSnapshot?: () => T): Signal<T>
/** `[isPending, startTransition]`: what the action's writes trigger runs after urgent updates. */
export function useTransition (): [Signal<boolean>, (action: () => void | Promise<void>) => void]
/** React's `startTransition`, for signal writes too; settles when the action and its updates are done. */
export function startTransition (action: () => void | Promise<void>): Promise<void>
/** A signal following `value` (a signal or a value) at transition priority. */
export function useDeferredValue<T> (value: T | ReadSignal<T>, initialValue?: T): Signal<T>
/** `[state, dispatch, isPending]`: actions run in order, each with the previous state. */
export function useActionState<S, P = void> (action: (state: S, payload: P) => S | Promise<S>, initialState: S,
  permalink?: string): [Signal<S>, (payload: P) => void, Signal<boolean>]
/** `[optimistic, addOptimistic]`: updates last until their transition or action ends. */
export function useOptimistic<S> (passthrough: S | ReadSignal<S>): [Memo<S>, (value: S) => void]
export function useOptimistic<S, A> (passthrough: S | ReadSignal<S>, reducer: (state: S, action: A) => S):
  [Memo<S>, (action: A) => void]

/** Rust's `use_signal`: a signal owned by this component. */
export function useSignal<T> (initial: T | (() => T)): Signal<T>
/** Rust's `Memo::new`: `useMemo(fn, deps = [])`. */
export function useComputed<T> (fn: () => T, deps?: DependencyList): Memo<T>
/** Re-render this component when `source` changes; returns its value (components
 *  rendered without this package's JSX runtime or createElement). */
export function useSignalValue<T> (source: ReadSignal<T>): T

export type { ReactElement }

// A signal is a valid child: `<span>Clicks: {count}</span>`.
declare module 'react' {
  interface DO_NOT_USE_OR_YOU_WILL_BE_FIRED_EXPERIMENTAL_REACT_NODES {
    buiSignal: ReadSignal<unknown>
  }
}

/** Blender's widgets as components: always drawn and handled like Blender.
 *  Their lowercase HTML counterparts look like a browser's form controls
 *  (and like Blender inside an element with the `styled` attribute). */
type BlenderProps = Record<string, unknown> & { children?: ReactNode }
export function Button (props: BlenderProps): ReactNode
export function Checkbox (props: BlenderProps): ReactNode
export function Radio (props: BlenderProps): ReactNode
export function NumberField (props: BlenderProps): ReactNode
export function NumberSlider (props: BlenderProps): ReactNode
export function TextField (props: BlenderProps): ReactNode
export function TextArea (props: BlenderProps): ReactNode
export function ColorSwatch (props: BlenderProps): ReactNode
export function Dropdown (props: BlenderProps): ReactNode
export function ProgressBar (props: BlenderProps): ReactNode
