// The task list of todo_dom.js written in Svelte 5 (@blender-ui/svelte):
// TodoSvelte.svelte, state in runes.
//   node --import ./bindings/svelte/register.js examples/site/js/todo_svelte.mjs
//   bun --preload ./bindings/svelte/register.js examples/site/js/todo_svelte.mjs
import { render } from '../../../bindings/svelte/index.js'
import Tasks from './TodoSvelte.svelte'

export function build (options = {}) {
  const r = render(Tasks, { title: 'Tasks (Svelte)', width: 440, height: 370, ...options })
  r.window.body.setAttribute('styled', '')
  return r
}

if (import.meta.main ?? process.argv[1]?.endsWith('todo_svelte.mjs')) build({ theme: process.env.BUI_THEME })
