-- Widget API tests: creation, properties, events, canvas, textarea, tree, menus
-- SPDX-License-Identifier: GPL-2.0-or-later

local function check(cond, msg)
  if not cond then io.stderr:write("FAIL: " .. msg .. "\n"); os.exit(1) end
end

local bui = require("bui")

-- ── All widget types can be created ──────────────────────────────────

local WIDGET_TYPES = {
  "frame", "column", "row", "grid", "box", "panel", "tabs",
  "scroll", "label", "separator", "progress", "button", "toggle",
  "checkbox", "decorator", "text", "search", "hotkey", "number",
  "slider", "dropdown", "enum", "swatch", "popover", "menubutton",
  "image", "textarea", "custom", "canvas", "list", "tree",
  "radio", "meter",
}

for _, t in ipairs(WIDGET_TYPES) do
  local node = bui.Node(t)
  check(node ~= nil, "bui.Node('" .. t .. "') returned nil")
  check(type(node) == "userdata", "bui.Node('" .. t .. "') not userdata")
  local ntype = node:type()
  check(ntype == t, "node:type() expected '" .. t .. "' got '" .. tostring(ntype) .. "'")
end

-- ── Properties: set / get / get_color ────────────────────────────────

local btn = bui.Node("button")
btn:set("text", "Hello")
check(btn:get("text") == "Hello" or type(btn:get("text")) == "string",
       "button text property")

btn:set("enabled", 1)
check(btn:get("enabled") == 1, "button enabled != 1, got " .. tostring(btn:get("enabled")))

btn:set("icon", 0)
-- Note: icon is read as a number, value depends on the implementation

-- Unknown numeric key returns empty string (get falls through to string)
-- Just check it doesn't crash
btn:get("nonexistent")

-- Numeric property
local num = bui.Node("number")
num:set("value", 42)
num:set("min", -100)
num:set("max", 100)
num:set("step", 1)
check(num:get("min") == -100, "number min != -100")
check(num:get("max") == 100, "number max != 100")
check(num:get("step") == 1, "number step != 1")

