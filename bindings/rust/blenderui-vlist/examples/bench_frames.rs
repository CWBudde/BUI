//! Whole-frame cost of the chat list core (layout + paint into a counting
//! sink), one process per configuration, one JSON line out.
//!
//! ```text
//! cargo build --release -p blenderui-vlist --example bench_frames
//! target/release/examples/bench_frames --n 1000000 --source vec
//! target/release/examples/bench_frames --n 10000000 --source log --dir <scratch dir>
//! node blenderui-vlist/bench/run.mjs      # 3 runs per size, medians, results.{json,md}
//! ```
//!
//! What is measured (Windows: working set and private bytes come from the
//! process, `measure.rs`; heap bytes from a counting allocator):
//!
//! * `build_ms`: building the source's index (the list's height index over
//!   the items, no item touched), `first_ms`: layout + paint of the first
//!   frame opened at the newest item, `data_ms`: creating the data.
//! * scroll: 3000 frames of 57 px (3 lines) from the middle: per-frame
//!   layout + paint, p50 / p95 / p99 / max, allocations per frame.
//! * jump: 300 jumps to random items (layout + paint of the landing frame).
//! * stream (in-memory sources): the last message grows by a token per
//!   frame while pinned to the bottom (edit + layout + paint).
//! * memory after all of it.
//!
//! The text width is a cheap synthetic function (a length times a constant),
//! and the sink only counts calls: this is the list's own cost, without the
//! engine's text shaper or rasterizer. The blenderui and gpui adapters add
//! theirs.

#![allow(clippy::too_many_lines, clippy::type_complexity, clippy::needless_option_as_deref, clippy::useless_format, clippy::map_unwrap_or, clippy::missing_const_for_fn)]

#[allow(clippy::all, clippy::pedantic, clippy::use_self, clippy::missing_const_for_fn, clippy::redundant_clone)]
#[path = "../../bench/common/measure.rs"]
mod measure;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

