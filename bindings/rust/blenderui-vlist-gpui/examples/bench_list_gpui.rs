//! Head-to-head on the same gpui (gpui-pre 0.3.11), the same chat messages
//! and the same window: this crate's variable list vs gpui's own `list`
//! (variable height, a `SumTree`, items are elements) vs gpui-component
//! 0.9.0's `virtual_list` (variable item sizes, which the caller must supply
//! for every item up front).
//!
//! ```text
//! cargo build --release --features bench --example bench_list_gpui -j 1
//! node bench/run.mjs                 # per size, 3 runs per library, medians
//! target/release/examples/bench_list_gpui.exe --lib ours --n 1000000
//! ```
//!
//! One process runs one library at one size and prints one JSON line.
//! A watchdog thread ends the process (still printing a JSON line that says
//! what failed and how far it got) when it exceeds a memory cap or a time
//! limit, so a library that cannot hold a size does not take the machine
//! down. 10M messages come from the file-backed log in all three (the
//! competitors fetch each row's record from it when they render it).
//!
//! Measured per library:
//!
//! * `setup_ms`: building the library's state for `n` items (ours: the height
//!   index; `list`: the SumTree; `virtual_list`: measuring every message for
//!   the sizes it needs, with the same wrapping function).
//! * `first_frame_ms`: process start (after the data exists) to the first
//!   painted frame, opened at the newest message.
//! * scroll: continuous, 57 px per frame, from the middle of the list:
//!   per-frame **render to end of paint** on the main thread (GPU submission
//!   and vsync wait excluded), p50/p95/p99, frames per second, process CPU
//!   time per frame (all threads), heap allocations per frame (UI thread).
//! * still: repainting a fixed position (the steady-state cost).
//! * jump: 40 jumps to random items, render-to-paint of the landing frame.
//! * memory: working set above the baseline taken before anything was built.
//!
//! Caveat that favours the competitors: `virtual_list` is given sizes
//! computed with a cheap synthetic text width, so its rows may wrap
//! differently from what it draws; `list` measures real elements. Ours
//! measures with gpui's real text shaper.

#![allow(clippy::too_many_lines, clippy::all, clippy::pedantic)]

#[path = "../../bench/common/measure.rs"]
mod measure;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell as StdCell;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use blenderui_vlist_gpui as ours;
use blenderui_vlist_gpui::{Align, ItemSource, Options};
use blenderui_vlist::log::{LogSource, LogWriter};
use blenderui_vlist::source::{Page, Record, FLAG_DIVIDER, FLAG_HEADER};
use blenderui_vlist::{synth, ChatStyle, Measurer, TextLayout, VecSource};
use gpui::{
    AnyElement, App, AppContext as _, Bounds, Context, Entity, IntoElement, ListAlignment, ListOffset, ListState, ParentElement as _,
    Render, ScrollStrategy, SharedString, Size, Styled as _, Window, WindowBounds, WindowOptions, canvas, div, list, point, px, rgb, size,
};
use gpui_component as comp;

// ---- counting allocator -----------------------------------------------------------------------------

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);
thread_local! {
    static MAIN_ALLOCS: StdCell<u64> = const { StdCell::new(0) };
}

// SAFETY: forwards to the system allocator; the counters are an atomic and a const thread-local.
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

// ---- the window ---------------------------------------------------------------------------------------

const W: f32 = 700.0;
const H: f32 = 600.0;
const SCROLL_PX: f32 = 57.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lib {
    Ours,
    GpuiList,
    Virtual,
    Blank,
}

static SCROLLING: AtomicBool = AtomicBool::new(false);
static REPAINT: AtomicBool = AtomicBool::new(false);
static RECORDING: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static LAST_NS: AtomicU64 = AtomicU64::new(0);
static TIMES: Mutex<Vec<f64>> = Mutex::new(Vec::new());
static STAGE: AtomicUsize = AtomicUsize::new(0);
const STAGES: [&str; 6] = ["setup", "first frame", "scroll", "still", "jump", "done"];

thread_local! {
    static FRAME_START: StdCell<Option<Instant>> = const { StdCell::new(None) };
}

type Dyn = Rc<dyn ItemSource>;

#[derive(Clone)]
enum View {
    Ours(Entity<ours::ListState<Dyn, ChatStyle>>),
    List(ListState, Dyn),
    Virtual(Entity<Rows>, comp::VirtualListScrollHandle, Rc<Vec<Size<gpui::Pixels>>>),
    Blank,
}

