# blenderui-table

A framework-independent, virtualized data table: the logic once, for every UI
framework. `std` only, no dependencies.

The design is Blender's spreadsheet editor
(`source/blender/editors/space_spreadsheet`) with Blender's other UI
optimizations. A frame costs O(visible cells), however many rows there are.

| Blender | Here |
|---|---|
| `get_visible_rows`, column culling in `spreadsheet_draw.cc` | `visible`: range math in `f64` (saturating, NaN-safe), column bisection |
| `IndexMask`, `spreadsheet_row_filter.cc` | `mask::RowMask`, `sort`: sorting and filtering never reorder data |
| `GVArray` column values | `cell::CellSource` / `Cell`: typed, lazy cells, formatted into stack buffers (`fmt::CellBuf`, no allocation) |
| `UI_widgetbase_draw_cache` (batched quads) | `paint`: a selection of any number of rows is one quad, a column separator one quad |
| BLF glyph cache | `text_cache::TextCache`: bounded, generation-tagged cache of shaped text runs or widths |

## Layers

```text
 your data ----> CellSource            (cells, typed; sort kind)
 TableModel      state: columns, mask, selection, scroll, size
   interact      pointer / wheel / keyboard  -> model + queued TableEvents
   paint         model + data -> PaintSink   (rects, text, clips)
 TableCore<H>    model + delegate + the glue every adapter needs, behind `Host`
```

An adapter for a framework (`blenderui`'s `table` module, `blenderui-table-gpui`)
implements three small things and nothing else:

1. `Host`: names the framework's window and context types and forwards the
   delegate hooks, the clipboard and redraw calls (a dozen one-liners).
2. `PaintSink`: draws rectangles, lines and text, scopes clips, and owns the
   framework's text cache.
3. An element that feeds `TableCore` the framework's pointer, wheel and key
   events (as `Button`, `Mods`, `Key`) and calls `paint_table` while painting.

Sorting, filtering, selection, keyboard navigation, column resize and move,
TSV copy, scrolling, hit testing and every pixel of layout are in this crate.

## Sorting

`sort::sorted_by_column` sorts a `RowMask`, stable, parallel above 32k rows:

* A column whose cells are all numbers of one variant, or all text, sorts
  through **typed keys**: one pass extracts an order-preserving `u64` per row
  (integers, floats by `f64::total_cmp`, the first 8 bytes of text), then
  `(key, position)` pairs sort with no cell access and no dynamic call per
  comparison. Text resolves equal prefixes with a stable sort of those runs.
* Anything else (mixed variants, `SortKind::Custom`) sorts with a comparator
  on the index list: the generic path the typed one must equal. The tests
  check they agree for every direction, on filtered masks, with `NaN`,
  infinities and ties.

## Tests

`cargo test -p blenderui-table`: unit tests for every module (reference
implementations, seeded random inputs), model tests through the public API
(input, sorting, keyboard, copy, drags), a randomized event soak that checks
the invariants after every event, extreme sizes (`usize::MAX / 4` rows, `NaN`
and infinite scrolls, zero and negative sizes), and counting-allocator tests
that a steady-state paint and a text-cache hit allocate nothing.

License: GPL-3.0-or-later, like the rest of BUI.
