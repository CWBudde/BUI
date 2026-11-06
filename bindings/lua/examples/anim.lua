-- Interactive animation demo using pump-driven frame loop
-- Shows animated progress, color cycling, and a moving indicator
local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 12)

local title = bui.Node("label")
title:set("text", "BUI Animation — Lua")
root:add(title)

-- Animated progress bar
local prog_label = bui.Node("label")
prog_label:set("text", "Progress:")
root:add(prog_label)

local prog = bui.Node("progress")
prog:set("value", 0.5)
root:add(prog)
prog:geom(bui.GEOM_PACK, {fill = 1})

-- Animated color swatch
local swatch_label = bui.Node("label")
swatch_label:set("text", "Color Cycle:")
root:add(swatch_label)

local swatch = bui.Node("swatch")
swatch:set("color", {0.28, 0.45, 0.70, 1.0})
root:add(swatch)
swatch:geom(bui.GEOM_PACK, {fill = 1})

-- Animated slider
local slider_label = bui.Node("label")
slider_label:set("text", "Wave:")
root:add(slider_label)

local slider = bui.Node("slider")
slider:set("min", 0)
slider:set("max", 100)
slider:set("value", 50)
slider:set("enabled", 0)
root:add(slider)
slider:geom(bui.GEOM_PACK, {fill = 1})

-- Frame counter
local frame_lbl = bui.Node("label")
frame_lbl:set("text", "Frame: 0")
root:add(frame_lbl)

-- Controls
local control_row = bui.Node("row")
control_row:set("padding", 6)

local running = true
local toggle_btn = bui.Node("toggle")
toggle_btn:set("text", "Running")
toggle_btn:set("value", 1)
toggle_btn:on(bui.EV_CHANGE, function()
  running = toggle_btn:get("value") > 0
end)
control_row:add(toggle_btn)

local reset_btn = bui.Node("button")
reset_btn:set("text", "Reset")
reset_btn:on(bui.EV_CLICK, function()
  prog:set("value", 0.5)
  swatch:set("color", {0.28, 0.45, 0.70, 1.0})
  slider:set("value", 50)
end)
control_row:add(reset_btn)

root:add(control_row)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Animation — Lua", 360, 320)

if not win then
  print("No window system available")
  return
end

win:show()

-- Custom animation loop using pump instead of bui.run()
local frame = 0
while win:is_open() do
  if running then
    frame = frame + 1
    local t = frame * 0.03

    -- Animate progress bar (sine wave)
    prog:set("value", math.sin(t) * 0.45 + 0.5)

    -- Animate color swatch (color cycling)
    local r = math.sin(t) * 0.5 + 0.5
    local g = math.sin(t + 2.094) * 0.5 + 0.5
    local b = math.sin(t + 4.188) * 0.5 + 0.5
    swatch:set("color", {r, g, b, 1.0})

    -- Animate slider (triangle wave)
    local phase = (t * 30) % 100
    slider:set("value", phase)

    frame_lbl:set("text", "Frame: " .. frame)
  end

  bui.pump()
end

print("Animator closed (" .. frame .. " frames)")
