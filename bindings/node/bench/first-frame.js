// A small React app that quits once its first frame is on screen, printing
// how long that took (ms since the runtime started) and the resident memory
// then (first-frame-report.js). Used by bench/startup.js.
'use strict'
const { render, createElement: h, useState } = require('@blender-ui/react')

function App () {
  const [count, setCount] = useState(0)
  return h('main', { className: 'p-6 flex flex-col gap-3 bg-[#2b2b2b] text-[#e6e6e6]' },
    h('h1', { className: 'text-xl font-bold' }, 'Hello'),
    h('div', { className: 'flex items-center gap-3' },
      h('span', null, 'Clicks: ', count),
      h('button', { onClick: () => setCount(c => c + 1) }, 'Add one')))
}

require('./first-frame-report')(render(h(App), { title: 'first frame', width: 360, height: 200, gpu: true }))
