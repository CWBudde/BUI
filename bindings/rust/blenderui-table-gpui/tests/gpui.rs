//! The adapter on gpui's test platform: layout, input dispatched through the
//! window, sorting, selection, the keyboard and the custom-cell escape hatch.
//! (The test platform lays out and paints into a scene without a GPU, so
//! pixels are not checked here; the head-to-head benchmark measures real frames.)

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use blenderui_table_gpui::{Cell, CellSource, Column, ColumnSort, DataTable, TableDelegate, TableState};
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, IntoElement, Modifiers, ParentElement as _, Render, ScrollDelta,
    ScrollEvent, Styled as _, TestAppContext, Window, div, point, px,
};

/// rows x 6 numbers; column 2 can be a custom cell.
struct Grid {
    rows: usize,
    custom: bool,
    built: Arc<AtomicUsize>,
}

const fn val(r: usize, c: usize) -> u64 {
    (r as u64 * 2_654_435_761 + c as u64 * 40_503) % 100_003
}

impl CellSource for Grid {
    fn cell(&self, row: usize, col: usize) -> Cell<'_> {
        if self.custom && col == 2 { Cell::Empty } else { Cell::U64(val(row, col)) }
    }
}

impl TableDelegate for Grid {
    fn columns_count(&self, _: &App) -> usize {
        6
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        Column::new(format!("c{ix}"), format!("Col {ix}")).sortable().width(px(100.0_f32))
    }
    fn has_custom_cells(&self) -> bool {
        self.custom
    }
    fn is_custom_cell(&self, _: usize, col: usize) -> bool {
        self.custom && col == 2
    }
    fn render_td(&mut self, row: usize, _: usize, _: &mut Window, _: &mut Context<TableState<Self>>) -> AnyElement {
        self.built.fetch_add(1, Ordering::Relaxed);
        div().child(format!("rich {row}")).into_any_element()
    }
}

struct View(Entity<TableState<Grid>>);

impl Render for View {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(600.0)).h(px(300.0)).child(DataTable::new(&self.0).stripe(true).unit(px(24.0)).header_row_height(px(24.0)))
    }
}

fn setup(cx: &mut TestAppContext, rows: usize, custom: bool) -> (Entity<TableState<Grid>>, Arc<AtomicUsize>, &mut gpui::VisualTestContext) {
    let built = Arc::new(AtomicUsize::new(0));
    let b = built.clone();
    let (view, cx) = cx.add_window_view(move |window, cx| {
        View(cx.new(|cx| TableState::new(Grid { rows, custom, built: b }, window, cx)))
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let table = view.read_with(cx, |v, _| v.0.clone());
    (table, built, cx)
}

fn redraw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn only_the_visible_cells_are_in_range_and_the_wheel_scrolls(cx: &mut TestAppContext) {
    let (table, _, cx) = setup(cx, 1_000_000, false);
    let first = table.read_with(cx, |t, _| t.visible_range().clone());
    assert_eq!(first.rows().start, 0);
    assert!(first.rows().len() <= 13, "{:?}", first.rows());
    assert!(first.cols().len() <= 6);
    cx.simulate_event(ScrollEvent {
        position: point(px(300.0), px(150.0)),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-2400.0))),
        ..Default::default()
    });
    redraw(cx);
    let moved = table.read_with(cx, |t, _| t.visible_range().rows().clone());
    assert!(moved.start >= 90, "{moved:?}");
    assert!(moved.len() <= 13);
    // scrolling to the end of a million rows
    table.update(cx, |t, _| t.scroll_by(0.0, 1e12));
    redraw(cx);
    assert_eq!(table.read_with(cx, |t, _| t.visible_range().rows().end), 1_000_000);
}

#[gpui::test]
fn clicks_select_rows_and_the_keyboard_moves_the_selection(cx: &mut TestAppContext) {
    let (table, _, cx) = setup(cx, 100, false);
    let row_y = |r: usize| px(24.0 + 24.0 * r as f32 + 12.0);
    cx.simulate_click(point(px(250.0), row_y(2)), Modifiers::none());
    assert_eq!(table.read_with(cx, |t, _| t.selected_row()), Some(2));
    cx.simulate_keystrokes("down down");
    assert_eq!(table.read_with(cx, |t, _| t.selected_row()), Some(4));
    cx.simulate_keystrokes("up");
    assert_eq!(table.read_with(cx, |t, _| t.selected_row()), Some(3));
    cx.simulate_click(point(px(250.0), row_y(6)), Modifiers::shift());
    assert_eq!(table.read_with(cx, |t, _| t.selected_rows().collect::<Vec<_>>()), vec![3, 4, 5, 6]);
    cx.simulate_keystrokes("escape");
    assert_eq!(table.read_with(cx, |t, _| t.selected_rows().count()), 0);
}

#[gpui::test]
fn a_header_click_sorts_by_index_mask_and_cycles(cx: &mut TestAppContext) {
    let n = 5000;
    let (table, _, cx) = setup(cx, n, false);
    let min = (0..n).map(|r| val(r, 1)).min().unwrap();
    let max = (0..n).map(|r| val(r, 1)).max().unwrap();
    let header = point(px(150.0), px(12.0)); // column 1 spans 100..200
    cx.simulate_click(header, Modifiers::none());
    assert_eq!(table.read_with(cx, |t, _| t.sorted_by()), Some((1, ColumnSort::Ascending)));
    assert_eq!(table.read_with(cx, |t, _| val(t.row_at(0), 1)), min);
    cx.simulate_click(header, Modifiers::none());
    assert_eq!(table.read_with(cx, |t, _| val(t.row_at(0), 1)), max);
    cx.simulate_click(header, Modifiers::none());
    assert_eq!(table.read_with(cx, |t, _| (t.sorted_by(), t.row_at(7))), (None, 7));
}

#[gpui::test]
fn custom_cells_are_built_only_for_the_visible_rows(cx: &mut TestAppContext) {
    let (table, built, cx) = setup(cx, 1_000_000, true);
    let visible = table.read_with(cx, |t, _| t.visible_range().rows().len());
    assert!(built.load(Ordering::Relaxed) >= visible, "{} custom cells for {visible} rows", built.load(Ordering::Relaxed));
    assert!(built.load(Ordering::Relaxed) <= 3 * visible + 3, "{} custom cells for {visible} rows", built.load(Ordering::Relaxed));
    built.store(0, Ordering::Relaxed);
    redraw(cx);
    let per_frame = built.load(Ordering::Relaxed);
    assert!(per_frame >= visible && per_frame <= visible + 2, "{per_frame} per frame for {visible} rows");
}

#[gpui::test]
fn the_selection_api_and_events(cx: &mut TestAppContext) {
    let (table, _, cx) = setup(cx, 1000, false);
    table.update(cx, |t, cx| t.set_selected_row(500, cx));
    assert_eq!(table.read_with(cx, |t, _| t.selected_row()), Some(500));
    redraw(cx);
    // the selected row was scrolled into view
    assert!(table.read_with(cx, |t, _| t.visible_range().rows().contains(&500)));
    table.update(cx, |t, cx| t.set_filter(|d, r| val(r, 1).is_multiple_of(2) && d.rows > 0, cx));
    assert!(table.read_with(cx, |t, _| t.rows()) < 1000);
    table.update(cx, |t, cx| t.clear_filter(cx));
    assert_eq!(table.read_with(cx, |t, _| t.rows()), 1000);
    assert_eq!(table.read_with(cx, |t, _| t.selection_text()), "");
}
