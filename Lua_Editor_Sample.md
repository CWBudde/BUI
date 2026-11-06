# Lua Table Editor — Design Document

## Overview

A BUI-based interactive Lua table editor running under `bui-lua`. It accepts
any Lua table (nested, mixed types, arrays, sparse keys) and renders it as a
three-column tree view (Key | Type | Value). Users navigate the tree, edit
keys and values, add/delete entries, and export the result as a Lua source
string.

```
bui-lua bindings/lua/examples/lua_table_editor.lua
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     Lua Table Editor                              │
│                                                                   │
│  ┌────────────── DEMO TABLES ─────────────────────────────────┐   │
│  │  Person Record    Project Config    Game State              │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                             ▼                                      │
│  ┌────────────── DATA MODEL ──────────────────────────────────┐   │
│  │  A plain Lua table. Edited in-place.                       │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                             ▼                                      │
│  ┌────────────── TREE BUILDER ────────────────────────────────┐   │
│  │  Recursive walk of the data table. Creates BKTreeItem      │   │
│  │  objects with 3-column values (Key, Type, Value).          │   │
│  │  Stores metadata in a parallel Lua table keyed by          │   │
│  │  numeric item ids (item:set("_id", id)).                   │   │
│  └──────────────────────────┬──────────────────────────────────┘   │
│                             ▼                                      │
│  ┌────────────── BUI WINDOW ──────────────────────────────────┐   │
│  │                                                             │   │
│  │  ┌──── TOOLBAR ────────────────────────────────────────┐   │   │
│  │  │  [Demo ▼]  [+ Add]  [Delete]  [Export]              │   │   │
│  │  └──────────────────────────────────────────────────────┘   │   │
│  │                                                             │   │
│  │  ┌── MAIN ROW ─────────────────────────────────────────┐   │   │
│  │  │                                                    │   │   │
│  │  │  ┌────── TREE ─────────────┐  ┌── EDITOR PANEL ──┐│   │   │
│  │  │  │ Key │ Type │ Value      │  │ Key: [________]  ││   │   │
│  │  │  ├─────┼──────┼────────────┤  │ Type: [▼      ]  ││   │   │
│  │  │  │ name │str   │"Alice"    │  │ Value:[________] ││   │   │
│  │  │  │ age  │num   │30         │  │ Toggle:[____]    ││   │   │
│  │  │  │ ▶addr│table│{3 items}   │  │ [Apply][Clear]   ││   │   │
│  │  │  └─────┴──────┴────────────┘  └──────────────────┘│   │   │
│  │  └────────────────────────────────────────────────────┘   │   │
│  │                                                             │   │
│  │  ┌── STATUS ───────────────────────────────────────────┐   │   │
│  │  │  Loaded: Person Record | Entries: 9                  │   │   │
│  │  └──────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

---

## BUI Widgets Used

| Widget | Role | Details |
|--------|------|---------|
| `tree` | Table display | Three columns (Key, Type, Value). Column widths proportional via `"0 2 1 3"`. |
| `BKTreeItem` | Row data | Created recursively from the table. `set_values()` for column display. `set("_id", id)` for stable metadata lookup. |
| `dropdown` | Demo selector + type picker | `set_strs("items", {...})` populates choices. `EV_CHANGE` fires on selection. |
| `button` | Toolbar actions | Add, Delete, Export. Each wired to its handler via `EV_CLICK`. |
| `text` | Key / Value editing | Accepts typed input for string and number values. `EV_SUBMIT` on Enter for quick apply. |
| `toggle` | Boolean editing | Shows On/Off slider for boolean values. |
| `label` | Headers, status | Static labels throughout the layout + status bar updates. |
| `row` / `column` | Layout | `row` for toolbar and main area; `column` for vertical stacking and the editor panel. |

---

## Data Flow

### Loading a Demo (or any Lua table)

1. User selects a demo from the dropdown → `EV_CHANGE` fires
2. `switch_demo(name, table)` sets `data = table`, clears `item_meta`
3. `rebuild_tree(tree, data)` recursively walks the table:
   - For each `(key, value)` pair, creates a `BKTreeItem` with `set_values({key_str, type_name, formatted_value})`
   - Assigns a unique numeric id via `item:set("_id", id)`
   - Stores `{type, key, value, parent}` in `item_meta[id]`
   - For sub-tables, recurses and adds children with `parent_item:add(child)`
4. `tree:set_roots(roots)` installs the top-level items

### Editing a Value

1. User clicks a row in the tree → `EV_CHANGE` fires
2. `refresh_editor()` reads `tree:selected()`, gets the `_id`, looks up `item_meta[id]`
3. Editor panel populates with current values:
   - `editor_key:set("value", tostring(meta.key))`
   - `editor_type:set("value", type_index)` — dropdown selection
   - `editor_val:set("value", string_value)` — for string/number
   - `editor_bool:set("value", 0/1)` — for boolean
4. User edits fields, clicks "Apply" → `apply_edit()`:
   - Reads new key, type, value from editor widgets
   - `meta.parent[meta.key] = nil` — removes old entry
   - `meta.parent[new_key] = new_val` — inserts new entry
   - `rebuild_tree(tree, data)` — rebuilds the tree from the updated table

### Exporting

1. User clicks "Export" → `export_table()`
2. `serialize(data)` recursively walks the table and builds a Lua source string
3. Prints the result to stdout

---

## Implementation Details

### item_meta Structure

```lua
item_meta[id] = {
  id     = number,      -- unique numeric id (matches BKTreeItem._id)
  type   = string,      -- "string" | "number" | "boolean" | "table" | "array" | "nil"
  key    = any,         -- original key in the parent table (number or string)
  value  = any,         -- current value
  parent = table,       -- reference to the parent Lua table
}
```

### rebuild_tree Algorithm

```
rebuild_tree(tree, source_table):
    item_meta = {}
    id_counter = 1000
    root_items = {}
    for each (key, value) in sorted pairs of source_table:
        create BKTreeItem(key_str, icon=0)
        set_values({key_str, type_name(value), format_value(value)})
        assign _id = next_id()
        store {type, key, value, parent=source_table} in item_meta[_id]
        if type(value) == "table":
            build_subtree(item, value, value)
        append to root_items
    tree:set_roots(root_items)
    tree:relayout()
