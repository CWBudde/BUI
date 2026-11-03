'use strict'
const test = require('node:test')
const assert = require('node:assert')
const { App } = require('..')

test('tree, styles, Tailwind, events, screenshot (headless)', async (t) => {
  const app = new App({ headless: true })
  t.after(() => app.quit())
  const win = app.window({ width: 320, height: 200 })
  win.body.style = 'margin: 0; background: #000'
  const box = win.createElement('div')
  box.className = 'w-[100px] h-[40px] bg-[#ff0000]'
  win.body.appendChild(box)
  const btn = win.createElement('div')
  btn.style = { width: 80, height: 30, backgroundColor: '#0000ff' }
  const label = win.createTextNode('0')
  btn.appendChild(label)
  win.body.appendChild(btn)
  let clicks = 0
  const clicked = new Promise(resolve => btn.addEventListener('click', e => {
    clicks++
    label.data = String(clicks)
    btn.style = { width: 80, height: 30, backgroundColor: '#00ff00' }
    resolve(e)
  }))

  const r = await btn.getBoundingClientRect()
  assert.deepStrictEqual([r.x, r.y, r.width, r.height], [0, 40, 80, 30])
  let shot = await win.screenshot()
  const at = (s, x, y) => { const i = (y * s.width + x) * 4; return [...s.data.subarray(i, i + 3)] }
  assert.deepStrictEqual(at(shot, 10, 10), [255, 0, 0], 'Tailwind arbitrary class')
  assert.deepStrictEqual(at(shot, 70, 60), [0, 0, 255], 'style object')

  win.click(40, 55)
  const e = await clicked
  assert.strictEqual(e.type, 'click')
  assert.strictEqual(e.target, btn)
  shot = await win.screenshot()
  assert.deepStrictEqual(at(shot, 70, 60), [0, 255, 0], 'handler re-styled it')
  assert.strictEqual(btn.textContent, '1')
})

test('requestAnimationFrame runs callbacks per frame (headless)', async (t) => {
  const app = new App({ headless: true })
  t.after(() => app.quit())
  const win = app.window({ width: 100, height: 100 })
  const box = win.createElement('div')
  win.body.appendChild(box)
  let frames = 0
  await new Promise(resolve => {
    const step = () => {
      box.style = { width: ++frames, height: 10 }
      if (frames < 5) win.requestAnimationFrame(step)
      else resolve()
    }
    win.requestAnimationFrame(step)
  })
  const r = await box.getBoundingClientRect()
  assert.strictEqual(frames, 5)
  assert.strictEqual(r.width, 5, 'the last frame\'s change rendered')
})

test('signals bound to attributes and text update only their node (headless)', async (t) => {
  const { App, signal, memo } = require('..')
  const app = new App({ headless: true })
  t.after(() => app.quit())
  const win = app.window({ width: 100, height: 60 })
  win.body.style = 'margin: 0; background: #000'
  const color = signal('#0000ff')
  const n = signal(1)
  const box = win.createElement('div')
  box.style = memo(() => ({ width: 40, height: 20, backgroundColor: color.value }))
  const label = win.createElement('span')
  label.textContent = memo(() => `n = ${n.value}`)
  win.body.appendChild(box)
  win.body.appendChild(label)
  const tick = () => new Promise(r => setTimeout(r, 30))
  const at = (s, x, y) => { const i = (y * s.width + x) * 4; return [...s.data.subarray(i, i + 3)] }
  await tick()
  assert.deepStrictEqual(at(await win.screenshot(), 10, 10), [0, 0, 255])
  assert.strictEqual(label.textContent, 'n = 1')
  color.value = '#00ff00'
  n.value = 2
  assert.strictEqual(label.textContent, 'n = 2')
  await tick()
  assert.deepStrictEqual(at(await win.screenshot(), 10, 10), [0, 255, 0])
  box.style = 'width: 40px; height: 20px; background: #ff0000' // a plain value unbinds
  color.value = '#ffffff'
  await tick()
  assert.deepStrictEqual(at(await win.screenshot(), 10, 10), [255, 0, 0])
})
