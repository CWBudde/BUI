local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Color Ramp Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local rp = bui.Node("ramp")
rp:ramp_set({
  {0.0, 0.0, 0.0, 1.0, 1.0},
  {0.33, 0.2, 0.5, 0.8, 1.0},
  {0.66, 0.8, 0.2, 0.3, 1.0},
  {1.0, 1.0, 1.0, 0.2, 1.0},
})
rp:ramp_active(0)
root:add(rp)
rp:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 6})

local info = bui.Node("label")
info:set("text", "")

local eval_title = bui.Node("label")
eval_title:set("text", "Ramp evaluation:")
root:add(eval_title)
eval_title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local sw = bui.Node("swatch")
sw:set("color", {0, 0, 0, 1})
root:add(sw)
sw:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 4})

rp:on(bui.EV_CHANGE, function()
  local r, g, b, a = rp:ramp_eval(0.5)
  sw:set("color", {r, g, b, a})
  info:set("text", string.format("Mid-point: %.2f %.2f %.2f %.2f", r, g, b, a))
end)

root:add(info)
info:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Ramp — Lua", 360, 280)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(360, 280, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
