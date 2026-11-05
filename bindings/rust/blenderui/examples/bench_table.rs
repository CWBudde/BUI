//! Data table benchmark: 1,000,000 rows x 20 columns, painted `DataTable`
//! vs `uniform_list` rows of cell elements vs a naive element per row.
//!
//! ```text
//! cargo run --release --example bench_table -j 1
//! ```
//! Prints one JSON-ish line per measurement (ms per frame is the whole
//! pipeline: render, layout, paint and the software raster of 900x500 px).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::Instant;

use blenderui::prelude::*;
use blenderui::table::{Cell as TableCell, CellSource, Column, DataTable, TableDelegate, TableState};

struct Counting;
thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }
// SAFETY: forwards to the system allocator; the counter is a const thread-local.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.set(COUNT.get() + 1);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Counting = Counting;

#[cfg(windows)]
fn rss_mb() -> f64 {
    #[repr(C)]
    struct Pmc { cb: u32, fc: u32, peak: usize, ws: usize, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize }
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(h: isize, p: *mut Pmc, cb: u32) -> i32;
    }
    let mut m = Pmc { cb: std::mem::size_of::<Pmc>() as u32, fc: 0, peak: 0, ws: 0, a: 0, b: 0, c: 0, d: 0, e: 0, f: 0 };
    // SAFETY: a correctly sized, writable PROCESS_MEMORY_COUNTERS.
    unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut m, m.cb) };
    m.ws as f64 / 1048576.0
}
#[cfg(not(windows))]
fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/statm").ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<f64>().ok()).map_or(0.0, |p| p * 4096.0 / 1048576.0)
}

const COLS: usize = 20;
const ROWS: usize = 1_000_000;

struct Big;
impl CellSource for Big {
    fn cell(&self, row: usize, col: usize) -> TableCell<'_> {
        TableCell::U64(cell(row, col))
    }
}

impl TableDelegate for Big {
    fn columns_count(&self, _: &App) -> usize {
        COLS
    }
    fn rows_count(&self, _: &App) -> usize {
        ROWS
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        Column::new(format!("c{ix}"), format!("Column {ix}")).sortable().width(px(100.0))
    }
}

fn cell(r: usize, c: usize) -> u64 {
    (r as u64).wrapping_mul(2_654_435_761).wrapping_add(c as u64 * 40_503) % 1_000_003
}

struct Painted(Entity<TableState<Big>>);
impl Render for Painted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(div().w(px(900.0)).h(px(500.0)).child(DataTable::new(&self.0).stripe(true)))
    }
}

/// `n` rows of 20 cell elements each in a `uniform_list` (gpui's way).
struct Listed(usize);
impl Render for Listed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(
            uniform_list("rows", self.0, |range, _, _| {
                range.map(|r| div().flex().children((0..COLS).map(move |c| div().w(px(100.0)).child(cell(r, c).to_string()))))
                    .collect::<Vec<_>>()
            })
            .item_height(px(20.0)).w(px(900.0)).h(px(500.0)),
        )
    }
}

/// `n` rows as plain elements in a scroll container (no virtualization).
struct Naive(usize);
impl Render for Naive {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        body().m_0().child(
            div().id("naive").overflow_y_scroll().w(px(900.0)).h(px(500.0)).children((0..self.0).map(|r| {
                div().flex().h(px(20.0)).children((0..COLS).map(move |c| div().w(px(100.0)).child(cell(r, c).to_string())))
            })),
        )
    }
}

fn opts() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds { origin: point(px(0.0), px(0.0)), size: size(px(900.0), px(500.0)) })),
        ..Default::default()
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    let base = rss_mb();
    if which == "all" || which == "painted" {
        Application::headless().run(|cx| {
            let t0 = Instant::now();
            let mut table = None;
            let win = cx.open_window(opts(), |w, cx| {
                let t = cx.new(|cx| TableState::new(Big, w, cx).row_header(true));
                table = Some(t);
                cx.new(|_| Painted(t))
            }).unwrap();
            let t = table.unwrap();
            let build = ms(t0);
            let t1 = Instant::now();
            cx.draw(win.id);
            cx.draw(win.id);
            let first = ms(t1);
            let frames = 300;
            COUNT.set(0);
            let t2 = Instant::now();
            for _ in 0..frames {
                t.update(cx, |s, _| s.scroll_by(0.0, 37.0));
                cx.notify_all();
                cx.draw(win.id);
            }
            let per = ms(t2) / f64::from(frames);
            let allocs = COUNT.get() / frames as usize;
            // sort by a column through a header click: an index permutation of 1M rows
            let t3 = Instant::now();
            cx.simulate_click(win.id, 100.0, 10.0);
            let sort = ms(t3);
            println!("painted  rows={ROWS} cols={COLS} build_ms={build:.2} first_frame_ms={first:.2} scroll_frame_ms={per:.3} \
                      allocs_per_full_frame={allocs} sort_click_ms={sort:.1} rss_delta_mb={:.1}", rss_mb() - base);
        });
    }
    for (name, n, rows) in [("uniform_list", 1usize, ROWS), ("naive_2000", 2usize, 2000), ("naive_20000", 2usize, 20_000)] {
        if which != "all" && which != name {
            continue;
        }
        let base = rss_mb();
        Application::headless().run(move |cx| {
            let t0 = Instant::now();
            let win = if n == 1 {
                cx.open_window(opts(), |_, cx| cx.new(|_| Listed(rows))).unwrap().id
            } else {
                cx.open_window(opts(), |_, cx| cx.new(|_| Naive(rows))).unwrap().id
            };
            cx.draw(win);
            cx.draw(win);
            let first = ms(t0);
            let frames = if n == 1 { 100 } else { 10 };
            COUNT.set(0);
            let t2 = Instant::now();
            for k in 0..frames {
                let id = if n == 1 { "rows" } else { "naive" };
                if let Some(w) = cx.window(win) {
                    let (_, y) = w.scroll_offset(id);
                    let _ = (y, k);
                }
                cx.simulate_scroll(win, 100.0, 100.0, -1);
                cx.draw(win);
            }
            let per = ms(t2) / f64::from(frames);
            println!("{name} rows={rows} cols={COLS} first_frame_ms={first:.1} scroll_frame_ms={per:.2} \
                      allocs_per_frame={} rss_delta_mb={:.1}", COUNT.get() / frames as usize, rss_mb() - base);
        });
    }
}