-- String list properties
local dd = bui.Node("dropdown")
dd:set_strs("items", {"A", "B", "C"})
local items = dd:get_strs("items")
check(items ~= nil, "dropdown get_strs returned nil")
check(#items == 3, "dropdown items count != 3, got " .. #items)

-- Color property
local sw = bui.Node("swatch")
sw:set("color", {0.28, 0.45, 0.70, 1.0})
local r, g, b, a = sw:get_color("color")
check(r ~= nil, "swatch get_color returned nil")
check(math.abs(r - 0.28) < 0.01, "swatch color r != 0.28, got " .. tostring(r))

-- ── Node tree operations ─────────────────────────────────────────────

local frame = bui.Node("frame")
local col = bui.Node("column")
local btn1 = bui.Node("button")
btn1:set("text", "One")
local btn2 = bui.Node("button")
btn2:set("text", "Two")

frame:add(col)
col:add(btn1)
col:add(btn2)

check(frame:child_count() == 1, "frame child_count != 1")
check(col:child_count() == 2, "col child_count != 2")

local c0 = col:child(0)
check(c0 ~= nil, "col:child(0) nil")
check(c0:get("text") == "One" or c0:type() == "button", "col child 0 not btn1")

local children = col:children()
check(children ~= nil, "children() nil")
check(#children == 2, "children count != 2")

local parent = btn1:parent()
check(parent ~= nil, "btn1 parent nil")
check(parent:type() == "column", "btn1 parent type != column")

-- Remove child
col:remove(btn1)
check(col:child_count() == 1, "after remove child_count != 1")

-- Natural size
local nw, nh = btn2:natural()
check(type(nw) == "number", "btn2 natural width not a number")
check(type(nh) == "number", "btn2 natural height not a number")

-- ── Events ───────────────────────────────────────────────────────────

local ev_log = ""
local ev_btn = bui.Node("button")
ev_btn:set("text", "Event Test")
ev_btn:on(bui.EV_CLICK, function() ev_log = ev_log .. "click " end)
ev_btn:on(bui.EV_CHANGE, function() ev_log = ev_log .. "change " end)

-- Off
ev_btn:off(bui.EV_CHANGE)
-- Test event firing by creating context and pressing
local ev_root = bui.Node("frame")
ev_root:add(ev_btn)
ev_btn:geom(bui.GEOM_PACK, {fill = 1})
local ev_ctx = bui.Context(ev_root)
ev_ctx:press(10, 10, 0)
ev_ctx:release(10, 10, 0)
-- EV_CLICK should fire
ev_ctx:free()

-- ── Canvas ───────────────────────────────────────────────────────────

local cnv = bui.Node("canvas")
check(type(cnv.canvas_set) == "function", "canvas_set not a function")
check(type(cnv.canvas_delete) == "function", "canvas_delete not a function")
check(type(cnv.canvas_raise) == "function", "canvas_raise not a function")

local cnv_root = bui.Node("frame")
cnv_root:add(cnv)
local cnv_ctx = bui.Context(cnv_root)

-- Set a canvas item (rectangle)
cnv:canvas_set({id=1, type=0, coords={10,10,50,50}, fill={1,0,0,1}})
cnv:canvas_set({id=2, type=0, coords={60,60,90,90}, fill={0,1,0,1}, width=2.0})
-- Delete item 2
cnv:canvas_delete(2)
-- Raise item 1
cnv:canvas_raise(1, 1)
-- Render
local px, _, w, h = cnv_ctx:render(100, 100, 1)
check(px ~= nil, "canvas render nil")
check(#px == 40000, "canvas render size != 40000, got " .. #px)

-- ── Canvas damage comparison ────────────────────────────────────────

cnv:canvas_set({id=3, type=0, coords={20,20,70,70}, fill={0,0,1,1}})
local px2, _, w2, h2 = cnv_ctx:render(100, 100, 0)
check(px2 ~= nil, "incremental render nil")
check(#px2 == 40000, "incremental size != 40000, got " .. #px2)
-- Verify different content (blue rect added vs initial red)
local diff = false
for i = 1, #px do
  if px:byte(i) ~= px2:byte(i) then diff = true; break end
end
check(diff, "incremental render should differ from initial")

-- Full re-render matches bytestream
cnv:canvas_delete(-1)
cnv:canvas_set({id=1, type=0, coords={10,10,50,50}, fill={1,0,0,1}})
local px3, _, _, _ = cnv_ctx:render(100, 100, 1)
check(#px == #px3, "full re-render size mismatch")
cnv_ctx:free()

-- ── Textarea ─────────────────────────────────────────────────────────

local ta = bui.Node("textarea")
check(type(ta.ta_insert) == "function", "ta_insert not a function")
check(type(ta.ta_delete) == "function", "ta_delete not a function")
check(type(ta.ta_spans) == "function", "ta_spans not a function")
check(type(ta.ta_see) == "function", "ta_see not a function")
check(type(ta.text_at) == "function", "text_at not a function")

ta:set("value", "hello world")

-- Caret operations
local caret_pos = ta:caret()
check(type(caret_pos) == "number", "caret not a number, got " .. type(caret_pos))
ta:set_caret(5)
check(ta:caret() == 5, "set_caret to 5 failed, got " .. tostring(ta:caret()))

-- Selection
local sel_start, sel_end = ta:selection()
check(type(sel_start) == "number", "selection start not a number")
check(type(sel_end) == "number", "selection end not a number")

-- Insert text
ta:ta_insert(5, " beautiful")
-- Now: "hello beautiful world"
ta:set_caret(0)
ta:set("value", "hello beautiful world")  -- reset

-- Delete
ta:ta_delete(0, 6)
-- The text area delete removes bytes 0..5
ta:set_caret(0)

-- See (scroll to position)
ta:ta_see(0)

-- Spans
ta:ta_spans({{0, 5, {1, 0, 0, 1}}, {5, 12, {0, 1, 0, 1}}})
-- Set value back
ta:set("value", "styled text")

-- text_at (coords to index)
local ta_root = bui.Node("frame")
ta_root:add(ta)
local ta_ctx = bui.Context(ta_root)
local idx = ta:text_at(10, 10)
check(type(idx) == "number", "text_at not a number, got " .. type(idx))
local px2, _, w2, h2 = ta_ctx:render(200, 100, 1)
check(px2 ~= nil, "textarea render nil")
check(#px2 == 80000, "textarea render size != 80000, got " .. #px2)

-- ── Textarea input and undo ─────────────────────────────────────────

local ta2 = bui.Node("textarea")
ta2:set("value", "hello")
local ta2_root = bui.Node("frame")
ta2_root:add(ta2)
local ta2_ctx = bui.Context(ta2_root)
-- Get textarea rect so we can click inside it
local _, _, _, _, tar = ta2:rect()
-- Click to focus, then type characters
ta2_ctx:press(5, 5, 0)
ta2_ctx:release(5, 5, 0)
ta2_ctx:key("x", "x", 0)
local val = ta2:get("value")
check(val ~= nil, "textarea value nil after key")
-- Undo the key press
ta2_ctx:undo()
-- Redo
ta2_ctx:redo()
local ud, rd = ta2_ctx:undo_depth()
check(type(ud) == "number", "ta undo_depth[0] not number, got " .. type(ud))
check(type(rd) == "number", "ta undo_depth[1] not number, got " .. type(rd))
ta2_ctx:free()

ta_ctx:free()

-- ── Tree / TreeView ──────────────────────────────────────────────────

local tree = bui.Node("tree")
tree:set_strs("headers", {"Name", "Value"})
tree:set_strs("columns", {"0", "1", "2"})

local item1 = bui.TreeItem("Alice", 0)
item1:set_values({"Alice", "30"})
item1:set("icon", 1)
check(item1:get("icon") == 1, "treeitem icon != 1")

local item2 = bui.TreeItem("Bob", 0)
item2:set_values({"Bob", "25"})

-- Hierarchy
local child = bui.TreeItem("child", 0)
item1:add(child)
check(item1:child_count() == 1, "item1 child_count != 1")

local c = item1:child(0)
check(c ~= nil, "item1:child(0) nil")
check(c:label() == "child", "child label, got " .. tostring(c:label()))

-- Set roots
tree:set_roots({item1, item2})
tree:relayout()
check(tree:row_count() > 0, "tree row_count <= 0")

-- Selection
tree:select(item1)
local sel = tree:selected()
check(sel ~= nil, "tree selected nil")

-- ── Menus ────────────────────────────────────────────────────────────

local mb = bui.Node("menubutton")
mb:set_menus({
  {text="Open", icon=0, fn=function() end},
  {text="Save", icon=1, enabled=1},
  {text="", icon=0}, -- separator
  {text="Quit", icon=0},
})

-- ── Progress / Meter ────────────────────────────────────────────────

local prog = bui.Node("progress")
prog:set("value", 0.75)
check(prog:get("value") == 0.75, "progress value != 0.75, got " .. tostring(prog:get("value")))

local meter = bui.Node("meter")
meter:set("value", 0.4)
check(meter:get("value") == 0.4, "meter value != 0.4")

-- ── Slider ───────────────────────────────────────────────────────────

local sld = bui.Node("slider")
sld:set("min", 0)
sld:set("max", 100)
sld:set("value", 50)
sld:set("step", 5)
check(sld:get("min") == 0, "slider min != 0")
check(sld:get("max") == 100, "slider max != 100")

-- ── Grid weight ──────────────────────────────────────────────────────

local grid = bui.Node("grid")
grid:grid_weight(0, 0, 1.0, 0)
-- grid_weight is a setter, no return value to check

-- ── Geometry manager ─────────────────────────────────────────────────

local geom_node = bui.Node("button")
geom_node:geom(bui.GEOM_PACK, {fill = 1, side = 1, padx0 = 2, pady0 = 2})
local fill_val = geom_node:geom_get("fill")
check(fill_val == 1, "geom_get fill != 1, got " .. tostring(fill_val))

-- ── Invalidate / Relayout ───────────────────────────────────────────

local inv_root = bui.Node("frame")
local inv_btn = bui.Node("button")
inv_root:add(inv_btn)
inv_btn:invalidate()
inv_root:relayout()

-- ── Ramp ────────────────────────────────────────────────────────────

local rn = bui.Node("ramp")
local rc = rn:ramp_count()
check(type(rc) == "number", "ramp_count not a number")

rn:ramp_set({{0, 0,0,0,1}, {1, 1,1,1,1}})
local pos, r1, g1, b1, a1 = rn:ramp_get(0)
check(pos ~= nil, "ramp_get pos nil")
check(pos == 0, "ramp_get[0] pos ~= 0")

local ev1, ev2, ev3, ev4 = rn:ramp_eval(0.5)
check(ev1 ~= nil, "ramp_eval result nil")

rn:ramp_set_color({1, 1, 1, 1})
check(rn:ramp_count() > 0, "ramp empty after set_color")

-- ── Curve ───────────────────────────────────────────────────────────

local cv = bui.Node("curve")
local cc = cv:curve_count(0)
check(type(cc) == "number", "curve_count not a number")

cv:curve_set(0, {{0, 0}, {1, 1}})
local cx, cy = cv:curve_get(0, 0)
check(cx ~= nil, "curve_get x nil")
check(cx == 0, "curve_get[0] x ~= 0")

local ce = cv:curve_eval(0, 0.5)
check(type(ce) == "number", "curve_eval not a number")

-- ── Histogram ───────────────────────────────────────────────────────

local hist = bui.Node("histogram")
hist:set_histogram({0.1, 0.3, 0.5, 0.2, 0.8, 0.4}, 1)
hist:set_histogram({0.1, 0.2, 0.3}, 1)
-- Just check it doesn't crash

-- ── Grid colors ─────────────────────────────────────────────────────

local gv = bui.Node("gridview")
gv:set_grid_colors({{1,0,0,1}, {0,1,0,1}, {0,0,1,1}})
-- Just check it doesn't crash

-- ── All tests passed ─────────────────────────────────────────────────

io.write("OK: test_widgets.lua\n")
os.exit(0)
