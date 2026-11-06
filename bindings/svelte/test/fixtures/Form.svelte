<!-- Bindings, Blender's widgets, snippets, context and lifecycle. -->
<script>
  import { setContext, onMount, onDestroy } from 'svelte'
  import { Button, NumberSlider } from '../../index.js'
  import Card from './Card.svelte'
  let { log } = $props()
  let name = $state('')
  let agreed = $state(false)
  let clicks = $state(0)
  setContext('theme', 'blender')
  onMount(() => log.push('mount'))
  onDestroy(() => log.push('destroy'))
</script>

<input id="name" bind:value={name} />
<input id="agree" type="checkbox" bind:checked={agreed} />
<p id="hello">{name ? `Hello, ${name}!` : ''}{agreed ? ' (agreed)' : ''}</p>
<Button id="apply" onclick={() => clicks++}>Apply {clicks}</Button>
<NumberSlider id="size" min={0} max={10} value={2.5} />
<Card>
  {#snippet title()}<b>Title</b>{/snippet}
  body text
</Card>
