<script>
  let { onrender = () => {} } = $props()
  let n = $state(0)
  onrender()
</script>

<div class="flex flex-col gap-2" style="margin: 0">
  <div id="btn" style:width="80px" style:height="30px" style:background-color={n ? '#00ff00' : '#0000ff'}
    onclick={() => n++}>count {n}</div>
  {#if n > 0}<div class="w-[50px] h-[10px] bg-[#ff0000]"></div>{/if}
</div>
