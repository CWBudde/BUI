local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 0)

local mb = bui.Node("menubar")
mb:set_menus({
  {
    text = "File",
    sub = {
      {text = "New", icon = bui.icon_id("FILE_NEW"),
       fn = function() print("File > New") end},
      {text = "Open", icon = bui.icon_id("FILE_FOLDER"),
       fn = function() print("File > Open") end},
      {text = "-"},
      {text = "Save", icon = bui.icon_id("FILE_TICK"),
       fn = function() print("File > Save") end},
      {text = "Exit", icon = bui.icon_id("QUIT"),
       fn = function() print("File > Exit") end},
    },
  },
  {
    text = "Edit",
    sub = {
      {text = "Undo", icon = bui.icon_id("LOOP_BACK"),
       fn = function() print("Edit > Undo") end},
      {text = "Redo", icon = bui.icon_id("LOOP_FORWARDS"),
       fn = function() print("Edit > Redo") end},
      {text = "-"},
      {text = "Cut", icon = bui.icon_id("CUT"),
       fn = function() print("Edit > Cut") end},
      {text = "Copy", icon = bui.icon_id("COPYDOWN"),
       fn = function() print("Edit > Copy") end},
      {text = "Paste", icon = bui.icon_id("PASTEDOWN"),
       fn = function() print("Edit > Paste") end},
    },
  },
  {
    text = "View",
    sub = {
      {text = "Zoom In",  fn = function() print("View > Zoom In") end},
      {text = "Zoom Out", fn = function() print("View > Zoom Out") end},
    },
  },
})
root:add(mb)
mb:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local body = bui.Node("panel")
body:set("text", "Content Area")
root:add(body)
body:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Menubar — Lua", 500, 300)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(500, 300, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
