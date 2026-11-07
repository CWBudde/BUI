-- Todo list example with list/tree view
local bui = require("bui")

-- Build the layout
local root = bui.Node("frame")
root:set("padding", 8)
local ctx  -- declared early for closure capture

local header = bui.Node("label")
header:set("text", "Todo List")
header:set("value", 18)
root:add(header)
header:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local input_row = bui.Node("row")
root:add(input_row)
input_row:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local field = bui.Node("text")
field:set("placeholder", "Add a task...")
input_row:add(field)
field:geom(bui.GEOM_PACK, {fill = 1})

local tasks = {}

local add_btn = bui.Node("button")
add_btn:set("text", "Add")
add_btn:on(bui.EV_CLICK, function()
    -- Commit any pending text edit before reading the field
    ctx:focus_node(nil)
    local text = field:get("text")
    if text and #text > 0 then
        table.insert(tasks, text)
        field:set("value", "")
        refresh_list()
    end
end)
input_row:add(add_btn)

-- Separator
local sep = bui.Node("separator")
root:add(sep)
sep:geom(bui.GEOM_PACK, {side = 0, fill = 1})

-- Task list
local task_list = bui.Node("column")
task_list:set("padding", 4)
root:add(task_list)
task_list:geom(bui.GEOM_PACK, {side = 0, expand = 1, fill = 3})

local function refresh_list()
    for i = task_list:child_count() - 1, 0, -1 do
        task_list:remove(task_list:child(i))
    end
    for _, t in ipairs(tasks) do
        local l = bui.Node("label")
        l:set("text", t)
        l:geom(bui.GEOM_PACK, {side = 0, fill = 1})
        task_list:add(l)
    end
    root:relayout()
end

local clear = bui.Node("button")
clear:set("text", "Clear All")
clear:on(bui.EV_CLICK, function()
    tasks = {}
    refresh_list()
end)
root:add(clear)
clear:geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})

ctx = bui.Context(root)
local win = bui.Window(ctx, "Todo List", 320, 400)

if win then
    win:show()
    bui.run()
else
    local px, _, w, h = ctx:render(320, 400, 1)
    print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
