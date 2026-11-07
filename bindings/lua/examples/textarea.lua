local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "Textarea Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local ta = bui.Node("textarea")
ta:set("text", "Hello from BUI Lua!\nThis is a multiline text area.\nYou can edit, select, and navigate\nwith keyboard shortcuts.")
root:add(ta)
ta:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local row = bui.Node("row")
row:set("padding", 4)

local count_lbl = bui.Node("label")
count_lbl:set("text", "Chars: 0")
row:add(count_lbl)
count_lbl:geom(bui.GEOM_PACK, {side = 0})

local sel_lbl = bui.Node("label")
sel_lbl:set("text", "Sel: 0-0")
row:add(sel_lbl)
sel_lbl:geom(bui.GEOM_PACK, {side = 0})

ta:on(bui.EV_CHANGE, function()
  local text = ta:get("text")
  count_lbl:set("text", "Chars: " .. #(text or ""))
  local start, stop = ta:selection()
  sel_lbl:set("text", "Sel: " .. start .. "-" .. stop)
end)

root:add(row)
row:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Textarea — Lua", 400, 300)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(400, 300, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
