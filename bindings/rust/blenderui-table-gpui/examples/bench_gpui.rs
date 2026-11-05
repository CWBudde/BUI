//! Head-to-head: this crate's `DataTable` vs gpui-component 0.9.0's `DataTable`,
//! on the same gpui (gpui-pre 0.3.11), the same data, in a real window.
//!
//! ```text
//! cargo build --release --features bench --example bench_gpui -j 1
//! node bench/run.mjs            # 3 runs per configuration, medians, writes bench/results.*
//! target/release/examples/bench_gpui.exe --lib ours --rows 1000000 --cols 20
//! ```
//!
//! One process runs one library, so neither sees the other's memory or
//! threads. It prints one JSON line. What is measured:
//!
//! * `first_frame_ms`: process start to the first painted frame (data
//!   generation excluded: `data_ms`).
//! * scroll (continuous, two rows per frame, driven from the root view's
//!   render): per-frame **render to end of paint** time on the main thread
//!   (building elements, layout, prepaint, paint: what the application can
//!   influence; GPU submission and vsync wait excluded), its mean and
//!   percentiles, frames per second, process CPU time per frame (all threads,
//!   `QueryProcessCycleTime`), heap allocations per frame (global counting
//!   allocator: this thread, and the whole process).
//! * sort of 1M rows by a numeric and by a string column (the library's own
//!   path: ours sorts a row mask, the component's delegate sorts its row index
//!   with `sort_by`, as its own examples sort their data).
//! * selection and keyboard latency: the event is injected through the
//!   window's real dispatch (`Window::dispatch_event`); latency is the dispatch
//!   call plus the next frame's render-to-paint time (the vsync wait is not
//!   counted).
//! * memory: working set after the scroll, peak working set, private bytes
//!   (minus the data's own size, the baseline before the window exists).

#![allow(clippy::too_many_lines)]

#[path = "../../bench/common/measure.rs"]
mod measure;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell as StdCell;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use blenderui_table_gpui as ours;
use gpui::{
    App, AppContext as _, Bounds, Context, Entity, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    ParentElement as _, PlatformInput, Render, SharedString, Styled as _, Window, WindowBounds, WindowOptions, canvas, div,
    point, px, rgb, size, IntoElement,
};
use gpui_component as comp;

// ---- counting allocator -------------------------------------------------------------------------------

struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
thread_local! {
    static MAIN_ALLOCS: StdCell<u64> = const { StdCell::new(0) };
}

// SAFETY: forwards to the system allocator; the counters are an atomic and a const thread-local
// without a destructor, so counting never allocates.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        let _ = MAIN_ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        let _ = MAIN_ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn main_allocs() -> u64 {
    MAIN_ALLOCS.with(StdCell::get)
}

// ---- the data -----------------------------------------------------------------------------------------

const NAMES: [&str; 62] = [
    "Cube", "Sphere", "Suzanne", "Torus", "Cone", "Plane", "Cylinder", "Icosphere", "Grid", "Circle", "Monkey", "Empty",
    "Camera", "Light", "Armature", "Lattice", "Curve", "Surface", "Metaball", "Text", "Volume", "Speaker", "Probe", "Force",
    "Bone", "Collection", "Material", "Texture", "Image", "World", "Scene", "Action", "Brush", "Palette", "Mask", "Movie",
    "Sound", "Font", "Node", "Group", "Library", "Object", "Mesh", "Vertex", "Edge", "Face", "Loop", "UV", "Shape", "Key",
    "Driver", "Modifier", "Constraint", "Particle", "Hair", "Cloth", "Fluid", "Smoke", "Rigid", "Soft", "Track", "Marker",
];

/// Rows x columns: column 0 the row number, 1 a name, 2 a price (2 decimals), the rest numbers.
struct Data {
    rows: usize,
    cols: usize,
    name: Vec<u8>,
    price: Vec<f64>,
    nums: Vec<Vec<u64>>,
}

