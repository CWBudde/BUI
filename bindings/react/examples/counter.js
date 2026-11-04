// A React counter in a native BUI window, without a build step (createElement).
//   node examples/counter.js      (or: bun examples/counter.js)
// The hooks are React's, by React's names, with signals for state: `count`
// below updates its text node without re-rendering Counter; `name.value`,
// read while rendering, re-renders it when the name changes.
'use strict'
const { render, createElement: h, useState } = require('..')

function Counter () {
  const [count, setCount] = useState(0)
  const [name, setName] = useState('')
  return h('main', { className: 'p-6 flex flex-col gap-3' },
    h('h1', { className: 'text-xl font-bold' }, 'BUI + React'),
    h('div', { className: 'flex items-center gap-3' },
      h('span', null, 'Clicks: ', count),
      h('button', { onClick: () => setCount(c => c + 1) }, 'Add one')),
    h('input', { placeholder: 'Your name', value: name, onChange: e => setName(e.target.value) }),
    name.value && h('p', { className: 'text-[#9ab8e8]' }, `Hello, ${name}!`))
}

const { app, window } = render(h(Counter), { title: 'BUI + React', width: 420, height: 260 })
window.body.setAttribute('styled', '')
window.body.className = 'm-0 bg-[#2b2b2b] text-[#e6e6e6]'
if (process.env.BUI_EXAMPLE_SECONDS) setTimeout(() => app.quit(), Number(process.env.BUI_EXAMPLE_SECONDS) * 1000)
