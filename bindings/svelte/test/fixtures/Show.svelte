<!-- A subtree hidden and kept, and a transition (it commits without animating). -->
<script>
  import { fade } from 'svelte/transition'
  let { expose } = $props()
  let show = $state(true)
  expose((v) => { show = v })
</script>

<div>
  <span id="kid" hidden={!show}>kept</span>
  {#if show}<b id="vt" transition:fade>shown</b>{/if}
</div>
