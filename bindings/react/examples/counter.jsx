/** @jsxImportSource @blender-ui/react */
// The same counter in JSX. Bun runs it as is: `bun examples/counter.jsx`.
// Elsewhere, compile with `"jsxImportSource": "@blender-ui/react"`.
import { render, useState } from '@blender-ui/react'

function Counter () {
  const [count, setCount] = useState(0)
  const [name, setName] = useState('')
  return (
    <main className="p-6 flex flex-col gap-3">
      <h1 className="text-xl font-bold">BUI + React</h1>
      <div className="flex items-center gap-3">
        <span>Clicks: {count}</span>
        <button onClick={() => setCount(c => c + 1)}>Add one</button>
      </div>
      <input placeholder="Your name" value={name} onChange={e => setName(e.target.value)} />
      {name.value && <p className="text-[#9ab8e8]">Hello, {name}!</p>}
    </main>
  )
}

const { app, window } = render(<Counter />, { title: 'BUI + React', width: 420, height: 260 })
window.body.setAttribute('styled', '')
window.body.className = 'm-0 bg-[#2b2b2b] text-[#e6e6e6]'
if (process.env.BUI_EXAMPLE_SECONDS) setTimeout(() => app.quit(), Number(process.env.BUI_EXAMPLE_SECONDS) * 1000)
