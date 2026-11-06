<!-- BUI signals passed as values (no `$`): their nodes are bound to them. -->
<script>
  let { style, label, cls, onrender } = $props()
  onrender()
</script>

<div id="bound" style={style} class={cls} title={label}>{label}</div>
<p class="scoped">x</p>

<style>
  .scoped { color: red; }
</style>
