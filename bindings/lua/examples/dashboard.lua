-- Render farm dashboard: tabs, panels, progress, and status widgets
local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 0)

-- Header (pack from top, natural height, fill horizontal)
local header = bui.Node("row")
header:set("padding", 8)
local htitle = bui.Node("label")
htitle:set("text", "Render Farm Dashboard")
header:add(htitle)
local job_btn = bui.Node("button")
job_btn:set("text", "New Job")
job_btn:set("icon", bui.icon_id("ADD"))
job_btn:geom(bui.GEOM_PACK, {side = 1})
header:add(job_btn)
root:add(header)
header:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Tabs (pack from top, natural height, fill horizontal)
local tabs = bui.Node("tabs")
tabs:set_strs("items", {"Overview", "Jobs", "Nodes", "Settings"})
root:add(tabs)
tabs:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Content area (expand = take remaining vertical space, fill = fill both axes)
local content_area = bui.Node("frame")
content_area:set("padding", 8)
root:add(content_area)
content_area:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local status_lbl = bui.Node("label")
status_lbl:set("text", "Ready")

-- Job data
local JOB_DATA = {
  {"robot_arm.blend", "Cycles", "1-48", 0.72},
  {"forest_scene.blend", "Eevee",  "1-24", 0.45},
  {"studio_setup.blend", "Workbench", "1-12", 0.90},
  {"particle_sim.blend", "Cycles", "1-96", 0.18},
}

local NODE_DATA = {
  {"Node-01", "rendering", 0.65},
  {"Node-02", "idle", 0.0},
  {"Node-03", "rendering", 0.35},
  {"Node-04", "completed", 1.0},
  {"Node-05", "rendering", 0.80},
  {"Node-06", "offline", 0.0},
}

local function build_overview()
  local col = bui.Node("frame")
  col:set("padding", 4)
  col:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

  local cards_row = bui.Node("row")
  cards_row:set("padding", 4)
  cards_row:geom(bui.GEOM_PACK, {side = 0, fill = 1})
  local function card(label, val)
    local c = bui.Node("column")
    c:set("padding", 4)
    local l = bui.Node("label"); l:set("text", label); c:add(l)
    local v = bui.Node("label"); v:set("text", val); c:add(v)
    c:geom(bui.GEOM_PACK, {fill = 1, side = 1})
    return c
  end
  cards_row:add(card("Active Jobs", "4"))
  cards_row:add(card("Nodes Online", "6/8"))
  cards_row:add(card("Queue Depth", "12"))
  cards_row:add(card("Avg Time", "4.2m"))
  col:add(cards_row)

  local status_panel = bui.Node("panel")
  status_panel:set("text", "System Status")
  status_panel:set("collapsed", 0)
  status_panel:geom(bui.GEOM_PACK, {side = 0, fill = 1})
  local body = status_panel:child(0)
  if body then
    body:set("padding", 4)
    body:add(status_lbl)
  end
  col:add(status_panel)

  return col
end

local function build_jobs()
  local col = bui.Node("frame")
  col:set("padding", 4)
  col:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

  local hl = bui.Node("label")
  hl:set("text", "Active Render Jobs")
  col:add(hl)

  for _, job in ipairs(JOB_DATA) do
    local r = bui.Node("row")
    r:set("padding", 2)
    local nl = bui.Node("label"); nl:set("text", job[1]); r:add(nl)
    local el = bui.Node("label"); el:set("text", "[" .. job[2] .. "]"); r:add(el)
    local fl = bui.Node("label"); fl:set("text", job[3]); r:add(fl)
    local bar = bui.Node("progress"); bar:set("factor", job[4]); bar:geom(bui.GEOM_PACK, {fill = 1}); r:add(bar)
    local pl = bui.Node("label"); pl:set("text", math.floor(job[4] * 100) .. "%"); r:add(pl)
    col:add(r)
  end

  local btn = bui.Node("button")
  btn:set("text", "Cancel Selected")
  btn:on(bui.EV_CLICK, function() status_lbl:set("text", "Cancel requested") end)
  btn:geom(bui.GEOM_PACK, {fill = 1, pady0 = 6})
  col:add(btn)

  return col
end

local function build_nodes()
  local col = bui.Node("frame")
  col:set("padding", 4)
  col:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

  local wt = bui.Node("toggle")
  wt:set("text", "Wake idle nodes")
  wt:set("value", 1)
  col:add(wt)

  for _, nd in ipairs(NODE_DATA) do
    local r = bui.Node("row")
    r:set("padding", 2)
    local nl = bui.Node("label"); nl:set("text", nd[1]); r:add(nl)
    local sl = bui.Node("label"); sl:set("text", nd[2]); r:add(sl)
    local bar = bui.Node("progress"); bar:set("factor", nd[3]); bar:geom(bui.GEOM_PACK, {fill = 1}); r:add(bar)
    col:add(r)
  end

  return col
end

local function build_settings()
  local col = bui.Node("frame")
  col:set("padding", 4)
  col:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

  local function prop_row(label_text, widget)
    local r = bui.Node("row")
    r:set("padding", 2)
    local l = bui.Node("label"); l:set("text", label_text); r:add(l)
    widget:geom(bui.GEOM_PACK, {fill = 1})
    r:add(widget)
    return r
  end

  local max_num = bui.Node("number")
  max_num:set("value", 4); max_num:set("min", 1); max_num:set("max", 32)
  col:add(prop_row("Max jobs:", max_num))

  local prio_dd = bui.Node("dropdown")
  prio_dd:set_strs("items", {"Low", "Normal", "High", "Critical"})
  col:add(prop_row("Priority:", prio_dd))

  local ecb = bui.Node("checkbox"); ecb:set("text", "Email on completion"); ecb:set("value", 1); col:add(ecb)
  local scb = bui.Node("checkbox"); scb:set("text", "Sound on error");  scb:set("value", 0); col:add(scb)

  local btn = bui.Node("button")
  btn:set("text", "Save Settings")
  btn:on(bui.EV_CLICK, function() status_lbl:set("text", "Settings saved") end)
  btn:geom(bui.GEOM_PACK, {fill = 1, pady0 = 6})
  col:add(btn)

  return col
end

local pages = {build_overview(), build_jobs(), build_nodes(), build_settings()}
local current_page = pages[1]
content_area:add(current_page)

tabs:on(bui.EV_CHANGE, function()
  local idx = tabs:get("value") + 1
  if idx < 1 or idx > #pages then return end
  content_area:remove(current_page)
  current_page = pages[idx]
  content_area:add(current_page)
  content_area:relayout()
end)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Dashboard — Lua", 620, 520)
if win then
  win:show()
  bui.run()
else
  local px, _, w, h = ctx:render(620, 520, 1)
  print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
