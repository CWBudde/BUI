#![allow(clippy::float_cmp, clippy::many_single_char_names)]
//! The model end to end through its public API: input, sorting, selection,
//! keyboard navigation, copying, extreme sizes and a randomized event soak.

use blenderui_table::{
    Button, Cell, CellSource, Column, ColumnSort, Hit, Key, Mods, Point, TableEvent, TableModel, TableSelection,
};

/// `rows` x 5; column 0 is the row, 1 a sparse value, 2 text with a tab, 3 a float.
struct Data;

const fn val(r: usize) -> u64 {
    (r as u64 * 2_654_435_761) % 997
}

impl CellSource for Data {
    fn cell(&self, r: usize, c: usize) -> Cell<'_> {
        match c {
            0 => Cell::U64(r as u64),
            1 => Cell::U64(val(r)),
            2 => Cell::Str(if r.is_multiple_of(2) { "a\tb" } else { "plain" }),
            3 => Cell::F64(r as f64 / 4.0, 2),
            _ => Cell::Empty,
        }
    }
}

const NONE: Mods = Mods { shift: false, control: false, alt: false };
const SHIFT: Mods = Mods { shift: true, control: false, alt: false };
const CTRL: Mods = Mods { shift: false, control: true, alt: false };
const HEADER: f32 = 22.0;
const ROW: f32 = 20.0;

fn table(rows: usize) -> TableModel {
    let mut t = TableModel::new();
    let cols = vec![
        Column::new("id", "ID").sortable().width(80.0_f32).fixed_left(),
        Column::new("v", "V").sortable().width(100.0_f32),
        Column::new("t", "T").sortable().width(100.0_f32),
        Column::new("f", "F").sortable().width(100.0_f32),
        Column::new("e", "E").width(100.0_f32),
    ];
    t.set_columns(cols, None);
    t.set_rows(rows, &Data);
    t.begin_frame(400.0, 200.0, 20.0, 1.0);
    t
}

fn click(t: &mut TableModel, x: f32, y: f32, mods: Mods) {
    t.pointer_down(Point { x, y }, Button::Left, mods, 1);
    t.pointer_up(&Data, true);
}

fn row_y(r: usize) -> f32 {
    HEADER + ROW * r as f32 + ROW / 2.0
}

#[test]
fn clicks_select_rows_with_modifiers() {
    let mut t = table(50);
    click(&mut t, 200.0, row_y(2), NONE);
    assert_eq!(t.selected_row(), Some(2));
    click(&mut t, 200.0, row_y(5), SHIFT);
    assert_eq!(t.selected_rows().collect::<Vec<_>>(), vec![2, 3, 4, 5]);
    click(&mut t, 200.0, row_y(0), CTRL);
    assert_eq!(t.selected_rows().count(), 5);
    assert_eq!(t.selection(), TableSelection::Row(0));
    let events = t.take_events();
    assert!(events.contains(&TableEvent::SelectRow(2)));
    assert!(t.take_events().is_empty());
}

#[test]
fn keyboard_navigation_wraps_extends_and_pages() {
    let mut t = table(30);
    assert!(t.key_down(Key::Down, NONE, &Data));
    assert_eq!(t.selected_row(), Some(0));
    assert!(t.key_down(Key::Up, NONE, &Data)); // wraps to the last
    assert_eq!(t.selected_row(), Some(29));
    assert!(t.key_down(Key::Down, NONE, &Data)); // and back
    assert_eq!(t.selected_row(), Some(0));
    t.key_down(Key::PageDown, NONE, &Data);
    assert_eq!(t.selected_row(), Some(8)); // (200 - 22) / 20 = 8 rows per page
    t.key_down(Key::Down, SHIFT, &Data);
    t.key_down(Key::Down, SHIFT, &Data);
    assert_eq!(t.selected_rows().collect::<Vec<_>>(), vec![8, 9, 10]);
    t.key_down(Key::End, NONE, &Data);
    assert_eq!(t.selected_row(), Some(29));
    t.key_down(Key::Down, SHIFT, &Data); // no wrap while extending
    assert_eq!(t.selected_row(), Some(29));
    t.key_down(Key::Home, NONE, &Data);
    assert_eq!(t.selected_row(), Some(0));
    // the selected row is always scrolled into view
    t.key_down(Key::End, NONE, &Data);
    let g = t.geo();
    let (_, sy) = t.scroll_offset();
    assert!((sy + f64::from(g.body_h)) >= 30.0 * f64::from(g.row_h) - 1.0);
    assert!(t.key_down(Key::A, CTRL, &Data));
    assert_eq!(t.selected_rows().count(), 30);
    assert!(t.key_down(Key::Escape, NONE, &Data));
    assert_eq!(t.selected_rows().count(), 0);
    assert!(!t.key_down(Key::Other, NONE, &Data));
}

