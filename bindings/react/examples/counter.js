// A React counter in a native BUI window (no build step: React.createElement).
// With a JSX toolchain, write the same with JSX, e.g. `npx tsx counter.jsx`.
//   node examples/counter.js
'use strict'
const React = require('react')
const { render } = require('..')
const h = React.createElement

function Counter () {
  const [count, setCount] = React.useState(0)
  const [name, setName] = React.useState('')
  return h('main', { className: 'p-6 flex flex-col gap-3' },
    h('h1', { className: 'text-xl font-bold' }, 'BUI + React'),
    h('div', { className: 'flex items-center gap-3' },
      h('span', null, `Clicks: ${count}`),
      h('button', { onClick: () => setCount(c => c + 1) }, 'Add one')),
    h('input', { placeholder: 'Your name', value: name, onChange: e => setName(e.target.value) }),
    name && h('p', { className: 'text-[#9ab8e8]' }, `Hello, ${name}!`))
}

const { app, window } = render(h(Counter), { title: 'BUI + React', width: 420, height: 260 })
window.body.className = 'm-0 bg-[#2b2b2b] text-[#e6e6e6]'
if (process.env.BUI_EXAMPLE_SECONDS) setTimeout(() => app.quit(), Number(process.env.BUI_EXAMPLE_SECONDS) * 1000)
