<script>
  import { getContext } from 'svelte'
  let { title, children } = $props()
  const theme = getContext('theme')
</script>

<section id="card" data-theme={theme}>{@render title()}<div id="card-body">{@render children()}</div></section>
