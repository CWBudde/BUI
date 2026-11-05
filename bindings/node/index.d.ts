// Types for @blender-ui/native.
export interface AppOptions {
  /** Draw on the GPU: Direct3D 11 on Windows, OpenGL 3.3 on Linux. */
  gpu?: boolean
  /** GPU text from glyph outlines (Slug, default) or a per-glyph atlas. */
  slug?: boolean
  /** No native windows: tests, screenshots, CI. */
  headless?: boolean
  /** Blender's 'dark' (default) or 'light' theme (BUI_THEME otherwise). */
  theme?: 'dark' | 'light'
  /** Quit when the last window closes (default true). */
  quitOnLastWindowClosed?: boolean
}

export interface WindowOptions {
  title?: string
  width?: number
  height?: number
  resizable?: boolean
  gpu?: boolean
}

export type EventType = 'click' | 'mousedown' | 'mouseup' | 'mousemove' | 'mouseenter' | 'mouseleave' |
  'wheel' | 'keydown' | 'keyup' | 'change' | 'input'

export class BuiEvent {
  type: EventType
  target: Element
  currentTarget: Element
  clientX: number
  clientY: number
  x: number
  y: number
  button: number
  detail: number
  deltaY: number
  key?: string
  char?: string
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
  timeStamp: number
  preventDefault (): void
  stopPropagation (): void
}

export class Node {
  readonly ownerWindow: Window
  readonly id: number
  parentNode: Element | null
  childNodes: Node[]
  readonly firstChild: Node | null
  readonly lastChild: Node | null
  readonly nextSibling: Node | null
  textContent: string
  appendChild<T extends Node> (child: T): T
  insertBefore<T extends Node> (child: T, before: Node | null): T
  removeChild<T extends Node> (child: T): T
  remove (): void
  /** Free the node and its subtree natively (after removing it). */
  destroy (): void
}

export class TextNode extends Node {
  /** A string, or a signal (the text then follows it). */
  data: string | ReadSignal<unknown>
  nodeValue: string
}

export class Element extends Node {
  readonly tagName: string
  readonly localName: string
  className: string
  /** CSS text, or an object such as `{ backgroundColor: 'red', width: 40 }`. */
  style: string | Record<string, string | number | null | undefined> | ReadSignal<string | Record<string, string | number | null | undefined>>
  /** Form controls: the latest value from input / change events. */
  value?: string
  checked?: boolean
  /** A value, or a signal: the attribute then follows it (null / false removes it). */
  setAttribute (name: string, value: unknown): void
  getAttribute (name: string): string | null
  hasAttribute (name: string): boolean
  removeAttribute (name: string): void
  addEventListener (type: EventType, listener: (event: BuiEvent) => void): void
  removeEventListener (type: EventType, listener: (event: BuiEvent) => void): void
  getBoundingClientRect (): Promise<{ x: number, y: number, width: number, height: number } | null>
}

export class Window {
  readonly app: App
  readonly id: number
  readonly body: Element
  closed: boolean
  createElement (tag: string): Element
  createTextNode (text: string | ReadSignal<unknown>): TextNode
  /** Send pending changes now (they are otherwise sent once per tick). */
  flush (): void
  /** Run `cb` before the next frame, paced by the display. */
  requestAnimationFrame (cb: (time: number) => void): number
  cancelAnimationFrame (id: number): void
  screenshot (): Promise<{ width: number, height: number, data: Buffer }>
  click (x: number, y: number): void
  close (): void
  on (event: 'close', listener: () => void): this
}

export class App {
  constructor (options?: AppOptions)
  readonly headless: boolean
  readonly windows: Map<number, Window>
  quitOnLastWindowClosed: boolean
  window (options?: WindowOptions): Window
  setSlug (on: boolean): void
  /** Blender's 'dark' (default) or 'light' theme; open windows redraw. */
  setTheme (name: 'dark' | 'light'): void
  quit (): void
}

export function styleText (style: Record<string, string | number | null | undefined>): string

// Signals: BUI's hooks (the Rust crate's Signal / Memo / Effect).
// Bind one to an attribute, `style` or text and that node alone updates.

export interface ReadSignal<T> {
  /** Read, subscribing the running memo / effect. */
  readonly value: T
  get (): T
  /** Read without subscribing. */
  peek (): T
  with<R> (fn: (value: T) => R): R
  /** Call `fn` now and on every change; returns the unsubscribe function. */
  subscribe (fn: (value: T) => void): () => void
  /** `count + 1`, `count > 5` read the value (`===` and truthiness don't: use `.value`). */
  valueOf (): T
}

export class Signal<T> implements ReadSignal<T> {
  constructor (value: T, options?: { equals?: (a: T, b: T) => boolean })
  value: T
  get (): T
  peek (): T
  set (value: T): void
  update (fn: (value: T) => T): void
  with<R> (fn: (value: T) => R): R
  subscribe (fn: (value: T) => void): () => void
  valueOf (): T
}

export class Memo<T> implements ReadSignal<T> {
  readonly value: T
  get (): T
  peek (): T
  with<R> (fn: (value: T) => R): R
  subscribe (fn: (value: T) => void): () => void
  valueOf (): T
}

export function signal<T> (value: T, options?: { equals?: (a: T, b: T) => boolean }): Signal<T>
/** A lazily derived, cached value, recomputed when the signals it read change. */
export function memo<T> (fn: () => T, options?: { equals?: (a: T, b: T) => boolean }): Memo<T>
export const computed: typeof memo
/** Runs `fn` now and when what it read changes; `fn` may return a cleanup. Returns `dispose`. */
export function effect (fn: () => void | (() => void)): () => void
/** Several writes, one update. */
export function batch<R> (fn: () => R): R
export function untrack<R> (fn: () => R): R
/** Leptos / Solid style getter and setter. */
export function createSignal<T> (value: T): [() => T, (value: T | ((prev: T) => T)) => void]
export function isSignal (x: unknown): x is ReadSignal<unknown>
/** Writes in `fn` update what they affect after urgent work, in slices. Settles
 *  when done; rejects with the error of an async `fn`. */
export function startTransition (fn: () => void | Promise<void>): Promise<void>
/** Run `fn` at urgent priority, even inside a transition. */
export function urgent<R> (fn: () => R): R

/** A ref whose `.current` effects and memos can follow (reading it while a
 *  React component renders doesn't subscribe). */
export class Ref<T> extends Signal<T> { current: T }
export function ref<T> (value: T): Ref<T>
/** Run `fn` outside any owning effect: effects it creates live until disposed. */
export function unowned<R> (fn: () => R): R

/** The addon (internal), plus counters for benchmarks. */
export const native: {
  /** CPU cycles of every thread of this process (Windows), or null. */
  processCpuCycles (): number | null
  /** The x86-64 time-stamp counter, or null. */
  timestampCounter (): number | null
  [key: string]: unknown
}
