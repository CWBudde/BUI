-- Basic bui-lua test: module loads, constants exist, factories return types.
-- Returns exit code 0 on success, 1 on failure.
-- SPDX-License-Identifier: GPL-2.0-or-later

local function check(cond, msg)
  if not cond then
    io.stderr:write("FAIL: " .. msg .. "\n")
    os.exit(1)
  end
end

-- Module loads
local bui = require("bui")
check(bui ~= nil, "require('bui') returned nil")

-- Version constants
check(type(bui._VERSION) == "string", "bui._VERSION not a string")
check(bui._VERSION:match("^%d+%.%d+%.%d+"), "bui._VERSION format: " .. bui._VERSION)
check(type(bui.KIT_VERSION) == "string", "bui.KIT_VERSION not a string")

-- Module-level functions
check(type(bui.Node) == "function", "bui.Node not a function")
check(type(bui.Context) == "function", "bui.Context not a function")
check(type(bui.Window) == "function", "bui.Window not a function")
check(type(bui.TreeItem) == "function", "bui.TreeItem not a function")
check(type(bui.run) == "function", "bui.run not a function")
check(type(bui.pump) == "function", "bui.pump not a function")
check(type(bui.version) == "function", "bui.version not a function")

-- bui.version() returns three numbers
local major, minor, patch = bui.version()
check(type(major) == "number", "bui.version() major not a number")
check(type(minor) == "number", "bui.version() minor not a number")
check(type(patch) == "number", "bui.version() patch not a number")

-- Event constants
check(bui.EV_CHANGE ~= nil, "bui.EV_CHANGE missing")
check(bui.EV_CLICK ~= nil, "bui.EV_CLICK missing")
check(bui.EV_CALL ~= nil, "bui.EV_CALL missing")
check(bui.EV_PRESS ~= nil, "bui.EV_PRESS missing")
check(bui.EV_KEY ~= nil, "bui.EV_KEY missing")
check(bui.EV_SUBMIT ~= nil, "bui.EV_SUBMIT missing")
check(bui.EV_SCROLL ~= nil, "bui.EV_SCROLL missing")

-- Cursor constants
check(bui.CURSOR_DEFAULT ~= nil, "bui.CURSOR_DEFAULT missing")
check(bui.CURSOR_TEXT ~= nil, "bui.CURSOR_TEXT missing")
check(bui.CURSOR_HAND ~= nil, "bui.CURSOR_HAND missing")

-- Geometry constants
check(bui.GEOM_NONE ~= nil, "bui.GEOM_NONE missing")
check(bui.GEOM_PACK ~= nil, "bui.GEOM_PACK missing")
check(bui.GEOM_GRID ~= nil, "bui.GEOM_GRID missing")

-- Modifier constants
check(bui.MOD_SHIFT ~= nil, "bui.MOD_SHIFT missing")
check(bui.MOD_CTRL ~= nil, "bui.MOD_CTRL missing")
check(bui.MOD_ALT ~= nil, "bui.MOD_ALT missing")

-- Image format constants
check(bui.IMAGE_NONE ~= nil, "bui.IMAGE_NONE missing")
check(bui.IMAGE_PNG ~= nil, "bui.IMAGE_PNG missing")
check(bui.IMAGE_JPEG ~= nil, "bui.IMAGE_JPEG missing")

-- Widget type constants
check(bui.WIDGET_BUTTON ~= nil, "bui.WIDGET_BUTTON missing")
check(bui.WIDGET_TOGGLE ~= nil, "bui.WIDGET_TOGGLE missing")
check(bui.WIDGET_LABEL ~= nil, "bui.WIDGET_LABEL missing")

-- Node factory
local btn = bui.Node("button")
check(btn ~= nil, "bui.Node('button') returned nil")
check(type(btn) == "userdata", "bui.Node('button') not userdata")

-- Node properties
btn:set("text", "Hello")
check(btn:get("text") == "Hello" or type(btn:get("text")) == "string",
       "btn:get('text') not a string")
btn:set("enabled", 1)
btn:set("value", 1)

-- TreeItem factory
local item = bui.TreeItem("test", 0)
check(item ~= nil, "bui.TreeItem() returned nil")
item:set("icon", 3)
check(item:get("icon") == 3, "TreeItem.icon not 3")
item:set_values({"a", "b", "c"})

-- Sub-items and hierarchy
local child = bui.TreeItem("child", 0)
item:add(child)
check(item:child_count() == 1, "TreeItem.child_count not 1")
local c = item:child(0)
check(c ~= nil, "TreeItem.child(0) returned nil")

-- Context creation (headless)
local root = bui.Node("frame")
root:set("padding", 8)
local ctx = bui.Context(root)
check(ctx ~= nil, "bui.Context() returned nil")
check(type(ctx) == "userdata", "bui.Context() not userdata")

-- Headless render
local px, regions, w, h = ctx:render(100, 100, 1)
check(px ~= nil, "ctx:render() returned nil pixels")
check(type(px) == "string", "ctx:render() pixels not a string")
check(#px == 100 * 100 * 4, "ctx:render() pixel size mismatch: " .. #px .. " vs " .. (100 * 100 * 4))
check(w == 100, "ctx:render() width: " .. tostring(w))
check(h == 100, "ctx:render() height: " .. tostring(h))

-- Needs render
local nr = ctx:needs_render()
check(type(nr) == "boolean" or type(nr) == "number", "needs_render should be boolean/number, got " .. type(nr))

-- Input forwarding
local ok = ctx:press(50, 50, 0)
ctx:release(50, 50, 0)
ctx:motion(60, 60, 0)
ctx:key("a", "a", 0)
ctx:key_release("a", bui.MOD_CTRL)

-- Context cursor
local cursor = ctx:cursor()
check(type(cursor) == "number", "ctx:cursor() not a number")

-- Undo / redo
ctx:undo()
ctx:redo()

-- Theme
check(type(bui.set_theme) == "function", "bui.set_theme not a function")
check(type(bui.theme_name) == "function", "bui.theme_name not a function")
check(type(bui.theme_color) == "function", "bui.theme_color not a function")

-- Unit
local u = bui.unit()
check(type(u) == "number", "bui.unit() not a number")

-- Close popups
ctx:close_popups()

-- Free context
ctx:free()

-- All tests passed
io.write("OK: test_basic.lua\n")
os.exit(0)
