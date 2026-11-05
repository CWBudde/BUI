//! A framework-independent, virtualized data table: the logic once, for every
//! UI framework.
//!
//! The design is Blender's spreadsheet editor
//! (`source/blender/editors/space_spreadsheet`) with Blender's other UI
//! optimizations:
//!
//! | Blender | here |
//! |---|---|
//! | `get_visible_rows`, column culling in `spreadsheet_draw.cc` | [`visible`]: O(visible) range math in `f64`, column bisection |
//! | `IndexMask`, `spreadsheet_row_filter.cc` | [`mask::RowMask`] and [`sort`]: sorting and filtering never reorder data |
//! | `GVArray` column values | [`cell::CellSource`] / [`cell::Cell`]: typed, lazy, formatted into stack buffers ([`fmt`]) |
//! | `UI_widgetbase_draw_cache` (batched quads) | [`paint`]: a run of selected rows is one quad |
//! | BLF glyph cache | [`text_cache::TextCache`]: bounded, generation-tagged text-run cache |
//!
//! Layering: [`model::TableModel`] holds all state and is driven by
//! [`interact`] (input) and [`paint`] (output through [`paint::PaintSink`]).
//! A framework adapter owns a model next to its data, translates its events
//! into [`input`] types, implements `PaintSink`, and answers the
//! [`event::TableEvent`]s the model queues (hooks, clipboard). `std` only.

// Deliberate: layout math compares f32 sentinels exactly, the geometry uses short
// names (x, y, w, h, l, r), `u64 as i64` keys flip the sign bit on purpose.
#![allow(clippy::float_cmp, clippy::many_single_char_names, clippy::cast_possible_wrap, clippy::items_after_statements)]

pub mod cell;
pub mod column;
pub mod event;
pub mod fmt;
pub mod geometry;
pub mod input;
pub mod interact;
pub mod mask;
pub mod model;
pub mod paint;
pub mod selection;
pub mod state;
pub mod sort;
pub use blenderui_paint::{geom, text_cache};
pub mod vec_table;
pub mod visible;

#[cfg(test)]
mod alloc_probe;
#[cfg(test)]
mod test_util;

pub use cell::{Cell, CellKind, CellSource, SortKind};
pub use column::{Column, ColumnFixed, ColumnGroup, ColumnLayout, ColumnSort, Text, TextAlign};
pub use event::{Size, TableEvent, TableOptions, TableSelection, TableVisibleRange};
pub use fmt::CellBuf;
pub use geom::{Point, Rect, Rgba};
pub use geometry::{Geometry, Hit};
pub use input::{Button, Key, Mods};
pub use mask::RowMask;
pub use model::{Drag, FrameReport, TableModel};
pub use paint::{paint_table, with_scratch, PaintScratch, PaintSink, Palette};
pub use selection::RowSelection;
pub use state::{Host, TableCore};
pub use text_cache::TextCache;
pub use vec_table::VecTable;
pub use visible::{clamp_scroll, max_scroll, scroll_to_show, visible_cols, visible_rows};
