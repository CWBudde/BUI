-- Widget gallery: demonstrates various BUI widget types
local bui = require("bui")

local root = bui.Node("scroll")
root:set("padding", 8)

-- Create a column inside the scroll for the gallery items
local col = bui.Node("column")
col:set("padding", 4)
root:add(col)

local function add_section(title, widget_fn)
    local sep = bui.Node("separator")
    col:add(sep)

    local heading = bui.Node("label")
    heading:set("text", title)
    col:add(heading)

    widget_fn()
end

-- Buttons
add_section("Buttons", function()
    local btn = bui.Node("button")
    btn:set("text", "Normal Button")
    col:add(btn)
    btn:geom(bui.GEOM_PACK, {fill = 1})

    local toggle = bui.Node("toggle")
    toggle:set("text", "Toggle")
    toggle:set("value", 1)
    col:add(toggle)
    toggle:geom(bui.GEOM_PACK, {fill = 1})

    local checkbox = bui.Node("checkbox")
    checkbox:set("text", "Checkbox")
    checkbox:set("value", 1)
    col:add(checkbox)
end)

-- Text input
add_section("Text Fields", function()
    local text = bui.Node("text")
    text:set("placeholder", "Type here...")
    col:add(text)
    text:geom(bui.GEOM_PACK, {fill = 1})

    local search = bui.Node("search")
    search:set("placeholder", "Search...")
    col:add(search)
    search:geom(bui.GEOM_PACK, {fill = 1})
end)

-- Numeric controls
add_section("Numeric Controls", function()
    local number = bui.Node("number")
    number:set("label", "Value:")
    number:set("value", 42)
    col:add(number)
    number:geom(bui.GEOM_PACK, {fill = 1})

    local slider = bui.Node("slider")
    slider:set("label", "Slider:")
    slider:set("min", 0)
    slider:set("max", 100)
    slider:set("value", 50)
    col:add(slider)
    slider:geom(bui.GEOM_PACK, {fill = 1})
end)

-- Dropdown
add_section("Dropdown", function()
    local dropdown = bui.Node("dropdown")
    dropdown:set_strs("items", {"Option A", "Option B", "Option C", "Option D"})
    col:add(dropdown)
    dropdown:geom(bui.GEOM_PACK, {fill = 1})
end)

-- Progress bar
add_section("Progress", function()
    local progress = bui.Node("progress")
    progress:set("value", 0.65)
    col:add(progress)
    progress:geom(bui.GEOM_PACK, {fill = 1})
end)

-- Labels
add_section("Labels", function()
    local label = bui.Node("label")
    label:set("text", "Hello, BUI from Lua!")
    col:add(label)

    local small = bui.Node("label")
    small:set("text", "Smaller text")
    col:add(small)
end)

local ctx = bui.Context(root)
local win = bui.Window(ctx, "BUI Widget Gallery", 400, 600)

if win then
    win:show()
    bui.run()
else
    local px, _, w, h = ctx:render(400, 600, 1)
    print("No window system. Headless render: " .. w .. "x" .. h .. " " .. #px .. " bytes")
end
