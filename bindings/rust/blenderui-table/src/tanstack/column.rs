//! [`ColumnDef`]: TanStack's column definition.

use std::sync::Arc;

use crate::cell::Cell;
use crate::column::{BadgeVariant, CellRender, NumberFormat, Text, TextAlign};

/// A row type whose fields can be read by name: what makes
/// `ColumnDef::accessor_key("email")` work like TanStack's `accessorKey`
/// (Rust has no reflection, so the type lists its fields once).
///
/// ```
/// use blenderui_table::{Cell, tanstack::Record};
/// struct Payment { id: String, amount: f64 }
/// impl Record for Payment {
///     const FIELDS: &'static [&'static str] = &["id", "amount"];
///     fn field(&self, field: usize) -> Cell<'_> {
///         match field { 0 => Cell::Str(&self.id), _ => Cell::F64(self.amount, 2) }
///     }
/// }
/// ```
pub trait Record: Send + Sync + 'static {
    /// The field names, in the order of [`field`](Self::field)'s index.
    const FIELDS: &'static [&'static str];

    /// The value of field `field` (an index into [`FIELDS`](Self::FIELDS)).
    fn field(&self, field: usize) -> Cell<'_>;
}

/// Picks the badge variant from the cell's text.
pub(crate) type BadgeFn = Arc<dyn Fn(&str) -> BadgeVariant + Send + Sync>;

/// How a column reads its cell from a row.
pub(crate) enum Accessor<T> {
    /// `accessorFn` / `accessorKey`.
    Read(Arc<dyn for<'a> Fn(&'a T) -> Cell<'a> + Send + Sync>),
    /// A display column (`select`, `actions`): no data of its own.
    Display,
    /// `accessorKey` naming a field the row type does not have.
    Missing(Text),
}

impl<T> Clone for Accessor<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Read(f) => Self::Read(Arc::clone(f)),
            Self::Display => Self::Display,
            Self::Missing(k) => Self::Missing(k.clone()),
        }
    }
}

/// A column header: shadcn's `header` string or header component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Header {
    /// `header: "Email"`.
    Text(Text),
    /// `header: ({ column }) => <DataTableColumnHeader column={column} title="Email" />`:
    /// the title with a sort icon and a menu (Asc, Desc, Hide).
    Menu(Text),
    /// The select-all checkbox (`<Checkbox checked={table.getIsAllPageRowsSelected()} .. />`).
    SelectAll,
}

impl From<&'static str> for Header {
    fn from(s: &'static str) -> Self {
        Self::Text(s.into())
    }
}

impl From<String> for Header {
    fn from(s: String) -> Self {
        Self::Text(s.into())
    }
}

impl Header {
    /// A sortable header with a menu.
    #[must_use]
    pub fn menu(title: impl Into<Text>) -> Self {
        Self::Menu(title.into())
    }

    /// The header's text (empty for the select-all checkbox).
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Self::Text(t) | Self::Menu(t) => t,
            Self::SelectAll => "",
        }
    }
}

/// TanStack's `meta`: what the painted table needs to know beyond the
/// definition (alignment, number format, pinning).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnMeta {
    pub align: TextAlign,
    pub format: NumberFormat,
    /// Stays at the left edge while the table scrolls sideways.
    pub pinned: bool,
}

impl ColumnMeta {
    /// Right-aligned cells.
    #[must_use]
    pub fn right() -> Self {
        Self { align: TextAlign::Right, ..Self::default() }
    }

    /// Centered cells.
    #[must_use]
    pub fn center() -> Self {
        Self { align: TextAlign::Center, ..Self::default() }
    }

    /// US-dollar numbers (`$1,234.50`).
    #[must_use]
    pub fn usd(mut self) -> Self {
        self.format = NumberFormat::usd();
        self
    }

    /// Any number format.
    #[must_use]
    pub fn format(mut self, format: NumberFormat) -> Self {
        self.format = format;
        self
    }

    /// Pinned to the left edge.
    #[must_use]
    pub const fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }
}

/// A column definition (TanStack's `ColumnDef<TData>`).
///
/// ```ignore
/// let columns = vec![
///     ColumnDef::select(),
///     ColumnDef::accessor_key("status").header("Status").badge(|s| BadgeVariant::Secondary),
///     ColumnDef::accessor_key("email").header(Header::menu("Email")),
///     ColumnDef::accessor_key("amount").header("Amount").meta(ColumnMeta::right().usd()),
/// ];
/// ```
#[allow(clippy::struct_excessive_bools)]
pub struct ColumnDef<T> {
    /// `id` (defaults to the accessor key).
    pub id: Text,
    pub header: Header,
    /// How the cells are drawn (`cell`).
    pub cell: CellRender,
    /// `enableSorting`.
    pub enable_sorting: bool,
    /// `enableHiding`.
    pub enable_hiding: bool,
    /// `enableColumnFilter`.
    pub enable_column_filter: bool,
    /// `enableGlobalFilter`.
    pub enable_global_filter: bool,
    /// `enableResizing`.
    pub enable_resizing: bool,
    /// `size`, px.
    pub size: f32,
    /// `minSize`, px.
    pub min_size: f32,
    /// `maxSize`, px.
    pub max_size: f32,
    pub meta: ColumnMeta,
    pub(crate) accessor: Accessor<T>,
    pub(crate) badge: Option<BadgeFn>,
}

impl<T> Clone for ColumnDef<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            header: self.header.clone(),
            cell: self.cell,
            enable_sorting: self.enable_sorting,
            enable_hiding: self.enable_hiding,
            enable_column_filter: self.enable_column_filter,
            enable_global_filter: self.enable_global_filter,
            enable_resizing: self.enable_resizing,
            size: self.size,
            min_size: self.min_size,
            max_size: self.max_size,
            meta: self.meta.clone(),
            accessor: self.accessor.clone(),
            badge: self.badge.clone(),
        }
    }
}

