//! TanStack Table v8's model (the engine under shadcn/ui's Data Table) on
//! top of the table core, framework-independent.
//!
//! | TanStack | here |
//! |---|---|
//! | `ColumnDef` (`accessorKey`, `accessorFn`, `id`, `header`, `cell`, `size`, `meta` ...) | [`ColumnDef`] |
//! | `useReactTable` options / table instance | [`TableOptions`] / [`Table`] |
//! | `SortingState`, `ColumnFiltersState`, `VisibilityState`, `RowSelectionState`, `PaginationState` | the types of the same names |
//! | controlled `state` and `onXChange` | [`PartialState`] and [`Change`] |
//! | `getCoreRowModel` / `getFilteredRowModel` / `getSortedRowModel` / `getPaginationRowModel` | [`Table::core_row_model`], [`Table::filtered_row_model`], [`Table::sorted_row_model`], [`Table::row_model`] |
//!
//! The row models are index lists ([`RowMask`](crate::RowMask), Blender's
//! `IndexMask`): sorting, filtering and paging never copy a row, and a
//! sort uses the table core's typed keys. The painted cells come from
//! [`Table`]'s [`CellSource`](crate::CellSource) (the current page), so the
//! table stays O(visible cells) whatever the row count. A shadcn
//! `cell: ({ row }) => <JSX/>` is a [`CellRender`](crate::CellRender): the
//! closest typed equivalent.

mod column;
mod state;
mod table;

pub use column::{actions_column, select_column, ColumnDef, ColumnMeta, Header, Record};
pub use state::{
    Change, ColumnFilter, ColumnFiltersState, PaginationState, PartialState, RowSelectionState, SortingRule, SortingState, TableState,
    VisibilityState,
};
pub use table::{ColumnInfo, Table, TableError, TableOptions};
