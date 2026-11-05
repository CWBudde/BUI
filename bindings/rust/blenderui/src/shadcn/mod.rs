//! shadcn/ui's API for the two big components: the **Data Table** (the
//! TanStack Table recipe) and a **virtual list** (TanStack Virtual's API in
//! shadcn's component style). Everything here is a thin adapter: the logic
//! is in the cores (`blenderui_table::tanstack`, `blenderui_vlist`), the
//! painting and input in [`crate::table`] and [`crate::vlist`].
//!
//! ```ignore
//! use blenderui::shadcn::*;
//!
//! // rsx! / #[component] syntax
//! rsx! { <DataTable columns={columns} data={data} filter_column="email" /> }
//! // gpui builder syntax
//! DataTable::new(columns, data).filter_column("email")
//! ```
//!
//! Not in the prelude, like [`crate::table`] and [`crate::vlist`]: it is
//! `use blenderui::shadcn::*;` when you want it.
//!
//! * [`table`]: [`DataTable`], [`ColumnDef`], [`use_table`] and the helper
//!   components (`DataTableColumnHeader`, `DataTablePagination`,
//!   `DataTableViewOptions`, `DataTableToolbar`).
//! * [`virtual_list`]: [`VirtualList`], [`ChatList`] and the
//!   TanStack Virtual option names.

pub mod table;
pub mod virtual_list;

/// shadcn's `Input` is Blender's text field.
pub use crate::components::TextField as Input;
pub use crate::components::{Button, Checkbox};
pub use table::*;
pub use virtual_list::*;

use crate::Rgba;

/// A color of the active theme (`"menu_back.inner"` ...), or `fallback`.
pub(crate) fn theme(key: &str, fallback: Rgba) -> Rgba {
    crate::theme_color(key).map_or(fallback, |[r, g, b, a]| Rgba { r, g, b, a })
}
