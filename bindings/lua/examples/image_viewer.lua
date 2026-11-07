-- Image viewer: load images from data bytes or file paths, display in window
-- SPDX-License-Identifier: GPL-2.0-or-later

local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)
local ctx

local title = bui.Node("label")
title:set("text", "BUI Image Viewer — Lua")
title:set("value", 18)
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Generate a test image: a simple gradient + grid pattern
local W, H = 128, 128
local function make_test_pixels()
  local pixels = {}
  for y = 0, H - 1 do
    for x = 0, W - 1 do
      local r = math.floor(x / W * 255)
      local g = math.floor(y / H * 255)
      local b = math.floor((x + y) / (W + H) * 255)
      local a = 255
      -- Grid lines every 16 pixels
      if x % 16 < 1 or y % 16 < 1 then r, g, b = 255, 255, 255 end
      pixels[#pixels + 1] = string.char(r, g, b, a)
    end
  end
  return table.concat(pixels)
end

-- Display the test image on an image node
local img = bui.Node("image")
img:set_image(make_test_pixels(), W, H)
root:add(img)
img:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

-- Controls
local ctrl_row = bui.Node("row")
ctrl_row:set("padding", 4)
root:add(ctrl_row)
ctrl_row:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local fit_dd = bui.Node("dropdown")
fit_dd:set_strs("items", {"Fill", "Contain", "Cover", "None", "Scale Down"})
fit_dd:set("value", 0)
ctrl_row:add(fit_dd)
fit_dd:geom(bui.GEOM_PACK, {fill = 1})

fit_dd:on(bui.EV_CHANGE, function()
  local fit_val = fit_dd:get("value")
  img:set_image_source("", fit_val)
  img:set_image(make_test_pixels(), W, H)
end)

-- Image info label
local info = bui.Node("label")
info:set("text", string.format("Test image: %dx%d RGBA", W, H))
root:add(info)
info:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Image decode from embedded PNG-like data
local info2 = bui.Node("label")
info2:set("text", "Probe a file with: bui.image_probe_file('path.png')")
root:add(info2)
info2:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Demonstration: probe + decode from a PNG file if one is given as arg
if arg and arg[1] then
  local path = arg[1]
  local fmt, pw, ph, pf = bui.image_probe_file(path)
  if fmt then
    info2:set("text", string.format("Probe: %s %dx%d %d frames", fmt, pw, ph, pf))
  end
  local ib = bui.image_load(path, 0, 0)
  if ib then
    local iw, ih, frames = ib:size()
    local fmt_name = ib:format()
    info2:set("text", string.format("Loaded: %s %dx%d (%d frames)", path, iw, ih, frames))
    local pix = ib:pixels(0)
    if pix and #pix == iw * ih * 4 then
      img:set_image(pix, iw, ih)
      info:set("text", string.format("Loaded: %dx%d from %s", iw, ih, path))
    end
    ib:free()
  end
end

ctx = bui.Context(root)

local win = bui.Window(ctx, "BUI Image Viewer — Lua", 400, 360)

if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(400, 360, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
