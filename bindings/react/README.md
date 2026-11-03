# @blender-ui/react

A React 19 renderer for [BUI](https://github.com/ppmpreetham/BUI): JSX
rendered with Blender's UI toolkit in native windows, with no browser and no
Electron. It runs on Node.js and Bun. React's hooks are exported by their own
names, with signals underneath, so a state change can update a single node
without re-rendering its component.

```sh
npm install @blender-ui/react react
```

```jsx
import { render, useState } from '@blender-ui/react'

function Counter () {
  const [count, setCount] = useState(0)
  return (
    <main className="p-6 flex flex-col gap-3 bg-[#2b2b2b] text-[#e6e6e6]">
      <span>Clicks: {count}</span>
      <button onClick={() => setCount(c => c + 1)}>Add one</button>
    </main>
  )
}

render(<Counter />, { title: 'Counter', width: 360, height: 200, gpu: true })
```

Set `"jsxImportSource": "@blender-ui/react"` in your `tsconfig.json`, or in
your bundler's JSX options. This does two things: components re-render when
a signal they read while rendering changes, and in TypeScript, host-element
props accept signals.

## Hooks

The hooks have React's names, arguments and timing, but the state they hold
is a **signal**:

- **Passed as a prop or child** (`<span>{count}</span>`,
  `style={styleMemo}`): a change updates that node. The component doesn't
  re-render.
- **Read while rendering** (`count.value`; in plain JavaScript also
  `count + 1` or `count > 5`): the component re-renders when it changes, as
  with React. TypeScript needs `.value` for arithmetic.
- **Conditions and `===` need `.value`.** A signal is an object, so it's always
  truthy: write `{isPending.value && <Spinner />}` and `count.value === 0`.

| Hook | Returns |
|---|---|
| `useState(init)` | `[signal, setState]` |
| `useReducer(reducer, arg, init?)` | `[signal, dispatch]` |
| `useMemo(fn, deps?)` | a memo, recomputed when the signals it reads change |
| `useRef(v)` | `{ current }`, observable by effects and memos |
| `useCallback(fn, deps?)` / `useEffectEvent(fn)` | a function whose calls are batched |
| `useEffect` / `useLayoutEffect` / `useInsertionEffect` | React's timing and deps, plus a re-run when signals the effect read change |
| `useSyncExternalStore(subscribe, get)` | a signal following the store |
| `useTransition()` | `[isPending, startTransition]`; signal updates run after urgent ones |
| `useDeferredValue(value, initial?)` | a signal that follows `value` at low priority |
| `useActionState(action, initial)` | `[state, dispatch, isPending]` |
| `useOptimistic(state, reducer?)` | `[optimistic, addOptimistic]` |
| `use`, `useContext`, `useId`, `useDebugValue`, `useImperativeHandle` | React's own |

Also exported:

- **Rust-style names:** `useSignal(v)` and `useComputed(fn)`.
- **`useSignalValue(sig)`:** re-renders its component when `sig` changes.
- **Plain signals:** `signal`, `computed`, `effect`, `batch`, `untrack`,
  `startTransition`.

Want React's plain-value hooks instead? Import them from `react`; they work
here too.

Every hook is built from React's public hooks. A test suite runs each one
beside React's original, in development and production builds and under
StrictMode. The suite fails when a new React release adds a hook this package
doesn't cover yet.

## The renderer

- **Host components are HTML elements.** `className` takes Tailwind classes,
  `style` takes strings, objects or signals. `onClick`, `onChange` and the
  other mouse and keyboard handlers work. Events bubble, and
  `stopPropagation` works.
- **One native batch per commit:** only what changed crosses to BUI's UI
  thread.
- **React 19.3 features:** `<Suspense>`, `<Activity>`, transitions, actions,
  and `<ViewTransition>` (it commits, without animating).
- **API:** `render(element, options)` opens a window. `createRoot(window)`
  renders into a window you opened with
  [`@blender-ui/native`](https://www.npmjs.com/package/@blender-ui/native).
  `act(fn)` is for tests.

## Requirements

- Windows x64 for native windows; `headless: true` renders off-screen.
- Node.js 18 or newer, or Bun.
- React 19.3.

Guide, benchmarks and internals:
[docs/javascript.md](https://github.com/ppmpreetham/BUI/blob/main/docs/javascript.md).
License: GPL-2.0-or-later.
