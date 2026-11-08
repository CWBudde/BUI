local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Enum Row Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local desc = bui.Node("label")
desc:set("text", "Enum shows all options as inline buttons")
root:add(desc)
desc:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local en = bui.Node("enum")
en:set_strs("items", {"Low", "Medium", "High", "Critical"})
en:set("value", 1)
root:add(en)
en:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 6})

local status = bui.Node("label")
status:set("text", "Selected: Medium")
root:add(status)
status:geom(bui.GEOM_PACK, {side = 0, fill = 1})

en:on(bui.EV_CHANGE, function()
  local idx = en:get("value")
  local items = en:get_strs("items")
  status:set("text", "Selected: " .. items[idx + 1])
end)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Enum — Lua", 360, 180)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(360, 180, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
