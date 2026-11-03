//! Node.js bindings (napi-rs): BUI driven from JavaScript, GPUIX style.
//!
//! BUI runs on its own UI thread: Node's thread is never blocked, and an
//! idle UI costs no CPU. JavaScript keeps a retained element tree and sends
//! only what changed: per window and per tick, one `Int32Array` of ops plus
//! a string table (no tree serialization, no per-node calls). The UI thread
//! applies the ops to its mirror of the tree and re-renders; events come back
//! through a thread-safe function.
//!
//! The JavaScript side lives in `bindings/node` (`@blenderui/native`, with
//! `@blenderui/react` on top).
//!
//! SPDX-License-Identifier: GPL-2.0-or-later

use std::collections::HashMap;
use std::sync::mpsc;

use blenderui::prelude::*;
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
/// mouseenter, mouseleave, wheel, keydown, keyup, change, input, close).
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

/// Ops (`commit`): opcode, then its arguments; `s` = index into the strings.
mod op {
    pub const CREATE: i32 = 0; // id, s(tag)
    pub const TEXT: i32 = 1; // id, s(text)
    pub const SET_ATTR: i32 = 2; // id, s(name), s(value)
    pub const REMOVE_ATTR: i32 = 3; // id, s(name)
    pub const APPEND: i32 = 4; // parent, child
    pub const INSERT: i32 = 5; // parent, child, before
    pub const REMOVE: i32 = 6; // parent, child
    pub const SET_TEXT: i32 = 7; // id, s(text)
    pub const LISTEN: i32 = 8; // id, event bits (replaces)
    pub const FREE: i32 = 9; // id
}

/// Event bits (`LISTEN`), matching `bindings/node/index.js`.
mod ev {
    pub const CLICK: u32 = 1;
    pub const MOUSE_DOWN: u32 = 2;
    pub const MOUSE_UP: u32 = 4;
    pub const MOUSE_MOVE: u32 = 8;
    pub const HOVER: u32 = 16;
    pub const WHEEL: u32 = 32;
    pub const KEY_DOWN: u32 = 64;
    pub const KEY_UP: u32 = 128;
    pub const CHANGE: u32 = 256;
    pub const INPUT: u32 = 512;
}

// ==========================================================================
// The retained tree (UI thread)

struct JsNode {
    tag: &'static str,
    key: SharedString,
    text: Option<SharedString>,
    attrs: Vec<(SharedString, SharedString)>,
    /// The `style` attribute, parsed once when set (renders reuse it).
    style: Option<Box<[blenderui::css::Decl]>>,
    children: Vec<u32>,
    events: u32,
}

/// `style="..."` parsed into declarations whose values are shared.
fn parse_style(css: &str) -> Box<[blenderui::css::Decl]> {
    blenderui::css::parse_declarations(css).into_iter().map(|d| blenderui::css::Decl {
        value: blenderui::css::Value::Shared(std::sync::Arc::from(&*d.value)),
        ..d
    }).collect()
}

/// One window's tree; node ids are per window and dense (root = 0).
struct JsView {
    window: u32,
    nodes: Vec<Option<JsNode>>,
    tsfn: Tsfn,
    /// JS asked for an animation frame: tell it when this one renders.
    raf: bool,
    /// The next vsync frame is already scheduled (changes just wait for it).
    frame_scheduled: bool,
}

