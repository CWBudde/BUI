//! `DataTableViewOptions`: shadcn's `data-table-view-options.tsx`, the
//! column-visibility dropdown.

use super::handle::TableHandle;
use crate::prelude::*;
use crate::shadcn::theme;

/// A "View" button that opens a list of the columns that can be hidden, each
/// with a check that toggles it (`column.toggleVisibility`).
#[component]
pub fn DataTableViewOptions(table: TableHandle) {
    let (open, set_open) = use_state(false);
    let (text, hover) = (theme("menu_item.text", black()), theme("menu_item.inner_sel", Rgba::TRANSPARENT));
    let accent = theme("menu_item.inner_sel", Rgba { r: 0.3, g: 0.55, b: 0.85, a: 1.0 });
    let rows = table.columns().into_iter().filter(|c| c.can_hide).map(|c| {
        let id = c.id.clone();
        let mark = div()
            .w(px(12.0))
            .h(px(12.0))
            .rounded_sm()
            .border_1()
            .border_color(text)
            .when(c.visible, |d| d.bg(accent).border_color(accent));
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .hover(move |s| s.bg(hover).text_color(theme("menu_item.text_sel", black())))
            .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut App| table.toggle_column_visibility(id.clone(), None, cx))
            .child(mark)
            .child(c.title.to_string())
    });
    let panel = div()
        .absolute()
        .right(px(0.0))
        .top(px(30.0))
        .z_index(20)
        .min_w(px(160.0))
        .flex()
        .flex_col()
        .p_1()
        .rounded_md()
        .border_1()
        .border_color(theme("menu_back.outline", Rgba::TRANSPARENT))
        .bg(theme("menu_back.inner", Rgba { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }))
        .text_color(text)
        .text_size(px(13.0))
        .child(div().px_2().py_1().opacity(0.6).child("Toggle columns"))
        .children(rows);
    rsx! {
        <div relative>
            <Button onClick={move || set_open(!open())}>"View"</Button>
            {if open() { panel }}
        </div>
    }
}
