local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Curve Mapping Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local cv = bui.Node("curve")
cv:curve_set(0, {{0, 0}, {0.25, 0.4}, {0.5, 0.5}, {0.75, 0.6}, {1, 1}})
cv:curve_set(1, {{0, 0}, {0.3, 0.7}, {0.7, 0.3}, {1, 1}})
cv:curve_set(2, {{0, 0}, {1, 1}})
cv:curve_set(3, {{0, 0}, {0.5, 0.8}, {1, 1}})
root:add(cv)
cv:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local status = bui.Node("label")
status:set("text", "Curve has " .. cv:curve_count(0) .. " points in channel 0")
root:add(status)
status:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local val_title = bui.Node("label")
val_title:set("text", "Evaluate channel 0 at x=0.5: " .. string.format("%.2f", cv:curve_eval(0, 0.5)))
root:add(val_title)
val_title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Curve — Lua", 360, 300)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(360, 300, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
