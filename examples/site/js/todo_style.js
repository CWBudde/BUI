// The task list's data and stylesheet, shared by todo_dom.js and todo_react.js.
// Colours are translucent over the `styled` background (Blender's theme), so
// the page follows BUI_THEME=dark|light.
'use strict'

const TASKS = [
  { text: 'Retopologize the robot arm', tag: 'Modeling', done: true },
  { text: 'Bake normal maps (4K)', tag: 'Texturing', done: true },
  { text: 'Rig the gripper fingers', tag: 'Rigging', done: false },
  { text: 'Key the pick-and-place loop', tag: 'Animation', done: false },
  { text: 'Light the studio turntable', tag: 'Lighting', done: false },
  { text: 'Render 240 frames on the farm', tag: 'Rendering', done: false }
]

const TAG_COLORS = {
  Modeling: '#4772b3', Texturing: '#b36b47', Rigging: '#8a5cc2',
  Animation: '#3a9b7a', Lighting: '#c29a3c', Rendering: '#c2475f'
}

const CSS = `
body { margin: 0; padding: 16px 18px; min-height: 100vh; box-sizing: border-box; }
h1 { font-size: 20px; margin: 0; }
.head { display: flex; align-items: baseline; gap: 10px; margin-bottom: 12px; }
.muted { opacity: 0.6; font-size: 12px; }
.add { display: flex; gap: 6px; margin-bottom: 10px; }
.add input { flex: 1; }
.filters { display: flex; gap: 4px; margin-bottom: 8px; }
.filter { padding: 2px 12px; border-radius: 10px; font-size: 12px; background: rgba(128, 128, 128, 0.18); }
.filter.on { background: #4772b3; color: white; }
.list { border-radius: 8px; background: rgba(128, 128, 128, 0.12); padding: 4px 0; }
.item { display: flex; align-items: center; gap: 10px; height: 32px; padding: 0 10px; }
.item + .item { border-top: 1px solid rgba(128, 128, 128, 0.18); }
.item .text { flex: 1; }
.item.done .text { opacity: 0.5; text-decoration: line-through; }
.tag { font-size: 10px; padding: 1px 8px; border-radius: 8px; color: white; }
.foot { display: flex; align-items: center; gap: 10px; margin-top: 10px; }
.foot .bar { flex: 1; }
`

module.exports = { TASKS, TAG_COLORS, CSS }