fn mix(r: u64, c: u64) -> u64 {
    let mut x = (r + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (c + 7).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    x ^= x >> 29;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^ (x >> 32)
}

impl Data {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            name: (0..rows).map(|r| (mix(r as u64, 1) % NAMES.len() as u64) as u8).collect(),
            price: (0..rows).map(|r| (mix(r as u64, 2) % 1_000_000) as f64 / 100.0).collect(),
            nums: (3..cols).map(|c| (0..rows).map(|r| mix(r as u64, c as u64) % 1_000_003).collect()).collect(),
        }
    }

    fn text(&self, r: usize, c: usize) -> String {
        match c {
            0 => r.to_string(),
            1 => NAMES[self.name[r] as usize].to_owned(),
            2 => format!("{:.2}", self.price[r]),
            _ => self.nums[c - 3][r].to_string(),
        }
    }

    fn order(&self, a: usize, b: usize, c: usize) -> std::cmp::Ordering {
        match c {
            0 => a.cmp(&b),
            1 => NAMES[self.name[a] as usize].cmp(NAMES[self.name[b] as usize]),
            2 => self.price[a].total_cmp(&self.price[b]),
            _ => self.nums[c - 3][a].cmp(&self.nums[c - 3][b]),
        }
    }
}

// ---- this crate ---------------------------------------------------------------------------------------

impl ours::CellSource for Data {
    fn cell(&self, r: usize, c: usize) -> ours::Cell<'_> {
        match c {
            0 => ours::Cell::U64(r as u64),
            1 => ours::Cell::Str(NAMES[self.name[r] as usize]),
            2 => ours::Cell::F64(self.price[r], 2),
            _ => ours::Cell::U64(self.nums[c - 3][r]),
        }
    }
}

impl ours::TableDelegate for Data {
    fn columns_count(&self, _: &App) -> usize {
        self.cols
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows
    }
    fn column(&self, ix: usize, _: &App) -> ours::Column {
        ours::Column::new(format!("c{ix}"), format!("Column {ix}")).sortable().width(px(110.0))
    }
}

// ---- gpui-component -----------------------------------------------------------------------------------

struct CompDelegate {
    data: Data,
    /// Display row -> data row (the delegate sorts this, not the data).
    order: Vec<u32>,
}

impl comp::table::TableDelegate for CompDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.data.cols
    }
    fn rows_count(&self, _: &App) -> usize {
        self.data.rows
    }
    fn column(&self, ix: usize, _: &App) -> comp::table::Column {
        comp::table::Column::new(format!("c{ix}"), format!("Column {ix}")).sortable().width(px(110.0))
    }
    fn perform_sort(&mut self, col_ix: usize, sort: comp::table::ColumnSort, _: &mut Window, _: &mut Context<comp::table::TableState<Self>>) {
        let data = &self.data;
        match sort {
            comp::table::ColumnSort::Default => self.order.sort_unstable(),
            comp::table::ColumnSort::Ascending => self.order.sort_by(|&a, &b| data.order(a as usize, b as usize, col_ix)),
            comp::table::ColumnSort::Descending => self.order.sort_by(|&a, &b| data.order(b as usize, a as usize, col_ix)),
        }
    }
    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window, _: &mut Context<comp::table::TableState<Self>>) -> impl IntoElement {
        SharedString::from(self.data.text(self.order[row_ix] as usize, col_ix))
    }
}

// ---- the harness --------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lib {
    Ours,
    Component,
    /// The same window and views without a table: the framework's own per-frame cost.
    Blank,
}

const TABLE_W: f32 = 1200.0;
const TABLE_H: f32 = 700.0;
const ROW_H: f32 = 32.0;
const SCROLL_ROWS_PER_FRAME: usize = 2;

static SCROLLING: AtomicBool = AtomicBool::new(false);
/// Ask for another frame after each one (scrolling or not).
static REPAINT: AtomicBool = AtomicBool::new(false);
static RECORDING: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static TIMES: Mutex<Vec<f64>> = Mutex::new(Vec::new());
/// Nanoseconds of the last frame's render-to-paint time.
static LAST_NS: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static FRAME_START: StdCell<Option<Instant>> = const { StdCell::new(None) };
    /// The table entity, for the stages that act on it.
    static TABLE: StdCell<Option<Table>> = const { StdCell::new(None) };
}

fn table() -> Table {
    let t = TABLE.take();
    let c = t.clone().expect("the table exists");
    TABLE.set(t);
    c
}

#[derive(Clone)]
enum Table {
    Ours(Entity<ours::TableState<Data>>),
    Component(Entity<comp::table::TableState<CompDelegate>>),
    Blank,
}

struct Harness {
    table: Table,
    frame: usize,
}

