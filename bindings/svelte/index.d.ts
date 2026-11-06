// Types for @blender-ui/svelte.
import type { Component, Snippet } from 'svelte'
import type { App, Window } from '@blender-ui/native'
import type { RenderOptions } from '@blender-ui/native/host'

export interface MountOptions<P> {
  props?: P
  /** Context for the component tree (Svelte's `mount` option). */
  context?: Map<unknown, unknown>
}

export interface Root {
  /** Mount `Component` (replacing what this root showed); returns its exports. */
  render<P extends Record<string, any>> (Component: Component<P>, options?: MountOptions<P>): Record<string, any>
  unmount (): void
}

/** A root rendering Svelte components into `window`'s body (unmounted when the window closes). */
export function createRoot (window: Window): Root

/** Open a window (and an App, unless `app` is given) and mount `Component` in it. */
export function render<P extends Record<string, any>> (Component: Component<P>, options?: RenderOptions & MountOptions<P>):
  { app: App, window: Window, root: Root, component: Record<string, any> }

/** Run `fn` and apply Svelte's pending updates now (tests). */
export function act<R> (fn?: () => R): R

/** Blender's widgets as components: always drawn and handled like Blender.
 *  Their lowercase HTML counterparts look like a browser's form controls
 *  (and like Blender inside an element with the `styled` attribute). */
type BlenderProps = Record<string, unknown> & { children?: Snippet }
export const Button: Component<BlenderProps>
export const Checkbox: Component<BlenderProps>
export const Radio: Component<BlenderProps>
export const NumberField: Component<BlenderProps>
export const NumberSlider: Component<BlenderProps>
export const TextField: Component<BlenderProps>
export const TextArea: Component<BlenderProps>
export const ColorSwatch: Component<BlenderProps>
export const Dropdown: Component<BlenderProps>
export const ProgressBar: Component<BlenderProps>

// Signals: each is a Svelte store too (`$count` follows it in a component).
export { signal, effect, batch, untrack, createSignal, ref, isSignal, startTransition, Signal, Memo } from '@blender-ui/native'
/** A derived, cached signal (`memo` in @blender-ui/native). */
export { memo as computed } from '@blender-ui/native'
