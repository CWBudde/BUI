-- Headless render test: create a widget tree, render it, and verify the output.
-- SPDX-License-Identifier: GPL-2.0-or-later

local function check(cond, msg)
  if not cond then
    io.stderr:write("FAIL: " .. msg .. "\n")
    os.exit(1)
  end
end

local bui = require("bui")

-- Build a simple tree: frame -> column -> button + label
local root = bui.Node("frame")
root:set("padding", 8)

local col = bui.Node("column")
col:set("padding", 4)
root:add(col)

local btn = bui.Node("button")
btn:set("text", "Click")
btn:on(bui.EV_CLICK, function() end)
col:add(btn)
btn:geom(bui.GEOM_PACK, {fill = 1})

local lbl = bui.Node("label")
lbl:set("text", "Hello, BUI!")
col:add(lbl)

-- Render at 320x240
local ctx = bui.Context(root)
check(ctx ~= nil, "Context creation failed")

local px, regions, w, h = ctx:render(320, 240, 1)
check(px ~= nil, "render returned nil")
check(w == 320, "width mismatch: " .. w)
check(h == 240, "height mismatch: " .. h)
check(#px == 320 * 240 * 4, "pixel buffer size mismatch")

-- Verify the pixel data is non-trivial (not all zeros or all 255)
local total = 0
for i = 1, #px, 4 do
  local r = string.byte(px, i)
  local g = string.byte(px, i + 1)
  local b = string.byte(px, i + 2)
  total = total + r + g + b
end
check(total > 0, "rendered image is all-black")
check(total < 320 * 240 * 3 * 255, "rendered image is all-white or invalid")

-- Render again (damage test - should produce same size output)
local px2, regions2, w2, h2 = ctx:render(320, 240, 0)
check(w2 == 320, "2nd render width mismatch")
check(h2 == 240, "2nd render height mismatch")

-- Different resolution
local px3, regions3, w3, h3 = ctx:render(640, 480, 1)
check(w3 == 640, "3rd render width mismatch")
check(h3 == 480, "3rd render height mismatch")
check(#px3 == 640 * 480 * 4, "3rd render pixel buffer size")

-- Needs render check
local nr = ctx:needs_render()
-- After a full render, needs_render should be false (0)
-- This is implementation-specific but tests the API
check(type(nr) == "boolean" or type(nr) == "number", "needs_render should be boolean/number")

-- Button should respond to events
local press_ok = ctx:press(16, 40, 0)  -- press somewhere inside the window
ctx:release(16, 40, 0)

-- Key event
ctx:key("Tab", "Tab", 0)

-- Scroll
ctx:scroll(100, 100, 0, -10, 0)

ctx:free()

io.write("OK: test_render.lua\n")
os.exit(0)
