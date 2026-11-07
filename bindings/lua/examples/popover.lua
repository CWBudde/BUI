local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Popover Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local status = bui.Node("label")
status:set("text", "Click the popover button below")
root:add(status)
status:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local pop = bui.Node("popover")
pop:set("text", "Click Me")
pop:set("icon", bui.icon_id("DOWNARROW_HLT"))
root:add(pop)
pop:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 6})

local body = bui.Node("column")
body:set("padding", 8)

local bl = bui.Node("label")
bl:set("text", "Popup Content")
body:add(bl)

local bb = bui.Node("button")
bb:set("text", "Action")
bb:on(bui.EV_CLICK, function()
  status:set("text", "Popover action triggered")
end)
body:add(bb)

pop:add(body)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Popover — Lua", 300, 250)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(300, 250, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
