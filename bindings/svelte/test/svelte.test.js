// The scenarios of @blender-ui/react's test/react.test.js, in Svelte 5, and
// what Svelte adds: bindings, keyed lists, snippets, context, lifecycle.
import test from 'node:test'
import assert from 'node:assert'
import { render, act, signal, Button } from '../index.js'
import { HANDLERS } from '../events.js'
import Counter from './fixtures/Counter.svelte'
import Panel from './fixtures/Panel.svelte'
import Modes from './fixtures/Modes.svelte'
import Show from './fixtures/Show.svelte'
import Form from './fixtures/Form.svelte'
import Blocks from './fixtures/Blocks.svelte'

const tick = (ms = 30) => new Promise(r => setTimeout(r, ms))
const at = (s, x, y) => { const i = (y * s.width + x) * 4; return [...s.data.subarray(i, i + 3)] }
const finder = (window) => (id) => [...window._nodes.values()].find(n => n.getAttribute?.('id') === id)
const elements = (el) => el.childNodes.filter(n => n.nodeType === 1)

test('Svelte: render, state, events, conditional children (headless)', async (t) => {
  let renders = 0
  const { app, window } = render(Counter, { props: { onrender: () => renders++ }, headless: true, width: 200, height: 120 })
  t.after(() => app.quit())
  window.body.style = 'margin: 0; background: #000'
  await tick()
  let shot = await window.screenshot()
  assert.deepStrictEqual(at(shot, 70, 20), [0, 0, 255])
  const btn = finder(window)('btn')
  assert.strictEqual(btn.textContent, 'count 0')
  window.click(10, 10)
  await tick()
  assert.strictEqual(btn.textContent, 'count 1', 'state update changed the text')
  shot = await window.screenshot()
  assert.deepStrictEqual(at(shot, 70, 20), [0, 255, 0], 'style updated')
  assert.deepStrictEqual(at(shot, 10, 42), [255, 0, 0], 'conditional child mounted (8px gap below the button)')
  assert.strictEqual(renders, 1, 'the component ran once: runes updated the nodes')
})

test('Svelte: BUI signals are stores; writes update their nodes without re-running the component', async (t) => {
  const external = signal(0)
  let sig
  let renders = 0
  const { app, window } = render(Panel, {
    props: { external, expose: (n) => { sig = n }, onrender: () => renders++ },
    headless: true, width: 200, height: 120,
  })
  t.after(() => app.quit())
  window.body.style = 'margin: 0; background: #000'
  await tick()
  const find = finder(window)
  assert.strictEqual(find('box').textContent, 'count 0')
  assert.deepStrictEqual(at(await window.screenshot(), 70, 20), [0, 0, 255])
  window.click(10, 10)
  await tick()
  assert.strictEqual(find('box').textContent, 'count 1')
  assert.deepStrictEqual(at(await window.screenshot(), 70, 20), [0, 255, 0])
  sig.value = 2
  await tick()
  assert.strictEqual(find('box').textContent, 'count 2')
  external.value = 5
  await tick()
  assert.strictEqual(find('ext').textContent, 'ext 5')
  assert.strictEqual(renders, 1, 'Panel ran once: signal writes went straight to the nodes')
})

test('Svelte: text and elements take turns; keyed lists reorder; dropped nodes are freed', async (t) => {
  let set
  const { app, window } = render(Modes, { props: { expose: (s) => { set = s } }, headless: true, width: 100, height: 100 })
  t.after(() => app.quit())
  const find = finder(window)
  await tick()
  assert.strictEqual(find('box').textContent, 'hello')
  assert.ok(find('in')._listeners.get('change')?.size, 'onchange listens natively')
  act(() => set.mode('el'))
  await tick()
  assert.strictEqual(elements(find('box'))[0].localName, 'span')
  assert.strictEqual(find('box').textContent, 'inner')
  assert.strictEqual(find('in').getAttribute('type'), 'checkbox')
  act(() => set.mode('none'))
  await tick()
  assert.deepStrictEqual(elements(find('box')).map(e => e.localName), ['input'], 'only the input left')
  const li = () => elements(find('list')).map(e => e.textContent)
  assert.deepStrictEqual(li(), ['1', '2', '3'])
  const two = elements(find('list'))[1]
  act(() => set.items([3, 2, 1]))
  await tick()
  assert.deepStrictEqual(li(), ['3', '2', '1'])
  assert.strictEqual(elements(find('list'))[1], two, 'keyed: the node moved, not re-created')
  const before = window._nodes.size
  act(() => { set.mode('el'); set.items([3, 2, 1, 4, 5]) })
  await tick()
  assert.ok(window._nodes.size > before)
  act(() => { set.mode('none'); set.items([3, 2, 1]) })
  await tick()
  assert.strictEqual(window._nodes.size, before, 'removed elements and their text are freed')
})

