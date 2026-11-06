<!-- {#await}, {#key}, class objects, bind:group and a scoped <style>. -->
<script>
  let { promise, expose } = $props()
  let version = $state(1)
  let active = $state(false)
  let flavor = $state('a')
  expose({ bump: () => version++, toggle: () => (active = !active) })
</script>

<div id="await">{#await promise}loading{:then value}got {value}{/await}</div>
{#key version}<span id="key">v{version}</span>{/key}
<span id="cls" class={{ on: active, base: true }}>x</span>
<input id="fa" type="radio" value="a" bind:group={flavor} />
<input id="fb" type="radio" value="b" bind:group={flavor} />
<p id="flavor">{flavor}</p>
<div class="red"></div>

<style>
  .red { width: 40px; height: 20px; background: #ff0000; }
</style>
