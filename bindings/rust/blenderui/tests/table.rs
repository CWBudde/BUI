//! The data table and `uniform_list`, end to end in off-screen windows.

use std::ffi::CStr;

use blenderui::prelude::*;
use blenderui::table::{Cell, CellSource, Column, ColumnSort, DataTable, TableDelegate, TableState};

const HEADER: f32 = 22.0; // 1.1 * UI_UNIT_Y
const ROW: f32 = 20.0; // UI_UNIT_Y

/// rows x 20 columns of derived numbers; column 0 is the row, pinned.
struct Grid {
    rows: usize,
}

fn val(r: usize, c: usize) -> u64 {
    if c == 0 { r as u64 } else { (r as u64 * 2_654_435_761 + c as u64 * 40_503) % 100_003 }
}

impl CellSource for Grid {
    fn cell(&self, row: usize, col: usize) -> Cell<'_> {
        Cell::U64(val(row, col))
    }
}

impl TableDelegate for Grid {
    fn columns_count(&self, _: &App) -> usize {
        20
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        if ix == 0 {
            Column::new("id", "ID").sortable().width(px(80.0)).fixed_left()
        } else {
            Column::new(format!("c{ix}"), format!("Col {ix}")).sortable().width(px(100.0))
        }
    }
}

struct View {
    table: Entity<TableState<Grid>>,
}

impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(div().w(px(800.0)).h(px(400.0)).child(DataTable::new(&self.table).stripe(true)))
    }
}

fn open(cx: &mut App, rows: usize, row_header: bool) -> (WindowId, Entity<TableState<Grid>>) {
    let opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(800.0), px(400.0)),
        })),
        ..Default::default()
    };
    let mut handle = None;
    let win = cx
        .open_window(opts, |window, cx| {
            let table = cx.new(|cx| TableState::new(Grid { rows }, window, cx).row_header(row_header));
            handle = Some(table);
            cx.new(|_| View { table })
        })
        .unwrap();
    (win.id, handle.unwrap())
}

fn frame(cx: &mut App, id: WindowId) -> (usize, usize, Vec<u32>) {
    cx.draw(id); // the first frame learns the table's size, the second draws with it
    cx.draw(id)
}

fn band(img: &(usize, usize, Vec<u32>), x0: usize, x1: usize, y0: usize, y1: usize) -> Vec<u32> {
    (y0..y1).flat_map(|y| img.2[y * img.0 + x0..y * img.0 + x1].to_vec()).collect()
}

fn px_at(img: &(usize, usize, Vec<u32>), x: usize, y: usize) -> u32 {
    img.2[y * img.0 + x]
}

fn run(theme: &str, f: impl FnOnce(&mut App)) {
    Application::headless().theme(theme).run(f);
}

#[test]
fn only_visible_rows_are_touched_and_scrolling_moves_the_range() {
    run("dark", |cx| {
        let (id, t) = open(cx, 1_000_000, false);
        frame(cx, id);
        let first = t.read(cx).visible_range().clone();
        assert_eq!(first.rows().start, 0);
        assert!(first.rows().len() <= 20, "{:?}", first.rows());
        assert!(first.cols().len() < 20);
        cx.simulate_scroll(id, 400.0, 200.0, -10);
        frame(cx, id);
        let moved = t.read(cx).visible_range().clone();
        assert!(moved.rows().start >= 25, "{:?}", moved.rows());
        assert!(moved.rows().len() <= 20);
        // the scroll extent is the full table
        t.update(cx, |s, _| s.scroll_by(0.0, 1e12));
        cx.notify_all();
        frame(cx, id);
        assert_eq!(t.read(cx).visible_range().rows().end, 1_000_000);
    });
}

#[test]
fn stripes_header_stickiness_and_pinned_columns() {
    run("dark", |cx| {
        let (id, t) = open(cx, 100_000, false);
        let a = frame(cx, id);
        // header differs from the body; row 1 is striped, row 0 is not
        let hdr = px_at(&a, 775, 4);
        let r0 = px_at(&a, 770, (HEADER + ROW * 0.5) as usize);
        let r1 = px_at(&a, 770, (HEADER + ROW * 1.5) as usize);
        let r2 = px_at(&a, 770, (HEADER + ROW * 2.5) as usize);
        assert_ne!(hdr, r0);
        assert_ne!(r0, r1);
        assert_eq!(r0, r2);
        // scrolling down keeps the header band pixel for pixel
        t.update(cx, |s, _| s.scroll_by(0.0, 4321.0));
        cx.notify_all();
        let b = frame(cx, id);
        assert_eq!(band(&a, 0, 800, 0, HEADER as usize - 1), band(&b, 0, 800, 0, HEADER as usize - 1));
        assert_ne!(band(&a, 0, 800, 30, 380), band(&b, 0, 800, 30, 380));
        // scrolling right keeps the pinned column (x < 80) and moves the rest
        t.update(cx, |s, _| s.scroll_by(0.0, -1e9));
        cx.notify_all();
        let c = frame(cx, id);
        t.update(cx, |s, _| s.scroll_by(250.0, 0.0));
        cx.notify_all();
        let d = frame(cx, id);
        assert_eq!(band(&c, 0, 79, 0, 400), band(&d, 0, 79, 0, 400));
        assert_ne!(band(&c, 100, 800, 0, 400), band(&d, 100, 800, 0, 400));
    });
}

