//! Node.js / Bun bindings (napi-rs): BUI driven from JavaScript, GPUIX style.
//!
//! BUI runs on its own UI thread: the JavaScript thread is never blocked,
//! and an idle UI costs no CPU. JavaScript keeps a retained element tree and
//! sends only what changed: per window and per tick, one `Int32Array` of ops
//! plus a string table. The UI thread applies them to its mirror of the tree
//! (`blenderui-jsview`, shared with the QuickJS runtime) and re-renders;
//! events come back through a thread-safe function.
//!
//! The JavaScript side lives in `bindings/node` (`@blender-ui/native`, with
//! `@blender-ui/react` on top).
//!
//! SPDX-License-Identifier: GPL-2.0-or-later

use std::sync::{Arc, mpsc};

use blenderui::prelude::*;
use blenderui_jsview as view;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{
    ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode,
};
use napi::{Env, JsFunction, JsObject};
use napi_derive::napi;

type Tsfn = ThreadsafeFunction<BuiEvent, ErrorStrategy::Fatal>;

// ==========================================================================
// Values crossing the boundary

/// An event from a window (`kind`: click, mousedown, mouseup, mousemove,
/// mouseenter, mouseleave, wheel, keydown, keyup, change, input, frame, close).
#[napi(object)]
#[derive(Clone, Default)]
pub struct BuiEvent {
    pub kind: String,
    pub window: u32,
    /// The node whose listener fired.
    pub target: u32,
    pub x: f64,
    pub y: f64,
    /// 0 left, 1 middle, 2 right (DOM numbering).
    pub button: u32,
    pub click_count: u32,
    pub delta: f64,
    pub key: Option<String>,
    pub key_char: Option<String>,
    pub value: Option<String>,
    pub checked: bool,
    pub number: f64,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl From<view::Event> for BuiEvent {
    fn from(e: view::Event) -> BuiEvent {
        BuiEvent {
            kind: e.kind, window: e.window, target: e.target, x: e.x, y: e.y, button: e.button,
            click_count: e.click_count, delta: e.delta, key: e.key, key_char: e.key_char, value: e.value,
            checked: e.checked, number: e.number, shift: e.shift, ctrl: e.ctrl, alt: e.alt,
        }
    }
}

#[napi(object)]
pub struct AppOptions {
    /// Draw on the GPU (Direct3D 11).
    pub gpu: Option<bool>,
    /// GPU text from glyph outlines (Slug) or a glyph atlas.
    pub slug: Option<bool>,
    /// No native windows (tests, screenshots, CI).
    pub headless: Option<bool>,
}

#[napi(object)]
#[derive(Clone)]
pub struct WindowConfig {
    pub title: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub resizable: Option<bool>,
    pub gpu: Option<bool>,
}

#[napi(object)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[napi(object)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// RGBA, row-major.
    pub data: Buffer,
}

// ==========================================================================
// The UI thread

type Job = Box<dyn FnOnce(&mut App) + Send>;

/// How jobs reach the UI thread: the native loop's sender, or (headless) a
/// channel the UI thread blocks on.
#[derive(Clone)]
enum Link {
    Native(UiSender),
    Headless(mpsc::Sender<Job>),
}

impl Link {
    fn send(&self, f: impl FnOnce(&mut App) + Send + 'static) -> bool {
        match self {
            Link::Native(s) => s.send(f),
            Link::Headless(tx) => tx.send(Box::new(f)).is_ok(),
        }
    }
}

/// The headless loop ends after the job that set this.
struct Quit;

/// Events to JavaScript, from the UI thread.
fn sink(tsfn: &Tsfn) -> view::Sink {
    let t = tsfn.clone();
    Arc::new(move |e: view::Event| {
        t.call(BuiEvent::from(e), ThreadsafeFunctionCallMode::NonBlocking);
    })
}

// ==========================================================================
// The JavaScript class

/// A BUI application: owns the UI thread.
#[napi]
pub struct NativeApp {
    link: Link,
    headless: bool,
}

