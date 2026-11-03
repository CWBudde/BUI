# @blenderui/react

A React 19 renderer for BUI: JSX rendered with Blender's UI in native
windows, over [`@blenderui/native`](../node).

```jsx
import { useState } from 'react'
import { render } from '@blenderui/react'

function Counter () {
  const [n, setN] = useState(0)
  return <button className="m-4" onClick={() => setN(n + 1)}>Clicked {n} times</button>
}

render(<Counter />, { title: 'Counter', width: 320, height: 160 })
```

BUI's hooks work alongside React's. A signal passed as a prop or child
updates that node alone, without re-rendering the component:

```jsx
import { render, useSignal } from '@blenderui/react'

function Counter () {
  const n = useSignal(0) // also useComputed, useSignalEffect, useSignalValue
  return <button className="m-4" onClick={() => n.value++}>Clicked {n} times</button>
}
```

Guide and benchmarks: [docs/javascript.md](../../docs/javascript.md).
License: GPL-2.0-or-later.
