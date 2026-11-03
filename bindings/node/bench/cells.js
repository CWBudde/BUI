// The Rust benchmark's "one" mode from JavaScript: a 1000x800 window with a
// grid of N cells; every frame (16 ms) cell 0 changes colour and label.
//   node bench/cells.js [cells=3000] [--react [--signals]] [--gpu] [--seconds=5]
// --signals: cell 0's style and label are BUI signals; the grid renders once.
// Prints CPU per update (whole process: JS thread + BUI's UI thread).
'use strict'
const args = process.argv.slice(2)
const cells = Number(args.find(a => /^\d+$/.test(a)) ?? 3000)
const useReact = args.includes('--react')
const useSignals = args.includes('--signals')
const gpu = args.includes('--gpu')
const seconds = Number((args.find(a => a.startsWith('--seconds=')) ?? '--seconds=5').split('=')[1])
const warmup = 2

// same grid and colours as bindings/rust/bench/common/measure.rs
const W = 1000, H = 800, GAP = 2
const cols = Math.max(1, Math.ceil(Math.sqrt(cells * W / H)))
const rows = Math.ceil(cells / cols)
const cw = Math.floor((W - GAP) / cols - GAP), ch = Math.floor((H - GAP) / rows - GAP)
const font = Math.floor(Math.min(14, Math.max(8, ((H - GAP) / rows - GAP) * 0.55)))
function color (i, t) {
  const h = ((i * 7 + t * 3) % 360) / 60, s = 0.55, v = 0.75, c = v * s
  const x = c * (1 - Math.abs((h % 2) - 1)), m = v - c
  const [r, g, b] = [[c, x, 0], [x, c, 0], [0, c, x], [0, x, c], [x, 0, c], [c, 0, x]][Math.floor(h) % 6]
  const hex = (u) => Math.round((u + m) * 255).toString(16).padStart(2, '0')
  return `#${hex(r)}${hex(g)}${hex(b)}`
}
const label = (i, t) => String((i * 13 + t) % 1000)
const cellStyle = (i, t) => `width: ${cw}px; height: ${ch}px; background: ${color(i, t)}; border-radius: 3px; ` +
  `display: flex; align-items: center; justify-content: center; font-size: ${font}px; color: #fff`
const gridStyle = `width: 100%; height: 100%; background: #303030; display: flex; flex-wrap: wrap; ` +
  `align-content: flex-start; gap: ${GAP}px; padding: ${GAP}px`

let tick
let app
let win
if (useReact) {
  const React = require(require.resolve('react', { paths: [require('node:path').join(__dirname, '../../react')] }))
  const { render } = require('../../react')
  const h = React.createElement
  let setT
  const { signal, batch } = require('..')
  const style0 = signal(cellStyle(0, 0))
  const label0 = signal(label(0, 0))
  function Grid () {
    const [t, set] = React.useState(0)
    setT = set
    const kids = []
    for (let i = 0; i < cells; i++) {
      const ti = i === 0 ? t : 0
      if (i === 0 && useSignals) kids.push(h('div', { key: i, style: style0 }, label0))
      else kids.push(h('div', { key: i, style: cellStyle(i, ti) }, label(i, ti)))
    }
    return h('div', { style: gridStyle }, kids)
  }
  const r = render(h(Grid), { title: 'BUI bench (React)', width: W, height: H, gpu })
  app = r.app
  win = r.window
  r.window.body.style = 'margin: 0'
  tick = useSignals
    ? (t) => batch(() => { style0.value = cellStyle(0, t); label0.value = label(0, t) })
    : (t) => setT(t)
} else {
  const { App } = require('..')
  app = new App({ gpu })
  win = app.window({ title: 'BUI bench (JS)', width: W, height: H })
  win.body.style = 'margin: 0'
  const grid = win.createElement('div')
  grid.style = gridStyle
  let first, firstText
  for (let i = 0; i < cells; i++) {
    const c = win.createElement('div')
    c.style = cellStyle(i, 0)
    const text = win.createTextNode(label(i, 0))
    c.appendChild(text)
    grid.appendChild(c)
    if (i === 0) { first = c; firstText = text }
  }
  win.body.appendChild(grid)
  tick = (t) => { first.style = cellStyle(0, t); firstText.data = label(0, t) }
}

let t = 0, updates = 0, cpu0, wall0, rss = 0, samples = 0, running = true
// one update per displayed frame (vsync), like the Rust benchmark
const frame = () => { if (!running) return; tick(++t); updates++; win.requestAnimationFrame(frame) }
setTimeout(() => win.requestAnimationFrame(frame), 200)
setTimeout(() => { cpu0 = process.cpuUsage(); wall0 = performance.now(); updates = 0 }, warmup * 1000)
const sampler = setInterval(() => { rss += process.memoryUsage.rss(); samples++ }, 250)
setTimeout(() => {
  const cpu = process.cpuUsage(cpu0), wall = performance.now() - wall0
  const ms = (cpu.user + cpu.system) / 1000
  running = false; clearInterval(sampler)
  console.log(JSON.stringify({
    app: useReact ? (useSignals ? 'blenderui-react-signals' : 'blenderui-react') : 'blenderui-js', gpu, cells, updates_per_s: +(updates / wall * 1000).toFixed(1),
    cpu_pct: +(ms / wall * 100).toFixed(1), cpu_ms_per_update: +(ms / updates).toFixed(2),
    rss_mb: +(rss / samples / 1048576).toFixed(1),
  }))
  app.quit()
}, (warmup + seconds) * 1000)
