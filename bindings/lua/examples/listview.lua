local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)

local title = bui.Node("label")
title:set("text", "List View Demo")
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local lv = bui.Node("list")
lv:set_strs("items", {
  "Apple", "Banana", "Cherry", "Date", "Elderberry",
  "Fig", "Grape", "Honeydew", "Kiwi", "Lemon",
  "Mango", "Nectarine", "Orange", "Papaya", "Quince",
})
root:add(lv)
lv:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local status = bui.Node("label")
status:set("text", "Select a fruit")
root:add(status)
status:geom(bui.GEOM_PACK, {side = 0, fill = 1})

lv:on(bui.EV_CHANGE, function()
  local idx = lv:get("value")
  if idx >= 0 then
    local items = lv:get_strs("items")
    status:set("text", "Selected: " .. (items[idx + 1] or "?"))
  end
end)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI List View — Lua", 300, 400)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(300, 400, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
