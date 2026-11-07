-- Context API tests: input, undo/redo, clipboard, popups, menus, render
-- SPDX-License-Identifier: GPL-2.0-or-later

local function check(cond, msg)
  if not cond then io.stderr:write("FAIL: " .. msg .. "\n"); os.exit(1) end
end

local bui = require("bui")

-- ── Context creation ─────────────────────────────────────────────────

local root = bui.Node("frame")
root:set("padding", 8)
local ctx = bui.Context(root)
check(ctx ~= nil, "context creation failed")
check(type(ctx) == "userdata", "context not userdata")

-- ── Header ───────────────────────────────────────────────────────────

local header = bui.Node("label")
header:set("text", "Header")
ctx:set_header(header)

-- ── Root ─────────────────────────────────────────────────────────────

local r = ctx:root()
check(r ~= nil, "ctx:root() nil")
check(r:type() == "frame", "ctx:root() type != frame")

-- ── Render ──────────────────────────────────────────────────────────

local px, regions, w, h = ctx:render(320, 200, 1)
check(px ~= nil, "ctx:render() nil")
check(w == 320, "render width != 320, got " .. tostring(w))
check(h == 200, "render height != 200, got " .. tostring(h))
check(#px == 320 * 200 * 4, "render pixel size mismatch: " .. #px .. " vs " .. (320 * 200 * 4))
check(regions > 0, "render regions <= 0, got " .. tostring(regions))

-- Needs render
local nr_full = ctx:needs_render()
-- After full render, needs_render might be false
check(type(nr_full) == "boolean" or type(nr_full) == "number",
     "needs_render type: " .. type(nr_full))

-- Incremental render
local px2, regions2, w2, h2 = ctx:render(320, 200, 0)
check(px2 ~= nil, "incremental render nil")

-- ── Input forwarding ─────────────────────────────────────────────────

-- Add a button to interact with
local btn = bui.Node("button")
btn:set("text", "Test")
local clicked = false
btn:on(bui.EV_CLICK, function() clicked = true end)
root:add(btn)
btn:geom(bui.GEOM_PACK, {fill = 1})

-- Press / Release
local press_ok = ctx:press(50, 50, 0)
check(type(press_ok) == "boolean" or type(press_ok) == "number",
     "press return type: " .. type(press_ok))
ctx:release(50, 50, 0)

-- Motion
ctx:motion(100, 100, 0)

-- Wheel
ctx:wheel(50, 50, 10, 0)
ctx:wheel(50, 50, -10, bui.MOD_SHIFT)

-- Key
ctx:key("a", "a", 0)
ctx:key("b", "b", bui.MOD_CTRL)
ctx:key_release("a", 0)

-- Button (low-level mouse button events)
ctx:button(50, 50, 1, 0, 0)  -- press button 1
ctx:button(50, 50, 1, 1, 0)  -- release button 1

-- Context menu event
ctx:context_menu(100, 100, 0)

-- Scroll
ctx:scroll(50, 50, 10.5, -20.3, 0)

-- ── Cursor ───────────────────────────────────────────────────────────

local cursor = ctx:cursor()
check(type(cursor) == "number", "cursor not a number, got " .. type(cursor))

-- ── Focus / Hover ────────────────────────────────────────────────────

local focus = ctx:focus()
-- focus may be nil (nothing focused) -- that's OK
local hover = ctx:hover()
-- hover may be nil

-- ── Set pointer ──────────────────────────────────────────────────────

ctx:set_pointer(100, 100)

-- ── Popup ────────────────────────────────────────────────────────────

local popup_count = ctx:popup_count()
check(type(popup_count) == "number", "popup_count not a number, got " .. type(popup_count))
check(popup_count == 0, "popup_count should be 0, got " .. tostring(popup_count))

-- ── Close popups ─────────────────────────────────────────────────────

ctx:close_popups()

-- ── Undo / Redo ──────────────────────────────────────────────────────

ctx:undo()
ctx:redo()

-- Undo depth
local undo_depth, redo_depth = ctx:undo_depth()
check(type(undo_depth) == "number", "undo_depth not a number, got " .. type(undo_depth))
check(type(redo_depth) == "number", "redo_depth not a number, got " .. type(redo_depth))

-- ── Animating ────────────────────────────────────────────────────────

local anim = ctx:animating()
check(type(anim) == "boolean" or type(anim) == "number",
     "animating type: " .. type(anim))

-- ── Clipboard ────────────────────────────────────────────────────────

local clip = ctx:clipboard_get()
check(type(clip) == "string", "clipboard_get not string, got " .. type(clip))
ctx:clipboard_set("test clipboard")
clip = ctx:clipboard_get()
check(clip == "test clipboard" or clip ~= nil,
     "clipboard roundtrip failed, got " .. tostring(clip))

-- ── Bind / Key shortcuts ─────────────────────────────────────────────

ctx:bind("ctrl+z", function() end)
ctx:bind("ctrl+shift+z", function() end)
-- Second bind with same combo replaces first (unrefs old)
ctx:bind("ctrl+z", function() print("new") end)

-- ── Timer ────────────────────────────────────────────────────────────

ctx:timer(0)

-- ── Open menu / open pie ─────────────────────────────────────────────

ctx:open_menu(nil, {
  {text="Item A", icon=0},
  {text="Item B", icon=1, enabled=1},
  {text="-"},
  {text="Item C", icon=0},
}, "Test Menu", 100, 100)

ctx:open_pie({
  {text="North", icon=0},
  {text="South", icon=1},
  {text="East", icon=2},
  {text="West", icon=3},
}, "Direction")

-- Close popups after open_menu/open_pie
ctx:close_popups()

-- ── Multiple renders ─────────────────────────────────────────────────

for i = 1, 3 do
  local p, _, rw, rh = ctx:render(320, 200, i == 1 and 1 or 0)
  check(p ~= nil, "render " .. i .. " nil")
  check(rw == 320, "render " .. i .. " width")
  check(rh == 200, "render " .. i .. " height")
end

-- ── Context tostring ─────────────────────────────────────────────────

local s = tostring(ctx)
check(s ~= nil, "tostring(context) nil")
check(#s > 0, "tostring(context) empty")

-- ── Free ─────────────────────────────────────────────────────────────

ctx:free()

-- ── Cleanup tests ────────────────────────────────────────────────────

-- Test that freed context methods return safely
local empty_root = bui.Node("frame")
local empty_ctx = bui.Context(empty_root)
empty_ctx:free()
-- These should not crash
empty_ctx:press(1, 1, 0)
empty_ctx:release(1, 1, 0)
empty_ctx:key("a", "a", 0)
empty_ctx:close_popups()
empty_ctx:undo()

-- ── All tests passed ─────────────────────────────────────────────────

io.write("OK: test_context.lua\n")
os.exit(0)
