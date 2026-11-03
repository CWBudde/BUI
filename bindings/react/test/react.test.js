'use strict'
const test = require('node:test')
const assert = require('node:assert')
const React = require('react')
const { render } = require('..')
const h = React.createElement

function Counter () {
  const [n, setN] = React.useState(0)
  return h('div', { className: 'flex flex-col gap-2', style: { margin: 0 } },
    h('div', {
      id: 'btn',
      style: { width: 80, height: 30, backgroundColor: n ? '#00ff00' : '#0000ff' },
      onClick: () => setN(n + 1),
    }, `count ${n}`),
    n > 0 && h('div', { className: 'w-[50px] h-[10px] bg-[#ff0000]' }))
}

const tick = (ms = 30) => new Promise(r => setTimeout(r, ms))
const at = (s, x, y) => { const i = (y * s.width + x) * 4; return [...s.data.subarray(i, i + 3)] }

test('React: render, state, events, conditional children (headless)', async (t) => {
  const { app, window } = render(h(Counter), { headless: true, width: 200, height: 120 })
  t.after(() => app.quit())
  window.body.style = 'margin: 0; background: #000'
  await tick()
  let shot = await window.screenshot()
  assert.deepStrictEqual(at(shot, 70, 20), [0, 0, 255])
  const btn = [...window._nodes.values()].find(n => n.getAttribute?.('id') === 'btn')
  assert.strictEqual(btn.textContent, 'count 0')
  window.click(10, 10)
  await tick()
  assert.strictEqual(btn.textContent, 'count 1', 'state update re-rendered the text')
  shot = await window.screenshot()
  assert.deepStrictEqual(at(shot, 70, 20), [0, 255, 0], 'style prop updated')
  assert.deepStrictEqual(at(shot, 10, 42), [255, 0, 0], 'conditional child mounted (8px gap below the button)')
})

test('React: BUI signals as props and children update without re-rendering', async (t) => {
  const { useSignal, useComputed, useSignalValue, signal } = require('..')
  const external = signal(0)
  let renders = 0
  let sig
  function Panel () {
    renders++
    const n = useSignal(0)
    sig = n
    const color = useComputed(() => (n.value % 2 ? '#00ff00' : '#0000ff'))
    const style = useComputed(() => ({ width: 80, height: 30, backgroundColor: color.value }))
    return h('div', { style: { margin: 0 } },
      h('div', { id: 'box', style, onClick: () => n.update(v => v + 1) }, 'count ', n),
      h(Watcher))
  }
  let watcherRenders = 0
  function Watcher () {
    watcherRenders++
    return h('span', null, `ext ${useSignalValue(external)}`)
  }
  const { app, window } = render(h(Panel), { headless: true, width: 200, height: 120 })
  t.after(() => app.quit())
  window.body.style = 'margin: 0; background: #000'
  await tick()
  const box = [...window._nodes.values()].find(n => n.getAttribute?.('id') === 'box')
  assert.strictEqual(box.textContent, 'count 0')
  assert.deepStrictEqual(at(await window.screenshot(), 70, 20), [0, 0, 255])
  window.click(10, 10)
  await tick()
  assert.strictEqual(box.textContent, 'count 1')
  assert.deepStrictEqual(at(await window.screenshot(), 70, 20), [0, 255, 0])
  sig.value = 2
  await tick()
  assert.strictEqual(box.textContent, 'count 2')
  assert.strictEqual(renders, 1, 'Panel rendered once: signal writes went straight to the nodes')
  external.value = 5
  await tick()
  assert.strictEqual(watcherRenders, 2, 'useSignalValue re-renders its component')
  const span = [...window._nodes.values()].find(n => n.textContent === 'ext 5' && n.localName === 'span')
  assert.ok(span)
})
