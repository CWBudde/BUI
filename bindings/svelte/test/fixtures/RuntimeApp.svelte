<script>
  let { onrender } = $props()
  let count = $state(0)
  let color = $state('#0000ff')
  onrender()
</script>

<main style="margin: 0">
  <div id="btn" style="width: 80px; height: 30px; background-color: {color}"
    onclick={() => { count++; color = '#00ff00' }}>count {count}</div>
  {#if count > 1}<p id="many">many</p>{/if}
</main>