impl Render for Harness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        FRAME_START.set(Some(Instant::now()));
        self.frame += 1;
        if SCROLLING.load(Ordering::Relaxed) {
            match &self.table {
                Table::Ours(t) => {
                    t.update(cx, |s, _| s.scroll_by_px(0.0, f64::from(ROW_H) * SCROLL_ROWS_PER_FRAME as f64, window));
                }
                Table::Component(t) => {
                    let row = self.frame * SCROLL_ROWS_PER_FRAME;
                    t.update(cx, |s, cx| s.scroll_to_row(row, cx));
                }
                Table::Blank => {}
            }
        }
        if REPAINT.load(Ordering::Relaxed) {
            window.request_animation_frame();
        }
        let holder = div().w(px(TABLE_W)).h(px(TABLE_H));
        let table = match &self.table {
            Table::Ours(t) => holder
                .child(
                    ours::DataTable::new(t)
                        .stripe(true)
                        .unit(px(ROW_H))
                        .text_size(px(14.0))
                        .header_row_height(px(ROW_H)),
                )
                .into_any_element(),
            Table::Component(t) => holder.child(comp::table::DataTable::new(t).stripe(true)).into_any_element(),
            Table::Blank => holder.into_any_element(),
        };
        div()
            .size_full()
            .bg(rgb(0x1d1d1d))
            .child(table)
            // painted after the table: marks the end of the frame's paint
            .child(canvas(|_, _, _| (), |_, (), _, _| frame_painted()).absolute().size(px(0.0)))
    }
}

fn frame_painted() {
    let Some(t0) = FRAME_START.get() else { return };
    let ns = t0.elapsed().as_nanos() as u64;
    LAST_NS.store(ns, Ordering::Relaxed);
    FRAMES.fetch_add(1, Ordering::Relaxed);
    if RECORDING.load(Ordering::Relaxed) {
        TIMES.lock().unwrap().push(ns as f64 / 1e6);
    }
}

// ---- statistics ---------------------------------------------------------------------------------------

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[(((sorted.len() - 1) as f64) * p).round() as usize]
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    pct(v, 0.5)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

struct Args {
    lib: Lib,
    rows: usize,
    cols: usize,
    secs: f64,
    warm: f64,
}

fn args() -> Args {
    let a: Vec<String> = std::env::args().collect();
    let get = |k: &str, d: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned().unwrap_or_else(|| d.to_owned());
    Args {
        lib: match get("--lib", "ours").as_str() {
            "component" => Lib::Component,
            "blank" => Lib::Blank,
            _ => Lib::Ours,
        },
        rows: get("--rows", "1000000").parse().unwrap(),
        cols: get("--cols", "20").parse().unwrap(),
        secs: get("--seconds", "8").parse().unwrap(),
        warm: get("--warmup", "2").parse().unwrap(),
    }
}

fn mb(b: usize) -> f64 {
    b as f64 / (1024.0 * 1024.0)
}

/// Wait until a frame has been painted after this call.
async fn next_frame(cx: &mut gpui::AsyncApp) {
    let n = FRAMES.load(Ordering::Relaxed);
    let t0 = Instant::now();
    while FRAMES.load(Ordering::Relaxed) == n && t0.elapsed() < Duration::from_millis(500) {
        cx.background_executor().timer(Duration::from_millis(1)).await;
    }
}

fn main() {
    let t_start = Instant::now();
    let a = args();
    let hz = std::thread::spawn(|| measure::tsc_hz(Duration::from_millis(300)));
    let data = Data::new(a.rows, a.cols);
    let data_ms = ms(t_start.elapsed());
    let base = measure::memory();
    let hz = hz.join().unwrap();
    let app = gpui_platform::application().with_assets(gpui_kit_assets::Assets);
    app.run(move |cx: &mut App| {
        comp::init(cx);
        let bounds = Bounds::centered(None, size(px(1280.0), px(740.0)), cx);
        let mut data = Some(data);
        let lib = a.lib;
        let window = cx
            .open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() }, move |window, cx| {
                let data = data.take().unwrap();
                let table = match lib {
                    Lib::Ours => Table::Ours(cx.new(|cx| ours::TableState::new(data, window, cx))),
                    Lib::Blank => Table::Blank,
                    Lib::Component => {
                        let order = (0..data.rows as u32).collect();
                        Table::Component(cx.new(|cx| comp::table::TableState::new(CompDelegate { data, order }, window, cx)))
                    }
                };
                TABLE.set(Some(table.clone()));
                let view = cx.new(|_| Harness { table, frame: 0 });
                cx.new(|cx| comp::Root::new(view, window, cx))
            })
            .unwrap();
        // the Root wraps the harness: the table state handles for the stages
        let handle = window.into();
        cx.activate(true);
        cx.spawn(async move |cx| run(cx, handle, a, t_start, data_ms, base.working_set, hz).await).detach();
    });
}

