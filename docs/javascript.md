# JavaScript and React

BUI runs from Node.js through a native addon (napi-rs), the way
[GPUIX](https://www.npmjs.com/package/@gpuix/react) drives GPUI: native
windows (CPU or Direct3D 11), no browser, with HTML elements, CSS and
Tailwind classes, and the same engine as the Rust crate.

| Package | What it is |
|---|---|
| `@blenderui/native` (`bindings/node`) | A DOM-like API: `createElement`, `setAttribute`, `appendChild`, `addEventListener`, `requestAnimationFrame` |
| `@blenderui/react` (`bindings/react`) | A React 19 renderer on top of it |

Status: Windows x64 with native windows. Elsewhere, `headless: true` works
(tests, screenshots). The browser (WebAssembly) build is next.

## Plain JavaScript

```js
const { App } = require('@blenderui/native')

const app = new App({ gpu: true })
const win = app.window({ title: 'Hello', width: 360, height: 220 })
win.body.className = 'm-0 p-6 flex flex-col gap-3 bg-[#2b2b2b] text-[#e6e6e6]'

const label = win.createElement('span')
label.textContent = 'Clicks: 0'
const button = win.createElement('button')
button.textContent = 'Add one'
let n = 0
button.addEventListener('click', () => { label.textContent = `Clicks: ${++n}` })
win.body.appendChild(label)
win.body.appendChild(button)
```

Elements take the attributes the Rust crate does: `class` / `className`
(Tailwind classes included), `style` (a CSS string or an object such as
`{ backgroundColor: 'red', width: 40 }`), `id`, `href`, `value`, `src` ...
Events: `click`, `mousedown`, `mouseup`, `mousemove`, `mouseenter`,
`mouseleave`, `wheel`, `keydown`, `keyup`, `input`, `change`.

`win.requestAnimationFrame(cb)` runs `cb` before the next frame, paced by the
display; what it changes renders in that frame. `el.getBoundingClientRect()`
and `win.screenshot()` return promises. `new App({ headless: true })` renders
off-screen, and `win.click(x, y)` simulates input (for tests).

## React

```jsx
import { useState } from 'react'
import { render } from '@blenderui/react'

function Counter () {
  const [count, setCount] = useState(0)
  return (
    <main className="p-6 flex flex-col gap-3">
      <span>Clicks: {count}</span>
      <button onClick={() => setCount(c => c + 1)}>Add one</button>
    </main>
  )
}

render(<Counter />, { title: 'BUI + React', width: 420, height: 220, gpu: true })
```

Host components are HTML elements. `className`, `style` objects, `htmlFor`
and boolean attributes work as in React DOM. `onChange` fires on every edit
of a text field, as in React DOM. Event handlers run as discrete updates, so
a click's state change commits immediately. `createRoot(window)` renders
into a window you opened yourself.

## Hooks and signals

React's hooks work as usual. BUI's own hooks, the Rust crate's `Signal`,
`Memo` and `Effect`, come along too, in both packages:

| Rust (`blenderui::hooks`) | JavaScript (`@blenderui/native`) | React component (`@blenderui/react`) |
|---|---|---|
| `use_signal(v)` / `RwSignal` | `signal(v)` | `useSignal(v)` |
| `Memo::new(f)` | `memo(f)` (alias `computed`) | `useComputed(f)` |
| `Effect::new(f)` | `effect(f)` (returns `dispose`) | `useSignalEffect(f)` |
| `signal(v)` / `create_signal(v)` | `createSignal(v)` (`[get, set]`) | |
| `use_state`, `use_effect`, `use_memo`, context | | React's own hooks |

A signal is read with `.value` (or `.get()`), written with `.value = v`,
`.set(v)` or `.update(fn)`. Memos and effects track what they read, like
Leptos: no dependency arrays. `batch(fn)` groups writes into one update.

The point of signals is what they skip. Pass a signal where a value goes and
the node follows it:

```jsx
import { render, useSignal, useComputed } from '@blenderui/react'

function Counter () {
  const count = useSignal(0)
  const style = useComputed(() => ({ color: count.value > 9 ? '#e5a50a' : '#e6e6e6' }))
  return (
    <main className="p-6 flex flex-col gap-3">
      <span style={style}>Clicks: {count}</span>
      <button onClick={() => count.value++}>Add one</button>
    </main>
  )
}
```

A click writes `count`, which updates one text node and one `style`. `Counter`
renders once, ever. Without the reconciler running, there's no new element
tree, no props diff and almost no garbage, which is why the React numbers
below drop to those of the plain API. `useSignalValue(sig)` does the opposite
when you want it: it re-renders its component on changes.

In plain JavaScript, the same binding works with `el.setAttribute(name, sig)`,
`el.style = sig`, `el.textContent = sig` and `win.createTextNode(sig)`. A
plain value unbinds; `destroy()` frees the bindings of a subtree.

## How it works

- **A UI thread.** BUI runs its own message loop on a thread of its own. Node's
  thread is never blocked, and an idle UI uses no CPU. A message-only window
  wakes the loop from Node (`BK_window_wake`), and it survives Win32's modal
  move and resize loops.
- **Only changes cross.** JavaScript keeps the tree. Each tick, a window sends
  one `Int32Array` of ops (create, set attribute, append, insert, remove,
  set text, listen) and a string table that is deduplicated per batch. React
  sends one batch per commit.
- **Retained mirror.** The UI thread applies the ops to its copy of the tree
  and rebuilds elements from it, keyed per node. Layout caches and form state
  persist, and a `style` attribute is parsed once, when it is set.
- **Events** come back through a napi thread-safe function. They are
  asynchronous, so `preventDefault()` has no native effect.
- **Animation.** While `requestAnimationFrame` is active, the native side
  schedules the next vsync during the current render, as browsers do. The
  changes JavaScript makes in its callback join that frame, giving one rebuild
  per frame.

## Performance

A 1000x800 window with a grid of N cells, where one cell changes colour and
label every frame. This is the Rust benchmark's `one` mode, driven from
`requestAnimationFrame` (`node bench/cells.js N [--gpu] [--react]`). UI work
is the UI thread's per-frame time from `BUI_PROFILE=1`. It is wall time, which
is reliable here, unlike Windows' tick-based CPU accounting for
vsync-aligned threads.

| 3000 cells, GPU | Rust | JS (plain) | React, signals | React, state |
|---|---:|---:|---:|---:|
| Frames per second | 60 | 55–57 | 56 | 53–58 |
| UI thread work per frame | 10.0 ms | 12.9 ms | ≈ plain JS | 14.1 ms |
| Resident memory | ~42 MB | ~87 MB | ~95 MB | ~184 MB |

"React, state" re-renders the 3,000-cell grid every frame with `useState`;
"React, signals" binds cell 0's style and label to signals
(`--react --signals`), so the grid renders once.

At 1,000 cells, all of them hold 60 fps. Where the memory goes:

- **Node itself:** 32 MB before anything loads.
- **A GPU window:** about 21 MB (driver, swap chain, glyph atlas). Rust pays
  this too.
- **The native tree:** about 7 MB more than Rust for 3,000 cells, because the
  UI thread keeps its own copy of the tree.
- **React re-rendering:** the live JS heap is only about 15 MB, but each
  re-render of the grid creates about 3,000 elements, props objects and
  strings. V8 grows its heap to about 113 MB to absorb that garbage, and
  Windows keeps counting the pages. Signals avoid the re-render, and so the
  garbage.

## Building

```sh
cd bindings/node && npm run build   # cargo build -p blenderui-node --release, copies the .node file
npm test                            # headless tests
node examples/counter.js
cd ../react && npm install && npm test && node examples/counter.js
```

The build needs what the Rust crate needs: CMake and a C compiler. With
MinGW, put its `bin` folder on `PATH`.
