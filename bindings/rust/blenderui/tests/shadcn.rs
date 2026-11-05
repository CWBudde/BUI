//! shadcn's Data Table and the virtual list, end to end in off-screen
//! windows: the rsx! and builder syntaxes, sorting, filtering, pagination,
//! selection, visibility, controlled and uncontrolled state.

use std::cell::Cell as StdCell;
use std::rc::Rc;
use std::sync::Arc;

use blenderui::prelude::*;
use blenderui::shadcn::*;

const STATUS: [&str; 4] = ["pending", "processing", "success", "failed"];

struct Payment {
    id: String,
    status: &'static str,
    email: String,
    amount: f64,
}

impl Record for Payment {
    const FIELDS: &'static [&'static str] = &["id", "status", "email", "amount"];
    fn field(&self, field: usize) -> Cell<'_> {
        match field {
            0 => Cell::Str(&self.id),
            1 => Cell::Str(self.status),
            2 => Cell::Str(&self.email),
            _ => Cell::F64(self.amount, 2),
        }
    }
}

fn payments(n: usize) -> Arc<[Payment]> {
    (0..n)
        .map(|i| Payment {
            id: format!("p{i:04}"),
            status: STATUS[i % 4],
            email: format!("user{}@example.com", (i * 37) % 100),
            amount: 10.0 + ((i * 7919) % 100_000) as f64 / 100.0,
        })
        .collect()
}

fn columns() -> Arc<[ColumnDef<Payment>]> {
    vec![
        select_column(),
        ColumnDef::accessor_key("status").header("Status").size(120.0).badge(|s| match s {
            "success" => BadgeVariant::Default,
            "failed" => BadgeVariant::Destructive,
            _ => BadgeVariant::Secondary,
        }),
        ColumnDef::accessor_key("email").header(Header::menu("Email")).size(240.0),
        ColumnDef::accessor_key("amount").header("Amount").size(120.0).meta(ColumnMeta::right().usd()),
        actions_column(),
    ]
    .into()
}

/// The table, with whichever syntax and props a test wants.
struct View {
    columns: Arc<[ColumnDef<Payment>]>,
    data: Arc<[Payment]>,
    handle: StdCell<Option<TableHandle>>,
    mode: Mode,
    changes: Rc<StdCell<usize>>,
}

#[derive(Copy, Clone)]
enum Mode {
    /// `<DataTable table columns data />`, no toolbar.
    Rsx,
    /// `DataTable::new(..)` builder chain with the toolbar.
    Builder,
    /// Controlled sorting held by the view (the signal), applied back.
    Controlled,
    /// Controlled sorting that never changes: the parent refuses.
    Refusing,
}

impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let table = use_table();
        self.handle.set(Some(table));
        let (columns, data) = (self.columns.clone(), self.data.clone());
        let sorting = use_signal(SortingState::new());
        let changes = self.changes.clone();
        let inner: AnyElement = match self.mode {
            Mode::Rsx => rsx! { <DataTable table={table} columns={columns} data={data} toolbar={false} h={px(300.0)} w={px(900.0)} /> }.into_any_element(),
            Mode::Builder => DataTable::new(columns, data).table(table).filter_column("email").w(px(900.0)).h(px(380.0)).into_any_element(),
            Mode::Controlled => DataTable::new(columns, data)
                .table(table)
                .toolbar(false)
                .sorting(sorting.get())
                .on_sorting_change(move |s| sorting.set(s.clone()))
                .w(px(900.0))
                .h(px(300.0))
                .into_any_element(),
            Mode::Refusing => DataTable::new(columns, data)
                .table(table)
                .toolbar(false)
                .sorting(SortingState::new())
                .on_sorting_change(move |_| changes.set(changes.get() + 1))
                .w(px(900.0))
                .h(px(300.0))
                .into_any_element(),
        };
        body().m_0().child(inner)
    }
}