#[test]
fn light_and_dark_themes_differ() {
    let mut shots = Vec::new();
    for theme in ["dark", "light"] {
        run(theme, |cx| {
            let (id, _) = open(cx, 1000, true);
            shots.push(px_at(&frame(cx, id), 790, 300));
        });
    }
    assert_ne!(shots[0], shots[1]);
    // dark is darker
    let lum = |p: u32| (p >> 16 & 255) + (p >> 8 & 255) + (p & 255);
    assert!(lum(shots[0]) < lum(shots[1]));
}

#[test]
fn dragging_a_header_border_resizes_the_column() {
    run("dark", |cx| {
        let (id, t) = open(cx, 1000, false);
        frame(cx, id);
        assert_eq!(t.read(cx).col_width(0).map(|p| p.0), Some(80.0));
        cx.simulate_drag(id, (80.0, 10.0), (130.0, 10.0));
        let w = t.read(cx).col_width(0).unwrap().0;
        assert!((w - 130.0).abs() <= 1.0, "{w}");
        // clamped to the column's limits
        cx.simulate_drag(id, (130.0, 10.0), (2.0, 10.0));
        assert_eq!(t.read(cx).col_width(0).unwrap().0, 20.0);
    });
}

#[test]
fn header_click_sorts_by_index_mask_and_cycles() {
    run("dark", |cx| {
        let n = 5000;
        let (id, t) = open(cx, n, false);
        frame(cx, id);
        let min = (0..n).map(|r| val(r, 1)).min().unwrap();
        let max = (0..n).map(|r| val(r, 1)).max().unwrap();
        cx.simulate_click(id, 130.0, 10.0);
        assert_eq!(t.read(cx).sorted_by(), Some((1, ColumnSort::Ascending)));
        assert_eq!(val(t.read(cx).row_at(0), 1), min);
        cx.simulate_click(id, 130.0, 10.0);
        assert_eq!(t.read(cx).sorted_by(), Some((1, ColumnSort::Descending)));
        assert_eq!(val(t.read(cx).row_at(0), 1), max);
        cx.simulate_click(id, 130.0, 10.0);
        assert_eq!(t.read(cx).sorted_by(), None);
        assert_eq!(t.read(cx).row_at(7), 7);
    });
}

#[test]
fn selection_keyboard_and_tsv_copy() {
    run("dark", |cx| {
        let (id, t) = open(cx, 5, false);
        frame(cx, id);
        let row_y = |r: usize| HEADER + ROW * r as f32 + ROW / 2.0;
        cx.simulate_click(id, 300.0, row_y(2));
        assert_eq!(t.read(cx).selected_row(), Some(2));
        cx.simulate_key(id, "", "Down");
        assert_eq!(t.read(cx).selected_row(), Some(3));
        cx.simulate_key(id, "", "Up");
        cx.simulate_key(id, "", "Up");
        assert_eq!(t.read(cx).selected_row(), Some(1));
        cx.simulate_click_with(id, 300.0, row_y(3), false, true); // shift: range
        assert_eq!(t.read(cx).selected_rows().collect::<Vec<_>>(), vec![1, 2, 3]);
        cx.simulate_click_with(id, 300.0, row_y(0), true, false); // ctrl: toggle
        assert_eq!(t.read(cx).selected_rows().count(), 4);
        cx.simulate_key(id, "", "End");
        assert_eq!(t.read(cx).selected_row(), Some(4));
        cx.simulate_key_with(id, "a", "a", true, false);
        assert_eq!(t.read(cx).selected_rows().count(), 5);
        cx.simulate_key_with(id, "c", "c", true, false);
        let text = t.read(cx).selection_text();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 5);
        let cells: Vec<&str> = lines[3].split('\t').collect();
        assert_eq!(cells.len(), 20);
        assert_eq!(cells[0], "3");
        assert_eq!(cells[1], val(3, 1).to_string());
        // and it reached the clipboard
        // SAFETY: the kit returns a NUL-terminated malloc'd string (leaked: a test).
        let clip = unsafe {
            let p = blenderui::sys::BK_clipboard_get(std::ptr::null_mut());
            CStr::from_ptr(p).to_string_lossy().into_owned()
        };
        assert_eq!(clip, text);
        cx.simulate_key(id, "", "Escape");
        assert_eq!(t.read(cx).selected_rows().count(), 0);
    });
}

struct Items;

impl Render for Items {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(
            uniform_list("items", 1_000_000, |range, _, _| {
                range.map(|i| div().id(format!("item-{i}")).child(format!("Item {i}"))).collect::<Vec<_>>()
            })
            .item_height(px(20.0))
            .h(px(200.0))
            .w(px(300.0)),
        )
    }
}

#[test]
fn uniform_list_builds_only_the_visible_items() {
    run("dark", |cx| {
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(400.0), px(300.0)),
            })),
            ..Default::default()
        };
        let id = cx.open_window(opts, |_, cx| cx.new(|_| Items)).unwrap().id;
        cx.draw(id);
        cx.draw(id);
        assert!(cx.element_bounds(id, "item-0").is_some());
        assert!(cx.element_bounds(id, "item-9").is_some());
        assert!(cx.element_bounds(id, "item-40").is_none());
        // 100 wheel lines = 4000 px
        cx.simulate_scroll(id, 100.0, 100.0, -100);
        cx.draw(id);
        assert!(cx.element_bounds(id, "item-0").is_none());
        let off = cx.window(id).unwrap().scroll_offset("items").1;
        assert!(off > 3000.0, "{off}");
        assert!(cx.element_bounds(id, &format!("item-{}", (off / 20.0) as usize + 3)).is_some());
    });
}

