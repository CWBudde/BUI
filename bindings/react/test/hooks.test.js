// Each signal twin beside its React original: same inputs, same observable
// results. And coverage: every hook React exports is re-exported, and has a
// twin or a reason in NO_TWIN (so a React upgrade that adds one fails here).
'use strict'
const test = require('node:test')
const assert = require('node:assert')
const React = require('react')
const bui = require('..')
const { App } = require('@blender-ui/native')
const h = React.createElement

const tick = (ms = 30) => new Promise(r => setTimeout(r, ms))
let app
test.before(() => { app = new App({ headless: true }) })
test.after(() => app.quit())

function mount (element, strict = false) {
  const r = bui.render(strict ? h(React.StrictMode, null, element) : element,
    { app, headless: true, width: 120, height: 80 })
  return r
}
function unmount (r) {
  r.root.unmount()
  r.window.close()
}

test('coverage: every React hook is exported by its name: a signal version, or React\'s own with a reason', () => {
  const hooks = Object.keys(React).filter(k => /^use([A-Z]|$)/.test(k))
  assert.ok(hooks.length >= 18, 'found React\'s hooks')
  for (const k of hooks) {
    assert.strictEqual(typeof bui[k], 'function', `${k} is exported`)
    if (k in bui.NO_TWIN) assert.strictEqual(bui[k], React[k], `${k} is React's own`)
    else {
      assert.notStrictEqual(bui[k], React[k],
        `React ${React.version} has ${k}: add its signal version to hooks.js, or a reason to NO_TWIN`)
    }
  }
  const extra = ['useSignal', 'useComputed', 'useSignalValue']
  for (const k of Object.keys(bui)) {
    if (/^use([A-Z]|$)/.test(k) && !extra.includes(k)) assert.ok(k in React, `${k} is a React hook`)
  }
})

test('render tracking: a render that reads a signal re-renders; one that passes it on does not', async () => {
  const c = bui.createElement
  let readerRenders = 0, passerRenders = 0, set
  function Reader ({ n }) {
    readerRenders++
    return c('span', null, n.value > 1 ? 'big' : 'small', ' ', n + 1)
  }
  function Passer () {
    passerRenders++
    const [n, setN] = bui.useState(1)
    set = setN
    return c('div', null, n, c(Reader, { n }))
  }
  const r = mount(c(Passer))
  await tick()
  const text = () => [...r.window._nodes.values()].filter(x => x.nodeType === 3).map(x => x.data).join('|')
  assert.strictEqual(text(), '1|small| |2')
  set(2)
  await tick()
  assert.strictEqual(text(), '2|big| |3', 'Reader re-rendered with the new value')
  assert.strictEqual(passerRenders, 1, 'Passer only passed the signal: no re-render')
  assert.strictEqual(readerRenders, 2)
  unmount(r)
  await tick()
  set(3)
  await tick()
  assert.strictEqual(readerRenders, 2, 'unmounted: no longer subscribed')
})

test('render tracking under StrictMode, memo and forwardRef; refs read in render do not subscribe', async () => {
  const c = bui.createElement
  const s = bui.signal(0)
  let renders = 0
  const Inner = React.memo(React.forwardRef(function Inner (props, ref) {
    renders++
    const own = bui.useRef(0)
    own.current++ // a render-time ref write must not loop
    return c('span', { ref }, `v${s.value}`)
  }))
  const r = mount(c(Inner, {}), true)
  await tick()
  const before = renders
  s.value = 1
  await tick()
  assert.ok(renders > before, 'still subscribed after StrictMode\'s unmount and remount')
  const after = renders
  await tick(60)
  assert.strictEqual(renders, after, 'settled: no render loop')
  assert.ok([...r.window._nodes.values()].some(x => x.data === 'v1'))
  unmount(r)
})

