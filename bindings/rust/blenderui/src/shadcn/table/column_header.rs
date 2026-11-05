//! `DataTableColumnHeader`: the sort menu of a sortable header (Asc, Desc,
//! Hide), as in shadcn's `data-table-column-header.tsx`.

use super::handle::TableHandle;
use crate::prelude::*;
use crate::shadcn::theme;

/// The menu of column `column`'s header: Asc, Desc and Hide, driving `table`.
/// The [`DataTable`](super::DataTable) opens it under a header made with
/// [`Header::menu`](super::Header::menu); it is a component of its own so it
/// can also be shown anywhere else (it closes the table's header menu when an
/// item is chosen).
#[component]
pub fn DataTableColumnHeader(table: TableHandle, column: &str) {
    let info = table.columns().into_iter().find(|c| c.id == column);
    let (can_sort, can_hide) = info.as_ref().map_or((false, false), |c| (c.can_sort, c.can_hide));
    let item = |label: &'static str, enabled: bool, id: String, run: fn(TableHandle, &str, &mut App)| {
        let (hover, text) = (theme("menu_item.inner_sel", Rgba::TRANSPARENT), theme("menu_item.text", black()));
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .text_color(text)
            .when(enabled, |d| d.cursor_pointer().hover(move |s| s.bg(hover).text_color(theme("menu_item.text_sel", black()))))
            .when(!enabled, |d| d.opacity(0.4))
            .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                if enabled {
                    run(table, &id, cx);
                    table.menu.set(None);
                }
            })
            .child(label)
    };
    div()
        .flex()
        .flex_col()
        .p_1()
        .rounded_md()
        .border_1()
        .border_color(theme("menu_back.outline", Rgba::TRANSPARENT))
        .bg(theme("menu_back.inner", Rgba { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }))
        .text_size(px(13.0))
        .child(item("Asc", can_sort, column.to_string(), |t, id, cx| t.toggle_column_sorting(id.to_string(), Some(false), cx)))
        .child(item("Desc", can_sort, column.to_string(), |t, id, cx| t.toggle_column_sorting(id.to_string(), Some(true), cx)))
        .child(div().h(px(1.0)).my_1().bg(theme("menu_back.outline", Rgba::TRANSPARENT)))
        .child(item("Hide", can_hide, column.to_string(), |t, id, cx| t.toggle_column_visibility(id.to_string(), Some(false), cx)))
}