#[test]
fn navigation_on_an_empty_table_does_nothing() {
    let mut t = table(0);
    for k in [Key::Down, Key::Up, Key::PageDown, Key::Home, Key::End] {
        t.key_down(k, NONE, &Data);
        assert_eq!(t.selected_row(), None);
    }
    t.key_down(Key::A, CTRL, &Data);
    assert_eq!(t.selected_rows().count(), 0);
}

#[test]
fn left_right_move_the_cell_selection_between_columns() {
    let mut t = table(10);
    t.opts.cell_selectable = true;
    t.set_selected_cell(3, 1);
    t.key_down(Key::Right, NONE, &Data);
    // view order: id (pinned), v, t, f, e
    assert_eq!(t.selected_cell(), Some((3, 2)));
    t.key_down(Key::Left, NONE, &Data);
    t.key_down(Key::Left, NONE, &Data);
    assert_eq!(t.selected_cell(), Some((3, 0)));
    t.key_down(Key::Left, NONE, &Data); // wraps
    assert_eq!(t.selected_cell(), Some((3, 4)));
    t.opts.loop_selection = false;
    t.key_down(Key::Right, NONE, &Data);
    assert_eq!(t.selected_cell(), Some((3, 4)));
}

#[test]
fn header_click_sorts_by_mask_and_cycles() {
    let mut t = table(5000);
    let hy = HEADER / 2.0;
    let min = (0..5000).map(val).min().unwrap();
    click(&mut t, 130.0, hy, NONE); // the V column starts at 80
    assert_eq!(t.sorted_by(), Some((1, ColumnSort::Ascending)));
    assert_eq!(val(t.row_at(0)), min);
    assert!((0..4999).all(|i| val(t.row_at(i)) <= val(t.row_at(i + 1))));
    click(&mut t, 130.0, hy, NONE);
    assert_eq!(t.sorted_by(), Some((1, ColumnSort::Descending)));
    assert!((0..4999).all(|i| val(t.row_at(i)) >= val(t.row_at(i + 1))));
    click(&mut t, 130.0, hy, NONE);
    assert_eq!(t.sorted_by(), None);
    assert_eq!(t.row_at(7), 7);
    let sorts = t.take_events().into_iter().filter(|e| matches!(e, TableEvent::Sorted(..))).count();
    assert_eq!(sorts, 3);
}

#[test]
fn a_filter_composes_with_the_sort_and_shows_the_right_rows() {
    let mut t = table(1000);
    click(&mut t, 130.0, HEADER / 2.0, NONE);
    t.set_filter(&Data, |r| r % 10 == 0);
    assert_eq!(t.rows(), 100);
    assert!((0..99).all(|i| val(t.row_at(i)) <= val(t.row_at(i + 1))));
    assert!((0..100).all(|i| t.row_at(i).is_multiple_of(10)));
    t.clear_filter(&Data);
    assert_eq!(t.rows(), 1000);
    // selection maps display rows to data rows through the mask
    t.set_selected_row(500);
    assert_eq!(t.selected_row(), Some(500));
}

#[test]
fn copy_is_tsv_in_display_order_with_cells_sanitized() {
    let mut t = table(6);
    click(&mut t, 200.0, row_y(0), NONE);
    click(&mut t, 200.0, row_y(1), SHIFT);
    assert!(t.key_down(Key::C, CTRL, &Data));
    let copied = t.take_events().into_iter().find_map(|e| if let TableEvent::Copied(s) = e { Some(s) } else { None });
    let text = copied.expect("a copy event");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    // pinned id first, then v, the text with its tab turned into a space, the float, the empty cell
    let cells: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(cells, vec!["0", &val(0).to_string(), "a b", "0.00", ""]);
    assert_eq!(lines[1].split('\t').nth(3), Some("0.25"));
    // nothing selected: nothing copied
    t.key_down(Key::Escape, NONE, &Data);
    t.take_events();
    t.key_down(Key::C, CTRL, &Data);
    assert!(t.take_events().iter().all(|e| !matches!(e, TableEvent::Copied(_))));
}