test('nodes of a render React throws away are freed', async () => {
  const c = bui.createElement
  let resolve
  const promise = new Promise(r => { resolve = r })
  let suspend = true
  function Slow () {
    if (suspend) throw promise
    return c('b', null, 'done')
  }
  function App () {
    return c(React.Suspense, { fallback: c('i', null, 'wait') }, c('section', null, c('p', null, 'a'), c(Slow)))
  }
  const r = mount(c(App))
  await tick()
  const live = () => r.window._nodes.size
  const n0 = live()
  suspend = false
  resolve()
  await tick(500) // React 19 shows a fallback for at least 300 ms
  assert.ok([...r.window._nodes.values()].some(x => x.data === 'done'))
  const attached = (x) => { while (x.parentNode) x = x.parentNode; return x === r.window.body }
  const orphans = [...r.window._nodes.values()].filter(x => x.id !== 0 && !attached(x))
  assert.deepStrictEqual(orphans.map(x => x.localName ?? x.data), [], `no detached nodes left (had ${n0})`)
  unmount(r)
})

test('signal useState matches useState (values, updaters, bail-out) and skips re-renders', async () => {
  let renders = 0, rv, rset, sv, sset
  function C () {
    renders++
    ;[rv, rset] = React.useState(() => 1)
    ;[sv, sset] = bui.useState(() => 1)
    return null
  }
  const r = mount(h(C))
  await tick()
  assert.strictEqual(sv.value, rv)
  const steps = [5, (p) => p * 2, 10, 10, (p) => p + 1]
  for (const step of steps) {
    rset(step)
    sset(step)
    await tick()
    assert.strictEqual(sv.value, rv, `after ${step}`)
  }
  const before = renders
  sset(99)
  await tick()
  assert.strictEqual(renders, before, 'a signal write re-renders nothing')
  assert.strictEqual(sv.value, 99)
  assert.strictEqual(rset.length === undefined, false)
  unmount(r)
})

test('signal useReducer matches useReducer (init, actions, same-state bail-out)', async () => {
  const reducer = (s, a) => a.type === 'add' ? { n: s.n + a.by } : a.type === 'noop' ? s : { n: 0 }
  const init = (arg) => ({ n: arg * 10 })
  let rv, rd, sv, sd
  function C () {
    ;[rv, rd] = React.useReducer(reducer, 2, init)
    ;[sv, sd] = bui.useReducer(reducer, 2, init)
    return null
  }
  const r = mount(h(C))
  await tick()
  assert.deepStrictEqual(sv.value, rv)
  for (const a of [{ type: 'add', by: 3 }, { type: 'noop' }, { type: 'reset' }, { type: 'add', by: 7 }]) {
    const prevSignal = sv.peek()
    rd(a)
    sd(a)
    await tick()
    assert.deepStrictEqual(sv.value, rv)
    if (a.type === 'noop') assert.strictEqual(sv.peek(), prevSignal, 'same state object kept')
  }
  unmount(r)
})

test('signal useMemo matches useMemo across deps, and follows signals', async () => {
  const src = bui.signal(1)
  let rv, m, rRuns = 0, sRuns = 0, setX
  function C () {
    const [x, sx] = React.useState(1)
    setX = sx
    rv = React.useMemo(() => { rRuns++; return x * 100 }, [x])
    m = bui.useMemo(() => { sRuns++; return x * 100 + src.value }, [x])
    return h('div', null, m)
  }
  const r = mount(h(C))
  await tick()
  assert.strictEqual(m.value, rv + 1)
  setX(1) // same deps
  await tick()
  setX(2)
  await tick()
  assert.strictEqual(m.value, rv + 1)
  assert.strictEqual(rRuns, 2)
  src.value = 5 // only the signal memo depends on it
  assert.strictEqual(m.value, rv + 5)
  const text = [...r.window._nodes.values()].find(n => n.nodeType === 3 && n.data === '205')
  assert.ok(text, 'the bound text node follows the memo')
  assert.ok(sRuns <= 4, `signal memo recomputed only when needed (${sRuns})`)
  unmount(r)
})

