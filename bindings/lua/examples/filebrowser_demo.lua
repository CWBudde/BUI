-- File browser demo: browse, filter, and open files
-- SPDX-License-Identifier: GPL-2.0-or-later

local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)
local ctx

local title = bui.Node("label")
title:set("text", "File Browser Demo")
title:set("value", 18)
root:add(title)
title:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Controls
local control_row = bui.Node("row")
control_row:set("padding", 4)
root:add(control_row)
control_row:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local mode_dd = bui.Node("dropdown")
mode_dd:set_strs("items", {"Open", "Save", "Directory"})
mode_dd:set("value", 0)
control_row:add(mode_dd)
mode_dd:geom(bui.GEOM_PACK, {fill = 1})

local multi_cb = bui.Node("checkbox")
multi_cb:set("text", "Multi")
multi_cb:set("value", 0)
control_row:add(multi_cb)
multi_cb:geom(bui.GEOM_PACK, {side = 0})

local hidden_cb = bui.Node("checkbox")
hidden_cb:set("text", "Hidden")
hidden_cb:set("value", 0)
control_row:add(hidden_cb)
hidden_cb:geom(bui.GEOM_PACK, {side = 0})

-- File browser node
local fb = bui.Node("filebrowser")
fb:set("dir", os.getenv("HOME") or "/tmp")
fb:set("mode", 0)
fb:set("hidden", 0)
fb:set("sort", 0)
root:add(fb)
fb:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

-- Mode change handler
mode_dd:on(bui.EV_CHANGE, function()
  fb:set("mode", mode_dd:get("value"))
end)

multi_cb:on(bui.EV_CHANGE, function()
  fb:set("multiple", multi_cb:get("value"))
end)

hidden_cb:on(bui.EV_CHANGE, function()
  fb:set("hidden", hidden_cb:get("value"))
end)

-- Status bar
local status = bui.Node("label")
status:set("text", "Browse for files. Select and press Enter to open.")
root:add(status)
status:geom(bui.GEOM_PACK, {side = 0, fill = 1})

ctx = bui.Context(root)

-- Listen for submit to read the result
fb:on(bui.EV_SUBMIT, function()
  local state = fb:get("state")
  if state > 0 then
    local result = fb:get_strs("result")
    if result and #result > 0 then
      status:set("text", "Chosen: " .. result[1])
    else
      status:set("text", "File selected")
    end
  elseif state < 0 then
    status:set("text", "Cancelled")
  end
end)

-- Keyboard shortcuts
ctx:bind("ctrl+o", function()
  fb:set("execute", 1)
end)
ctx:bind("ctrl+h", function()
  local h = fb:get("hidden")
  fb:set("hidden", h > 0 and 0 or 1)
  hidden_cb:set("value", fb:get("hidden"))
end)
ctx:bind("escape", function()
  fb:set("cancel", 1)
  status:set("text", "Cancelled")
end)

local win = bui.Window(ctx, "BUI File Browser — Lua", 640, 480)

if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(640, 480, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