#[test]
fn dragging_a_border_resizes_and_a_double_click_asks_for_a_fit() {
    let mut t = table(100);
    t.take_events();
    t.pointer_down(Point { x: 80.0, y: 10.0 }, Button::Left, NONE, 1);
    assert!(t.pointer_move(Point { x: 130.0, y: 10.0 }));
    t.pointer_up(&Data, true);
    assert_eq!(t.col_width(0), Some(130.0));
    t.pointer_down(Point { x: 130.0, y: 10.0 }, Button::Left, NONE, 1);
    t.pointer_move(Point { x: 1.0, y: 10.0 });
    t.pointer_up(&Data, true);
    assert_eq!(t.col_width(0), Some(20.0), "clamped to the minimum width");
    let events = t.take_events();
    assert!(events.iter().any(|e| matches!(e, TableEvent::ColumnWidthsChanged(w) if w[0] == 130.0)));
    t.pointer_down(Point { x: 20.0, y: 10.0 }, Button::Left, NONE, 2);
    assert_eq!(t.take_events(), vec![TableEvent::FitColumn(0)]);
    assert!(t.set_col_width(0, 64.0));
    assert_eq!(t.col_width(0), Some(64.0));
}

#[test]
fn dragging_a_header_moves_the_column_inside_its_strip() {
    let mut t = table(10);
    let (x1, x2) = (80.0 + 50.0, 80.0 + 250.0); // V, then F
    t.pointer_down(Point { x: x1, y: 10.0 }, Button::Left, NONE, 1);
    assert!(t.pointer_move(Point { x: x2, y: 10.0 }));
    t.pointer_up(&Data, true);
    assert_eq!(t.layout().column(3).key, "v");
    assert!(t.take_events().iter().any(|e| matches!(e, TableEvent::MoveColumn(1, 3))));
    // the pinned column cannot leave the pinned strip
    t.pointer_down(Point { x: 40.0, y: 10.0 }, Button::Left, NONE, 1);
    t.pointer_move(Point { x: 300.0, y: 10.0 });
    t.pointer_up(&Data, true);
    assert_eq!(t.layout().column(0).key, "id");
}

#[test]
fn wheel_scrolls_three_rows_per_line_and_shift_goes_sideways() {
    let mut t = table(1000);
    assert!(t.scroll_by(0.0, t.wheel_pixels(-1.0)));
    assert_eq!(t.scroll_offset().1, 60.0);
    t.scroll_by(0.0, t.wheel_pixels(5.0));
    assert_eq!(t.scroll_offset().1, 0.0);
    assert!(t.scroll_by(t.wheel_pixels(-1.0), 0.0));
    assert!(t.scroll_offset().0 > 0.0);
}

#[test]
fn scrolling_reports_what_became_visible() {
    let mut t = table(100_000);
    let r = t.begin_frame(400.0, 200.0, 20.0, 1.0);
    assert!(r.rows_changed.is_none() && r.cols_changed.is_none(), "nothing changed since the first frame");
    t.scroll_by(0.0, 20_000.0);
    let r = t.begin_frame(400.0, 200.0, 20.0, 1.0);
    assert_eq!(r.rows_changed.map(|r| r.start), Some(1000));
    assert!(r.rows_left > 90_000);
    assert!(t.scroll_to_row(5));
    assert!(!t.scroll_to_row(100_000));
    assert!(t.scroll_to_col(4));
    assert!(!t.scroll_to_col(0), "a pinned column never needs scrolling to");
}