/// The rows `virtual_list` renders for a visible range.
struct Rows {
    src: Dyn,
    page: Page,
}

impl Render for Rows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct Harness {
    view: View,
    frame: usize,
}

/// One chat row as a tree of gpui elements (what `list` and `virtual_list` need).
fn row(rec: &Record<'_>) -> AnyElement {
    let dim = rgb(0x8a8a8a);
    let mut col = div().flex().flex_col().w(px(W - 10.0 - 32.0 - 10.0 - 10.0 - 10.0)).text_size(px(14.0)).line_height(px(19.0)).text_color(rgb(0xe6e6e6));
    if rec.flags & FLAG_HEADER != 0 {
        col = col.child(
            div().h(px(20.0)).flex().gap(px(8.0)).child(SharedString::from(format!("User {}", rec.sender))).child(div().text_color(dim).child(SharedString::from(format!(
                "{:02}:{:02}",
                rec.time % 86_400 / 3600,
                rec.time / 60 % 60
            )))),
        );
    }
    col = col.child(SharedString::from(rec.text.to_owned()));
    if rec.media_h > 0 {
        col = col.child(div().mt(px(3.0)).h(px(f32::from(rec.media_h))).rounded(px(4.0)).bg(rgb(0x3a3a3a)));
    }
    let avatar = if rec.flags & FLAG_HEADER != 0 {
        div().size(px(32.0)).rounded_full().bg(rgb(0x4d70b3)).flex().justify_center().items_center().child(SharedString::from(((b'A' + (rec.sender % 26) as u8) as char).to_string()))
    } else {
        div().w(px(32.0))
    };
    let mut r = div().w_full().flex().flex_col();
    if rec.flags & FLAG_DIVIDER != 0 {
        r = r.child(div().h(px(30.0)).flex().justify_center().items_center().text_size(px(13.0)).text_color(dim).child("2023-11-14"));
    }
    r.child(div().flex().gap(px(10.0)).px(px(10.0)).pt(px(if rec.flags & FLAG_HEADER != 0 { 10.0 } else { 1.0 })).pb(px(3.0)).child(avatar).child(col)).into_any_element()
}

thread_local! {
    static VIEW: StdCell<Option<View>> = const { StdCell::new(None) };
}

fn view() -> View {
    let v = VIEW.take();
    let c = v.clone().expect("the view exists");
    VIEW.set(v);
    c
}

impl Render for Harness {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        FRAME_START.set(Some(Instant::now()));
        self.frame += 1;
        if SCROLLING.load(Ordering::Relaxed) {
            match &self.view {
                View::Ours(s) => {
                    s.update(cx, |s, _| s.view.scroll_by(f64::from(SCROLL_PX)));
                }
                View::List(s, _) => s.scroll_by(px(SCROLL_PX)),
                View::Virtual(_, h, _) => {
                    let o = h.base_handle().offset();
                    h.base_handle().set_offset(point(o.x, o.y - px(SCROLL_PX)));
                }
                View::Blank => {}
            }
        }
        if REPAINT.load(Ordering::Relaxed) {
            window.request_animation_frame();
        }
        let holder = div().w(px(W)).h(px(H));
        let body = match &self.view {
            View::Ours(s) => holder.child(ours::variable_list(s)).into_any_element(),
            View::List(s, src) => {
                let src = src.clone();
                holder
                    .child(
                        list(s.clone(), move |ix, _, _| {
                            let mut page = Page::new();
                            src.fetch(ix..ix + 1, &mut page);
                            page.record(ix).map_or_else(|| div().into_any_element(), |r| row(&r))
                        })
                        .size_full(),
                    )
                    .into_any_element()
            }
            View::Virtual(rows, h, sizes) => holder
                .child(
                    comp::v_virtual_list(rows.clone(), "rows", sizes.clone(), |rows: &mut Rows, range: Range<usize>, _, _| {
                        rows.src.fetch(range.clone(), &mut rows.page);
                        range.filter_map(|i| rows.page.record(i).map(|r| row(&r))).collect::<Vec<_>>()
                    })
                    .track_scroll(h),
                )
                .into_any_element(),
            View::Blank => holder.into_any_element(),
        };
        div().size_full().bg(rgb(0x1d1d1d)).child(body).child(canvas(|_, _, _| (), |_, (), _, _| frame_painted()).absolute().size(px(0.0)))
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

// ---- statistics -----------------------------------------------------------------------------------------

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[(((sorted.len() - 1) as f64) * p).round() as usize]
}

