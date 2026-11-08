local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Box Container Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local desc = bui.Node("label")
desc:set("text", "Box is a simple container with optional line separator")
root:add(desc)
desc:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local b1 = bui.Node("box")
b1:set("text", "Settings Group")
b1:set("factor", 0)
b1:set("padding", 6)
root:add(b1)
b1:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 6})

local cb1 = bui.Node("checkbox")
cb1:set("text", "Enable feature")
b1:add(cb1)

local cb2 = bui.Node("checkbox")
cb2:set("text", "Verbose logging")
b1:add(cb2)

local b2 = bui.Node("box")
b2:set("text", "Info")
b2:set("factor", 0)
b2:set("padding", 6)
root:add(b2)
b2:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 4})

local bl = bui.Node("label")
bl:set("text", "Box containers help organize related controls into visually grouped sections.")
b2:add(bl)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Box — Lua", 360, 300)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(360, 300, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
