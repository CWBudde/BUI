//! The table's vocabulary: options, selection kinds, events, the visible range.

use std::ops::Range;

use crate::column::ColumnSort;

/// What is selected (gpui-component's `TableSelection`; the row is the
/// source row index, whatever the sort order).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum TableSelection {
    #[default]
    None,
    Row(usize),
    Column(usize),
    Cell(usize, usize),
}

/// Events the table raises (gpui-component's `TableEvent`; rows are source
/// row indices, columns source column indices unless noted).
#[derive(Clone, Debug, PartialEq)]
pub enum TableEvent {
    SelectRow(usize),
    DoubleClickedRow(usize),
    SelectColumn(usize),
    SelectCell(usize, usize),
    DoubleClickedCell(usize, usize),
    /// A column was resized: all widths, view order.
    ColumnWidthsChanged(Vec<f32>),
    /// A column moved: from, to (view indices).
    MoveColumn(usize, usize),
    RightClickedRow(Option<usize>),
    RightClickedCell(usize, usize),
    ClearSelection,
    /// A header click sorted (source column, sort).
    Sorted(usize, ColumnSort),
    /// Ctrl+C: this text goes to the clipboard.
    Copied(String),
    /// A double click on a column's border asks the delegate to fit it
    /// (source column). Adapters answer it and do not forward it.
    FitColumn(usize),
    /// A click on a [`CellRender::Checkbox`](crate::CellRender::Checkbox)
    /// cell (source row, source column). The row selection is the
    /// delegate's: the table does not change it.
    CheckRow(usize, usize),
    /// A click on the header of a checkbox column (source column).
    CheckAll(usize),
    /// A click on a [`CellRender::Actions`](crate::CellRender::Actions) cell
    /// (source row, source column).
    CellAction(usize, usize),
    /// A click on the header of a [`Column::header_menu`](crate::Column::header_menu)
    /// column (source column): open its menu.
    HeaderMenu(usize),
}

/// Rows and columns in view (gpui-component's `TableVisibleRange`). Rows are
/// display rows, columns view columns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableVisibleRange {
    pub(crate) rows: Range<usize>,
    pub(crate) cols: Range<usize>,
}

impl TableVisibleRange {
    #[must_use]
    pub const fn rows(&self) -> &Range<usize> {
        &self.rows
    }

    #[must_use]
    pub const fn cols(&self) -> &Range<usize> {
        &self.cols
    }
}

/// Row height presets (gpui-component's `Size`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Size {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    #[must_use]
    pub const fn factor(self) -> f32 {
        match self {
            Self::XSmall => 0.8,
            Self::Small => 0.9,
            Self::Medium => 1.0,
            Self::Large => 1.25,
        }
    }
}

/// Table settings (the builder methods of `TableState` and `DataTable`).
#[derive(Copy, Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct TableOptions {
    pub sortable: bool,
    pub col_resizable: bool,
    pub col_movable: bool,
    pub row_selectable: bool,
    pub col_selectable: bool,
    pub cell_selectable: bool,
    /// The left index column (Blender's `draw_left_column_cell`).
    pub row_header: bool,
    pub select_on_modifier_click: bool,
    pub loop_selection: bool,
    pub stripe: bool,
    pub bordered: bool,
    pub size: Size,
    pub header_row_height: Option<f32>,
    /// Text size in logical pixels; `None`: 0.6 of the UI unit (Blender's ratio).
    pub text_size: Option<f32>,
    pub fixed_cols_border: bool,
    /// Vertical and horizontal scrollbars.
    pub scrollbars: (bool, bool),
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            sortable: true,
            col_resizable: true,
            col_movable: true,
            row_selectable: true,
            col_selectable: false,
            cell_selectable: false,
            row_header: false,
            select_on_modifier_click: true,
            loop_selection: true,
            stripe: false,
            bordered: true,
            size: Size::Medium,
            header_row_height: None,
            text_size: None,
            fixed_cols_border: true,
            scrollbars: (true, true),
        }
    }
}

/// The builder methods of `TableState` (gpui-component's names), generated
/// once for every adapter: `table_option_builders!();` inside an `impl` block
/// of a type whose `self.opts` (directly or through `DerefMut`) is a [`TableOptions`].
#[macro_export]
macro_rules! table_option_builders {
    () => {
        $crate::table_option_builders!(@each
            loop_selection col_movable col_resizable sortable row_selectable
            select_on_modifier_click col_selectable cell_selectable row_header);
    };
    (@each $($name:ident)+) => {
        $(
            #[must_use]
            pub fn $name(mut self, on: bool) -> Self {
                self.opts.$name = on;
                self
            }
        )+
    };
}