/// Tag names as the `&'static str` elements need: HTML tags from the
/// table, others interned once.
fn intern_tag(tag: &str, custom: &mut HashMap<String, &'static str>) -> &'static str {
    if let Some((t, _)) = blenderui::html::TAGS.iter().find(|(t, _)| *t == tag) {
        return t;
    }
    *custom.entry(tag.to_string()).or_insert_with(|| Box::leak(tag.to_string().into_boxed_str()))
}

impl JsView {
    fn node(&mut self, id: i32) -> Option<&mut JsNode> {
        self.nodes.get_mut(usize::try_from(id).ok()?)?.as_mut()
    }

    fn put(&mut self, id: i32, n: JsNode) {
        let Ok(i) = usize::try_from(id) else { return };
        if i >= self.nodes.len() {
            self.nodes.resize_with(i + 1, || None);
        }
        self.nodes[i] = Some(n);
    }

    fn apply(&mut self, ops: &[i32], strings: &[String], tags: &mut HashMap<String, &'static str>) {
        let s = |k: i32| strings.get(k as usize).map(String::as_str).unwrap_or("");
        let mut i = 0;
        while i < ops.len() {
            let a = |k: usize| ops.get(i + k).copied().unwrap_or(-1);
            match ops[i] {
                op::CREATE => {
                    let tag = intern_tag(s(a(2)), tags);
                    self.put(a(1), JsNode { tag, key: format!("n{}", a(1)).into(), text: None, attrs: Vec::new(),
                                            style: None, children: Vec::new(), events: 0 });
                    i += 3;
                }
                op::TEXT => {
                    let text = s(a(2)).to_string();
                    self.put(a(1), JsNode { tag: "", key: SharedString::Borrowed(""), text: Some(text.into()),
                                            attrs: Vec::new(), style: None, children: Vec::new(), events: 0 });
                    i += 3;
                }
                op::SET_ATTR => {
                    let (name, value) = (s(a(2)).to_string(), s(a(3)).to_string());
                    if let Some(n) = self.node(a(1)) {
                        if name == "style" {
                            n.style = Some(parse_style(&value));
                        }
                        match n.attrs.iter_mut().find(|(k, _)| *k == name) {
                            Some((_, v)) => *v = value.into(),
                            None => n.attrs.push((name.into(), value.into())),
                        }
                    }
                    i += 4;
                }
                op::REMOVE_ATTR => {
                    let name = s(a(2));
                    if let Some(n) = self.node(a(1)) {
                        if name == "style" {
                            n.style = None;
                        }
                        n.attrs.retain(|(k, _)| k != name);
                    }
                    i += 3;
                }
                op::APPEND => {
                    let child = a(2) as u32;
                    if let Some(n) = self.node(a(1)) {
                        n.children.retain(|&c| c != child);
                        n.children.push(child);
                    }
                    i += 3;
                }
                op::INSERT => {
                    let (child, before) = (a(2) as u32, a(3) as u32);
                    if let Some(n) = self.node(a(1)) {
                        n.children.retain(|&c| c != child);
                        let at = n.children.iter().position(|&c| c == before).unwrap_or(n.children.len());
                        n.children.insert(at, child);
                    }
                    i += 4;
                }
                op::REMOVE => {
                    let child = a(2) as u32;
                    if let Some(n) = self.node(a(1)) {
                        n.children.retain(|&c| c != child);
                    }
                    i += 3;
                }
                op::SET_TEXT => {
                    let text = s(a(2)).to_string();
                    if let Some(n) = self.node(a(1)) {
                        n.text = Some(text.into());
                    }
                    i += 3;
                }
                op::LISTEN => {
                    let bits = a(2) as u32;
                    if let Some(n) = self.node(a(1)) {
                        n.events = bits;
                    }
                    i += 3;
                }
                op::FREE => {
                    if let Ok(k) = usize::try_from(a(1)) {
                        if let Some(slot) = self.nodes.get_mut(k) {
                            *slot = None;
                        }
                    }
                    i += 2;
                }
                _ => break, // malformed: stop rather than misread
            }
        }
    }

    fn build(&self, id: u32) -> AnyElement {
        let Some(Some(n)) = self.nodes.get(id as usize) else {
            return String::new().into_any_element();
        };
        if let Some(t) = &n.text {
            return t.clone().into_any_element();
        }
        let mut d = Div::new(n.tag).key(n.key.clone());
        for (k, v) in &n.attrs {
            d = match &**k {
                "class" | "className" => d.class(v.clone()),
                "style" => d, // parsed: below
                "id" => d.id(v.clone()),
                _ => d.attr(k.clone(), v.clone()),
            };
        }
        if let Some(s) = &n.style {
            d = d.style(&s[..]);
        }
        if n.events != 0 {
            d = self.listeners(d, id, n.events);
        }
        for &c in &n.children {
            d = d.child(self.build(c));
        }
        d.into_any_element()
    }

    fn listeners(&self, mut d: Div, id: u32, bits: u32) -> Div {
        let base = BuiEvent { window: self.window, target: id, ..Default::default() };
        let send = {
            let t = self.tsfn.clone();
            move |e: BuiEvent| {
                t.call(e, ThreadsafeFunctionCallMode::NonBlocking);
            }
        };
        fn mods(e: &mut BuiEvent, m: &Modifiers) {
            (e.shift, e.ctrl, e.alt) = (m.shift, m.control, m.alt);
        }
        fn button(b: &MouseButton) -> u32 {
            match b {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
            }
        }
        macro_rules! on {
            ($bit:expr, $method:ident $(($arg:expr))?, |$e:ident, $ev:ident| $body:block) => {
                if bits & $bit != 0 {
                    let (send, base) = (send.clone(), base.clone());
                    d = d.$method($($arg,)? move |$ev, _, _| {
                        let mut $e = base.clone();
                        $body
                        send($e);
                    });
                }
            };
        }
        on!(ev::CLICK, on_click, |e, c| {
            e.kind = "click".into();
            (e.x, e.y) = (c.up.position.x.0 as f64, c.up.position.y.0 as f64);
            e.button = button(&c.up.button);
            e.click_count = c.down.click_count as u32;
            mods(&mut e, &c.up.modifiers);
        });
        on!(ev::MOUSE_DOWN, on_any_mouse_down, |e, m| {
            e.kind = "mousedown".into();
            (e.x, e.y) = (m.position.x.0 as f64, m.position.y.0 as f64);
            e.button = button(&m.button);
            e.click_count = m.click_count as u32;
            mods(&mut e, &m.modifiers);
        });
        for b in [MouseButton::Left, MouseButton::Middle, MouseButton::Right] {
            on!(ev::MOUSE_UP, on_mouse_up(b.clone()), |e, m| {
                e.kind = "mouseup".into();
                (e.x, e.y) = (m.position.x.0 as f64, m.position.y.0 as f64);
                e.button = button(&m.button);
                mods(&mut e, &m.modifiers);
            });
        }
        on!(ev::MOUSE_MOVE, on_mouse_move, |e, m| {
            e.kind = "mousemove".into();
            (e.x, e.y) = (m.position.x.0 as f64, m.position.y.0 as f64);
            mods(&mut e, &m.modifiers);
        });
        on!(ev::HOVER, on_hover, |e, h| {
            e.kind = if *h { "mouseenter" } else { "mouseleave" }.into();
        });
        on!(ev::WHEEL, on_scroll_wheel, |e, w| {
            e.kind = "wheel".into();
            (e.x, e.y) = (w.position.x.0 as f64, w.position.y.0 as f64);
            e.delta = w.delta as f64;
            mods(&mut e, &w.modifiers);
        });
        on!(ev::KEY_DOWN, on_key_down, |e, k| {
            e.kind = "keydown".into();
            e.key = Some(k.keystroke.key.clone());
            e.key_char = k.keystroke.key_char.clone();
            mods(&mut e, &k.keystroke.modifiers);
        });
        on!(ev::KEY_UP, on_key_up, |e, k| {
            e.kind = "keyup".into();
            e.key = Some(k.keystroke.key.clone());
            e.key_char = k.keystroke.key_char.clone();
            mods(&mut e, &k.keystroke.modifiers);
        });
        on!(ev::CHANGE, on_change, |e, c| {
            e.kind = "change".into();
            (e.value, e.checked, e.number) = (Some(c.value.clone()), c.checked, c.number);
        });
        on!(ev::INPUT, on_input, |e, c| {
            e.kind = "input".into();
            (e.value, e.checked, e.number) = (Some(c.value.clone()), c.checked, c.number);
        });
        d
    }
}

impl Render for JsView {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.frame_scheduled = false;
        if self.raf {
            // requestAnimationFrame: JS runs its callbacks while this frame
            // presents, and the next vsync frame is scheduled now (as a
            // browser does), so their changes make it into that frame
            self.raf = false;
            self.frame_scheduled = true;
            window.request_animation_frame();
            self.tsfn.call(BuiEvent { kind: "frame".into(), window: self.window, ..Default::default() },
                           ThreadsafeFunctionCallMode::NonBlocking);
        }
        self.build(0)
    }
}