impl<T> std::fmt::Debug for ColumnDef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColumnDef").field("id", &self.id).field("header", &self.header).field("cell", &self.cell).finish_non_exhaustive()
    }
}

impl<T> ColumnDef<T> {
    fn base(id: Text, accessor: Accessor<T>) -> Self {
        Self {
            header: Header::Text(id.clone()),
            id,
            cell: CellRender::Text,
            enable_sorting: true,
            enable_hiding: true,
            enable_column_filter: true,
            enable_global_filter: true,
            enable_resizing: true,
            size: 150.0,
            min_size: 20.0,
            max_size: 1200.0,
            meta: ColumnMeta::default(),
            accessor,
            badge: None,
        }
    }

    /// `accessorFn` with an `id`: any function of the row.
    #[must_use]
    pub fn accessor_fn(id: impl Into<Text>, f: impl for<'a> Fn(&'a T) -> Cell<'a> + Send + Sync + 'static) -> Self {
        Self::base(id.into(), Accessor::Read(Arc::new(f)))
    }

    /// A display column (`id` only): its cells come from the cell kind, as
    /// shadcn's `select` and `actions` columns do.
    #[must_use]
    pub fn display(id: impl Into<Text>) -> Self {
        let mut c = Self::base(id.into(), Accessor::Display);
        c.enable_sorting = false;
        c.enable_column_filter = false;
        c.enable_global_filter = false;
        c
    }

    /// `id`.
    #[must_use]
    pub fn id(mut self, id: impl Into<Text>) -> Self {
        self.id = id.into();
        self
    }

    /// `header`: a string, or [`Header::menu`] for the sortable header with a menu.
    #[must_use]
    pub fn header(mut self, header: impl Into<Header>) -> Self {
        self.header = header.into();
        self
    }

    /// `cell`: how the cells are drawn.
    #[must_use]
    pub const fn cell(mut self, cell: CellRender) -> Self {
        self.cell = cell;
        self
    }

    /// A [`CellRender::Badge`] column whose variant is chosen from the
    /// cell's text.
    #[must_use]
    pub fn badge(mut self, f: impl Fn(&str) -> BadgeVariant + Send + Sync + 'static) -> Self {
        self.cell = CellRender::Badge;
        self.badge = Some(Arc::new(f));
        self
    }

    #[must_use]
    pub const fn enable_sorting(mut self, on: bool) -> Self {
        self.enable_sorting = on;
        self
    }

    #[must_use]
    pub const fn enable_hiding(mut self, on: bool) -> Self {
        self.enable_hiding = on;
        self
    }

    #[must_use]
    pub const fn enable_column_filter(mut self, on: bool) -> Self {
        self.enable_column_filter = on;
        self
    }

    #[must_use]
    pub const fn enable_global_filter(mut self, on: bool) -> Self {
        self.enable_global_filter = on;
        self
    }

    #[must_use]
    pub const fn enable_resizing(mut self, on: bool) -> Self {
        self.enable_resizing = on;
        self
    }

    /// `size`, px.
    #[must_use]
    pub const fn size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }

    /// `minSize`, px.
    #[must_use]
    pub const fn min_size(mut self, px: f32) -> Self {
        self.min_size = px;
        self
    }

    /// `maxSize`, px.
    #[must_use]
    pub const fn max_size(mut self, px: f32) -> Self {
        self.max_size = px;
        self
    }

    /// `meta`.
    #[must_use]
    pub fn meta(mut self, meta: ColumnMeta) -> Self {
        self.meta = meta;
        self
    }

    /// Whether this column reads data (it is not a display column).
    #[must_use]
    pub const fn has_accessor(&self) -> bool {
        matches!(self.accessor, Accessor::Read(_))
    }

    pub(crate) fn read<'a>(&self, row: &'a T) -> Cell<'a> {
        match &self.accessor {
            Accessor::Read(f) => f(row),
            _ => Cell::Empty,
        }
    }
}

/// shadcn's `select` column: a checkbox per row and a select-all checkbox
/// in the header (`table.getIsAllPageRowsSelected()`).
#[must_use]
pub fn select_column<T>() -> ColumnDef<T> {
    ColumnDef::display("select")
        .header(Header::SelectAll)
        .cell(CellRender::Checkbox)
        .meta(ColumnMeta::center())
        .enable_hiding(false)
        .enable_resizing(false)
        .size(40.0)
        .min_size(40.0)
        .max_size(40.0)
}

/// shadcn's `actions` column: a "more" button per row; a click is reported
/// as a row action.
#[must_use]
pub fn actions_column<T>() -> ColumnDef<T> {
    ColumnDef::display("actions")
        .header("")
        .cell(CellRender::Actions)
        .enable_hiding(false)
        .enable_resizing(false)
        .size(48.0)
        .min_size(48.0)
        .max_size(48.0)
}

impl<T: Record> ColumnDef<T> {
    /// `accessorKey`: the field named `key` ([`Record::FIELDS`]); the `id`
    /// and the header text default to it. An unknown key is reported by
    /// [`Table::new`](super::Table::new), where the columns are validated.
    #[must_use]
    pub fn accessor_key(key: impl Into<Text>) -> Self {
        let key = key.into();
        let accessor = match T::FIELDS.iter().position(|f| *f == key) {
            Some(ix) => Accessor::Read(Arc::new(move |row: &T| row.field(ix))),
            None => Accessor::Missing(key.clone()),
        };
        Self::base(key, accessor)
    }
}
