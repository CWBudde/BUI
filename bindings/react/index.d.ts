// Types for @blenderui/react.
import type { ReactNode } from 'react'
import type { App, Window, AppOptions, WindowOptions } from '@blenderui/native'

export interface Root {
  render (element: ReactNode): void
  unmount (): void
}

export function createRoot (window: Window): Root

export function render (element: ReactNode, options?: WindowOptions & AppOptions & { app?: App }):
  { app: App, window: Window, root: Root }

export function act (fn: () => void): void

// BUI's hooks (the Rust crate's use_signal / Memo / Effect). Pass a signal
// as a prop or child and only that node updates: no re-render.
import type { Signal, Memo, ReadSignal } from '@blenderui/native'
export { signal, memo, computed, effect, batch, untrack, createSignal, Signal, Memo, ReadSignal } from '@blenderui/native'

/** A signal owned by this component (Rust's `use_signal`). */
export function useSignal<T> (initial: T | (() => T)): Signal<T>
/** A memo owned by this component (Rust's `Memo::new`). */
export function useComputed<T> (fn: () => T): Memo<T>
/** An effect owned by this component (Rust's `Effect::new`); may return a cleanup. */
export function useSignalEffect (fn: () => void | (() => void)): void
/** Re-render this component when `source` changes; returns its value. */
export function useSignalValue<T> (source: ReadSignal<T>): T

// A signal is a valid child: `<span>Clicks: {count}</span>`.
declare module 'react' {
  interface DO_NOT_USE_OR_YOU_WILL_BE_FIRED_EXPERIMENTAL_REACT_NODES {
    buiSignal: ReadSignal<unknown>
  }
}
