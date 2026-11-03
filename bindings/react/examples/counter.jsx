// The same counter in JSX (run with a JSX-aware runner: `npx tsx counter.jsx`).
import React, { useState } from 'react'
import { render } from '@blenderui/react'

function Counter () {
  const [count, setCount] = useState(0)
  return (
    <main className="p-6 flex flex-col gap-3">
      <h1 className="text-xl font-bold">BUI + React</h1>
      <div className="flex items-center gap-3">
        <span>Clicks: {count}</span>
        <button onClick={() => setCount(c => c + 1)}>Add one</button>
      </div>
    </main>
  )
}

render(<Counter />, { title: 'BUI + React', width: 420, height: 220 })