fn open(cx: &mut App, mode: Mode, n: usize) -> (WindowId, Entity<View>) {
    let opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(900.0), px(420.0)) })),
        ..Default::default()
    };
    let mut view = None;
    let win = cx
        .open_window(opts, |_, cx| {
            let v = cx.new(|_| View { columns: columns(), data: payments(n), handle: StdCell::new(None), mode, changes: Rc::new(StdCell::new(0)) });
            view = Some(v);
            v
        })
        .unwrap();
    (win.id, view.unwrap())
}

fn frames(cx: &mut App, id: WindowId) -> (usize, usize, Vec<u32>) {
    // the first frames create the table and learn its size
    for _ in 0..4 {
        cx.draw(id);
    }
    cx.draw(id)
}

fn handle(cx: &App, v: &Entity<View>) -> TableHandle {
    v.read(cx).handle.get().expect("rendered")
}

fn distinct(img: &(usize, usize, Vec<u32>)) -> usize {
    let mut v = img.2.clone();
    v.sort_unstable();
    v.dedup();
    v.len()
}

fn run(theme: &str, f: impl FnOnce(&mut App)) {
    Application::headless().theme(theme).run(f);
}

// the table sits at the window's origin without a toolbar: header 22 px, rows 20 px
const HEADER_Y: f32 = 11.0;
const fn row_y(r: usize) -> f32 {
    22.0 + 20.0 * r as f32 + 10.0
}

#[test]
fn rsx_data_table_renders_a_page() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Rsx, 95);
        let img = frames(cx, id);
        let t = handle(cx, &v).snapshot();
        assert_eq!((t.core_row_count, t.row_count, t.page_rows, t.page_count), (95, 95, 10, 10));
        assert_eq!(t.columns.iter().map(|c| c.id.as_ref()).collect::<Vec<_>>(), ["select", "status", "email", "amount", "actions"]);
        assert!(distinct(&img) > 40, "something was drawn: {}", distinct(&img));
        // the pagination footer is part of the recipe: it is elements, the table is not
        assert_eq!(handle(cx, &v).page_count(), 10);
    });
}

#[test]
fn builder_syntax_renders_the_same_table_with_a_toolbar() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Builder, 95);
        frames(cx, id);
        let h = handle(cx, &v);
        assert_eq!((h.snapshot().core_row_count, h.page_count()), (95, 10));
        // the toolbar's filter input drives the table
        h.set_column_filter_value("email", "user1@", cx);
        assert!(h.filtered_row_count() > 0 && h.filtered_row_count() < 95);
        assert_eq!(h.column_filter_value("email"), "user1@");
        h.set_column_filter_value("email", "", cx);
        assert_eq!(h.filtered_row_count(), 95);
    });
}

#[test]
fn pagination_commands() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Rsx, 95);
        frames(cx, id);
        let h = handle(cx, &v);
        h.next_page(cx);
        assert_eq!((h.page_index(), h.can_previous_page(), h.can_next_page()), (1, true, true));
        h.last_page(cx);
        assert_eq!((h.page_index(), h.snapshot().page_rows, h.can_next_page()), (9, 5, false));
        h.set_page_size(50, cx);
        assert_eq!((h.page_size(), h.page_count(), h.page_index()), (50, 2, 1));
        h.first_page(cx);
        assert_eq!((h.page_index(), h.snapshot().page_rows), (0, 50));
        h.set_page_index(99, cx);
        assert_eq!(h.page_index(), 1, "clamped to the last page");
        let img = frames(cx, id);
        assert!(distinct(&img) > 40);
    });
}