test('signal useCallback: identity follows deps like useCallback; calls are batched and untracked', async () => {
  const ids = { r: [], s: [] }
  let setDep
  const a = bui.signal(0), b = bui.signal(0)
  let cb
  function C () {
    const [dep, sd] = React.useState(0)
    const [, force] = React.useState(0)
    setDep = (v) => { sd(v); force(n => n + 1) }
    ids.r.push(React.useCallback(() => dep, [dep]))
    cb = bui.useCallback(() => { a.value++; b.value++; return dep }, [dep])
    ids.s.push(cb)
    return null
  }
  const r = mount(h(C))
  await tick()
  setDep(0)
  await tick()
  setDep(1)
  await tick()
  const same = (list) => list.slice(1).map((f, i) => f === list[i])
  assert.deepStrictEqual(same(ids.s), same(ids.r))
  let runs = 0
  const stop = bui.effect(() => { a.value; b.value; runs++; if (runs === 1) cb() })
  assert.strictEqual(runs, 2, 'two writes, one re-run; and calling cb inside did not subscribe to more')
  stop()
  unmount(r)
})

test('signal useRef matches useRef: same .current, attached to the element, observable', async () => {
  let rref, sref
  const seen = []
  function C () {
    rref = React.useRef(null)
    sref = bui.useRef(null)
    bui.useEffect(() => { seen.push(sref.current ? sref.current.localName : null) }, [])
    return h('div', null, h('section', { ref: rref }), h('section', { ref: sref }))
  }
  const r = mount(h(C))
  await tick()
  assert.strictEqual(rref.current.localName, 'section')
  assert.strictEqual(sref.current.localName, 'section')
  assert.deepStrictEqual(seen, ['section'], 'mounted before the effect ran (as with useRef)')
  unmount(r)
  await tick()
  assert.strictEqual(rref.current, null)
  assert.strictEqual(sref.current, null)
})

for (const strict of [false, true]) {
  test(`effect twins run when React's do, deps and cleanups alike${strict ? ' (StrictMode)' : ''}`, async () => {
    const log = { r: [], s: [] }
    let setX
    function C () {
      const [x, sx] = React.useState(0)
      setX = sx
      for (const [kind, rh, sh] of [['insertion', React.useInsertionEffect, bui.useInsertionEffect],
        ['layout', React.useLayoutEffect, bui.useLayoutEffect], ['effect', React.useEffect, bui.useEffect]]) {
        for (const [d, deps] of [['always', undefined], ['once', []], ['x', [x]]]) {
          rh(() => { log.r.push(`${kind} ${d} ${x}`); return () => log.r.push(`${kind} ${d} cleanup ${x}`) }, deps)
          sh(() => { log.s.push(`${kind} ${d} ${x}`); return () => log.s.push(`${kind} ${d} cleanup ${x}`) }, deps)
        }
      }
      return null
    }
    const r = mount(h(C), strict)
    await tick()
    setX(1)
    await tick()
    setX(1)
    await tick()
    unmount(r)
    await tick()
    assert.ok(log.r.length > 10)
    assert.deepStrictEqual(log.s, log.r)
  })
}

test('signal useEffect also re-runs when a signal it read changes (with cleanup)', async () => {
  const s = bui.signal('a')
  const log = []
  function C () {
    bui.useEffect(() => { const v = s.value; log.push(v); return () => log.push(`-${v}`) }, [])
    return null
  }
  const r = mount(h(C))
  await tick()
  s.value = 'b'
  unmount(r)
  await tick()
  assert.deepStrictEqual(log, ['a', '-a', 'b', '-b'])
})

