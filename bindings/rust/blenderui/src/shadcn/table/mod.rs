//! shadcn/ui's Data Table: TanStack's `ColumnDef`, table state and row
//! models over the painted table engine.
//!
//! | shadcn / TanStack | here |
//! |---|---|
//! | `ColumnDef<TData>` | [`ColumnDef`] |
//! | `accessorKey: "email"` | `ColumnDef::accessor_key("email")` (the row type implements [`Record`]) |
//! | `accessorFn`, `id` | `ColumnDef::accessor_fn("id", \|row\| Cell::Str(&row.email))` |
//! | `header: "Email"` / `({ column }) => <DataTableColumnHeader .../>` | `.header("Email")` / `.header(Header::menu("Email"))` |
//! | `cell: ({ row }) => <JSX/>` | `.cell(CellRender::..)`, `.badge(..)`, `.meta(..)`: typed cells (see the docs) |
//! | `useReactTable({ data, columns, ... })` | [`use_table`] + [`DataTable`] |
//! | `<DataTable columns={columns} data={data} />` | the same in `rsx!`, or `DataTable::new(columns, data)` |
//! | `<DataTablePagination table={table} />` | [`DataTablePagination`] |
//! | `<DataTableViewOptions table={table} />` | [`DataTableViewOptions`] |
//! | `onSortingChange`, `state`, `initialState` ... | `on_sorting_change`, `sorting`, `initial_state` ... |

mod column_header;
mod data_table;
mod delegate;
mod handle;
mod link;
mod pagination;
mod toolbar;
mod view_options;

pub use blenderui_table::tanstack::{
    actions_column, select_column, Change, ColumnDef, ColumnFilter, ColumnFiltersState, ColumnInfo, ColumnMeta, Header, PaginationState, PartialState, Record,
    RowSelectionState, SortingRule, SortingState, Table, TableError, TableOptions, TableState, VisibilityState,
};
pub use blenderui_table::{BadgeVariant, Cell, CellRender, ColumnSort, NumberFormat};
pub use column_header::DataTableColumnHeader;
pub use data_table::DataTable;
pub use handle::{use_table, Command, Snapshot, TableHandle};
pub use pagination::DataTablePagination;
pub use toolbar::DataTableToolbar;
pub use view_options::DataTableViewOptions;
