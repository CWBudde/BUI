-- Form controls showcase: all major BUI widget types with event handlers
local bui = require("bui")

local root = bui.Node("scroll")
root:set("padding", 8)

local col = bui.Node("column")
col:set("padding", 4)
root:add(col)

local title = bui.Node("label")
title:set("text", "BUI Form Controls — Lua")
col:add(title)

local function section(name)
  local sep = bui.Node("separator")
  col:add(sep)
  local h = bui.Node("label")
  h:set("text", name)
  col:add(h)
end

local function row(widgets)
  local r = bui.Node("row")
  r:set("padding", 2)
  for _, w in ipairs(widgets) do r:add(w) end
  col:add(r)
  return r
end

-- Buttons
section("Buttons")
local log_label = bui.Node("label")
log_label:set("text", "Click a button →")
col:add(log_label)
local btn = bui.Node("button")
btn:set("text", "Click Me")
btn:on(bui.EV_CLICK, function()
  log_label:set("text", "Button clicked!")
end)
local toggle = bui.Node("toggle")
toggle:set("text", "Toggle Me")
toggle:set("value", 0)
toggle:on(bui.EV_CHANGE, function()
  local v = toggle:get("value")
  log_label:set("text", "Toggle: " .. (v > 0 and "ON" or "OFF"))
end)
local cb = bui.Node("checkbox")
cb:set("text", "Check Me")
cb:set("value", 0)
cb:on(bui.EV_CHANGE, function()
  local v = cb:get("value")
  log_label:set("text", "Checkbox: " .. (v > 0 and "checked" or "unchecked"))
end)
row({btn, toggle, cb})

-- Text inputs
section("Text Input")
local text = bui.Node("text")
text:set("placeholder", "Type something...")
text:on(bui.EV_CHANGE, function()
  log_label:set("text", "Text: " .. (text:get("value") or ""))
end)
col:add(text)
text:geom(bui.GEOM_PACK, {fill = 1})

local search = bui.Node("search")
search:set("placeholder", "Search...")
col:add(search)
search:geom(bui.GEOM_PACK, {fill = 1})

local hotkey = bui.Node("hotkey")
hotkey:set("placeholder", "Set hotkey...")
col:add(hotkey)
hotkey:geom(bui.GEOM_PACK, {fill = 1})

-- Numeric
section("Numeric Controls")
local num = bui.Node("number")
num:set("label", "Value:")
num:set("value", 42)
num:set("min", -100)
num:set("max", 100)
num:on(bui.EV_CHANGE, function()
  log_label:set("text", "Number: " .. (num:get("value") or 0))
end)
col:add(num)
num:geom(bui.GEOM_PACK, {fill = 1})

local slider = bui.Node("slider")
slider:set("label", "Slider:")
slider:set("min", 0)
slider:set("max", 100)
slider:set("value", 50)
slider:on(bui.EV_CHANGE, function()
  log_label:set("text", "Slider: " .. (slider:get("value") or 0))
end)
col:add(slider)
slider:geom(bui.GEOM_PACK, {fill = 1})

-- Selection
section("Selection")

local dd = bui.Node("dropdown")
dd:set_strs("items", {"Object Mode", "Edit Mode", "Sculpt Mode", "Vertex Paint"})
dd:on(bui.EV_CHANGE, function()
  local idx = dd:get("value")
  log_label:set("text", "Dropdown: " .. tostring(idx))
end)
col:add(dd)
dd:geom(bui.GEOM_PACK, {fill = 1})

local enum_vals = {"X", "Y", "Z"}
local ed = bui.Node("enum")
ed:set_strs("items", enum_vals)
ed:set("value", 0)
ed:on(bui.EV_CHANGE, function()
  local idx = ed:get("value")
  log_label:set("text", "Enum: " .. (enum_vals[idx + 1] or ""))
end)
col:add(ed)
ed:geom(bui.GEOM_PACK, {fill = 1})

local radio_row = bui.Node("row")
radio_row:set("padding", 4)
for _, rname in ipairs({"Option A", "Option B", "Option C"}) do
  local rb = bui.Node("radio")
  rb:set("text", rname)
  rb:set("value", 0)
  rb:on(bui.EV_CHANGE, function()
    if rb:get("value") > 0 then
      log_label:set("text", "Radio: " .. rname)
    end
  end)
  radio_row:add(rb)
end
col:add(radio_row)

-- Visual
section("Visual Widgets")
local sw = bui.Node("swatch")
sw:set("color", {0.28, 0.45, 0.70, 1.0})
col:add(sw)
sw:geom(bui.GEOM_PACK, {fill = 1})

local prog = bui.Node("progress")
prog:set("value", 0.65)
col:add(prog)
prog:geom(bui.GEOM_PACK, {fill = 1})

local meter = bui.Node("meter")
meter:set("value", 0.4)
col:add(meter)
meter:geom(bui.GEOM_PACK, {fill = 1})

-- Color swatch (click to open HSV picker)
section("Color Picker")
local color_swatch = bui.Node("swatch")
color_swatch:set("color", {0.28, 0.45, 0.70, 1.0})
color_swatch:on(bui.EV_CHANGE, function()
  log_label:set("text", "Color changed!")
end)
col:add(color_swatch)
color_swatch:geom(bui.GEOM_PACK, {fill = 1})

-- Decorator
section("Decorator")
local dec = bui.Node("decorator")
dec:set("text", "Decorated Content")
col:add(dec)
dec:geom(bui.GEOM_PACK, {fill = 1})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Forms — Lua", 420, 700)
if win then
  win:show()
  bui.run()
end