fn mb(b: usize) -> f64 {
    b as f64 / (1024.0 * 1024.0)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

struct Args {
    lib: Lib,
    n: usize,
    secs: f64,
    warm: f64,
    dir: PathBuf,
    cap_mb: f64,
    limit_s: f64,
}

fn args() -> Args {
    let a: Vec<String> = std::env::args().collect();
    let get = |k: &str, d: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned().unwrap_or_else(|| d.to_owned());
    Args {
        lib: match get("--lib", "ours").as_str() {
            "list" => Lib::GpuiList,
            "virtual" => Lib::Virtual,
            "blank" => Lib::Blank,
            _ => Lib::Ours,
        },
        n: get("--n", "100000").parse().unwrap(),
        secs: get("--seconds", "6").parse().unwrap(),
        warm: get("--warmup", "2").parse().unwrap(),
        dir: PathBuf::from(get("--dir", "target/bench-logs")),
        cap_mb: get("--cap-mb", "4096").parse().unwrap(),
        limit_s: get("--limit-seconds", "150").parse().unwrap(),
    }
}

fn lib_name(l: Lib) -> &'static str {
    match l {
        Lib::Ours => "blenderui-vlist-gpui",
        Lib::GpuiList => "gpui list (SumTree)",
        Lib::Virtual => "gpui-component virtual_list",
        Lib::Blank => "blank (no list)",
    }
}

/// Bits of the setup time (ms) for the watchdog report.
static SETUP_MS_BITS: AtomicU64 = AtomicU64::new(0);

/// Ends the process, printing what failed, when memory or time run out.
fn watchdog(a: &Args, base_ws: usize) {
    let (cap, limit, lib, n) = (a.cap_mb, a.limit_s, lib_name(a.lib), a.n);
    let start = Instant::now();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let ws = measure::memory().working_set;
            let over = mb(ws) - mb(base_ws);
            let why = if over > cap {
                Some("memory cap")
            } else if start.elapsed().as_secs_f64() > limit {
                Some("time limit")
            } else {
                None
            };
            if let Some(why) = why {
                println!(
                    "{{\"lib\":\"{lib}\",\"n\":{n},\"failed\":\"{why}\",\"stage\":\"{}\",\"elapsed_s\":{:.1},\"ram_above_data_mb\":{:.0},\"setup_ms\":{:.0},\"cap_mb\":{cap:.0},\"limit_s\":{limit:.0}}}",
                    STAGES[STAGE.load(Ordering::Relaxed).min(5)],
                    start.elapsed().as_secs_f64(),
                    over,
                    f64::from_bits(SETUP_MS_BITS.load(Ordering::Relaxed)),
                );
                std::process::exit(3);
            }
        }
    });
}

/// The data: in memory up to a million messages, the file-backed log above.
fn make_source(a: &Args) -> Dyn {
    if a.n <= 1_000_000 {
        return Rc::new(VecSource::from_messages(synth::messages(0, a.n as u64)));
    }
    std::fs::create_dir_all(&a.dir).expect("dir");
    let base = a.dir.join(format!("chat{}", a.n));
    if LogSource::open(&base).map(|s| s.len()).unwrap_or(0) != a.n {
        let mut w = LogWriter::create(&base).expect("create log");
        let mut prev = None;
        for i in 0..a.n as u64 {
            let mut m = synth::message(i);
            m.flags = blenderui_vlist::chat::classify(prev, m.sender, m.time);
            prev = Some((m.sender, m.time));
            w.append(&m).expect("write");
        }
        w.finish().expect("flush");
    }
    Rc::new(LogSource::open(&base).expect("open log"))
}

/// The heights `virtual_list` needs, one per message, from our wrapping.
fn virtual_sizes(src: &Dyn) -> Rc<Vec<Size<gpui::Pixels>>> {
    let style = ChatStyle::new(ours::Palette::blender_dark());
    let mut text = TextLayout::new(4096);
    let mut tw = |s: &str, z: f32| s.len() as f32 * z * 0.5;
    let mut page = Page::new();
    let mut sizes = Vec::with_capacity(src.len());
    let mut start = 0;
    while start < src.len() {
        let end = (start + 4096).min(src.len());
        src.fetch(start..end, &mut page);
        for i in start..end {
            let h = page.record(i).map_or(60.0, |r| text.height(&r, &style, W, &mut tw).px() as f32);
            sizes.push(size(px(W), px(h)));
        }
        text.next_generation();
        start = end;
    }
    Rc::new(sizes)
}

