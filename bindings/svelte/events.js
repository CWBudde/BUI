// Svelte's event delegation on BUI. Svelte keeps `onclick`-style handlers on
// the element (`element[symbol].click = fn`) and listens once, on the mount
// target. BUI sends an event to the deepest element *listening* for it, so an
// element with a delegated handler must listen natively too: the handlers
// object below does that when Svelte stores a handler, wherever it stores it
// from (compiled templates, spreads, <svelte:element>).
//
// The one place this package touches Svelte's internals: `delegated` (which
// compiled components call) tells us Svelte's symbol. test/svelte.test.js
// fails if a Svelte release changes it.
//
// SPDX-License-Identifier: GPL-2.0-or-later
import native from '@blender-ui/native'
import { delegated } from 'svelte/internal/client'

const { Element, EVENT_BITS } = native

const probe = {}
delegated('click', probe, null)
/** Svelte's key for an element's delegated handlers. */
export const HANDLERS = Object.getOwnPropertySymbols(probe)[0]
if (HANDLERS === undefined) throw new Error('@blender-ui/svelte: this Svelte version stores event handlers differently; update @blender-ui/svelte')

const nativeTarget = () => {} // the listener that makes an element a native target

/** An element's delegated handlers; setting one of BUI's event types listens natively. */
class Handlers {
  constructor (el) { Object.defineProperty(this, '_el', { value: el }) }
}
for (const type of Object.keys(EVENT_BITS)) {
  const key = Symbol(type)
  Object.defineProperty(Handlers.prototype, type, {
    get () { return this[key] },
    set (fn) {
      const had = this[key] != null
      this[key] = fn
      if (fn != null && !had) this._el.addEventListener(type, nativeTarget)
      else if (fn == null && had) this._el.removeEventListener(type, nativeTarget)
    },
    enumerable: true,
  })
}

Object.defineProperty(Element.prototype, HANDLERS, {
  get () { return (this._handlers ??= new Handlers(this)) },
  set () {}, // Svelte's `??= {}`: the getter never returns nothing
  configurable: true,
})