/// UI-thread state: JS window ids -> windows.
#[derive(Default)]
struct UiState {
    windows: HashMap<u32, WindowHandle<JsView>>,
    by_native: HashMap<usize, u32>,
    tags: HashMap<String, &'static str>,
}

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
            let close = tsfn.clone();
            a.run(move |cx| {
                cx.set_global(UiState::default());
                cx.set_global(close.clone());
                cx.on_window_closed(move |id, cx| {
                    let js = cx.global::<UiState>().by_native.get(&id.0).copied();
                    if let Some(w) = js {
                        close.call(BuiEvent { kind: "close".into(), window: w, ..Default::default() },
                                   ThreadsafeFunctionCallMode::NonBlocking);
                        cx.global_mut::<UiState>().windows.remove(&w);
                    }
                });
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
            // the loop ended: let Node exit
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
        let (title, w, h, resizable, gpu) = match config {
            Some(c) => (c.title, c.width.unwrap_or(800.0), c.height.unwrap_or(600.0), c.resizable, c.gpu),
            None => (None, 800.0, 600.0, None, None),
        };
        self.link.send(move |cx| {
            let tsfn = cx.global::<Tsfn>().clone();
            let opts = blenderui::WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(w as f32), px(h as f32)),
                })),
                titlebar: title.map(|t| TitlebarOptions { title: Some(t.into()) }),
                is_resizable: resizable,
                gpu,
                ..Default::default()
            };
            let handle = cx.open_window(opts, |_, cx| {
                cx.new(|_| {
                    let mut v = JsView { window: id, nodes: Vec::new(), tsfn, raf: false, frame_scheduled: false };
                    v.put(0, JsNode { tag: "body", key: "n0".into(), text: None, attrs: Vec::new(),
                                      style: None, children: Vec::new(), events: 0 });
                    v
                })
            });
            if let Ok(handle) = handle {
                let st = cx.global_mut::<UiState>();
                st.by_native.insert(handle.id.0, id);
                st.windows.insert(id, handle);
            }
        });
    }

    /// Apply a batch of ops to window `window` and re-render it: right away,
    /// or (`animated`, during requestAnimationFrame) on the next vsync.
    #[napi]
    pub fn commit(&self, window: u32, ops: Int32Array, strings: Vec<String>, animated: Option<bool>) {
        let ops: Vec<i32> = ops.to_vec();
        let animated = animated.unwrap_or(false);
        self.link.send(move |cx| {
            let Some(handle) = cx.global::<UiState>().windows.get(&window).cloned() else { return };
            let mut tags = std::mem::take(&mut cx.global_mut::<UiState>().tags);
            handle.update(cx, |v, w, _| {
                v.apply(&ops, &strings, &mut tags);
                if !animated {
                    w.refresh(); // this window only
                } else if !v.frame_scheduled {
                    v.frame_scheduled = true;
                    w.request_animation_frame();
                }
            });
            cx.global_mut::<UiState>().tags = tags;
        });
    }

    /// Ask for a `frame` event when window `window` next renders (vsync).
    #[napi]
    pub fn request_frame(&self, window: u32) {
        self.link.send(move |cx| {
            if let Some(handle) = cx.global::<UiState>().windows.get(&window).cloned() {
                handle.update(cx, |v, w, _| {
                    v.raf = true;
                    if !v.frame_scheduled {
                        v.frame_scheduled = true;
                        w.request_animation_frame();
                    }
                });
            }
        });
    }

    #[napi]
    pub fn close_window(&self, window: u32) {
        self.link.send(move |cx| {
            if let Some(h) = cx.global_mut::<UiState>().windows.remove(&window) {
                cx.close_window(h.id);
            }
        });
    }

    /// Draw on the GPU from now on (new windows) / GPU text mode.
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
            let ids: Vec<WindowId> = cx.global::<UiState>().windows.values().map(|h| h.id).collect();
            for id in ids {
                cx.close_window(id);
            }
            cx.global_mut::<UiState>().windows.clear();
            cx.set_global(Quit);
            cx.quit();
        });
    }

    /// Render window `window` and resolve with its pixels: `{ width, height,
    /// data }` (RGBA bytes, device pixels).
    #[napi(ts_return_type = "Promise<{ width: number, height: number, data: Buffer }>")]
    pub fn screenshot(&self, env: Env, window: u32) -> Result<JsObject> {
        let (deferred, promise) = env.create_deferred()?;
        self.link.send(move |cx| {
            let Some(handle) = cx.global::<UiState>().windows.get(&window).cloned() else {
                deferred.reject(Error::from_reason("no such window"));
                return;
            };
            let (w, h, px) = cx.draw(handle.id);
            let mut rgba = Vec::with_capacity(px.len() * 4);
            for p in px {
                rgba.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
            }
            deferred.resolve(move |_| Ok(Screenshot { width: w as u32, height: h as u32, data: rgba.into() }));
        });
        Ok(promise)
    }

    /// Bounds of node `node` (logical pixels), or null.
    #[napi(ts_return_type = "Promise<Rect | null>")]
    pub fn bounds(&self, env: Env, window: u32, node: u32) -> Result<JsObject> {
        let (deferred, promise) = env.create_deferred()?;
        self.link.send(move |cx| {
            let handle = cx.global::<UiState>().windows.get(&window).cloned();
            let r = handle.and_then(|h| {
                cx.draw(h.id);
                cx.element_bounds(h.id, &format!("n{node}"))
            });
            deferred.resolve(move |_| Ok(r.map(|b| Rect { x: b.origin.x as f64, y: b.origin.y as f64,
                                                           width: b.size.width as f64,
                                                           height: b.size.height as f64 })));
        });
        Ok(promise)
    }

    /// Click at (x, y) in window `window` (tests; logical pixels).
    #[napi]
    pub fn simulate_click(&self, window: u32, x: f64, y: f64) {
        self.link.send(move |cx| {
            if let Some(h) = cx.global::<UiState>().windows.get(&window).cloned() {
                cx.simulate_click(h.id, x as f32, y as f32);
            }
        });
    }
}

/// The headless loop ends after the job that set this.
struct Quit;

#[napi(object)]
pub struct Screenshot {
    pub width: u32,
    pub height: u32,
    /// RGBA, row-major.
    pub data: Buffer,
}