#[test]
fn clicks_sort_select_and_open_menus() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Rsx, 95);
        let before = frames(cx, id);
        let h = handle(cx, &v);
        // the status header is a plain sortable header: a click sorts ascending
        cx.simulate_click(id, 100.0, HEADER_Y);
        assert_eq!(h.snapshot().sorting, vec![SortingRule::new("status", false)]);
        cx.simulate_click(id, 100.0, HEADER_Y);
        assert_eq!(h.snapshot().sorting, vec![SortingRule::new("status", true)]);
        cx.simulate_click(id, 100.0, HEADER_Y);
        assert!(h.snapshot().sorting.is_empty(), "asc, desc, then none");
        // a checkbox cell selects its row without sorting or highlighting by itself
        cx.simulate_click(id, 20.0, row_y(1));
        assert_eq!(h.selected_row_count(), 1);
        cx.simulate_click(id, 20.0, row_y(3));
        assert_eq!(h.selected_row_count(), 2);
        cx.simulate_click(id, 20.0, row_y(1));
        assert_eq!(h.selected_row_count(), 1, "a second click deselects");
        // the header checkbox selects the page, then clears it
        cx.simulate_click(id, 20.0, HEADER_Y);
        assert_eq!(h.selected_row_count(), 10);
        cx.simulate_click(id, 20.0, HEADER_Y);
        assert_eq!(h.selected_row_count(), 0);
        // a Header::menu column opens its menu instead of sorting
        let closed = frames(cx, id);
        cx.simulate_click(id, 40.0 + 120.0 + 80.0, HEADER_Y);
        let open_menu = frames(cx, id);
        assert!(h.snapshot().sorting.is_empty());
        assert_ne!(closed.2, open_menu.2, "the menu is drawn");
        assert_ne!(before.2, open_menu.2);
        // its items sort and hide
        h.toggle_column_sorting("email", Some(true), cx);
        assert_eq!(h.snapshot().sorting, vec![SortingRule::new("email", true)]);
        h.toggle_column_visibility("email", Some(false), cx);
        assert!(!h.columns().iter().find(|c| c.id == "email").unwrap().visible);
        h.toggle_column_visibility("select", Some(false), cx);
        assert!(h.columns().iter().find(|c| c.id == "select").unwrap().visible, "enable_hiding is off for the select column");
    });
}

#[test]
fn selection_spans_pages() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Rsx, 95);
        frames(cx, id);
        let h = handle(cx, &v);
        cx.simulate_click(id, 20.0, HEADER_Y);
        assert_eq!(h.selected_row_count(), 10);
        h.next_page(cx);
        assert_eq!(h.selected_row_count(), 10, "the count is of all filtered rows, not of the page");
        cx.simulate_click(id, 20.0, row_y(0));
        assert_eq!(h.selected_row_count(), 11);
        h.toggle_all_rows_selected(true, cx);
        assert_eq!(h.selected_row_count(), 95);
        h.toggle_all_rows_selected(false, cx);
        assert_eq!(h.selected_row_count(), 0);
    });
}

#[test]
fn controlled_state_is_owned_by_the_parent() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Controlled, 95);
        frames(cx, id);
        let h = handle(cx, &v);
        cx.simulate_click(id, 100.0, HEADER_Y); // the view's signal gets [status asc] and re-renders
        frames(cx, id);
        assert_eq!(h.snapshot().sorting, vec![SortingRule::new("status", false)]);
    });
}

#[test]
fn a_controlled_field_changes_only_when_the_parent_says_so() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Refusing, 95);
        frames(cx, id);
        let h = handle(cx, &v);
        cx.simulate_click(id, 100.0, HEADER_Y);
        frames(cx, id);
        assert!(h.snapshot().sorting.is_empty(), "a controlled field only changes when the parent says so");
        assert_eq!(v.read(cx).changes.get(), 1, "but the change is reported");
    });
}

#[test]
fn light_and_dark_both_render() {
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Builder, 40);
        let dark = frames(cx, id);
        handle(cx, &v).toggle_all_page_rows_selected(true, cx);
        let selected = frames(cx, id);
        assert_ne!(dark.2, selected.2, "selected rows are painted");
        assert!(blenderui::set_theme("light"));
        cx.notify_all();
        let light = frames(cx, id);
        assert_ne!(selected.2, light.2, "the Blender light theme");
        assert!(distinct(&light) > 40);
    });
}

#[test]
fn bad_columns_are_reported_where_they_are_given() {
    let bad: Arc<[ColumnDef<Payment>]> = vec![ColumnDef::accessor_key("nope")].into();
    assert_eq!(Table::new(TableOptions::new(bad.clone(), payments(1))).err(), Some(TableError::UnknownKey("nope".into())));
    run("dark", |cx| {
        let (id, v) = open(cx, Mode::Rsx, 5);
        v.update(cx, |view, _| view.columns = bad);
        cx.notify_all();
        // the view shows the error instead of a table; nothing panics
        let img = frames(cx, id);
        assert!(distinct(&img) > 2);
    });
}

