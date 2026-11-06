<!-- BUI signals in a component: each is a Svelte store (`$n`). -->
<script>
  import { signal, computed } from '../../index.js'
  let { external, expose, onrender } = $props()
  const n = signal(0)
  const color = computed(() => (n.value % 2 ? '#00ff00' : '#0000ff'))
  expose(n)
  onrender()
</script>

<div style="margin: 0">
  <div id="box" style="width: 80px; height: 30px; background-color: {$color}" onclick={() => n.update((v) => v + 1)}>count {$n}</div>
  <span id="ext">ext {$external}</span>
</div>
