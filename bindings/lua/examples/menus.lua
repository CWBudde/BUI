-- Menu and pie menu demo: submenus, context menus, pie menus
-- SPDX-License-Identifier: GPL-2.0-or-later

local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)
local ctx

-- Status label
local status = bui.Node("label")
status:set("text", "Right-click or use the menus below")
root:add(status)
status:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Context menu area (a large panel that responds to right-click)
local area = bui.Node("panel")
area:set("text", "Context Area (right-click here)")
root:add(area)
area:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

-- Menu button with submenus
local sub_label = bui.Node("label")
sub_label:set("text", "Submenu Demo:")
root:add(sub_label)
sub_label:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local mb = bui.Node("menubutton")
mb:set("text", "File Menu")
mb:set_menus({
  {
    text = "New", icon = bui.icon_id("FILE_NEW"),
    fn = function() status:set("text", "New file created") end,
  },
  {
    text = "Open", icon = bui.icon_id("FILE_FOLDER"),
    fn = function() status:set("text", "Open dialog") end,
  },
  {text = "-"},
  {
    text = "Recent", icon = bui.icon_id("RECENT"),
    sub = {
      {text = "doc1.lua", fn = function() status:set("text", "Opened doc1.lua") end},
      {text = "doc2.lua", fn = function() status:set("text", "Opened doc2.lua") end},
      {text = "doc3.lua", fn = function() status:set("text", "Opened doc3.lua") end},
    },
  },
  {text = "-"},
  {
    text = "Save", icon = bui.icon_id("FILE_TICK"),
    enabled = 0, fn = function() end,
  },
  {
    text = "Exit", icon = bui.icon_id("QUIT"),
    fn = function() status:set("text", "Exiting...") end,
  },
})
root:add(mb)
mb:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Pie menu button
local pie_label = bui.Node("label")
pie_label:set("text", "Pie Menu Demo:")
root:add(pie_label)
pie_label:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local pie_btn = bui.Node("button")
pie_btn:set("text", "Open Pie Menu")
pie_btn:on(bui.EV_CLICK, function()
  ctx:open_pie({
    {text = "North", icon = bui.icon_id("ARROW_UP")},
    {text = "South", icon = bui.icon_id("ARROW_DOWN")},
    {text = "East",  icon = bui.icon_id("ARROW_RIGHT")},
    {text = "West",  icon = bui.icon_id("ARROW_LEFT")},
  }, "Directions")
  status:set("text", "Pie menu opened")
end)
root:add(pie_btn)
pie_btn:geom(bui.GEOM_PACK, {side = 0, fill = 1, pady0 = 4})

-- Open menu at pointer button
local om_btn = bui.Node("button")
om_btn:set("text", "Open Menu at Pointer")
om_btn:on(bui.EV_CLICK, function()
  ctx:open_menu(nil, {
    {text = "Item A", icon = bui.icon_id("OBJECT_DATA"),
     fn = function() status:set("text", "Selected Item A") end},
    {text = "Item B", icon = bui.icon_id("MODIFIER"),
     fn = function() status:set("text", "Selected Item B") end},
    {text = "-"},
    {text = "Item C", icon = bui.icon_id("OUTLINER_COLLECTION"),
     checked = 1, fn = function() status:set("text", "Selected Item C") end},
  }, "Quick Menu", 100, 100)
  status:set("text", "Context menu opened")
end)
root:add(om_btn)
om_btn:geom(bui.GEOM_PACK, {side = 0, fill = 1})

ctx = bui.Context(root)

-- Right-click on the context area opens a menu
area:on(bui.EV_PRESS, function(_, x, y)
  ctx:open_menu(area, {
    {text = "Cut",  icon = bui.icon_id("CUT"),
     fn = function() status:set("text", "Cut") end},
    {text = "Copy", icon = bui.icon_id("COPYDOWN"),
     fn = function() status:set("text", "Copy") end},
    {text = "Paste", icon = bui.icon_id("PASTEDOWN"),
     fn = function() status:set("text", "Paste") end},
    {text = "-"},
    {text = "Delete", icon = bui.icon_id("X"),
     fn = function() status:set("text", "Delete") end},
  }, "Edit Actions", math.floor(x), math.floor(y))
end)

ctx:bind("ctrl+n", function()
  status:set("text", "Ctrl+N pressed (New)")
end)
ctx:bind("ctrl+s", function()
  status:set("text", "Ctrl+S pressed (Save)")
end)

local win = bui.Window(ctx, "BUI Menus — Lua", 400, 320)

if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(400, 320, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
