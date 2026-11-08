-- Data table demo: virtual table with tree view, sortable headers, selection
-- SPDX-License-Identifier: GPL-2.0-or-later

local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 8)
local ctx

-- Generate sample data: N items with name, type, size, date, status
local function generate_data(count)
  local names = {"report", "photo", "video", "notes", "backup", "config", "script", "archive"}
  local types = {".blend", ".png", ".mp4", ".txt", ".zip", ".json", ".lua", ".tar.gz"}
  local statuses = {"Done", "Pending", "Error", "Processing"}
  local data = {}
  for i = 1, count do
    data[i] = {
      name = names[(i % #names) + 1] .. "_" .. i .. types[(i % #types) + 1],
      type = types[(i % #types) + 1]:sub(2),
      size = math.random(100, 99999),
      status = statuses[(i % #statuses) + 1],
      date = string.format("2024-%02d-%02d", math.random(1, 12), math.random(1, 28)),
    }
  end
  return data
end

local data = generate_data(100)

-- Header row with column labels
local header_row = bui.Node("row")
header_row:set("padding", 2)
root:add(header_row)
header_row:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local function col_header(text, weight)
  local h = bui.Node("label")
  h:set("text", text)
  h:set("value", 14)
  header_row:add(h)
  h:geom(bui.GEOM_PACK, {fill = weight or 1, side = 1})
end

col_header("Name", 3)
col_header("Type", 1)
col_header("Size (KB)", 1)
col_header("Status", 1)
col_header("Date", 1)

-- Separator
local sep = bui.Node("separator")
root:add(sep)
sep:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Tree view (used as a flat list / data table)
local tree = bui.Node("tree")
tree:set_strs("headers", {"Name", "Type", "Size", "Status", "Date"})
tree:set_strs("columns", {"0", "1", "2", "3", "4"})
root:add(tree)
tree:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

-- Populate tree with data items
local roots = {}
for i, row in ipairs(data) do
  local sz
  if row.size >= 1000 then
    sz = string.format("%.1f", row.size / 1000)
  else
    sz = tostring(row.size)
  end
  local icon = row.type == "blend" and bui.icon_id("FILE_BLEND")
            or row.type == "png" and bui.icon_id("FILE_IMAGE")
            or row.type == "mp4" and bui.icon_id("FILE_MOVIE")
            or row.type == "zip" and bui.icon_id("FILE_ARCHIVE")
            or row.type == "lua" and bui.icon_id("FILE_SCRIPT")
            or bui.icon_id("FILE")

  local item = bui.TreeItem(row.name, icon)
  item:set_values({row.name, row.type, sz, row.status, row.date})
  roots[i] = item
end
tree:set_roots(roots)
tree:relayout()

-- Selection info
local sel_label = bui.Node("label")
sel_label:set("text", "Rows: " .. #data .. " | Select a row to inspect")
root:add(sel_label)
sel_label:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Event: selection changed
tree:on(bui.EV_CHANGE, function()
  local sel = tree:selected()
  if sel then
    local vals = {}
    for i = 0, 4 do
      vals[i + 1] = sel:get("c" .. i) or ""
    end
    sel_label:set("text", "Selected: " .. (vals[1] or "none") .. " | " .. #data .. " rows")
  end
end)

-- Info about row_at / row_rect (use via headless)
local info_row = bui.Node("row")
info_row:set("padding", 4)
root:add(info_row)
info_row:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local rc_label = bui.Node("label")
rc_label:set("text", "Rows: " .. tree:row_count())
info_row:add(rc_label)
rc_label:geom(bui.GEOM_PACK, {side = 0})

ctx = bui.Context(root)

local win = bui.Window(ctx, "BUI Data Table — Lua", 600, 400)

if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(600, 400, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
  -- In headless, exercise the tree API
  if tree:row_count() > 0 then
    local r = tree:tree_row(0)
    if r then sel_label:set("text", "Row 0 label: " .. (r:label() or "")) end
  end
end
