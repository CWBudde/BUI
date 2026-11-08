local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Grid Layout Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local grid = bui.Node("grid")
grid:set("padding", 4)
root:add(grid)
grid:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local colors = {
  {0.9, 0.3, 0.3}, {0.3, 0.9, 0.3}, {0.3, 0.3, 0.9},
  {0.9, 0.9, 0.3}, {0.9, 0.3, 0.9}, {0.3, 0.9, 0.9},
  {0.9, 0.6, 0.3}, {0.6, 0.9, 0.3}, {0.3, 0.6, 0.9},
  {0.8, 0.4, 0.6}, {0.4, 0.8, 0.6}, {0.6, 0.4, 0.8},
}

for i, c in ipairs(colors) do
  local cell = bui.Node("swatch")
  cell:set("color", {c[1], c[2], c[3], 1.0})
  cell:geom(bui.GEOM_GRID, {row = ((i - 1) % 4) + 1, column = math.ceil(i / 4)})
  grid:add(cell)
end

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Grid — Lua", 360, 300)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(360, 300, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
