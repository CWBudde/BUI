# @blenderui/native

Blender's UI from JavaScript: native windows (CPU or Direct3D 11), HTML
elements, CSS and Tailwind classes, driven from Node through napi-rs. BUI
runs on its own thread, and only the changes of each tick cross into Rust.

```js
const { App } = require('@blenderui/native')
const app = new App({ gpu: true })
const win = app.window({ title: 'Hello', width: 360, height: 200 })
const button = win.createElement('button')
button.className = 'm-4'
button.textContent = 'Click me'
button.addEventListener('click', () => { button.textContent = 'Clicked' })
win.body.appendChild(button)
```

Signals (BUI's hooks: `signal`, `memo`, `effect`, `batch`) bind to
attributes, `style` and text, so a change updates that node alone:

```js
const { signal } = require('@blenderui/native')
const clicks = signal(0)
button.textContent = clicks // follows the signal
button.addEventListener('click', () => clicks.value++)
```

React: [`@blenderui/react`](../react). Guide, internals and benchmarks:
[docs/javascript.md](../../docs/javascript.md). Windows x64 for native
windows; `headless: true` works anywhere. License: GPL-2.0-or-later.
