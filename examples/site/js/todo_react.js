// The task list of todo_dom.js written with React (@blender-ui/react), no
// build step (createElement); state with React's hooks.
//   node examples/site/js/todo_react.js         (or: bun ...)
'use strict'
const path = require('node:path')
const { render, createElement: h, useState, Button, Checkbox, ProgressBar, TextField } =
  require(path.join(__dirname, '../../../bindings/react'))
const { TASKS, TAG_COLORS, CSS } = require('./todo_style')

function Task ({ task, onToggle }) {
  return h('div', { className: task.done ? 'item done' : 'item' },
    h(Checkbox, { checked: task.done, onChange: onToggle }),
    h('span', { className: 'text' }, task.text),
    h('span', { className: 'tag', style: { backgroundColor: TAG_COLORS[task.tag] } }, task.tag))
}

function Tasks () {
  const [tasks, setTasks] = useState(TASKS)
  const [filter, setFilter] = useState('All')
  const [draft, setDraft] = useState('')
  const all = tasks.value ?? tasks
  const done = all.filter(t => t.done).length
  const pct = Math.round(100 * done / all.length)
  const shown = all.filter(t => filter.value === 'All' || (filter.value === 'Done') === t.done)
  return h('div', null,
    h('style', null, CSS),
    h('div', { className: 'head' },
      h('h1', null, 'Robot Arm · Sprint 4'),
      h('span', { className: 'muted' }, `${all.length - done} open · ${done} done`)),
    h('div', { className: 'add' },
      h(TextField, { placeholder: 'Add a task…', value: draft, onChange: e => setDraft(e.target.value) }),
      h(Button, { onClick: () => draft.value && setTasks(ts => [...ts, { text: draft.value, tag: 'Modeling', done: false }]) }, 'Add')),
    h('div', { className: 'filters' }, ['All', 'Active', 'Done'].map(f =>
      h('span', { key: f, className: filter.value === f ? 'filter on' : 'filter', onClick: () => setFilter(f) }, f))),
    h('div', { className: 'list' }, shown.map(t =>
      h(Task, { key: t.text, task: t, onToggle: () => setTasks(ts => ts.map(x => x === t ? { ...x, done: !x.done } : x)) }))),
    h('div', { className: 'foot' },
      h(ProgressBar, { className: 'bar', max: '100', value: String(pct) }),
      h('span', { className: 'muted' }, `${pct}% complete`)))
}

function build (options = {}) {
  const r = render(h(Tasks), { title: 'Tasks (React)', width: 440, height: 370, ...options })
  r.window.body.setAttribute('styled', '')
  return r
}

module.exports = { build }

if (require.main === module) build({ theme: process.env.BUI_THEME })
