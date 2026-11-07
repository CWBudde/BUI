-- Todo list example with list/tree view
local bui = require("bui")

-- Build the layout
local root = bui.Node("frame")
root:set("padding", 8)

local header = bui.Node("label")
header:set("text", "Todo List")
header:set("value", 18) -- font size
root:add(header)

local input_row = bui.Node("row")
root:add(input_row)

local field = bui.Node("text")
field:set("placeholder", "Add a task...")
input_row:add(field)
field:geom(bui.GEOM_PACK, {fill = 1, side = 0})

local add_btn = bui.Node("button")
add_btn:set("text", "Add")
local tasks = {}  -- Lua-side storage

add_btn:on(bui.EV_CLICK, function()
    local text = field:get("value")
    if text and #text > 0 then
        table.insert(tasks, text)
        field:set("value", "")
        -- In a real app, update a list node here
        print("Added:", text)
    end
end)
input_row:add(add_btn)

-- Separator
local sep = bui.Node("separator")
root:add(sep)

-- Task list (using a column for simplicity)
local task_list = bui.Node("column")
task_list:set("padding", 4)
root:add(task_list)

local function refresh_list()
    -- Rebuild task list nodes
    -- (Since we can't easily remove and re-add children in a loop,
    --  this demonstrates the concept)
    print("Tasks (" .. #tasks .. "):")
    for i, t in ipairs(tasks) do
        print("  " .. i .. ". " .. t)
    end
end

local refresh_btn = bui.Node("button")
refresh_btn:set("text", "Show Tasks")
refresh_btn:on(bui.EV_CLICK, function()
    refresh_list()
end)
root:add(refresh_btn)
refresh_btn:geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})

-- Clear button
local clear = bui.Node("button")
clear:set("text", "Clear All")
clear:on(bui.EV_CLICK, function()
    tasks = {}
    print("Tasks cleared")
end)
root:add(clear)
clear:geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "Todo List", 320, 400)

if win then
    win:show()
    bui.run()
else
    local px, _, w, h = ctx:render(320, 400, 1)
    print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