#[napi]
impl NativeApp {
    #[napi(constructor)]
    pub fn new(options: Option<AppOptions>,
               #[napi(ts_arg_type = "(event: BuiEvent) => void")] on_event: JsFunction) -> Result<Self> {
        let tsfn: Tsfn = on_event.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<BuiEvent>| {
            Ok(vec![ctx.value])
        })?;
        let (gpu, slug, headless) = match &options {
            Some(o) => (o.gpu, o.slug, o.headless.unwrap_or(false)),
            None => (None, None, false),
        };
        let headless = headless || !blenderui::native_windows();
        let (ltx, lrx) = mpsc::channel::<Link>();
        std::thread::Builder::new().name("bui-ui".into()).spawn(move || {
            let mut a = if headless { Application::headless() } else { Application::new().keep_alive(true) };
            if let Some(g) = gpu {
                a = a.gpu(g);
            }
            if let Some(s) = slug {
                a = a.slug(s);
            }
            let events = tsfn.clone();
            a.run(move |cx| {
                cx.set_global(events.clone());
                view::init(cx, sink(&events));
                if headless {
                    let (tx, rx) = mpsc::channel::<Job>();
                    let _ = ltx.send(Link::Headless(tx));
                    // jobs until every sender is gone
                    while let Ok(job) = rx.recv() {
                        job(cx);
                        cx.run_pending();
                        if cx.try_global::<Quit>().is_some() {
                            break;
                        }
                    }
                } else {
                    let _ = ltx.send(Link::Native(cx.ui_sender()));
                }
            });
            // the loop ended: let the runtime exit
            drop(tsfn);
        }).map_err(|e| Error::from_reason(format!("starting the UI thread: {e}")))?;
        let link = lrx.recv().map_err(|_| Error::from_reason("the UI thread failed to start"))?;
        Ok(NativeApp { link, headless })
    }

    /// Whether windows are off-screen.
    #[napi(getter)]
    pub fn headless(&self) -> bool {
        self.headless
    }

    /// Open window `id` (chosen by JS) with an empty `<body>` as node 0.
    #[napi]
    pub fn open_window(&self, id: u32, config: Option<WindowConfig>) {
        let c = config.map(|c| view::WindowConfig {
            title: c.title, width: c.width, height: c.height, resizable: c.resizable, gpu: c.gpu,
        }).unwrap_or_default();
        self.link.send(move |cx| {
            let s = sink(cx.global::<Tsfn>());
            view::open_window(cx, id, c, s);
        });
    }

    /// Apply a batch of ops to window `window` and re-render it: right away,
    /// or (`animated`, during requestAnimationFrame) on the next vsync.
    #[napi]
    pub fn commit(&self, window: u32, ops: Int32Array, strings: Vec<String>, animated: Option<bool>) {
        let ops: Vec<i32> = ops.to_vec();
        let animated = animated.unwrap_or(false);
        self.link.send(move |cx| view::commit(cx, window, &ops, &strings, animated));
    }

    /// Ask for a `frame` event when window `window` next renders (vsync).
    #[napi]
    pub fn request_frame(&self, window: u32) {
        self.link.send(move |cx| view::request_frame(cx, window));
    }

    #[napi]
    pub fn close_window(&self, window: u32) {
        self.link.send(move |cx| view::close_window(cx, window));
    }

    /// GPU text: Slug (glyph outlines) or the glyph atlas.
    #[napi]
    pub fn set_slug(&self, on: bool) {
        self.link.send(move |cx| {
            blenderui::set_slug(on);
            cx.notify_all();
        });
    }

    /// End the UI thread (closes every window).
    #[napi]
    pub fn quit(&self) {
        self.link.send(|cx| {
            view::close_all(cx);
            cx.set_global(Quit);
            cx.quit();
        });
    }

    /// Render window `window` and resolve with its pixels: `{ width, height,
    /// data }` (RGBA bytes, device pixels).
    #[napi(ts_return_type = "Promise<{ width: number, height: number, data: Buffer }>")]
    pub fn screenshot(&self, env: Env, window: u32) -> Result<JsObject> {
        let (deferred, promise) = env.create_deferred()?;
        self.link.send(move |cx| match view::screenshot(cx, window) {
            Some((width, height, rgba)) => {
                deferred.resolve(move |_| Ok(Screenshot { width, height, data: rgba.into() }));
            }
            None => deferred.reject(Error::from_reason("no such window")),
        });
        Ok(promise)
    }

    /// Bounds of node `node` (logical pixels), or null.
    #[napi(ts_return_type = "Promise<Rect | null>")]
    pub fn bounds(&self, env: Env, window: u32, node: u32) -> Result<JsObject> {
        let (deferred, promise) = env.create_deferred()?;
        self.link.send(move |cx| {
            let r = view::bounds(cx, window, node);
            deferred.resolve(move |_| Ok(r.map(|b| Rect { x: b.x, y: b.y, width: b.width, height: b.height })));
        });
        Ok(promise)
    }

    /// Click at (x, y) in window `window` (tests; logical pixels).
    #[napi]
    pub fn simulate_click(&self, window: u32, x: f64, y: f64) {
        self.link.send(move |cx| view::simulate_click(cx, window, x, y));
    }
}

// ==========================================================================
// Measuring (benchmarks): the same counters as bench/common/measure.rs.

/// CPU cycles used by every thread of this process (Windows; otherwise null).
/// Unlike tick-based CPU time, exact for threads that wake on each vsync.
#[napi]
pub fn process_cpu_cycles() -> Option<f64> {
    view::process_cpu_cycles()
}

/// The time-stamp counter (x86-64; otherwise null): divide cycle counts by
/// its rate, measured against the wall clock, to get CPU time.
#[napi]
pub fn timestamp_counter() -> Option<f64> {
    view::timestamp_counter()
}
