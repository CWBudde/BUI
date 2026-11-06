<!-- The task list of todo_dom.js and todo_react.js in Svelte 5 (@blender-ui/svelte). -->
<script>
  import { Button, Checkbox, ProgressBar, TextField } from '../../../bindings/svelte/index.js'
  import style from './todo_style.js'
  const { TASKS, TAG_COLORS, CSS } = style

  let tasks = $state(TASKS.map((t) => ({ ...t })))
  let filter = $state('All')
  let draft = $state('')
  const done = $derived(tasks.filter((t) => t.done).length)
  const pct = $derived(Math.round(100 * done / tasks.length))
  const shown = $derived(tasks.filter((t) => filter === 'All' || (filter === 'Done') === t.done))
</script>

<div>
  <svelte:element this={'style'}>{CSS}</svelte:element>
  <div class="head">
    <h1>Robot Arm · Sprint 4</h1>
    <span class="muted">{tasks.length - done} open · {done} done</span>
  </div>
  <div class="add">
    <TextField placeholder="Add a task…" value={draft} oninput={(e) => (draft = e.target.value)} />
    <Button onclick={() => draft && tasks.push({ text: draft, tag: 'Modeling', done: false })}>Add</Button>
  </div>
  <div class="filters">
    {#each ['All', 'Active', 'Done'] as f (f)}
      <span class={filter === f ? 'filter on' : 'filter'} onclick={() => (filter = f)}>{f}</span>
    {/each}
  </div>
  <div class="list">
    {#each shown as task (task.text)}
      <div class={task.done ? 'item done' : 'item'}>
        <Checkbox checked={task.done} onchange={() => (task.done = !task.done)} />
        <span class="text">{task.text}</span>
        <span class="tag" style:background-color={TAG_COLORS[task.tag]}>{task.tag}</span>
      </div>
    {/each}
  </div>
  <div class="foot">
    <ProgressBar class="bar" max="100" value={String(pct)} />
    <span class="muted">{pct}% complete</span>
  </div>
</div>
