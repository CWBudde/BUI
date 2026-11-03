// A counter in a native window, with the plain DOM-like API.
//   node examples/counter.js            (BUI_GPU=1 for the GPU renderer)
'use strict'
const { App } = require('..')

const app = new App({ gpu: process.env.BUI_GPU === '1' })
const win = app.window({ title: 'BUI from Node', width: 360, height: 220 })
win.body.className = 'm-0 p-6 flex flex-col gap-3 bg-[#2b2b2b] text-[#e6e6e6]'

const title = win.createElement('h1')
title.className = 'text-xl font-bold'
title.textContent = 'Hello from JavaScript'
win.body.appendChild(title)

const row = win.createElement('div')
row.className = 'flex items-center gap-3'
const label = win.createElement('span')
label.textContent = 'Clicks: 0'
const button = win.createElement('button')
button.textContent = 'Add one'
let n = 0
button.addEventListener('click', () => { label.textContent = `Clicks: ${++n}` })
row.appendChild(label)
row.appendChild(button)
win.body.appendChild(row)

win.on('close', () => console.log(`closed after ${n} clicks`))
if (process.env.BUI_EXAMPLE_SECONDS) {
  // CI / smoke tests: click once, then quit
  setTimeout(() => { const r = button.getBoundingClientRect(); r.then(b => win.click(b.x + 5, b.y + 5)) }, 300)
  setTimeout(() => { console.log(`label: ${label.textContent}`); app.quit() }, Number(process.env.BUI_EXAMPLE_SECONDS) * 1000)
}
