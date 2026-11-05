//! `DataTablePagination`: shadcn's `data-table-pagination.tsx`.

use super::handle::TableHandle;
use crate::prelude::*;

/// Rows per page, "Page x of y", first / previous / next / last, and "N of M
/// row(s) selected.". `page_sizes` are the choices of the "Rows per page"
/// menu (default 10, 20, 30, 40, 50).
#[component]
pub fn DataTablePagination(table: TableHandle, page_sizes: Option<Vec<usize>>) {
    let sizes = page_sizes.unwrap_or_else(|| vec![10, 20, 30, 40, 50]);
    let snap = table.snapshot();
    let selected = format!("{} of {} row(s) selected.", snap.selected_count, snap.row_count);
    let page = format!("Page {} of {}", snap.pagination.page_index + 1, snap.page_count);
    let (prev, next) = (table.can_previous_page(), table.can_next_page());
    let size = snap.pagination.page_size;
    let choices = sizes.clone();
    rsx! {
        <div flex items-center justify-between gap-4 px-2 text-sm>
            <div flex-1 whitespace-nowrap opacity-70>{selected}</div>
            <div flex items-center gap-6>
                <div flex items-center gap-2>
                    <span whitespace-nowrap>"Rows per page"</span>
                    <Dropdown onChange={move |e: &ChangeEvent, _: &mut Window, cx: &mut App| {
                        if let Some(&n) = choices.get(e.number as usize) {
                            table.set_page_size(n, cx);
                        }
                    }}>
                        {for n in sizes { <option selected={n == size}>{n.to_string()}</option> }}
                    </Dropdown>
                </div>
                <div flex w-28 items-center justify-center>{page}</div>
                <div flex items-center gap-2>
                    <Button disabled={!prev} onClick={move |_: &ClickEvent, _: &mut Window, cx: &mut App| table.first_page(cx)}>"<<"</Button>
                    <Button disabled={!prev} onClick={move |_: &ClickEvent, _: &mut Window, cx: &mut App| table.previous_page(cx)}>"<"</Button>
                    <Button disabled={!next} onClick={move |_: &ClickEvent, _: &mut Window, cx: &mut App| table.next_page(cx)}>">"</Button>
                    <Button disabled={!next} onClick={move |_: &ClickEvent, _: &mut Window, cx: &mut App| table.last_page(cx)}>">>"</Button>
                </div>
            </div>
        </div>
    }
}
