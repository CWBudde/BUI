local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Histogram Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local desc = bui.Node("label")
desc:set("text", "RGB histogram with random luminance")
root:add(desc)
desc:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local hg = bui.Node("histogram")
root:add(hg)
hg:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3, pady0 = 6})

local bins = {}
math.randomseed(42)
for i = 1, 256 do
  bins[i] = math.random() * (i < 50 and i / 50 or (256 - i) / 206)
end
hg:set_histogram(bins, 3)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Histogram — Lua", 360, 250)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(360, 250, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