/// What one measured stage found.
struct Stage {
    frames: u64,
    fps: f64,
    mean: f64,
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    cpu_ms: f64,
    main_allocs: f64,
    proc_allocs: f64,
}

/// Run frames continuously (scrolling or repainting in place) for `warm` seconds
/// unmeasured, then measure `secs` seconds.
async fn stage(cx: &mut gpui::AsyncApp, handle: gpui::AnyWindowHandle, warm: f64, secs: f64, hz: f64, scroll: bool) -> Stage {
    SCROLLING.store(scroll, Ordering::Relaxed);
    REPAINT.store(true, Ordering::Relaxed);
    // the first frame; each render then asks for the next
    let _ = cx.update(|app| handle.update(app, |_, window, _| window.refresh()));
    cx.background_executor().timer(Duration::from_secs_f64(warm)).await;
    TIMES.lock().unwrap().clear();
    let (f0, c0, p0, w0) = (FRAMES.load(Ordering::Relaxed), measure::cpu_cycles(), ALLOCS.load(Ordering::Relaxed), Instant::now());
    let m0 = cx.update(|_| main_allocs());
    RECORDING.store(true, Ordering::Relaxed);
    cx.background_executor().timer(Duration::from_secs_f64(secs)).await;
    RECORDING.store(false, Ordering::Relaxed);
    let (frames, wall) = (FRAMES.load(Ordering::Relaxed) - f0, w0.elapsed());
    let cpu = Duration::from_secs_f64((measure::cpu_cycles() - c0) as f64 / hz);
    let proc_allocs = ALLOCS.load(Ordering::Relaxed) - p0;
    let main_allocs = cx.update(|_| main_allocs()) - m0;
    SCROLLING.store(false, Ordering::Relaxed);
    REPAINT.store(false, Ordering::Relaxed);
    let mut times = std::mem::take(&mut *TIMES.lock().unwrap());
    times.sort_by(f64::total_cmp);
    let n = frames.max(1) as f64;
    Stage {
        frames,
        fps: frames as f64 / wall.as_secs_f64(),
        mean: times.iter().sum::<f64>() / times.len().max(1) as f64,
        p50: pct(&times, 0.5),
        p95: pct(&times, 0.95),
        p99: pct(&times, 0.99),
        max: times.last().copied().unwrap_or(0.0),
        cpu_ms: cpu.as_secs_f64() * 1000.0 / n,
        main_allocs: main_allocs as f64 / n,
        proc_allocs: proc_allocs as f64 / n,
    }
}

