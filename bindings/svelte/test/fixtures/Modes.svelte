<!-- Text and elements taking turns, a keyed list, and an input whose type changes. -->
<script>
  let { expose } = $props()
  let mode = $state('text')
  let items = $state([1, 2, 3])
  expose({ mode: (m) => { mode = m }, items: (list) => { items = list } })
</script>

<div id="box">{#if mode === 'text'}hello{:else if mode === 'el'}<span>inner</span>{/if}<input id="in" type={mode === 'el' ? 'checkbox' : 'text'} onchange={() => {}} /></div>
<ul id="list">{#each items as item (item)}<li>{item}</li>{/each}</ul>
