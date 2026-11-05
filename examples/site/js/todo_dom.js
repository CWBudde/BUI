// A task list with @blender-ui/native's DOM-like API (createElement,
// appendChild, className, addEventListener), in Blender's theme.
//   node examples/site/js/todo_dom.js          (or: bun ...)
// The same app with React: todo_react.js.
'use strict'
const path = require('node:path')
const { App } = require(path.join(__dirname, '../../../bindings/node'))
const { TASKS, TAG_COLORS, CSS } = require('./todo_style')

function build (app, size = { width: 440, height: 370 }) {
  const win = app.window({ title: 'Tasks', ...size })
  const doc = win
  const el = (tag, cls, text) => {
    const e = doc.createElement(tag)
    if (cls) e.className = cls
    if (text !== undefined) e.appendChild(doc.createTextNode(String(text)))
    return e
  }
  win.body.setAttribute('styled', '')
  const style = el('style', null, CSS)
  win.body.appendChild(style)

  const head = el('div', 'head')
  head.appendChild(el('h1', null, 'Robot Arm · Sprint 4'))
  const count = el('span', 'muted')
  head.appendChild(count)
  win.body.appendChild(head)

  const add = el('div', 'add')
  const input = doc.createElement('TextField')
  input.setAttribute('placeholder', 'Add a task…')
  add.appendChild(input)
  const addBtn = el('Button', null, 'Add')
  add.appendChild(addBtn)
  win.body.appendChild(add)

  const filters = el('div', 'filters')
  for (const [i, f] of ['All', 'Active', 'Done'].entries()) filters.appendChild(el('span', i === 0 ? 'filter on' : 'filter', f))
  win.body.appendChild(filters)

  const list = el('div', 'list')
  win.body.appendChild(list)
  const foot = el('div', 'foot')
  const progress = doc.createElement('ProgressBar')
  progress.className = 'bar'
  progress.setAttribute('max', '100')
  foot.appendChild(progress)
  const pct = el('span', 'muted')
  foot.appendChild(pct)
  win.body.appendChild(foot)

  const tasks = TASKS.map(t => ({ ...t }))
  const refresh = () => {
    for (const c of [...list.childNodes]) { list.removeChild(c); c.destroy() }
    for (const t of tasks) {
      const row = el('div', t.done ? 'item done' : 'item')
      const box = doc.createElement('Checkbox')
      if (t.done) box.setAttribute('checked', '')
      box.addEventListener('change', e => { t.done = e.target.checked; refresh() })
      row.appendChild(box)
      row.appendChild(el('span', 'text', t.text))
      const tag = el('span', 'tag', t.tag)
      tag.style = { backgroundColor: TAG_COLORS[t.tag] }
      row.appendChild(tag)
      list.appendChild(row)
    }
    const done = tasks.filter(t => t.done).length
    count.textContent = `${tasks.length - done} open · ${done} done`
    progress.setAttribute('value', String(Math.round(100 * done / tasks.length)))
    pct.textContent = `${Math.round(100 * done / tasks.length)}% complete`
  }
  addBtn.addEventListener('click', () => {
    if (input.value) { tasks.push({ text: input.value, tag: 'Modeling', done: false }); refresh() }
  })
  refresh()
  return win
}

module.exports = { build }

if (require.main === module) {
  const app = new App({ theme: process.env.BUI_THEME })
  build(app)
}
