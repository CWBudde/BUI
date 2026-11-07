-- Interactive counter example
local bui = require("bui")

local root = bui.Node("frame")
root:set("padding", 12)

-- Create the count label
local label = bui.Node("label")
label:set("text", "Count: 0")
root:add(label)
label:geom(bui.GEOM_PACK, {side = 0, fill = 1})

local count = 0

-- Increment button
local inc = bui.Node("button")
inc:set("text", "Increment")
inc:on(bui.EV_CLICK, function()
    count = count + 1
    label:set("text", "Count: " .. count)
end)
root:add(inc)
inc:geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})

-- Decrement button
local dec = bui.Node("button")
dec:set("text", "Decrement")
dec:on(bui.EV_CLICK, function()
    count = count - 1
    label:set("text", "Count: " .. count)
end)
root:add(dec)
dec:geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})

-- Reset button
local reset = bui.Node("button")
reset:set("text", "Reset")
reset:on(bui.EV_CLICK, function()
    count = 0
    label:set("text", "Count: 0")
end)
root:add(reset)
reset:geom(bui.GEOM_PACK, {fill = 1, pady0 = 4})

local ctx = bui.Context(root)
local win = bui.Window(ctx, "Counter", 240, 200)

if win then
    win:show()
    bui.run()
else
    local px, _, w, h = ctx:render(240, 200, 1)
    print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