/// Wait until a frame has been painted after this call.
async fn next_frame(cx: &mut gpui::AsyncApp) {
    let n = FRAMES.load(Ordering::Relaxed);
    let t0 = Instant::now();
    while FRAMES.load(Ordering::Relaxed) == n && t0.elapsed() < Duration::from_millis(2000) {
        cx.background_executor().timer(Duration::from_millis(1)).await;
    }
}

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
}

async fn stage(cx: &mut gpui::AsyncApp, handle: gpui::AnyWindowHandle, warm: f64, secs: f64, hz: f64, scroll: bool) -> Stage {
    SCROLLING.store(scroll, Ordering::Relaxed);
    REPAINT.store(true, Ordering::Relaxed);
    let _ = cx.update(|app| handle.update(app, |_, window, _| window.refresh()));
    cx.background_executor().timer(Duration::from_secs_f64(warm)).await;
    TIMES.lock().unwrap().clear();
    let (f0, c0, w0) = (FRAMES.load(Ordering::Relaxed), measure::cpu_cycles(), Instant::now());
    let m0 = cx.update(|_| main_allocs());
    RECORDING.store(true, Ordering::Relaxed);
    cx.background_executor().timer(Duration::from_secs_f64(secs)).await;
    RECORDING.store(false, Ordering::Relaxed);
    let (frames, wall) = (FRAMES.load(Ordering::Relaxed) - f0, w0.elapsed());
    let cpu = Duration::from_secs_f64((measure::cpu_cycles() - c0) as f64 / hz);
    let main_allocs = cx.update(|_| main_allocs()) - m0;
    SCROLLING.store(false, Ordering::Relaxed);
    REPAINT.store(false, Ordering::Relaxed);
    let mut t = std::mem::take(&mut *TIMES.lock().unwrap());
    t.sort_by(f64::total_cmp);
    let n = frames.max(1) as f64;
    Stage {
        frames,
        fps: frames as f64 / wall.as_secs_f64(),
        mean: t.iter().sum::<f64>() / t.len().max(1) as f64,
        p50: pct(&t, 0.5),
        p95: pct(&t, 0.95),
        p99: pct(&t, 0.99),
        max: t.last().copied().unwrap_or(0.0),
        cpu_ms: cpu.as_secs_f64() * 1000.0 / n,
        main_allocs: main_allocs as f64 / n,
    }
}

fn main() {
    let t_start = Instant::now();
    let a = args();
    let hz = std::thread::spawn(|| measure::tsc_hz(Duration::from_millis(300)));
    let src = make_source(&a);
    let data_ms = ms(t_start.elapsed());
    let base = measure::memory();
    watchdog(&a, base.working_set);
    let hz = hz.join().unwrap();
    let app = gpui_platform::application().with_assets(gpui_kit_assets::Assets);
    app.run(move |cx: &mut App| {
        comp::init(cx);
        let bounds = Bounds::centered(None, size(px(W + 40.0), px(H + 40.0)), cx);
        let lib = a.lib;
        let n = a.n;
        let src2 = src.clone();
        let window = cx
            .open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() }, move |window, cx| {
                let t = Instant::now();
                let view = match lib {
                    Lib::Ours => {
                        let list = ours::List::new(src2.clone(), ChatStyle::new(ours::Palette::blender_dark()), W, H, Options { start_at_end: true, ..Options::default() }).expect("index");
                        View::Ours(cx.new(|cx| ours::ListState::new(list, cx)))
                    }
                    Lib::GpuiList => {
                        let s = ListState::new(n, ListAlignment::Bottom, px(200.0));
                        s.scroll_to_end();
                        View::List(s, src2.clone())
                    }
                    Lib::Virtual => {
                        let sizes = virtual_sizes(&src2);
                        let h = comp::VirtualListScrollHandle::new();
                        h.scroll_to_bottom();
                        View::Virtual(cx.new(|_| Rows { src: src2.clone(), page: Page::new() }), h, sizes)
                    }
                    Lib::Blank => View::Blank,
                };
                SETUP_MS_BITS.store(ms(t.elapsed()).to_bits(), Ordering::Relaxed);
                STAGE.store(1, Ordering::Relaxed);
                VIEW.set(Some(view.clone()));
                let root = cx.new(|_| Harness { view, frame: 0 });
                cx.new(|cx| comp::Root::new(root, window, cx))
            })
            .unwrap();
        let handle = window.into();
        cx.activate(true);
        cx.spawn(async move |cx| run(cx, handle, a, t_start, data_ms, base.working_set, hz).await).detach();
    });
}

