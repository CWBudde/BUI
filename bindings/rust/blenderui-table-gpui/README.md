# blenderui-table-gpui

A `DataTable` for [gpui](https://gpui.rs) with the API of gpui-component's,
built on the shared [`blenderui-table`](../blenderui-table) core. It paints
only the visible cells, straight into the window's scene, instead of building
an element per cell.

```rust
use blenderui_table_gpui::*;

struct People;

impl CellSource for People {
    fn cell(&self, row: usize, col: usize) -> Cell<'_> {
        match col {
            0 => Cell::U64(row as u64),
            1 => Cell::Str(["Cube", "Sphere", "Torus"][row % 3]),
            _ => Cell::F64(row as f64 / 8.0, 2),
        }
    }
}

impl TableDelegate for People {
    fn columns_count(&self, _: &App) -> usize { 3 }
    fn rows_count(&self, _: &App) -> usize { 1_000_000 }
    fn column(&self, ix: usize, _: &App) -> Column {
        Column::new(format!("c{ix}"), ["ID", "Name", "Size"][ix]).sortable().width(px(110.0))
    }
}

let table = cx.new(|cx| TableState::new(People, window, cx));
// in render
div().size_full().child(DataTable::new(&table).stripe(true).on_select(|row, _, _| println!("{row}")))
```

Names follow gpui-component: `TableDelegate` (`columns_count`, `rows_count`,
`column`, `perform_sort`, `fit_column`, `move_column`, `group_headers`,
`loading`, `has_more`, `load_more`, `visible_rows_changed` ...), `Column`,
`ColumnSort`, `ColumnGroup`, `TableState` (`selected_row`, `set_selected_row`,
`visible_range`, `scroll_to_row`, `scroll_to_col`, `col_width`, `fit_col` ...),
`TableSelection`, `TableEvent`, `Size`, `DataTable` (`stripe`, `bordered`,
`header_row_height`, `scrollbar_visible`, `on_select` ...).

## What differs from gpui-component

* **Cells are values, not elements.** `CellSource::cell` returns a typed
  `Cell` (a borrowed `&str`, an integer, a float with its decimals); the table
  formats numbers into a stack buffer and paints the text. Where
  gpui-component's `render_td` builds an element for every visible cell on
  every frame, a frame here builds two elements in total.
* **Escape hatch for rich cells.** Return `true` from `has_custom_cells` and
  `is_custom_cell(row, col)` for the few cells that need real elements
  (buttons, badges), return `Cell::Empty` from `cell` for them, and build the
  element in `render_td`. Only visible custom cells are built.
* **The table sorts for you.** A header click builds a row index list (typed
  keys, parallel, stable) and the data is never reordered. To sort your own
  data in `perform_sort` instead, return `false` from `sort_by_mask`.
* **`CellSource` is `Sync`** (sorting reads cells from several threads).
* No built-in context menu popup or accessibility tree yet.

## How it is fast

The frame work is Blender's, in the shared core: O(visible) range math,
column culling by bisection, one quad per run of selected rows and per column
separator, numbers formatted without allocation, sorting by typed keys. The
gpui-specific parts are a `PaintSink` that calls `Window::paint_quad` and
paints shaped lines, and a cache of shaped lines keyed by `(text, color,
size)`, kept across frames so scrolling never reshapes a string it showed a
frame ago (it is cleared when the scale factor changes, and bounded to twice
the visible cells, 2,048 at least).

## Building

This crate is its own Cargo workspace, outside the main one, because gpui is a
very large build. On Windows gpui needs `fxc.exe`:

```text
set GPUI_FXC_PATH=C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\fxc.exe
cargo build --release -j 1
cargo test -j 1                       # the adapter on gpui's test platform
```

## Benchmark against gpui-component 0.9.0

```text
cargo build --release --features bench --example bench_gpui -j 1
node bench/run.mjs                    # 3 runs per configuration, medians
```

See `bench/results.md` for the last run and the website's data-table page for
the method and the caveats.

License: GPL-2.0-or-later, like the rest of BUI.
