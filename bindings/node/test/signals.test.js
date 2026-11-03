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
