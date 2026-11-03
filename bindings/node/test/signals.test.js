'use strict'
const test = require('node:test')
const assert = require('node:assert')
const { signal, memo, effect, batch, untrack, createSignal } = require('../signals')

test('signals: memo is lazy, cached and glitch-free', () => {
  const a = signal(1)
  const b = signal(2)
  let runs = 0
  const sum = memo(() => { runs++; return a.value + b.value })
  assert.strictEqual(runs, 0, 'lazy')
  assert.strictEqual(sum.value, 3)
  assert.strictEqual(sum.value, 3)
  assert.strictEqual(runs, 1, 'cached')
  const seen = []
  const parity = memo(() => sum.value % 2)
  const stop = effect(() => { seen.push(`${sum.value}:${parity.value}`) })
  batch(() => { a.value = 2; b.value = 3 })
  assert.deepStrictEqual(seen, ['3:1', '5:1'], 'one run per batch, never an inconsistent pair')
  a.value = 4 // sum 7, parity unchanged
  assert.deepStrictEqual(seen, ['3:1', '5:1', '7:1'])
  stop()
  a.value = 100
  assert.strictEqual(seen.length, 3, 'disposed effects stay quiet')
})

test('signals: equality cut-off, cleanups, nested effects, untrack', () => {
  const n = signal(1)
  const even = memo(() => n.value % 2 === 0)
  let runs = 0
  effect(() => { even.value; runs++ })
  n.value = 3 // even is still false: the effect doesn't re-run
  assert.strictEqual(runs, 1)
  n.value = 4
  assert.strictEqual(runs, 2)

  const log = []
  const outer = signal('x')
  const inner = signal(0)
  const stop = effect(() => {
    const o = outer.value
    effect(() => { log.push(`${o}${inner.value}`) })
    return () => log.push(`cleanup ${o}`)
  })
  inner.value = 1
  outer.value = 'y' // disposes the old inner effect
  inner.value = 2
  assert.deepStrictEqual(log, ['x0', 'x1', 'cleanup x', 'y1', 'y2'])
  stop()

  let reads = 0
  const s = signal(0)
  effect(() => { untrack(() => s.value); reads++ })
  s.value = 1
  assert.strictEqual(reads, 1, 'untracked reads do not subscribe')

  const [count, setCount] = createSignal(1)
  setCount(c => c + 1)
  assert.strictEqual(count(), 2)
})

test('signals: unobserved memos hold no subscriptions and stay correct', () => {
  const a = signal(1)
  const m = memo(() => a.value * 2)
  assert.strictEqual(m.value, 2)
  assert.strictEqual(a._subs.size, 0, 'read outside an effect: not subscribed (collectable)')
  a.value = 2
  assert.strictEqual(m.value, 4, 'revalidated on read')
  const seen = []
  const stop = effect(() => { seen.push(m.value) })
  assert.strictEqual(a._subs.size, 1, 'observed: subscribed')
  a.value = 3
  stop()
  assert.strictEqual(a._subs.size, 0, 'last observer gone: detached')
  a.value = 4
  assert.strictEqual(m.value, 8)
  const again = effect(() => { seen.push(m.value) })
  a.value = 5
  again()
  assert.deepStrictEqual(seen, [4, 6, 8, 10])
})

test('signals: transitions defer their effects; urgent writes go first', async () => {
  const { startTransition } = require('../signals')
  const a = signal(0), b = signal(0)
  const order = []
  const s1 = effect(() => { if (a.value) order.push('a') })
  const s2 = effect(() => { if (b.value) order.push('b') })
  const done = startTransition(() => { a.value = 1 })
  b.value = 1
  assert.deepStrictEqual(order, ['b'])
  await done
  assert.deepStrictEqual(order, ['b', 'a'])
  await assert.rejects(startTransition(async () => { throw new Error('x') }), /x/)
  s1()
  s2()
})

test('signals: a write inside an effect re-runs it (clamping), without re-entering', () => {
  const x = signal(15)
  let label = ''
  let depth = 0, maxDepth = 0
  effect(() => {
    depth++
    maxDepth = Math.max(maxDepth, depth)
    label = String(x.value)
    if (x.value > 10) x.value = 10
    depth--
  })
  assert.strictEqual(x.peek(), 10)
  assert.strictEqual(label, '10', 'the clamped value was rendered')
  assert.strictEqual(maxDepth, 1, 'never ran inside itself')
  x.value = 20
  assert.strictEqual(label, '10')
})

test('signals: a memo that throws recovers; its readers keep following it', () => {
  const a = signal(false), b = signal(1)
  const m = memo(() => { if (a.value) throw new Error('bad'); return b.value })
  const seen = []
  effect(() => { try { seen.push(m.value) } catch (e) { seen.push(e.message) } })
  a.value = true
  a.value = false
  b.value = 2
  assert.deepStrictEqual(seen, [1, 'bad', 1, 2])
  assert.throws(() => { a.value = true; m.peek() }, /bad/)
})

test('signals: an urgent write overtakes a pending transition through a memo', async () => {
  const { startTransition } = require('../signals')
  const s = signal(0)
  const m = memo(() => s.value * 2)
  const seen = []
  effect(() => { seen.push(m.value) })
  const done = startTransition(() => { s.value = 1 })
  s.value = 5 // urgent
  assert.deepStrictEqual(seen, [0, 10], 'ran now, not after the transition slice')
  await done
  assert.deepStrictEqual(seen, [0, 10])
})

test('signals: a second transition waits for an already deferred effect', async () => {
  const { startTransition } = require('../signals')
  const s = signal(0)
  const seen = []
  effect(() => { seen.push(s.value) })
  startTransition(() => { s.value = 1 })
  const second = startTransition(() => { s.value = 2 })
  await second
  assert.strictEqual(seen.at(-1), 2, 'resolved only after its value was applied')
})

test('signals: a failing first run leaves no subscription; bindings outlive outer effects', () => {
  const s = signal(1)
  assert.throws(() => effect(() => { s.value; throw new Error('first') }), /first/)
  assert.strictEqual(s._subs.size, 0)
  const { unowned } = require('../signals')
  const outer = signal(0)
  const inner = signal('a')
  let seen = ''
  effect(() => {
    outer.value
    unowned(() => effect(() => { seen = inner.value }))
  })
  outer.value = 1 // re-runs the outer effect
  inner.value = 'b'
  assert.strictEqual(seen, 'b')
})
