# JavaScript and React

BUI runs from JavaScript through a native addon built with napi-rs, much as
[GPUIX](https://www.npmjs.com/package/@gpuix/react) drives GPUI. You get native
windows drawn on the CPU or with Direct3D 11, HTML elements, CSS and Tailwind
classes, and the same engine as the Rust crate. There's no browser and no
Electron. It works on **Node.js** and **Bun**, and `bui build` turns an app
into a **single executable of about 5 MB**: BUI with an embedded QuickJS
engine ([Standalone apps](#standalone-apps)).

| Package | What it is |
|---|---|
| [`@blender-ui/native`](https://www.npmjs.com/package/@blender-ui/native) (`bindings/node`) | A DOM-like API (`createElement`, `setAttribute`, `appendChild`, `addEventListener`, `requestAnimationFrame`) and signals |
| [`@blender-ui/react`](https://www.npmjs.com/package/@blender-ui/react) (`bindings/react`) | A React 19.3 renderer; React's hooks by React's names, backed by signals |

**Platforms:** prebuilt for Windows x64. On other platforms, build the addon
from source (see [Building](#building)) and use `headless: true` for now. A
browser (WebAssembly) build is planned.

## Plain JavaScript

```js
const { App, signal } = require('@blender-ui/native')

const app = new App({ gpu: true })
const win = app.window({ title: 'Hello', width: 360, height: 220 })
win.body.setAttribute('styled', '') // Blender's look for the page (default: browser styling)
win.body.className = 'm-0 p-6 flex flex-col gap-3 bg-[#2b2b2b] text-[#e6e6e6]'

const clicks = signal(0)
const label = win.createElement('span')
label.textContent = clicks // bound: follows the signal
const button = win.createElement('button')
button.textContent = 'Add one'
button.addEventListener('click', () => clicks.value++)
win.body.appendChild(label)
win.body.appendChild(button)
```

**Attributes** are the ones the Rust crate takes:

- `class` / `className`, including Tailwind classes;
- `style`, as a CSS string or an object such as
  `{ backgroundColor: 'red', width: 40 }`;
- `id`, `href`, `value`, `src` and so on.

Any of them can be a signal.

**Events** are `click`, `mousedown`, `mouseup`, `mousemove`, `mouseenter`,
`mouseleave`, `wheel`, `keydown`, `keyup`, `input` and `change`.

- **Bubbling:** events bubble through the tree, except `mouseenter` and
  `mouseleave`, as in the DOM. `stopPropagation()` and
  `stopImmediatePropagation()` work.
- **`event.target`** is the deepest element listening for that event type.
- **`preventDefault()`** only marks the event: events reach JavaScript after
  BUI has handled them.

**Frames and measurement:** `win.requestAnimationFrame(cb)` runs `cb` before
the next frame, paced by the display, and what it changes renders in that
frame. `el.getBoundingClientRect()` and `win.screenshot()` return promises.

**Tests:** `new App({ headless: true })` renders off-screen, and
`win.click(x, y)` simulates input.

The tree behaves like the DOM:

- **Moving a node:** `appendChild` or `insertBefore` moves a node that already
  has a parent.
- **Errors:** inserting a node into itself, or before a node that isn't a
  child, throws.
- **Freeing nodes:** `destroy()` frees a removed subtree natively.

## React

```jsx
import { render, useState } from '@blender-ui/react'

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

Set `"jsxImportSource": "@blender-ui/react"` in `tsconfig.json` or in your
bundler's JSX options. For code without JSX, use this package's
`createElement`. The runtime does two things:

- **Render tracking:** a component that reads a signal while rendering
  re-renders when it changes.
- **TypeScript:** host-element props accept signals of their type.

Host components are HTML elements, as in React DOM:

- `className`, `style` objects, `htmlFor` and boolean attributes work.
- `onChange` fires on every edit of a text field. On checkboxes, radios,
  selects and other controls, it fires when the value changes, even if the
  control's `type` changes later.
- Event handlers run as discrete updates, so a click's state change commits
  immediately.

`<Suspense>`, `<Activity>`, transitions, actions and `<ViewTransition>` work;
view transitions commit without animating. `createRoot(window)` renders into a
window you opened yourself, and closing the window unmounts its root.

## Hooks and signals

`@blender-ui/react` exports React's hooks by React's names. The ones that hold
or react to state take React's arguments and run at React's moments, but the
state they hold is a **signal**:

- **Passed as a prop or child** (`<span>{count}</span>`, `style={style}`): a
  change updates that one node. The component doesn't re-render.
- **Read while rendering** (`count.value`, or in plain JavaScript also
  `count + 1`, `count > 5`, `${count}`): the component re-renders when it
  changes, exactly as with React's hooks. TypeScript needs `.value` for
  arithmetic.
- **Conditions and `===` need `.value`.** A signal is an object, so it's always
  truthy: write `{isPending.value && <Spinner />}` and `count.value === 0`.
  TypeScript catches `===`, but not truthiness.

```jsx
function Counter () {
  const [count, setCount] = useState(0)
  const style = useMemo(() => ({ color: count.value > 9 ? '#e5a50a' : '#e6e6e6' }), [])
  return (
    <main className="p-6 flex flex-col gap-3">
      <span style={style}>Clicks: {count}</span>
      <button onClick={() => setCount(c => c + 1)}>Add one</button>
    </main>
  )
}
```

A click updates one text node and one `style`. `Counter` renders once. With
no reconciler running, there's no new element tree, no props diff and almost
no garbage. That's why React with signals costs about what the plain API does
(see Performance).

| Hook | Returns | Notes |
|---|---|---|
| `useState(init)` | `[signal, setState]` | `setState(v)` or `setState(prev => next)`; the same value doesn't update |
| `useReducer(reducer, arg, init?)` | `[signal, dispatch]` | Uses the latest render's reducer, as React does |
| `useRef(v)` | a ref | Effects and memos see `.current` change (an element mounting re-runs them); reading it while rendering doesn't subscribe |
| `useMemo(fn, deps?)` | a memo | Recomputes when the signals it read change; replaced when `deps` change; without deps, every render |
| `useCallback(fn, deps?)` | a function | Identity follows `deps`; calls are batched and don't subscribe the caller |
| `useEffectEvent(fn)` | a function | Calls the latest render's `fn`; batched and untracked |
| `useEffect(fn, deps?)` | | React's timing, deps and cleanups; also re-runs when the signals it read change |
| `useLayoutEffect(fn, deps?)` | | Same, at layout time |
| `useInsertionEffect(fn, deps?)` | | Same, at insertion time |
| `useSyncExternalStore(subscribe, get)` | a signal | Follows the store without re-rendering |
| `useTransition()` | `[isPending, start]` | Updates caused by the writes in `start(action)` run after urgent ones, in 5 ms slices; `action` may be async |
| `useDeferredValue(value, initial?)` | a signal | Follows a value or a signal at transition priority |
| `useActionState(action, initial)` | `[state, dispatch, isPending]` | Actions queue and run in order; errors reach the error boundary |
| `useOptimistic(state, reducer?)` | `[optimistic, add]` | Updates last until their transition or action ends |

`use`, `useContext`, `useId`, `useDebugValue` and `useImperativeHandle` are
React's own. They hold no state; for context, pass a signal as the value and
consumers bind to it. React's plain-value hooks are still available from
`react`, and they work in this renderer too.

Also:

- **`useSignal(v)` and `useComputed(fn)`:** the Rust crate's names
  (`use_signal`, `Memo::new`).
- **`useSignalValue(sig)`:** re-renders its component when `sig` changes,
  for components rendered without this package's JSX runtime.
- **Reading and writing a signal:** read with `.value` or `.get()`; write
  with `.value = v`, `.set(v)` or `.update(fn)`.
- **Grouping updates:** `batch(fn)` turns several writes into one update, and
  `startTransition(fn)` makes the writes in `fn` low priority.
- **Signal names here:** `computed` is `memo` from `@blender-ui/native`. The
  name `memo` stays React's, for components.

**Keeping up with React.** Every hook is built from React's public hooks:
`useState`, `useReducer`, `useRef`, `useMemo`, the effect hooks and
`startTransition`. StrictMode, concurrent rendering and React's timing
therefore come from React itself. `bindings/react/test/hooks.test.js` runs
each hook beside React's original, in development and production builds and
under StrictMode. It fails when a React release adds a hook that has neither
a signal version nor an entry in `NO_TWIN`. That is how it caught React
19.2's `useEffectEvent`.

**Plain JavaScript** has the same core: `signal`, `memo`, `effect`, `batch`,
`untrack` and `startTransition`.

- **Binding:** `el.setAttribute(name, sig)`, `el.style = sig`,
  `el.textContent = sig` or `win.createTextNode(sig)`.
- **Unbinding:** setting a plain value unbinds the signal, and `destroy()`
  frees the bindings of a subtree.
- **Without the addon:** `require('@blender-ui/native/signals')`.

| Rust (`blenderui::hooks`) | `@blender-ui/native` | `@blender-ui/react` |
|---|---|---|
| `use_signal(v)` / `RwSignal` | `signal(v)` | `useSignal(v)` / `useState(v)` |
| `Memo::new(f)` | `memo(f)` | `useComputed(f)` / `useMemo(f, deps)` |
| `Effect::new(f)` | `effect(f)` (returns `dispose`) | `useEffect(f, [])` |
| `signal(v)` / `create_signal(v)` | `createSignal(v)` (`[get, set]`) | `useState(v)` |

The signals are push-pull and glitch-free:

- **Each memo or effect runs once per change:** a write marks direct readers
  dirty and indirect ones "check", and every memo or effect then runs at most
  once for it, after all its sources have settled.
- **Memos stop changes early:** a memo whose value didn't change stops the
  change from spreading further.
- **Unread memos hold nothing:** a memo that nothing observes keeps no
  subscriptions, so it is garbage once dropped.
- **Writes inside an effect:** a write to a signal the effect itself reads
  re-runs it after the current run.
- **Errors:** a memo that throws rethrows on read and recovers when its
  sources change.

## Standalone apps

`bui build` turns an app into one executable. It ships with `@blender-ui/native`
and needs [Bun](https://bun.sh) for bundling.

```sh
npx bui build app.jsx                # app.exe: BUI + QuickJS, about 5 MB
npx bui build app.jsx --bun          # app.exe: a Bun executable, about 115 MB
npx bui run app.jsx                  # run on the QuickJS runtime, no executable
```

Options:

- `-o <file>`: the output file.
- `--console`: keep a console window for logs. By default, Windows builds open
  no console.
- `--dev`: use React's development build.

**The default build** is `bui-runtime`, BUI with an embedded
[QuickJS](https://github.com/quickjs-ng/quickjs) engine (4.9 MB), with your
app's bundle appended. Your code and both packages run unchanged:

- **JavaScript runs on the UI thread**, as in a browser: no Node.js, no Bun,
  no browser, no webview.
- **What it provides:** `console`, timers, `queueMicrotask`, `performance` and
  a `process` with `env`, `argv`, `exit` and `platform`.
- **What it doesn't:** Node's modules (`fs`, `net` ...) aren't there. Apps that
  need them use `--bun`.

**With `--bun`** you get `bun build --compile`: the full Bun runtime with
Node's APIs and a JIT, and `@blender-ui/native`'s addon embedded in the
executable.

| A small React app, Windows x64 | QuickJS (default) | `--bun` |
|---|---:|---:|
| Executable | **4.9 MB** | 114 MB |
| First frame | **68 ms** | 79 ms |
| Launch to exit | **104 ms** | 147 ms |
| RAM at first frame | **19 MB** | 35 MB |
| RAM, 3,000 animated cells | **56–67 MB** | 80–118 MB |
| JavaScript speed | interpreter | JIT |

QuickJS has no JIT, so heavy JavaScript runs several times slower than on Bun
or Node. With BUI that matters less than it sounds:

- **The renderer is native:** layout, painting and the GPU cost the same on
  either runtime.
- **Signals keep JavaScript light:** a change updates a node without
  re-rendering components. Re-rendering a 3,000-element list every frame is
  where QuickJS falls behind (see Performance), so use signals there, or
  `--bun`.

The executable is built by appending the bundle to `bui-runtime`: nothing is
compiled per app, and a build takes well under a second. Prebuilt runtimes
are for Windows x64. On other platforms, `npm run build` in `bindings/node`
builds `bui-runtime` from source.

## How it works

- **A UI thread.** BUI runs its own message loop on a thread of its own, so
  the JavaScript thread is never blocked and an idle UI uses no CPU. A
  message-only window wakes the loop from JavaScript (`BK_window_wake`), and it
  survives Win32's modal move and resize loops.
- **Only changes cross.** JavaScript keeps the tree. Each tick, a window sends
  one `Int32Array` of operations (create, set attribute, append, insert,
  remove, set text, listen, free) plus a string table deduplicated per batch.
  React sends one batch per commit.
- **Retained mirror.** The UI thread applies the operations to its copy of the
  tree and rebuilds elements from it, keyed per node. Layout caches and form
  state persist, and a `style` attribute is parsed once, when it's set.
- **Events** come back through a napi thread-safe function. The deepest
  listening element gets each event natively, and JavaScript bubbles it.
- **Animation.** While `requestAnimationFrame` is active, the native side
  schedules the next vsync during the current render, as browsers do. Changes
  JavaScript makes in its callback join that frame, so there is one rebuild
  per frame.
- **Freeing.** Nodes that React removes, or creates in a render it throws away
  (an interrupted or suspended concurrent render), are freed natively. So is
  a closed window's tree: its bindings and listeners are released.

## Performance

### Startup

`node bench/startup.js` launches a small React app (`bench/first-frame.js`)
that quits as soon as its first frame is on screen. It does that 12 times for
each way to run an app, after one warm-up launch, and reports the medians.

- **First frame** is measured inside the app, from the runtime's start.
- **Launch to exit** is wall time measured from outside, including process
  creation and teardown.

| | First frame | Launch to exit | RAM at first frame | What you ship |
|---|---:|---:|---:|---:|
| **QuickJS executable** (`bui build`) | **68 ms** | **104 ms** | **19 MB** | **4.9 MB** |
| Bun executable (`bui build --bun`) | 79 ms | 147 ms | 35 MB | 114 MB |
| Bun (`bun app.js`) | 80 ms | 152 ms | 35 MB | Bun + the app |
| Node.js (`node app.js`) | 108 ms | 187 ms | 45 MB | Node + the app |
| Rust, for reference (the benchmark app) | 5 ms | ~92 ms | | 3.5 MB |

Timings vary by about ±20% between sessions on this laptop; the ranking
doesn't change. The first launch of a newly written executable can take
seconds while antivirus software scans it, which is why the warm-up launch
isn't counted.

### Animation

The Rust benchmark's `one` mode: a 1000×800 window with a grid of N cells,
where one cell changes colour and label every frame, driven from
`requestAnimationFrame`. Command: `bench/cells.js N --gpu [--react
[--signals]]`, run with `node`, `bun` or `node cli.js run` (the QuickJS
runtime).

- **"React, signals":** cell 0's style and label are signals, so the grid
  renders once.
- **"React, state":** the grid re-renders every frame, as with React's
  `useState`.
- **React build:** the benchmark uses React's production build; `--dev` uses
  the development build.

CPU is the whole process: JavaScript, BUI's UI thread and the runtime. It is
counted with the process cycle counter, as the Rust benchmark does
(`bindings/rust/bench`), because Windows' tick-based CPU time misses threads
that wake on each vsync.

The table shows medians of 3 runs, GPU renderer, on a Ryzen 7 5800U laptop
(Windows 11, 60 Hz) with Node.js 22.17 and Bun 1.4.4. Rust, Bun and Node ran
in one session; QuickJS (`bui-runtime`) ran in a later one.

| 3,000 cells | FPS | CPU per frame | RAM (resident) |
|---|---:|---:|---:|
| Rust | 59.6 | **12.0 ms** | **43 MB** |
| Bun, plain JS | **58.8** | 13.1 ms | 80 MB |
| Bun, React with signals | 58.4 | 13.3 ms | 85 MB |
| Bun, React with state | 55.8 | 20.8 ms | 118 MB |
| Node, plain JS | 55.8 | 14.4 ms | 88 MB |
| Node, React with signals | 56.7 | 14.0 ms | 93 MB |
| Node, React with state | 54.8 | 18.9 ms | 121 MB |
| QuickJS, plain JS | 59.1 | 16.3 ms | **56 MB** |
| QuickJS, React with signals | 58.7 | 14.9 ms | 61 MB |
| QuickJS, React with state | 19.7 | 51.0 ms | 67 MB |

| 1,000 cells (60 FPS, except as noted) | CPU per frame | RAM |
|---|---:|---:|
| Rust | **4.8 ms** | **32 MB** |
| Bun: plain JS / React with signals / React with state | 5.2 / 5.2 / 7.5 ms | 58 / 63 / 81 MB |
| Node: plain JS / React with signals / React with state | 5.6 / 5.6 / 6.9 ms | 68 / 71 / 97 MB |
| QuickJS: plain JS / React with signals / React with state (56.8 FPS) | 6.0 / 5.6 / 17.8 ms | **38 / 40 / 43 MB** |

What these show:

- **Overhead over Rust:** JavaScript adds 0.4–0.8 ms per frame at 1,000 cells
  and 1.1–2.4 ms at 3,000. That's the cost of sending changes to the UI
  thread, and of its copy of the tree.
- **Memory:** most of the extra RAM is the runtime itself. Node alone is about
  32 MB, and Bun's footprint is about 10 MB smaller.
- **QuickJS:** the leanest in memory: 38–67 MB, about 20–40 MB below Bun and
  Node. JavaScript shares the UI thread and has no JIT, so CPU per frame is a
  little higher with plain JS and signals. Re-rendering 3,000 elements every
  frame drops it to 20 FPS, so keep big lists on signals, or build with
  `--bun`.
- **React with signals:** costs what the plain API does. At 3,000 cells it
  uses about a quarter less CPU and memory than re-rendering, because the
  component never re-renders.
- **React's development build:** much heavier. Re-rendering 3,000 cells took
  about 184 MB, so measure with `NODE_ENV=production`.
- **Bun vs Node:** Bun is slightly faster with plain JS and signals, and
  leaner everywhere. Node is faster when React re-renders every frame.

Where the memory goes, for plain JS on Node at 3,000 cells:

- **Node itself:** 32 MB before anything loads.
- **A GPU window:** about 21 MB, for the driver, swap chain and glyph atlas.
  Rust pays this too.
- **The native tree:** about 7 MB more than Rust, because the UI thread keeps
  its own copy of the tree.
- **JavaScript objects:** a few MB live.

## Building

```sh
cd bindings
npm install                 # links the two packages (npm workspaces)
npm run build               # cargo build -p blenderui-node --release, copies the .node file
npm test                    # both packages' tests (headless)
node node/examples/counter.js
node react/examples/counter.js
bun node/bench/cells.js 3000 --gpu --react --signals   # Bun works too
```

The build needs what the Rust crate needs: Rust, CMake and a C compiler. With
MinGW, put its `bin` folder on `PATH`.

## Publishing

The packages are published under the [`@blender-ui`](https://www.npmjs.com/org/blender-ui)
npm organization. `@blender-ui/react` depends on `@blender-ui/native`
(`^0.2.0`), so publish the native package first, whenever it has changed.

```sh
npm login                                  # an account in the blender-ui org
cd bindings
npm install                                # link the workspace
cd node  && npm publish --access public    # @blender-ui/native (builds the addon and runtime, runs the tests)
cd ../react && npm publish --access public # @blender-ui/react (runs the tests)
```

- **`prepublishOnly`** runs the tests before uploading. For the native
  package it first builds the addon and `bui-runtime` (Rust, CMake and a C
  compiler; with MinGW, put its `bin` folder on `PATH`).
- **`prepack`** copies the repository's `LICENSE` into the package.
- **Previewing:** `npm pack --dry-run` lists what a package will contain;
  `npm pack` writes the `.tgz` you can install elsewhere to try it.
- **Releasing again:** bump `version` in the package's `package.json` first, since
  npm refuses to publish the same version twice. When the native package
  changes in a way React needs, raise React's `@blender-ui/native` range to
  match.

The native package ships `blenderui.win32-x64.node` and
`bui-runtime.win32-x64.exe`, and `os` / `cpu` in its `package.json` limit it
to Windows x64. Supporting more platforms means building both on each
platform, typically in CI, and publishing them as optional per-platform
packages, the napi-rs convention.