async fn run(cx: &mut gpui::AsyncApp, handle: gpui::AnyWindowHandle, a: Args, t_start: Instant, data_ms: f64, base_ws: usize, hz: f64) {
    // first frame
    while FRAMES.load(Ordering::Relaxed) == 0 {
        cx.background_executor().timer(Duration::from_millis(1)).await;
    }
    let first_frame_ms = ms(t_start.elapsed()) - data_ms;
    cx.background_executor().timer(Duration::from_millis(300)).await;

    // continuous scroll, then repainting the same rows (no new text to shape)
    let scroll = stage(cx, handle, a.warm, a.secs, hz, true).await;
    let after_scroll = measure::memory();
    let still = stage(cx, handle, 1.0, (a.secs / 2.0).max(2.0), hz, false).await;
    cx.background_executor().timer(Duration::from_millis(300)).await;

    // selection (a click on a row) and keyboard (Down) latency
    let mut select = Vec::new();
    let mut key = Vec::new();
    let mut key_works = false;
    for rep in 0..15usize {
        let pos = point(px(300.0), px(ROW_H * (3.0 + (rep % 8) as f32) + ROW_H / 2.0));
        let t = Instant::now();
        let _ = cx.update(|app| {
            handle.update(app, |_, window, app| {
                let down = MouseDownEvent { button: MouseButton::Left, position: pos, modifiers: Modifiers::default(), click_count: 1, first_mouse: false };
                let up = MouseUpEvent { button: MouseButton::Left, position: pos, modifiers: Modifiers::default(), click_count: 1 };
                window.dispatch_event(PlatformInput::MouseDown(down), app);
                window.dispatch_event(PlatformInput::MouseUp(up), app);
            })
        });
        let dispatch = ms(t.elapsed());
        next_frame(cx).await;
        select.push(dispatch + LAST_NS.load(Ordering::Relaxed) as f64 / 1e6);
        let t = Instant::now();
        let handled = cx
            .update(|app| handle.update(app, |_, window, app| window.dispatch_keystroke(Keystroke::parse("down").unwrap(), app)))
            .unwrap_or(false);
        let dispatch = ms(t.elapsed());
        next_frame(cx).await;
        key_works |= handled;
        key.push(dispatch + LAST_NS.load(Ordering::Relaxed) as f64 / 1e6);
        cx.background_executor().timer(Duration::from_millis(20)).await;
    }

    // sorts of all rows
    let mut sort_num = Vec::new();
    let mut sort_str = Vec::new();
    for (col, out) in [(3usize.min(a.cols - 1), &mut sort_num), (1, &mut sort_str)] {
        for sort in [ours::ColumnSort::Ascending, ours::ColumnSort::Descending, ours::ColumnSort::Ascending] {
            let (lib, tab) = (a.lib, table());
            let t = Instant::now();
            let _ = cx.update(|app| {
                handle.update(app, |_, window, app| match (&tab, lib) {
                    (Table::Ours(s), Lib::Ours) => s.update(app, |s, cx| s.sort_by(col, sort, window, cx)),
                    (Table::Component(s), Lib::Component) => s.update(app, |s, cx| {
                        let cs = match sort {
                            ours::ColumnSort::Ascending => comp::table::ColumnSort::Ascending,
                            ours::ColumnSort::Descending => comp::table::ColumnSort::Descending,
                            ours::ColumnSort::Default => comp::table::ColumnSort::Default,
                        };
                        comp::table::TableDelegate::perform_sort(s.delegate_mut(), col, cs, window, cx);
                        cx.notify();
                    }),
                    _ => {}
                })
            });
            out.push(ms(t.elapsed()));
            next_frame(cx).await;
        }
    }
    let end = measure::memory();
    let (sn, ss) = (median(&mut sort_num), median(&mut sort_str));
    println!(
        "{{\"lib\":\"{}\",\"rows\":{},\"cols\":{},\"data_ms\":{:.0},\"first_frame_ms\":{:.1},\"scroll_frames\":{},\"fps\":{:.1},\
         \"frame_ms_mean\":{:.3},\"frame_ms_p50\":{:.3},\"frame_ms_p95\":{:.3},\"frame_ms_p99\":{:.3},\"frame_ms_max\":{:.3},\
         \"cpu_ms_per_frame\":{:.3},\"main_allocs_per_frame\":{:.1},\"proc_allocs_per_frame\":{:.1},\
         \"still_frame_ms_p50\":{:.3},\"still_cpu_ms_per_frame\":{:.3},\"still_main_allocs_per_frame\":{:.1},\
         \"select_ms\":{:.3},\"key_ms\":{:.3},\"key_works\":{},\"sort_numeric_ms\":{:.1},\"sort_string_ms\":{:.1},\
         \"data_mb\":{:.1},\"ws_after_scroll_mb\":{:.1},\"ws_delta_mb\":{:.1},\"peak_ws_mb\":{:.1},\"private_mb\":{:.1}}}",
        match a.lib {
            Lib::Ours => "ours",
            Lib::Component => "gpui-component",
            Lib::Blank => "blank (no table)",
        },
        a.rows,
        a.cols,
        data_ms,
        first_frame_ms,
        scroll.frames,
        scroll.fps,
        scroll.mean,
        scroll.p50,
        scroll.p95,
        scroll.p99,
        scroll.max,
        scroll.cpu_ms,
        scroll.main_allocs,
        scroll.proc_allocs,
        still.p50,
        still.cpu_ms,
        still.main_allocs,
        median(&mut select),
        median(&mut key),
        key_works,
        sn,
        ss,
        mb(base_ws),
        mb(after_scroll.working_set),
        mb(after_scroll.working_set) - mb(base_ws),
        mb(end.peak_working_set),
        mb(end.private_bytes),
    );
    std::process::exit(0);
}

