// A headless Svelte app that checks itself, as bindings/node/test/fixtures/
// react-app.jsx does for React: rendering, a click, a conditional block and a
// screenshot. Prints one JSON line.
import { render } from '@blender-ui/svelte'
import App from './RuntimeApp.svelte'

let renders = 0
const { app, window } = render(App, { props: { onrender: () => renders++ }, width: 200, height: 120, headless: true })
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
  window.click(10, 10)
  await tick()
  const after = await window.screenshot()
  console.log(JSON.stringify({
    runtime: globalThis.__BUI_NATIVE__ ? 'quickjs' : (typeof Bun !== 'undefined' ? 'bun' : 'node'),
    before: pixel(before, 70, 25),
    after: pixel(after, 70, 25),
    once,
    many: Boolean(find('many')),
    renders,
  }))
  app.quit()
})()
