//! The table's painting entry: the shared sink ([`crate::sink`]) around
//! `blenderui_table::paint_table`.

use blenderui_table::{paint_table, with_scratch};

use super::delegate::TableDelegate;
use super::state::TableState;
use crate::geometry::Bounds;
use crate::sink::{current_palette, with_sink};
use crate::Painter;

impl<D: TableDelegate> TableState<D> {
    /// Draw the table into `p` (logical pixels, `b` = the element's box).
    pub(crate) fn draw(&self, b: Bounds<f32>, p: &mut Painter) {
        with_sink(p, b.origin, self.scale(), |sink| {
            with_scratch(|s| paint_table(self, self.delegate(), &current_palette(), self.delegate().empty_text(), s, sink));
        });
    }
}

#[cfg(test)]
mod tests {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    use crate::prelude::*;
    use crate::table::{Cell as TableCell, CellSource, Column, DataTable, TableDelegate, TableState};

    /// Counts the allocations the armed thread makes.
    struct Counting;

    thread_local! {
        static ARMED: Cell<bool> = const { Cell::new(false) };
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    // SAFETY: forwards to the system allocator; the counters are const-initialised
    // thread-locals without destructors, so counting never allocates.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            if ARMED.get() {
                COUNT.set(COUNT.get() + 1);
            }
            unsafe { System.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            unsafe { System.dealloc(p, l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            if ARMED.get() {
                COUNT.set(COUNT.get() + 1);
            }
            unsafe { System.realloc(p, l, n) }
        }
    }

    #[global_allocator]
    static A: Counting = Counting;

    struct Big;
    impl CellSource for Big {
        fn cell(&self, row: usize, col: usize) -> TableCell<'_> {
            TableCell::U64((row * 31 + col) as u64)
        }
    }
    impl TableDelegate for Big {
        fn columns_count(&self, _: &App) -> usize {
            20
        }
        fn rows_count(&self, _: &App) -> usize {
            1_000_000
        }
        fn column(&self, ix: usize, _: &App) -> Column {
            // a narrow column so some cells need the ellipsis path too
            Column::new(format!("c{ix}"), format!("Column {ix}")).sortable().width(px(if ix == 3 { 40.0 } else { 110.0 }))
        }
    }

    struct V(Entity<TableState<Big>>);
    impl Render for V {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            body().m_0().child(div().w(px(900.0)).h(px(500.0)).child(DataTable::new(&self.0).stripe(true)))
        }
    }

    #[test]
    fn steady_state_draw_does_not_allocate() {
        Application::headless().run(|cx| {
            let mut table = None;
            let win = cx
                .open_window(WindowOptions::default(), |w, cx| {
                    let t = cx.new(|cx| TableState::new(Big, w, cx).row_header(true));
                    table = Some(t);
                    cx.new(|_| V(t))
                })
                .unwrap();
            let t = table.unwrap();
            cx.draw(win.id);
            cx.draw(win.id);
            t.update(cx, |s, _| s.scroll_by(1000.0, 5_000_000.0));
            let st = t.read(cx);
            let b = st.bounds;
            assert!(b.size.width > 100.0);
            let mut p = Painter::new();
            for _ in 0..4 {
                p.reset();
                st.draw(b, &mut p); // warm: the display list, scratch strings and width cache grow
            }
            p.reset();
            ARMED.set(true);
            COUNT.set(0);
            st.draw(b, &mut p);
            ARMED.set(false);
            assert_eq!(COUNT.get(), 0, "a steady-state frame allocated");
        });
    }

    /// The shadcn layer paints through the same engine: a million rows with a
    /// checkbox, a badge, a currency, a progress bar and an actions column.
    #[derive(Clone, Copy)]
    struct Idx(u32);

    impl crate::shadcn::Record for Idx {
        const FIELDS: &'static [&'static str] = &["status", "amount", "pct"];
        fn field(&self, field: usize) -> TableCell<'_> {
            match field {
                0 => TableCell::Str(["pending", "processing", "success", "failed"][self.0 as usize % 4]),
                1 => TableCell::F64(f64::from(self.0 % 100_000) / 7.0, 2),
                _ => TableCell::U64(u64::from(self.0 % 101)),
            }
        }
    }

    #[test]
    fn steady_state_shadcn_table_does_not_allocate() {
        use crate::shadcn::{actions_column, select_column, BadgeVariant, CellRender, ColumnDef, ColumnMeta, Table, TableOptions};
        use std::sync::Arc;
        let columns: Arc<[ColumnDef<Idx>]> = vec![
            select_column(),
            ColumnDef::accessor_key("status").badge(|s| if s == "failed" { BadgeVariant::Destructive } else { BadgeVariant::Secondary }),
            ColumnDef::accessor_key("amount").meta(ColumnMeta::right().usd()),
            ColumnDef::accessor_key("pct").cell(CellRender::Progress),
            actions_column(),
        ]
        .into();
        let data: Arc<[Idx]> = (0..1_000_000u32).map(Idx).collect();
        Application::headless().run(|cx| {
            let table = Table::new(TableOptions::new(columns, data)).unwrap();
            assert_eq!(table.row_model().len(), 1_000_000);
            let mut handle = None;
            let win = cx
                .open_window(WindowOptions::default(), |w, cx| {
                    let t = cx.new(|cx| TableState::new(table, w, cx).row_selectable(false));
                    handle = Some(t);
                    cx.new(|_| ShadcnView(t))
                })
                .unwrap();
            let t = handle.unwrap();
            cx.draw(win.id);
            cx.draw(win.id);
            t.update(cx, |s, _| s.scroll_by(0.0, 5_000_000.0));
            let st = t.read(cx);
            let b = st.bounds;
            assert!(b.size.width > 100.0);
            let mut p = Painter::new();
            for _ in 0..4 {
                p.reset();
                st.draw(b, &mut p);
            }
            p.reset();
            ARMED.set(true);
            COUNT.set(0);
            st.draw(b, &mut p);
            ARMED.set(false);
            assert_eq!(COUNT.get(), 0, "a steady-state frame of the shadcn table allocated");
        });
    }

    struct ShadcnView(Entity<TableState<crate::shadcn::Table<Idx>>>);
    impl Render for ShadcnView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            body().m_0().child(div().w(px(900.0)).h(px(500.0)).child(DataTable::new(&self.0)))
        }
    }
}
