// What compiled components import instead of `svelte/internal/client`
// (compile.js rewrites the import): Svelte's runtime, with the functions its
// templates call on every update writing straight into BUI's nodes.
//
// - A plain value: Svelte's own versions go through the DOM API
//   (`text.nodeValue = s`, `el.style.cssText = s`, `el.className = s`) and
//   keep a cache property per node; here an update is one call on the node,
//   which compares with what it holds and queues the native op.
// - A BUI signal (`<div style={styleSignal}>{labelSignal}</div>`, no `$`):
//   the node is bound to it, so a write updates that node straight from the
//   signal and Svelte doesn't run at all (as with React's signal props).
//
// A redirected function is used only when Svelte's has the shape this file
// was written against (test/svelte.test.js checks each one against Svelte's,
// on the same inputs); otherwise Svelte's own runs, through the DOM.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import * as svelte from 'svelte/internal/client'
import native from '@blender-ui/native'

export * from 'svelte/internal/client'

const { isSignal, memo } = native

/** The functions redirected, and the parameter count of Svelte's versions. */
export const REDIRECTS = { set_text: 2, set_attribute: 4, set_style: 4, set_class: 6, clsx: 1 }

/** Those whose Svelte version still has that shape (the rest stay Svelte's). */
export const redirected = Object.keys(REDIRECTS).filter((k) => svelte[k]?.length === REDIRECTS[k])

const use = (name, fast) => (redirected.includes(name) ? fast : svelte[name])
// a node bound to a signal: the public setters, which unbind it first
const bound = (node) => node._bindings !== null

/** `{expression}` text. */
export const set_text = use('set_text', (text, value) => {
  if (isSignal(value) || bound(text)) text.data = isSignal(value) ? value : value == null ? '' : `${value}`
  else text._setText(value == null ? '' : `${value}`)
})

/** An attribute; strings (and signals) go straight through, other values
 *  Svelte's way (some of them set DOM properties). */
export const set_attribute = use('set_attribute', (element, attribute, value, skipWarning) => {
  if (isSignal(value) || (typeof value === 'string' && bound(element))) element.setAttribute(attribute, value)
  else if (typeof value === 'string') element._setAttr(attribute, value)
  else svelte.set_attribute(element, attribute, value, skipWarning)
})

/** `style="..."`; with `style:` directives, Svelte's way. */
export const set_style = use('set_style', (dom, value, prevStyles, nextStyles) => {
  if (nextStyles !== undefined || (value != null && typeof value !== 'string' && !isSignal(value))) {
    return svelte.set_style(dom, value, prevStyles, nextStyles)
  }
  if (value == null || value === '') dom.removeAttribute('style')
  else if (isSignal(value) || bound(dom)) dom.setAttribute('style', value)
  else dom._setAttr('style', value)
})

/** `class="..."` (with the component's scoping hash); with `class:`
 *  directives, Svelte's way. */
export const set_class = use('set_class', (dom, isHtml, value, hash, prevClasses, nextClasses) => {
  if (nextClasses !== undefined) return svelte.set_class(dom, isHtml, value, hash, prevClasses, nextClasses)
  if (isSignal(value)) return dom.setAttribute('class', hash ? memo(() => withHash(value.value, hash)) : value)
  const name = withHash(value, hash)
  if (name === '') dom.removeAttribute('class') // as Svelte: no class at all
  else if (bound(dom)) dom.setAttribute('class', name)
  else dom._setAttr('class', name)
})

/** `class={value}` is `clsx(value)` first: a signal passes through, to be bound. */
export const clsx = use('clsx', (value) => (isSignal(value) ? value : svelte.clsx(value)))

function withHash (value, hash) {
  const name = value == null ? '' : '' + value
  return hash ? (name ? name + ' ' + hash : hash) : name
}
