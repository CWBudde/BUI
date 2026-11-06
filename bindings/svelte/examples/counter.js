// A Svelte counter in a native BUI window.
//   node --import @blender-ui/svelte/register examples/counter.js
//   bun --preload @blender-ui/svelte/register examples/counter.js
//   npx bui build examples/counter.js     (one small executable)
import { render } from '@blender-ui/svelte'
import Counter from './Counter.svelte'

const { app, window } = render(Counter, { title: 'BUI + Svelte', width: 420, height: 260 })
window.body.setAttribute('styled', '')
window.body.className = 'm-0 bg-[#2b2b2b] text-[#e6e6e6]'
if (process.env.BUI_EXAMPLE_SECONDS) setTimeout(() => app.quit(), Number(process.env.BUI_EXAMPLE_SECONDS) * 1000)
