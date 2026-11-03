# @blender-ui/native

Blender's UI toolkit from JavaScript: native windows drawn on the CPU or with
Direct3D 11, HTML elements, CSS and Tailwind classes, and signals. It runs on
Node.js and Bun through a native addon (napi-rs), with no browser and no
Electron. For React, see
[`@blender-ui/react`](https://www.npmjs.com/package/@blender-ui/react).

```sh
npm install @blender-ui/native
```

```js
const { App, signal } = require('@blender-ui/native')

const app = new App({ gpu: true })
const win = app.window({ title: 'Hello', width: 360, height: 200 })
win.body.className = 'm-0 p-6 flex flex-col gap-3 bg-[#2b2b2b] text-[#e6e6e6]'

const clicks = signal(0)
const label = win.createElement('span')
label.textContent = clicks            // follows the signal
const button = win.createElement('button')
button.textContent = 'Add one'
button.addEventListener('click', () => clicks.value++)
win.body.appendChild(label)
win.body.appendChild(button)
```

## API

- **A DOM-like tree:** `createElement`, `createTextNode`, `appendChild`,
  `insertBefore`, `removeChild`, `setAttribute`, `className` (with Tailwind
  classes), `style` (a string or an object), `textContent`.
- **Events:** `addEventListener` for `click`, `mousedown`, `mouseup`,
  `mousemove`, `mouseenter`, `mouseleave`, `wheel`, `keydown`, `keyup`, `input`
  and `change`. They bubble through the tree, and `stopPropagation()` works.
  They arrive after BUI has handled them, so `preventDefault()` only marks the
  event.
- **Frames and measurement:** `win.requestAnimationFrame(cb)` is paced by the
  display. `el.getBoundingClientRect()` and `win.screenshot()` return
  promises.
- **Signals:** `signal`, `memo`, `effect`, `batch`, `untrack`,
  `startTransition`. Pass a signal to `setAttribute`, `style`, `textContent`
  or `createTextNode`, and its changes update that node alone. The signals are
  also available without the addon: `require('@blender-ui/native/signals')`.
- **Tests:** `new App({ headless: true })` renders off-screen, and
  `win.click(x, y)` simulates input.

## How it's fast

- **Its own UI thread:** BUI renders on a thread of its own, so it never
  blocks Node and an idle window uses no CPU.
- **Only changes cross:** each tick, a window sends one batch of operations
  (an `Int32Array` and a string table) to the UI thread.
- **Styles parsed once:** a `style` value is parsed when it's set, not every
  frame.

## Requirements

- **Prebuilt for Windows x64.** On other platforms, build from a clone of the
  [repository](https://github.com/ppmpreetham/BUI) with `npm run build` in
  `bindings/node`. That needs Rust, CMake and a C compiler; native windows
  there are not supported yet, so use `headless: true`.
- **Runtimes:** Node.js 18 or newer, or Bun.

Guide, benchmarks and internals:
[docs/javascript.md](https://github.com/ppmpreetham/BUI/blob/main/docs/javascript.md).
License: GPL-2.0-or-later.
