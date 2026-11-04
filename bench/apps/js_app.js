// Benchmark app, JavaScript (React on Node): the shared small UI. Prints
// "ready" once the first frame is on screen, then idles until the harness
// stops it.
'use strict'
const { render, createElement: h } = require('@blender-ui/react')

function App () {
  return h('main', { styled: '', className: 'p-3 flex flex-col gap-2' },
    h('span', null, 'Settings'),
    h('input', { defaultValue: 'Cube' }),
    h('label', null, h('input', { type: 'checkbox', defaultChecked: true }), ' Shadows'),
    h('input', { type: 'range', min: 0, max: 10 }),
    h('button', null, 'Apply'))
}

const { window } = render(h(App), { title: 'bench', width: 360, height: 260 })
window.requestAnimationFrame(() => console.log('ready'))
