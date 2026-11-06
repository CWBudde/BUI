<!-- bench/cells.js --svelte: the grid of the React benchmark in Svelte 5. Cell 0
     follows `t` (a rune), or BUI signals (`--signals`): bound to its node, they
     update it without Svelte running. -->
<script>
  let { cells, cellStyle, label, gridStyle, expose, style0 = null, label0 = null } = $props()
  let t = $state(0)
  expose((v) => { t = v })
  const rest = Array.from({ length: cells - 1 }, (_, i) => i + 1)
</script>

<div style={gridStyle}>
  {#if style0}<div style={style0}>{label0}</div>{:else}<div style={cellStyle(0, t)}>{label(0, t)}</div>{/if}
  {#each rest as i}<div style={cellStyle(i, 0)}>{label(i, 0)}</div>{/each}
</div>
