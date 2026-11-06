<!--
  A Blender widget: `tag` is one of @blender-ui/native's capitalized names
  (`NumberSlider` ...), which `document.createElement` makes a Blender widget.
  index.js gives each name its own component.
  SPDX-License-Identifier: GPL-2.0-or-later
-->
<script>
  let { tag, children, ...rest } = $props()
</script>

<svelte:element this={tag} {...rest}>{@render children?.()}</svelte:element>