test('Svelte: hidden keeps a subtree; transitions commit without animating', async (t) => {
  let setShow
  const { app, window } = render(Show, { props: { expose: (s) => { setShow = s } }, headless: true, width: 100, height: 100 })
  t.after(() => app.quit())
  const find = finder(window)
  await tick()
  assert.strictEqual(find('kid').hasAttribute('hidden'), false)
  assert.ok(find('vt'))
  setShow(false)
  await tick(100)
  assert.strictEqual(find('kid').hasAttribute('hidden'), true, 'hidden, and kept')
  assert.strictEqual(find('vt'), undefined, 'the outro finished and freed its node')
  setShow(true)
  await tick(100)
  assert.strictEqual(find('kid').hasAttribute('hidden'), false)
  assert.strictEqual(find('vt').textContent, 'shown')
})

test('Svelte: bindings, Blender widgets, snippets, context and lifecycle', async (t) => {
  const log = []
  const { app, window } = render(Form, { props: { log }, headless: true, width: 300, height: 300 })
  t.after(() => app.quit())
  const find = finder(window)
  await tick()
  assert.deepStrictEqual(log, ['mount'])
  find('name')._dispatch({ kind: 'input', value: 'Ada' })
  find('agree')._dispatch({ kind: 'change', checked: true })
  await tick()
  assert.strictEqual(find('hello').textContent, 'Hello, Ada! (agreed)', 'bind:value and bind:checked')
  const apply = find('apply')
  assert.strictEqual(apply.localName, 'button')
  assert.ok(apply.hasAttribute('bui-blender'), 'a Blender widget')
  assert.ok(apply._listeners.get('click')?.size, 'its onclick listens natively')
  apply._dispatch({ kind: 'click' })
  await tick()
  assert.strictEqual(apply.textContent, 'Apply 1')
  const size = find('size')
  assert.deepStrictEqual([size.localName, size.getAttribute('type'), size.getAttribute('max')], ['input', 'range', '10'])
  assert.strictEqual(find('card').getAttribute('data-theme'), 'blender', 'context')
  assert.strictEqual(find('card').textContent.trim(), 'Titlebody text', 'snippets')
  assert.strictEqual(typeof Button, 'function')
  window.close()
  assert.deepStrictEqual(log, ['mount', 'destroy'], 'closing the window unmounts')
})

test('Svelte: {#await}, {#key}, class objects, bind:group and a scoped <style>', async (t) => {
  let resolve
  let set
  const promise = new Promise((r) => { resolve = r })
  const { app, window } = render(Blocks, { props: { promise, expose: (s) => { set = s } }, headless: true, width: 100, height: 100 })
  t.after(() => app.quit())
  window.body.style = 'margin: 0; background: #000'
  const find = finder(window)
  await tick()
  assert.strictEqual(find('await').textContent, 'loading')
  resolve(42)
  await tick()
  assert.strictEqual(find('await').textContent, 'got 42')
  const first = find('key')
  act(() => set.bump())
  await tick()
  assert.notStrictEqual(find('key'), first, '{#key} re-created its content')
  assert.strictEqual(find('key').textContent, 'v2')
  assert.strictEqual(find('cls').className, 'base')
  act(() => set.toggle())
  assert.deepStrictEqual(find('cls').className.split(' ').sort(), ['base', 'on'])
  find('fb')._dispatch({ kind: 'change', checked: true })
  await tick()
  assert.strictEqual(find('flavor').textContent, 'b', 'bind:group')
  // (no id attribute: BUI measures elements without one)
  const red = [...window._nodes.values()].find((n) => n.getAttribute?.('class')?.startsWith('red'))
  assert.match(red.className, /svelte-/, 'scoped class')
  const box = await red.getBoundingClientRect()
  assert.deepStrictEqual([box.width, box.height], [40, 20], 'the component stylesheet applies')
  const styles = window.body.childNodes.filter((n) => n.localName === 'style')
  assert.strictEqual(styles.length, 1, 'one stylesheet per component, per window')
})

test('Svelte: the event-delegation symbol is found (fails when a Svelte release changes it)', () => {
  assert.strictEqual(typeof HANDLERS, 'symbol')
})
