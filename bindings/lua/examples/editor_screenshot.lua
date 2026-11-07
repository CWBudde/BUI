-- Headless screenshot of the table editor for documentation.
-- Usage: bui-lua bindings/lua/examples/editor_screenshot.lua
-- Output: /tmp/bui_editor_demo.ppm
--
-- Converts to PNG with:  convert /tmp/bui_editor_demo.ppm /tmp/bui_editor_demo.png
--
-- SPDX-License-Identifier: GPL-2.0-or-later

local bui = require("bui")

-- Build the editor scene: uses the same tree/person-record layout
-- as the full editor, rendered headless into a raster.

local root = bui.Node("frame")
root:set("padding", 6)

-- Toolbar
local tb = bui.Node("row")
tb:set("padding", 4)
local title = bui.Node("label")
title:set("text", "Lua Table Editor")
tb:add(title)

local dd = bui.Node("dropdown")
dd:set_strs("items", {"Person Record", "Project Config", "Game State"})
tb:add(dd)
dd:geom(bui.GEOM_PACK, {fill = 1, side = 1})

local add_btn = bui.Node("button"); add_btn:set("text", "+ Add"); tb:add(add_btn); add_btn:geom(bui.GEOM_PACK, {side = 1})
local del_btn = bui.Node("button"); del_btn:set("text", "Delete"); tb:add(del_btn); del_btn:geom(bui.GEOM_PACK, {side = 1})
local exp_btn = bui.Node("button"); exp_btn:set("text", "Export"); tb:add(exp_btn); exp_btn:geom(bui.GEOM_PACK, {side = 1})
root:add(tb)
tb:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Main area
local main_row = bui.Node("row")
root:add(main_row)
main_row:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

-- Tree
local tree = bui.Node("tree")
tree:set_strs("headers", {"Key", "Type", "Value"})
tree:set_strs("columns", {"0", "2", "1", "3"})
main_row:add(tree)
tree:geom(bui.GEOM_PACK, {fill = 1})

-- Helper to build a tree item
local function ti(parent, key, vtype, val, children)
  local item = bui.TreeItem(key, 0)
  item:set_values({key, vtype, val})
  if children then
    for _, c in ipairs(children) do
      local ci = bui.TreeItem(c[1], 0)
      ci:set_values({c[1], c[2], c[3]})
      item:add(ci)
    end
  end
  if parent then parent:add(item) end
  return item
end

-- Build Person Record tree
local roots = {}
local person = ti(nil, "Person Record", "table", "{9 items}")
local name_item = ti(person, "name", "string", '"Alice Smith"')
ti(person, "age", "number", "30")
ti(person, "active", "boolean", "true")
ti(person, "score", "number", "95.5")
ti(person, "email", "string", '"alice@example.com"')

local addr = ti(nil, "address", "table", "{5 items}")
ti(addr, "street", "string", '"123 Main St"')
ti(addr, "city", "string", '"Portland"')
ti(addr, "state", "string", '"OR"')
ti(addr, "zip", "number", "97201")
local coords = ti(nil, "coordinates", "array", "{2 items}")
ti(coords, "lat", "number", "45.5152")
ti(coords, "lon", "number", "-122.6784")
addr:add(coords)
person:add(addr)

local tags = ti(nil, "tags", "array", "{3 items}")
ti(tags, "[1]", "string", '"developer"')
ti(tags, "[2]", "string", '"designer"')
ti(tags, "[3]", "string", '"rust"')
person:add(tags)
ti(person, "metadata", "nil", "nil")

roots[1] = person
tree:set_roots(roots)
tree:relayout()

-- Select the "name" row to show selection highlight
tree:select(name_item)

-- Editor panel
local ep = bui.Node("column")
ep:set("padding", 6)
local function el(text) local n=bui.Node("label"); n:set("text",text); ep:add(n); return n end
local function eb(text) local n=bui.Node("button"); n:set("text",text); ep:add(n); return n end

el("Edit Entry")
el("Key:")
local ek = bui.Node("text"); ek:set("value", "name"); ep:add(ek)
el("Type:")
local et = bui.Node("dropdown"); et:set_strs("items", {"string", "number", "boolean", "table", "array", "nil"}); ep:add(et)
el("Value:")
local ev = bui.Node("text"); ev:set("value", '"Alice Smith"'); ep:add(ev)
el("Toggle:")
local etog = bui.Node("toggle"); etog:set("text", "Enabled"); etog:set("value", 1); ep:add(etog)
eb("Apply"):geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})
eb("Clear Editor"):geom(bui.GEOM_PACK, {fill = 1, pady0 = 2})
main_row:add(ep)
ep:geom(bui.GEOM_PACK, {side = 1}) -- pack from right, natural width

-- Status bar
local st = bui.Node("label")
st:set("text", "Loaded: Person Record | Entries: 9 | Select an entry to edit")
root:add(st)
st:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Render headless
local ctx = bui.Context(root)
local W, H = 820, 560
local px_data, regions, rw, rh = ctx:render(W, H, 1)
if not px_data then
  print("No raster data.")
  return
end

-- Write as PPM (BGRA premultiplied -> RGB)
local out_path = arg and arg[1] or "/tmp/bui_editor_demo.ppm"
local f = io.open(out_path, "wb")
f:write("P6\n" .. rw .. " " .. rh .. "\n255\n")
for y = 0, rh - 1 do
  for x = 0, rw - 1 do
    local i = (y * rw + x) * 4 + 1
    local b = string.byte(px_data, i)
    local g = string.byte(px_data, i + 1)
    local r = string.byte(px_data, i + 2)
    local a = string.byte(px_data, i + 3)
    -- un-premultiply
    if a > 0 and a < 255 then
      local fa = a / 255
      r = math.min(255, math.floor(r / fa + 0.5))
      g = math.min(255, math.floor(g / fa + 0.5))
      b = math.min(255, math.floor(b / fa + 0.5))
    end
    f:write(string.char(r, g, b))
  end
end
f:close()
print("Wrote " .. out_path .. " (" .. rw .. "x" .. rh .. ")")