```

### serialize Algorithm

Recursive pretty-printer that handles:
- **strings**: quoted with `%q`
- **numbers**: integer format when exact, `%.14g` for floats
- **booleans**: `true` / `false`
- **nil**: `nil`
- **tables**: `{ }` with indented children, auto-detects arrays (`{1, 2, 3}`) vs maps (`{a = 1, b = 2}`), compact inline for ≤3 items under 60 chars

---

## Demo Tables

### 1. Person Record

A flat-and-nested record mixing strings, numbers, booleans, and a sub-table with coordinates. Demonstrates basic scalar types + one level of nesting.

```lua
{
  name = "Alice Smith",
  age = 30,
  email = "alice@example.com",
  active = true,
  score = 95.5,
  tags = {"developer", "designer", "rust"},
  address = {
    street = "123 Main St",
    city = "Portland",
    state = "OR",
    zip = 97201,
    coordinates = {lat = 45.5152, lon = -122.6784}
  },
  metadata = nil
}
```

### 2. Project Config

Demonstrates multi-level nesting (3+ levels), mixed arrays of sub-tables, boolean feature flags, and a file-path string set. Good for testing deep tree navigation.

```lua
{
  name = "bui-lua",
  version = "0.1.0",
  debug = false,
  features = {
    images = true,
    video = false,
    gpu = true,
    limits = {
      max_windows = 10,
      max_fps = 60,
      cache_size_mb = 256
    }
  },
  paths = {
    data = "/usr/local/share/bui",
    cache = "/tmp/bui-cache",
    logs = "/var/log/bui"
  },
  users = {
    {id = 1, name = "admin", role = "owner"},
    {id = 2, name = "guest", role = "viewer"}
  }
}
```

### 3. Game State

Demonstrates arrays of primitives, deeply nested stat blocks, empty tables (`failed = {}`), and a sub-table of key-value pairs (`controls`). Exercises all editor features simultaneously.

```lua
{
  player = {
    name = "Hero",
    level = 12,
    health = 85,
    max_health = 100,
    mana = 42,
    max_mana = 80,
    inventory = {"sword", "shield", "potion", "ring", "map"},
    stats = {
      strength = 15,
      dexterity = 12,
      intelligence = 8,
      vitality = 14
    },
    position = {x = 120.5, y = 340.2, z = 0}
  },
  quests = {
    active = {"find_the_artifact", "defeat_the_dragon"},
    completed = {"tutorial", "first_blood", "side_quest_01"},
    failed = {}
  },
  settings = {
    volume = 0.8,
    fullscreen = true,
    resolution = {width = 1920, height = 1080},
    controls = {
      move_forward = "w",
      move_back = "s",
      jump = "space",
      interact = "e"
    }
  }
}
```

---

## Other Lua Examples

For parity with other BUI binding examples, these Lua examples have been
ported alongside the editor:

| Example | Ported From | Demonstrates |
|---------|------------|-------------|
| `bindings/lua/examples/hello.lua` | Rust `hello.rs`, Python `hello` | Minimal app, `bui.run()` |
| `bindings/lua/examples/counter.lua` | Rust `counter.rs`, Node `counter.js` | Stateful callbacks, `bui.EV_CLICK` |
| `bindings/lua/examples/todos.lua` | Rust `site_components.rs` | List management, text input |
| `bindings/lua/examples/gallery.lua` | Rust `showcase.rs` (widgets) | Widget showcase: button, toggle, checkbox, text, search, number, slider, dropdown, progress, label |
| `bindings/lua/examples/forms.lua` | Rust `forms.rs`, Python `components_demo.py` | All major widget types: button, toggle, checkbox, text, search, hotkey, number, slider, dropdown, enum, radio, swatch, progress, meter, color swatch/picker, decorator |
| `bindings/lua/examples/dashboard.lua` | Rust `site_dashboard.rs` | Complex layout: tabs, panels, progress bars, tab switching via `node:remove()`/`node:add()`, settings forms |
| `bindings/lua/examples/anim.lua` | Rust `site_animation.rs` | Pump-driven frame loop (progress sine wave, HSV color cycling, wave slider, frame counter), toggle play/pause, reset |
| `bindings/lua/examples/lua_table_editor.lua` | Original | Full table editor (see below) |

---

## Implementation Status

All limitations from the original design have been addressed in the current
implementation (`bindings/lua/examples/lua_table_editor.lua`):

| Feature | Implementation | Status |
|---------|---------------|--------|
| Type-aware widget swap | Dynamic swap between `text`, `number`, and `toggle` widgets per entry type | ✅ |
| Collapsible editor panel | Uses BUI `panel` node with native collapse support | ✅ |
| Undo/redo | Lua-side snapshot stack with Ctrl+Z / Ctrl+Shift+Z + toolbar buttons | ✅ |
| Export to file | File browser dialog via `bui.Node("filebrowser")` blocking pump loop (same pattern as C `BK_file_dialog`) | ✅ |
| Targeted subtree replacement | In-place `set_values()` for scalar edits; full rebuild only for structural changes | ✅ |
| Numeric/string key detection | Auto-detection (pure integer → number) + explicit "Numeric key" toggle | ✅ |
| Metatable awareness | `lua_type_name()` and `fmt_val()` check `getmetatable()` for `__name`/`__type` | ✅ |
| Search/filter | Real-time text search via `bui.EV_CHANGE` on a `text` node; filters tree by key/value | ✅ |
| Keyboard shortcuts | `ctx:bind("ctrl+z", ...)` and `ctx:bind("ctrl+shift+z", ...)` | ✅ |
