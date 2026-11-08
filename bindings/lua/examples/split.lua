local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Split Pane Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local hs = bui.Node("split")
hs:set("text", "Horizontal Split")
root:add(hs)
hs:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local left = bui.Node("panel")
left:set("text", "Left Panel")
hs:add(left)

local right = bui.Node("panel")
right:set("text", "Right Panel")
hs:add(right)

local vs = bui.Node("split")
vs:set("text", "Vertical Split")
vs:set("value", 1)
root:add(vs)
vs:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local top = bui.Node("panel")
top:set("text", "Top Panel")
vs:add(top)

local bottom = bui.Node("panel")
bottom:set("text", "Bottom Panel")
vs:add(bottom)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Split — Lua", 400, 300)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(400, 300, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
