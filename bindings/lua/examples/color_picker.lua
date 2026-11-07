-- Color picker demo: swatches, HSV wheel, eyedropper
-- SPDX-License-Identifier: GPL-2.0-or-later

local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)
local ctx

-- Status / color value display
local color_lbl = bui.Node("label")
color_lbl:set("text", "Click a swatch to open the color picker")
root:add(color_lbl)
color_lbl:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Color swatch (click to open HSV picker)
local sw = bui.Node("swatch")
sw:set("color", {0.28, 0.45, 0.70, 1.0})
sw:on(bui.EV_CHANGE, function()
  local r, g, b, a = sw:get_color("color")
  color_lbl:set("text", string.format("RGBA: %.2f, %.2f, %.2f, %.2f", r, g, b, a))
end)
root:add(sw)
sw:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 4})

-- Second swatch (different color)
local sw2_label = bui.Node("label")
sw2_label:set("text", "Pick another color:")
root:add(sw2_label)
sw2_label:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local sw2 = bui.Node("swatch")
sw2:set("color", {0.70, 0.28, 0.45, 1.0})
sw2:on(bui.EV_CHANGE, function()
  local r, g, b, a = sw2:get_color("color")
  color_lbl:set("text", string.format("Swatch 2: %.2f, %.2f, %.2f, %.2f", r, g, b, a))
end)
root:add(sw2)
sw2:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 4})

-- Color ramp demo
local ramp_label = bui.Node("label")
ramp_label:set("text", "Color Ramp:")
root:add(ramp_label)
ramp_label:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local ramp = bui.Node("ramp")
ramp:ramp_set({
  {0.0, 0.0, 0.0, 1.0, 1.0},
  {0.33, 1.0, 0.0, 0.0, 1.0},
  {0.66, 0.0, 1.0, 0.0, 1.0},
  {1.0, 1.0, 0.0, 0.0, 1.0},
})
ramp:on(bui.EV_CHANGE, function()
  local active = ramp:ramp_active()
  local pos, r, g, b, a = ramp:ramp_get(active)
  color_lbl:set("text", string.format("Ramp stop %d: pos=%.2f RGBA=%.2f,%.2f,%.2f,%.2f",
                                       active, pos, r, g, b, a))
end)
root:add(ramp)
ramp:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 4})

-- Evaluate ramp at a point
local eval_btn = bui.Node("button")
eval_btn:set("text", "Evaluate Ramp at 0.5")
eval_btn:set("icon", bui.icon_id("PLAY"))
eval_btn:on(bui.EV_CLICK, function()
  local r, g, b, a = ramp:ramp_eval(0.5)
  color_lbl:set("text", string.format("Ramp at 0.5: %.2f, %.2f, %.2f, %.2f", r, g, b, a))
end)
root:add(eval_btn)
eval_btn:geom(bui.GEOM_PACK, {side = 0, fill = 1})

ctx = bui.Context(root)

local win = bui.Window(ctx, "BUI Color Picker — Lua", 380, 340)

if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(380, 340, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
