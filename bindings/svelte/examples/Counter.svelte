<!-- The counter of @blender-ui/react's examples, in Svelte 5. -->
<script>
  import { Button } from '@blender-ui/svelte'
  let count = $state(0)
  let name = $state('')
</script>

<main class="p-6 flex flex-col gap-3">
  <h1 class="text-xl font-bold">BUI + Svelte</h1>
  <div class="flex items-center gap-3">
    <span>Clicks: {count}</span>
    <Button onclick={() => count++}>Add one</Button>
  </div>
  <input placeholder="Your name" bind:value={name} />
  {#if name}<p class="text-[#9ab8e8]">Hello, {name}!</p>{/if}
</main>
