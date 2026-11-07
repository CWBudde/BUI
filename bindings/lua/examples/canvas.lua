local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Canvas Drawing Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local cv = bui.Node("canvas")
root:add(cv)
cv:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

-- Rectangle
cv:canvas_set({
  id = 1, type = bui.ITEM_RECT,
  coords = {40, 30, 160, 100},
  fill = {0.2, 0.5, 0.8, 0.6},
  outline = {1, 1, 1, 1}, width = 2, radius = 8,
})

-- Oval
cv:canvas_set({
  id = 2, type = bui.ITEM_OVAL,
  coords = {120, 80, 80, 60},
  fill = {0.8, 0.3, 0.3, 0.5},
  outline = {1, 1, 1, 0.8}, width = 1.5,
})

-- Line
cv:canvas_set({
  id = 3, type = bui.ITEM_LINE,
  coords = {10, 120, 290, 120},
  outline = {1, 1, 0, 1}, width = 2,
})

-- Arc
cv:canvas_set({
  id = 4, type = bui.ITEM_ARC,
  coords = {150, 120, 60, 60},
  fill = {0.3, 0.8, 0.3, 0.3},
  outline = {1, 1, 1, 0.8}, width = 1.5,
  start = 0, extent = 270,
})

-- Text
cv:canvas_set({
  id = 5, type = bui.ITEM_TEXT,
  coords = {70, 150},
  text = "Hello Canvas",
  fill = {1, 1, 1, 1}, size = 18,
})

-- Polygon (triangle)
cv:canvas_set({
  id = 6, type = bui.ITEM_POLYGON,
  coords = {200, 25, 240, 25, 220, 65, 260, 40, 280, 70},
  fill = {0.9, 0.6, 0.2, 0.5},
  outline = {1, 1, 1, 0.8}, width = 1.5,
})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Canvas — Lua", 300, 200)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(300, 200, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