test('signal useSyncExternalStore matches useSyncExternalStore', async () => {
  let value = 1
  const listeners = new Set()
  const store = {
    subscribe: (l) => { listeners.add(l); return () => listeners.delete(l) },
    get: () => value,
    set: (v) => { value = v; for (const l of listeners) l() },
  }
  let rv, sv
  function C () {
    rv = React.useSyncExternalStore(store.subscribe, store.get)
    sv = bui.useSyncExternalStore(store.subscribe, store.get)
    return null
  }
  const r = mount(h(C))
  await tick()
  for (const v of [2, 2, 7]) {
    store.set(v)
    await tick()
    assert.strictEqual(sv.value, rv)
  }
  unmount(r)
  await tick()
  assert.strictEqual(listeners.size, 0, 'both unsubscribed')
})

test('signal useTransition matches useTransition: isPending through an async action', async () => {
  const seen = { r: [], s: [] }
  let rStart, sStart, sPending
  function C () {
    const [rp, rs] = React.useTransition()
    seen.r.push(rp)
    rStart = rs
    ;[sPending, sStart] = bui.useTransition()
    return null
  }
  const r = mount(h(C))
  await tick()
  const stop = bui.effect(() => { seen.s.push(sPending.value) })
  const action = () => new Promise(resolve => setTimeout(resolve, 50))
  rStart(action)
  sStart(action)
  await tick(10)
  assert.strictEqual(seen.s.at(-1), true)
  assert.strictEqual(seen.r.at(-1), true)
  await tick(100)
  assert.strictEqual(seen.s.at(-1), false)
  assert.strictEqual(seen.r.at(-1), false)
  const dedupe = (l) => l.filter((v, i) => i === 0 || v !== l[i - 1])
  assert.deepStrictEqual(dedupe(seen.s), dedupe(seen.r))
  stop()
  unmount(r)
})

test('signal transitions run their effects after urgent ones', async () => {
  const a = bui.signal(0), b = bui.signal(0)
  const order = []
  const s1 = bui.effect(() => { if (a.value) order.push('transition') })
  const s2 = bui.effect(() => { if (b.value) order.push('urgent') })
  const done = bui.startTransition(() => { a.value = 1 })
  assert.deepStrictEqual(order, [], 'deferred')
  b.value = 1
  assert.deepStrictEqual(order, ['urgent'])
  await done
  assert.deepStrictEqual(order, ['urgent', 'transition'])
  s1()
  s2()
})

test('signal useDeferredValue matches useDeferredValue: initialValue first, then the value', async () => {
  const seen = { r: [], s: [] }
  let setV, out
  function C () {
    const [v, sv] = React.useState('a')
    setV = sv
    seen.r.push(React.useDeferredValue(v, 'init'))
    out = bui.useDeferredValue(v, 'init')
    bui.useLayoutEffect(() => { seen.s.push(out.value) }, [])
    return null
  }
  const r = mount(h(C))
  await tick()
  setV('b')
  await tick()
  const dedupe = (l) => l.filter((v, i) => i === 0 || v !== l[i - 1])
  assert.deepStrictEqual(dedupe(seen.s), dedupe(seen.r))
  assert.deepStrictEqual(dedupe(seen.s), ['init', 'a', 'b'])
  // a signal source: follows without re-rendering
  const src = bui.signal(1)
  let def
  function D () { def = bui.useDeferredValue(src); return null }
  const r2 = mount(h(D))
  await tick()
  src.value = 2
  assert.strictEqual(def.peek(), 1, 'deferred: not yet')
  await tick()
  assert.strictEqual(def.peek(), 2)
  unmount(r)
  unmount(r2)
})

test('signal useActionState matches useActionState: queued, in order, with isPending', async () => {
  const action = async (prev, n) => { await tick(10); return prev + n }
  let rs, rd, rp, ss, sd, sp
  function C () {
    ;[rs, rd, rp] = React.useActionState(action, 0)
    ;[ss, sd, sp] = bui.useActionState(action, 0)
    return null
  }
  const r = mount(h(C))
  await tick()
  const pend = []
  const stop = bui.effect(() => { pend.push(sp.value) })
  React.startTransition(() => { rd(1); rd(2); rd(3) })
  sd(1); sd(2); sd(3)
  await tick(5)
  assert.strictEqual(sp.value, true)
  assert.strictEqual(rp, true)
  await tick(150)
  assert.strictEqual(ss.value, rs)
  assert.strictEqual(ss.value, 6)
  assert.strictEqual(rp, false)
  assert.deepStrictEqual(pend, [false, true, false], 'pending once across the queue')
  stop()
  unmount(r)
})