// -- the virtual list ---------------------------------------------------------------------------------------------------

struct Rows {
    handle: StdCell<Option<ListHandle>>,
    ranges: Rc<StdCell<usize>>,
}

impl Render for Rows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let list = use_virtual_list();
        self.handle.set(Some(list));
        let ranges = self.ranges.clone();
        let item = |i: usize, out: &mut String| {
            use std::fmt::Write;
            let _ = write!(out, "item {i} {}", "word ".repeat(i % 9));
        };
        body().m_0().child(
            VirtualList::new(10_000_000, item)
                .list(list)
                .estimate_size(40.0)
                .overscan(12)
                .on_range_change(move |_| ranges.set(ranges.get() + 1))
                .w(px(600.0))
                .h(px(300.0)),
        )
    }
}

#[test]
fn virtual_list_scrolls_by_index_and_offset() {
    run("dark", |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(600.0), px(300.0)) })),
            ..Default::default()
        };
        let mut view = None;
        let win = cx
            .open_window(opts, |_, cx| {
                let v = cx.new(|_| Rows { handle: StdCell::new(None), ranges: Rc::new(StdCell::new(0)) });
                view = Some(v);
                v
            })
            .unwrap();
        let v = view.unwrap();
        let first = {
            for _ in 0..4 {
                cx.draw(win.id);
            }
            cx.draw(win.id)
        };
        let h = v.read(cx).handle.get().unwrap();
        assert_eq!(h.count(), 10_000_000);
        assert_eq!(h.range().start, 0);
        assert!(h.range().len() < 60, "{:?}", h.range());
        assert!(v.read(cx).ranges.get() >= 1, "onChange fired");
        let info = h.view_info(cx).unwrap();
        assert!(info.total_size > 1.0e8, "ten million items are tall: {}", info.total_size);
        h.scroll_to_index(5_000_000, Align::Start, cx);
        for _ in 0..3 {
            cx.draw(win.id);
        }
        let r = h.range();
        assert!(r.start <= 5_000_000 && r.end > 5_000_000, "{r:?}");
        let moved = cx.draw(win.id);
        assert_ne!(first.2, moved.2);
        h.scroll_to_offset(0.0, cx);
        for _ in 0..3 {
            cx.draw(win.id);
        }
        assert_eq!(h.range().start, 0);
        h.measure_element(0, 120.0, cx);
        cx.draw(win.id);
    });
}

struct Chat {
    handle: StdCell<Option<ListHandle>>,
    messages: Arc<[Message]>,
}

impl Render for Chat {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let list = use_virtual_list();
        self.handle.set(Some(list));
        let messages = self.messages.clone();
        body().m_0().child(rsx! { <ChatList list={list} messages={messages} w={px(600.0)} h={px(300.0)} /> })
    }
}

#[test]
fn chat_list_opens_at_the_newest_message_and_takes_pushes() {
    run("dark", |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(600.0), px(300.0)) })),
            ..Default::default()
        };
        let mut view = None;
        let win = cx
            .open_window(opts, |_, cx| {
                let v = cx.new(|_| Chat { handle: StdCell::new(None), messages: blenderui::vlist::synth::messages(0, 200).into() });
                view = Some(v);
                v
            })
            .unwrap();
        for _ in 0..4 {
            cx.draw(win.id);
        }
        let h = view.unwrap().read(cx).handle.get().unwrap();
        assert_eq!(h.count(), 200);
        assert_eq!(h.range().end, 200, "open at the newest message");
        h.push(blenderui::vlist::synth::message(200), cx);
        for _ in 0..3 {
            cx.draw(win.id);
        }
        assert_eq!(h.count(), 201);
        assert_eq!(h.range().end, 201, "following the new message");
        h.scroll_to_index(0, Align::Start, cx);
        for _ in 0..3 {
            cx.draw(win.id);
        }
        assert_eq!(h.range().start, 0);
    });
}
