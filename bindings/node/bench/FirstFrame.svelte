<!-- bench/first-frame.js's React app, in Svelte 5 (bench/startup.js --svelte). -->
<script>
  let count = $state(0)
</script>

<main class="p-6 flex flex-col gap-3 bg-[#2b2b2b] text-[#e6e6e6]">
  <h1 class="text-xl font-bold">Hello</h1>
  <div class="flex items-center gap-3">
    <span>Clicks: {count}</span>
    <button onclick={() => count++}>Add one</button>
  </div>
</main>