#[test]
fn extreme_sizes_do_not_overflow_or_panic() {
    // a billion rows: the identity mask stores nothing
    let mut t = TableModel::new();
    t.set_columns(vec![Column::new("a", "A").width(100.0_f32)], None);
    t.set_rows(usize::MAX / 4, &Data);
    t.begin_frame(400.0, 200.0, 20.0, 1.0);
    t.scroll_by(0.0, 1e300);
    t.scroll_by(f64::NAN, f64::NAN);
    t.scroll_by(f64::INFINITY, f64::NEG_INFINITY);
    t.begin_frame(400.0, 200.0, 20.0, 1.0);
    let v = t.visible_range().rows().clone();
    assert!(v.start <= v.end && v.end <= t.rows());
    t.key_down(Key::End, NONE, &Data);
    t.key_down(Key::PageUp, NONE, &Data);
    // a zero-sized and a negative-sized table
    for (w, h) in [(0.0, 0.0), (-5.0, -5.0), (f32::NAN, 100.0), (1e9, 1e9)] {
        t.begin_frame(w, h, 20.0, 1.0);
        let _ = t.hit(Point { x: 3.0, y: 3.0 });
    }
    t.begin_frame(400.0, 200.0, 0.0, 0.0);
    assert!(t.geo().row_h >= 1.0);
}

#[test]
fn hit_testing_agrees_with_geometry() {
    let mut t = table(100);
    assert_eq!(t.hit(Point { x: 5.0, y: 5.0 }), Hit::Header(0));
    assert_eq!(t.hit(Point { x: 5.0, y: row_y(0) }), Hit::Cell(0, 0));
    assert_eq!(t.hit(Point { x: 200.0, y: row_y(3) }), Hit::Cell(3, 2));
    assert_eq!(t.hit(Point { x: -1.0, y: 5.0 }), Hit::None);
    assert_eq!(t.hit(Point { x: 80.0, y: 5.0 }), Hit::Border(0));
    t.scroll_by(0.0, 100.0);
    assert_eq!(t.hit(Point { x: 5.0, y: row_y(0) }), Hit::Cell(5, 0));
    assert!(!t.header_handles().is_empty());
}

/// A randomized soak: whatever the events, the model's invariants hold.
#[test]
fn random_events_keep_the_invariants() {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn f(&mut self, max: f32) -> f32 {
            (self.next() % 10_000) as f32 / 10_000.0 * max
        }
    }
    let keys = [Key::Up, Key::Down, Key::Left, Key::Right, Key::PageUp, Key::PageDown, Key::Home, Key::End, Key::Escape, Key::A, Key::C];
    for seed in 1..=8u64 {
        let mut rng = Rng(seed * 7919);
        let mut t = table([0usize, 1, 7, 300, 20_000][(seed % 5) as usize]);
        t.opts.cell_selectable = seed % 2 == 0;
        t.opts.col_selectable = seed % 3 == 0;
        t.opts.row_header = seed % 4 == 0;
        for _ in 0..3000 {
            let p = Point { x: rng.f(450.0) - 10.0, y: rng.f(260.0) - 10.0 };
            let mods = Mods { shift: rng.next().is_multiple_of(2), control: rng.next().is_multiple_of(3), alt: false };
            match rng.next() % 8 {
                0 => t.pointer_down(p, Button::Left, mods, 1 + (rng.next() % 2) as usize),
                1 => t.pointer_down(p, Button::Right, mods, 1),
                2 => {
                    t.pointer_move(p);
                }
                3 => t.pointer_up(&Data, rng.next().is_multiple_of(2)),
                4 => {
                    let px = t.wheel_pixels(rng.f(10.0) - 5.0);
                    t.scroll_by(if mods.shift { px } else { 0.0 }, if mods.shift { 0.0 } else { px });
                }
                5 => {
                    t.key_down(keys[(rng.next() % keys.len() as u64) as usize], mods, &Data);
                }
                6 => {
                    t.begin_frame(100.0 + rng.f(400.0), 50.0 + rng.f(300.0), 20.0, 1.0);
                }
                _ => {
                    t.take_events();
                }
            }
            let rows = t.rows();
            for r in t.selected_ranges() {
                assert!(r.start < r.end && r.end <= rows, "seed {seed}: {r:?} of {rows}");
            }
            let (sx, sy) = t.scroll_offset();
            assert!(sx >= 0.0 && sy >= 0.0 && sx.is_finite() && sy.is_finite());
            let g = t.geo();
            assert!(sy <= (rows as f64 * f64::from(g.row_h)).max(0.0), "scrolled past the end");
            let v = t.visible_range().rows();
            assert!(v.end <= rows);
        }
    }
}
