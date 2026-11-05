//! `DataTableToolbar`: the filter `Input` and the view options above a table
//! (the top row of shadcn's Data Table example).

use super::handle::TableHandle;
use super::view_options::DataTableViewOptions;
use crate::prelude::*;

/// A text field that filters column `filter_column` (`column.setFilterValue`),
/// or every column (`table.setGlobalFilter`) when no column is named, and the
/// "View" menu at the right (hide it with `view_options={false}`).
#[component]
pub fn DataTableToolbar(table: TableHandle, filter_column: Option<String>, placeholder: Option<String>, view_options: Option<bool>) {
    let current = filter_column.as_deref().map_or_else(|| table.global_filter(), |c| table.column_filter_value(c));
    let hint = placeholder.unwrap_or_else(|| filter_column.as_ref().map_or_else(|| "Filter...".to_owned(), |c| format!("Filter {c}...")));
    let on_change = move |e: &ChangeEvent, _: &mut Window, cx: &mut App| match &filter_column {
        Some(c) => table.set_column_filter_value(c.clone(), e.value.clone(), cx),
        None => table.set_global_filter(e.value.clone(), cx),
    };
    rsx! {
        <div flex items-center gap-2>
            <TextField w-64 placeholder={hint} value={current} onChange={on_change} />
            {if view_options.unwrap_or(true) { <div ml-auto><DataTableViewOptions table={table} /></div> }}
        </div>
    }
}