use blenderui_paint::testing::Rng;
use blenderui_paint::{PaintSink, Palette, Rect, Rgba};
use blenderui_vlist::log::{LogSource, LogWriter};
use blenderui_vlist::synth;
use blenderui_vlist::{Align, ChatList, ChatStyle, ItemSource, List, Options, VecSource};

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
thread_local! {
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

// SAFETY: forwards to the system allocator; counters are a relaxed atomic and a const thread-local.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        LIVE.fetch_add(l.size(), Relaxed);
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        LIVE.fetch_add(n, Relaxed);
        LIVE.fetch_sub(l.size(), Relaxed);
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn allocs() -> u64 {
    ALLOCS.with(Cell::get)
}

#[derive(Default)]
struct Null {
    calls: usize,
}

impl PaintSink for Null {
    fn fill(&mut self, _: Rect, _: Rgba) {
        self.calls += 1;
    }
    fn stroke(&mut self, _: Rect, _: Rgba, _: f32) {
        self.calls += 1;
    }
    fn round(&mut self, _: Rect, _: f32, _: Rgba) {
        self.calls += 1;
    }
    fn line(&mut self, _: (f32, f32), _: (f32, f32), _: Rgba, _: f32) {
        self.calls += 1;
    }
    fn text(&mut self, _: f32, _: f32, _: &str, _: f32, _: Rgba) {
        self.calls += 1;
    }
    fn text_width(&mut self, t: &str, size: f32) -> f32 {
        t.len() as f32 * size * 0.5
    }
    fn clipped(&mut self, _: Rect, f: &mut dyn FnMut(&mut dyn PaintSink)) {
        f(self);
    }
}

fn tw(s: &str, size: f32) -> f32 {
    s.len() as f32 * size * 0.5
}

struct Pct {
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    mean: f64,
}

fn pct(mut v: Vec<f64>) -> Pct {
    v.sort_by(f64::total_cmp);
    let at = |q: f64| v[((v.len() as f64 * q) as usize).min(v.len() - 1)];
    Pct { p50: at(0.5), p95: at(0.95), p99: at(0.99), max: v[v.len() - 1], mean: v.iter().sum::<f64>() / v.len() as f64 }
}

fn us(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e6
}

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

fn frame<S: ItemSource>(l: &mut List<S, ChatStyle>, sink: &mut Null) {
    l.layout(&mut tw);
    l.paint(sink);
}

struct Out {
    json: Vec<String>,
    base_ws: usize,
    base_private: usize,
    base_heap: usize,
}

impl Out {
    fn put(&mut self, k: &str, v: f64) {
        self.json.push(format!("\"{k}\":{v:.3}"));
    }
    /// Memory in use now, over the baseline taken before the data existed.
    fn memory(&mut self) {
        let m = measure::memory();
        self.put("working_set_mb", (m.working_set as f64 - self.base_ws as f64) / 1048576.0);
        self.put("peak_working_set_mb", (m.peak_working_set as f64 - self.base_ws as f64) / 1048576.0);
        self.put("private_mb", (m.private_bytes as f64 - self.base_private as f64) / 1048576.0);
        self.put("live_heap_mb", (LIVE.load(Relaxed) as f64 - self.base_heap as f64) / 1048576.0);
    }
    fn pct(&mut self, k: &str, p: &Pct) {
        for (s, v) in [("p50", p.p50), ("p95", p.p95), ("p99", p.p99), ("max", p.max), ("mean", p.mean)] {
            self.put(&format!("{k}_{s}_us"), v);
        }
    }
}

/// The measurement, for any source. `stream` edits the last message (in-memory sources only).
fn run<S: ItemSource>(mut list: List<S, ChatStyle>, n: usize, out: &mut Out, mut stream: Option<&mut dyn FnMut(&mut List<S, ChatStyle>, usize)>) {
    let mut sink = Null::default();
    let t = Instant::now();
    frame(&mut list, &mut sink);
    out.put("first_us", us(t));
    let mut rng = Rng::new(n as u64);

    // scroll from the middle: 3000 frames of three lines
    list.view.jump_to(n / 2, Align::Start);
    for _ in 0..30 {
        frame(&mut list, &mut sink);
    }
    let mut times = Vec::with_capacity(3000);
    let a0 = allocs();
    for _ in 0..3000 {
        let t = Instant::now();
        list.view.scroll_by(57.0);
        frame(&mut list, &mut sink);
        times.push(us(t));
    }
    out.put("scroll_allocs_per_frame", (allocs() - a0) as f64 / 3000.0);
    out.pct("scroll", &pct(times));

    // a settled frame (nothing moved): the cost of a steady frame
    let mut times = Vec::with_capacity(2000);
    let a0 = allocs();
    for _ in 0..2000 {
        let t = Instant::now();
        frame(&mut list, &mut sink);
        times.push(us(t));
    }
    out.put("settled_allocs_per_frame", (allocs() - a0) as f64 / 2000.0);
    out.pct("settled", &pct(times));

    // jumps
    let mut times = Vec::with_capacity(300);
    for _ in 0..300 {
        let item = rng.below(n as u64) as usize;
        let t = Instant::now();
        list.view.jump_to(item, Align::Center);
        frame(&mut list, &mut sink);
        times.push(us(t));
    }
    out.pct("jump", &pct(times));

    // streaming growth of the last message, pinned to the bottom
    if let Some(grow) = stream.as_deref_mut() {
        list.view.scroll_to_end();
        for _ in 0..20 {
            frame(&mut list, &mut sink);
        }
        let mut times = Vec::with_capacity(500);
        for k in 0..500 {
            let t = Instant::now();
            grow(&mut list, k);
            frame(&mut list, &mut sink);
            times.push(us(t));
        }
        out.pct("stream", &pct(times));
    }
    black_box(sink.calls);
    out.put("measured_items", list.view.index().measured_items() as f64);
    out.put("index_heap_bytes", list.view.index().heap_bytes() as f64);
    out.memory();
}

fn main() {
    let n: usize = arg("--n").and_then(|v| v.parse().ok()).expect("--n <items>");
    let source = arg("--source").unwrap_or_else(|| "vec".into());
    let base = measure::memory();
    let mut out = Out { json: vec![format!("\"lib\":\"blenderui-vlist\""), format!("\"n\":{n}"), format!("\"source\":\"{source}\"")], base_ws: base.working_set, base_private: base.private_bytes, base_heap: LIVE.load(Relaxed) };
    let base_heap = out.base_heap;
    let opts = Options { start_at_end: true, ..Options::default() };
    let pal = Palette::blender_dark();

    let t0 = Instant::now();
    if source == "log" {
        let dir = PathBuf::from(arg("--dir").expect("--dir <scratch dir> for the log"));
        std::fs::create_dir_all(&dir).expect("dir");
        let base = dir.join(format!("chat{n}"));
        let have = LogSource::open(&base).map(|s| s.len()).unwrap_or(0);
        if have != n {
            let mut w = LogWriter::create(&base).expect("create log");
            let mut prev = None;
            for i in 0..n as u64 {
                let mut m = synth::message(i);
                m.flags = blenderui_vlist::chat::classify(prev, m.sender, m.time);
                prev = Some((m.sender, m.time));
                w.append(&m).expect("write");
            }
            w.finish().expect("flush");
        }
        out.put("data_ms", t0.elapsed().as_secs_f64() * 1e3);
        let src = LogSource::open(&base).expect("open log");
        let before = (measure::memory(), LIVE.load(Relaxed));
        let t = Instant::now();
        let list = List::new(src, ChatStyle::new(pal), 700.0, 600.0, opts).expect("index");
        out.put("build_ms", t.elapsed().as_secs_f64() * 1e3);
        run(list, n, &mut out, None);
        let _ = before;
    } else {
        let msgs = synth::messages(0, n as u64);
        out.put("data_ms", t0.elapsed().as_secs_f64() * 1e3);
        let before_heap = LIVE.load(Relaxed);
        let t = Instant::now();
        let list = ChatList::from_messages(msgs, pal, 700.0, 600.0, opts);
        out.put("build_ms", t.elapsed().as_secs_f64() * 1e3);
        out.put("data_heap_bytes", (before_heap.saturating_sub(base_heap)) as f64);
        let mut text = String::new();
        let mut grow = |l: &mut ChatList, _k: usize| {
            text.push_str("tokens arrive ");
            let last = l.source.len() - 1;
            l.edit(last, &text);
        };
        run(list, n, &mut out, Some(&mut grow));
        black_box(VecSource::new());
    }
    println!("{{{}}}", out.json.join(","));
}
