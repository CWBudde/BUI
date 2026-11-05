/** @jsxImportSource @blender-ui/react */
// A headless React app that checks itself: rendering, a click, signals,
// re-rendering on a read, and a screenshot. Prints one JSON line.
import { render, useState } from '@blender-ui/react'

let renders = 0
function Counter () {
  renders++
  const [count, setCount] = useState(0)
  const [color, setColor] = useState('#0000ff')
  return (
    <main style={{ margin: 0 }}>
      <div id="btn" style={{ width: 80, height: 30, backgroundColor: color }}
        onClick={() => { setCount(c => c + 1); setColor('#00ff00') }}>count {count}</div>
      <Many count={count} />
    </main>
  )
}

// reads the signal while rendering: re-renders when it changes
function Many ({ count }) {
  return count.value > 1 ? <p id="many">many</p> : null
}

const { app, window } = render(<Counter />, { width: 200, height: 120, headless: true })
window.body.style = 'margin: 0; background: #000'
const tick = (ms = 40) => new Promise(r => setTimeout(r, ms))
const find = (id) => [...window._nodes.values()].find(n => n.getAttribute?.('id') === id)
// sampled inside the button but right of the "count N" label, so text never covers it
const pixel = (s, x, y) => { const i = (y * s.width + x) * 4; return [s.data[i], s.data[i + 1], s.data[i + 2]] }

;(async () => {
  await tick()
  const before = await window.screenshot()
  window.click(10, 10)
  await tick()
  const once = find('btn').textContent
  const rendersAfterOne = renders
  window.click(10, 10)
  await tick()
  const after = await window.screenshot()
  console.log(JSON.stringify({
    runtime: globalThis.__BUI_NATIVE__ ? 'quickjs' : (typeof Bun !== 'undefined' ? 'bun' : 'node'),
    before: pixel(before, 70, 25),
    after: pixel(after, 70, 25),
    once,
    rendersAfterOne,
    many: Boolean(find('many')),
    renders,
  }))
  app.quit()
})()