async fn run(cx: &mut gpui::AsyncApp, handle: gpui::AnyWindowHandle, a: Args, t_start: Instant, data_ms: f64, base_ws: usize, hz: f64) {
    while FRAMES.load(Ordering::Relaxed) == 0 {
        cx.background_executor().timer(Duration::from_millis(1)).await;
    }
    let first_frame_ms = ms(t_start.elapsed()) - data_ms;
    cx.background_executor().timer(Duration::from_millis(300)).await;
    let setup_ms = f64::from_bits(SETUP_MS_BITS.load(Ordering::Relaxed));
    let n = a.n;

    // move to the middle (the scroll stage starts there), then scroll continuously
    let _ = cx.update(|app| {
        handle.update(app, |_, window, app| {
            match view() {
                View::Ours(s) => s.update(app, |s, _| s.view.jump_to(n / 2, Align::Start)),
                View::List(s, _) => s.scroll_to(ListOffset { item_ix: n / 2, offset_in_item: px(0.0) }),
                View::Virtual(_, h, _) => h.scroll_to_item(n / 2, ScrollStrategy::Top),
                View::Blank => {}
            }
            window.refresh();
        })
    });
    next_frame(cx).await;
    STAGE.store(2, Ordering::Relaxed);
    let scroll = stage(cx, handle, a.warm, a.secs, hz, true).await;
    let after = measure::memory();
    STAGE.store(3, Ordering::Relaxed);
    let still = stage(cx, handle, 1.0, (a.secs / 2.0).max(2.0), hz, false).await;

    // jumps to random items
    STAGE.store(4, Ordering::Relaxed);
    let mut jumps = Vec::new();
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    for _ in 0..40 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let item = (x % n as u64) as usize;
        let t = Instant::now();
        let _ = cx.update(|app| {
            handle.update(app, |_, window, app| {
                match view() {
                    View::Ours(s) => s.update(app, |s, _| s.view.jump_to(item, Align::Start)),
                    View::List(s, _) => s.scroll_to(ListOffset { item_ix: item, offset_in_item: px(0.0) }),
                    View::Virtual(_, h, _) => h.scroll_to_item(item, ScrollStrategy::Top),
                    View::Blank => {}
                }
                window.refresh();
            })
        });
        let dispatch = ms(t.elapsed());
        next_frame(cx).await;
        jumps.push(dispatch + LAST_NS.load(Ordering::Relaxed) as f64 / 1e6);
        cx.background_executor().timer(Duration::from_millis(30)).await;
    }
    jumps.sort_by(f64::total_cmp);
    STAGE.store(5, Ordering::Relaxed);
    let end = measure::memory();
    println!(
        "{{\"lib\":\"{}\",\"n\":{},\"data_ms\":{:.0},\"setup_ms\":{:.1},\"first_frame_ms\":{:.1},\"scroll_frames\":{},\"fps\":{:.1},\
         \"frame_ms_mean\":{:.3},\"frame_ms_p50\":{:.3},\"frame_ms_p95\":{:.3},\"frame_ms_p99\":{:.3},\"frame_ms_max\":{:.3},\
         \"cpu_ms_per_frame\":{:.3},\"main_allocs_per_frame\":{:.1},\
         \"still_frame_ms_p50\":{:.3},\"still_cpu_ms_per_frame\":{:.3},\"still_main_allocs_per_frame\":{:.1},\
         \"jump_ms_p50\":{:.3},\"jump_ms_p95\":{:.3},\"jump_ms_max\":{:.3},\
         \"ram_above_data_mb\":{:.1},\"peak_above_data_mb\":{:.1},\"private_mb\":{:.1}}}",
        lib_name(a.lib),
        a.n,
        data_ms,
        setup_ms,
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
        still.p50,
        still.cpu_ms,
        still.main_allocs,
        pct(&jumps, 0.5),
        pct(&jumps, 0.95),
        jumps.last().copied().unwrap_or(0.0),
        mb(after.working_set) - mb(base_ws),
        mb(end.peak_working_set) - mb(base_ws),
        mb(end.private_bytes),
    );
    std::process::exit(0);
}