test('signal useActionState and useTransition send errors to the error boundary, as React does', async () => {
  class Boundary extends React.Component {
    constructor (p) { super(p); this.state = { error: null } }
    static getDerivedStateFromError (error) { return { error } }
    render () { return this.state.error ? h('span', null, `caught ${this.state.error.message}`) : this.props.children }
  }
  const caught = {}
  const fail = async () => { await tick(5); throw new Error('boom') }
  const cases = {
    actionState: (c) => { const [, d] = React.useActionState(fail, 0); c.go = () => React.startTransition(() => d()) },
    signalActionState: (c) => { const [, d] = bui.useActionState(fail, 0); c.go = () => d() },
    transition: (c) => { const [, start] = React.useTransition(); c.go = () => start(fail) },
    signalTransition: (c) => { const [, start] = bui.useTransition(); c.go = () => start(fail) },
    signalActionStateSync: (c) => {
      const [, d] = bui.useActionState(() => { throw new Error('boom') }, 0)
      c.go = () => d()
    },
  }
  for (const [kind, use] of Object.entries(cases)) {
    const c = {}
    function C () { use(c); return null }
    const r = mount(h(Boundary, null, h(C)))
    await tick()
    c.go()
    await tick(80)
    caught[kind] = [...r.window._nodes.values()].some(n => n.nodeType === 3 && n.data === 'caught boom')
    unmount(r)
  }
  assert.deepStrictEqual(caught, { actionState: true, signalActionState: true, transition: true, signalTransition: true, signalActionStateSync: true })
})

test('signal useOptimistic matches useOptimistic during and after an action', async () => {
  let rOpt, rAdd, rSet, sOpt, sAdd, sState, start
  function C () {
    const [rs, setRs] = React.useState(['a'])
    rSet = setRs
    ;[rOpt, rAdd] = React.useOptimistic(rs, (cur, item) => [...cur, item + '?'])
    sState = bui.useSignal(['a'])
    ;[sOpt, sAdd] = bui.useOptimistic(sState, (cur, item) => [...cur, item + '?'])
    ;[, start] = bui.useTransition()
    return null
  }
  const r = mount(h(C))
  await tick()
  start(async () => {
    rAdd('b')
    sAdd('b')
    await tick(40)
    React.startTransition(() => rSet(l => [...l, 'b']))
    sState.value = [...sState.peek(), 'b']
  })
  await tick(15)
  assert.deepStrictEqual(sOpt.value, ['a', 'b?'])
  assert.deepStrictEqual(rOpt, ['a', 'b?'])
  await tick(100)
  assert.deepStrictEqual(sOpt.value, ['a', 'b'])
  assert.deepStrictEqual(rOpt, ['a', 'b'])
  unmount(r)
})

test('signal useEffectEvent matches useEffectEvent: effects call the latest render\'s callback', async () => {
  const seen = { r: [], s: [] }
  let setX, fire
  function C () {
    const [x, sx] = React.useState(0)
    setX = sx
    const r = React.useEffectEvent(() => seen.r.push(x))
    const s = bui.useEffectEvent(() => seen.s.push(x))
    React.useEffect(() => { fire = () => { r(); s() } }, []) // captured once, on mount
    return null
  }
  const r = mount(h(C))
  await tick()
  fire()
  setX(5)
  await tick()
  fire()
  assert.deepStrictEqual(seen.s, seen.r)
  assert.deepStrictEqual(seen.s, [0, 5])
  unmount(r)
})
