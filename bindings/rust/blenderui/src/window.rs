//! Application, windows, entities and event dispatch, shaped like gpui:
//!
//! ```ignore
//! Application::new().run(|cx: &mut App| {
//!     cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Counter { n: 0 })).unwrap();
//! });
//! ```

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, c_double, c_int, c_void, CStr, CString};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::element::{AnyElement, IntoElement, Listener, Listeners, SharedString};
use crate::events::*;
use crate::geometry::{point, px, size, Bounds, Pixels, Size};
use crate::layout::{layout, Content, Frame, UiState};
use crate::paint::{paint_frame, Painter};
use crate::widgets::{WKind, Widgets};
use blenderui_sys as sys;

// ==========================================================================
// Entities

/// Shared state owned by the app (gpui's `Entity<T>`).
pub struct Entity<T> {
    inner: Rc<RefCell<Option<T>>>,
}

impl<T> Clone for Entity<T> {
    fn clone(&self) -> Self {
        Entity { inner: self.inner.clone() }
    }
}

impl<T: 'static> Entity<T> {
    /// Read the value.
    pub fn read<'a>(&'a self, _cx: &'a App) -> Ref<'a, T> {
        Ref::map(self.inner.borrow(), |v| v.as_ref().expect("entity is being built"))
    }

    /// Mutate the value with a [`Context`] for it.
    pub fn update<R>(&self, cx: &mut App, f: impl FnOnce(&mut T, &mut Context<T>) -> R) -> R {
        let mut slot = self.inner.borrow_mut();
        let value = slot.as_mut().expect("entity is being built");
        let mut ctx = Context { app: cx, entity: self.clone() };
        f(value, &mut ctx)
    }

    /// Same entity?
    pub fn entity_id(&self) -> usize {
        Rc::as_ptr(&self.inner) as *const () as usize
    }

    pub fn downgrade(&self) -> WeakEntity<T> {
        WeakEntity { inner: Rc::downgrade(&self.inner) }
    }
}

/// A weak handle (gpui's `WeakEntity<T>`).
pub struct WeakEntity<T> {
    inner: std::rc::Weak<RefCell<Option<T>>>,
}

impl<T> Clone for WeakEntity<T> {
    fn clone(&self) -> Self {
        WeakEntity { inner: self.inner.clone() }
    }
}

impl<T: 'static> WeakEntity<T> {
    pub fn upgrade(&self) -> Option<Entity<T>> {
        self.inner.upgrade().map(|inner| Entity { inner })
    }

    /// Update if still alive.
    pub fn update<R>(&self, cx: &mut App, f: impl FnOnce(&mut T, &mut Context<T>) -> R) -> Option<R> {
        self.upgrade().map(|e| e.update(cx, f))
    }
}

/// The app, seen from inside an entity (gpui's `Context<T>`); derefs to [`App`].
pub struct Context<'a, T> {
    app: &'a mut App,
    entity: Entity<T>,
}

impl<T> Deref for Context<'_, T> {
    type Target = App;
    fn deref(&self) -> &App {
        self.app
    }
}
impl<T> DerefMut for Context<'_, T> {
    fn deref_mut(&mut self) -> &mut App {
        self.app
    }
}

impl<T: 'static> Context<'_, T> {
    /// This entity.
    pub fn entity(&self) -> Entity<T> {
        self.entity.clone()
    }

    pub fn weak_entity(&self) -> WeakEntity<T> {
        self.entity.downgrade()
    }

    /// Re-render the windows showing this entity.
    pub fn notify(&mut self) {
        self.app.notify_all();
    }

    /// Turn a method-like closure into an element listener (gpui's `cx.listener`).
    pub fn listener<E: ?Sized>(
        &self,
        f: impl Fn(&mut T, &E, &mut Window, &mut Context<T>) + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let weak = self.entity.downgrade();
        move |e: &E, window: &mut Window, cx: &mut App| {
            if let Some(entity) = weak.upgrade() {
                entity.update(cx, |this, cx| f(this, e, window, cx));
            }
        }
    }
}

/// A view: something that renders an element tree (gpui's `Render`).
pub trait Render: 'static + Sized {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement;
}

// ==========================================================================
// The application

/// Window identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub usize);

/// A typed window handle (gpui's `WindowHandle<V>`).
pub struct WindowHandle<V> {
    pub id: WindowId,
    entity: Entity<V>,
}

impl<V> Clone for WindowHandle<V> {
    fn clone(&self) -> Self {
        WindowHandle { id: self.id, entity: self.entity.clone() }
    }
}

impl<V: 'static> WindowHandle<V> {
    /// The root view of the window.
    pub fn root(&self) -> Entity<V> {
        self.entity.clone()
    }

    /// Update the root view with its window.
    pub fn update<R>(&self, cx: &mut App,
                     f: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R) -> Option<R> {
        let mut w = cx.windows.get_mut(self.id.0)?.take()?;
        let r = self.entity.update(cx, |v, cx| f(v, &mut w, cx));
        cx.windows[self.id.0] = Some(w);
        Some(r)
    }
}

/// How a window opens (gpui's `WindowBounds`).
#[derive(Clone, Copy, Debug)]
pub enum WindowBounds {
    Windowed(Bounds<Pixels>),
    Maximized(Bounds<Pixels>),
    Fullscreen(Bounds<Pixels>),
}

#[derive(Clone, Debug, Default)]
pub struct TitlebarOptions {
    pub title: Option<SharedString>,
}

/// Options for [`App::open_window`] (gpui's `WindowOptions`).
#[derive(Clone, Debug, Default)]
pub struct WindowOptions {
    pub window_bounds: Option<WindowBounds>,
    pub titlebar: Option<TitlebarOptions>,
    pub is_resizable: Option<bool>,
    pub window_min_size: Option<Size<Pixels>>,
}

/// The application entry point (gpui's `Application`).
pub struct Application {
    headless: bool,
}

impl Default for Application {
    fn default() -> Self {
        Application::new()
    }
}

impl Application {
    /// Native windows.
    pub fn new() -> Application {
        Application { headless: false }
    }

    /// Off-screen windows (tests, screenshots, CI): nothing is shown, the
    /// `simulate_*` and `draw` methods drive it.
    pub fn headless() -> Application {
        Application { headless: true }
    }

    /// Run `f`, then the event loop until every window is closed.
    pub fn run(self, f: impl FnOnce(&mut App)) {
        // the C kit keeps global state: one application at a time per process
        static KIT: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = KIT.lock().unwrap_or_else(|e| e.into_inner());
        let headless = self.headless || unsafe { sys::BK_window_supported() } == 0;
        let mut app = App::create(headless);
        f(&mut app);
        app.flush();
        if headless {
            app.shutdown();
            return;
        }
        APP.with(|a| *a.borrow_mut() = Some(app));
        // SAFETY: plain message loop; callbacks re-enter through APP.
        unsafe { sys::BK_window_run() };
        if let Some(mut app) = APP.with(|a| a.borrow_mut().take()) {
            app.shutdown();
        }
    }
}

thread_local! {
    /// The running app while the native loop runs (callbacks borrow it).
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// Kit events that arrived while the app was busy.
    static QUEUE: RefCell<VecDeque<RawEvent>> = const { RefCell::new(VecDeque::new()) };
}

struct RawEvent {
    window: usize,
    node: *mut sys::BKNode,
    kind: c_int,
    text: String,
    a: f64,
    b: f64,
}

/// The application state (gpui's `App`).
pub struct App {
    windows: Vec<Option<Box<Window>>>,
    headless: bool,
    globals: HashMap<TypeId, Box<dyn Any>>,
    propagate: bool,
}

impl App {
    fn create(headless: bool) -> App {
        App { windows: Vec::new(), headless, globals: HashMap::new(), propagate: true }
    }

    /// Create an entity (gpui's `cx.new`).
    pub fn new<T: 'static>(&mut self, build: impl FnOnce(&mut Context<T>) -> T) -> Entity<T> {
        let entity = Entity { inner: Rc::new(RefCell::new(None)) };
        let mut ctx = Context { app: self, entity: entity.clone() };
        let v = build(&mut ctx);
        *entity.inner.borrow_mut() = Some(v);
        entity
    }

    /// Open a window whose root view is built by `build`.
    pub fn open_window<V: Render>(
        &mut self,
        options: WindowOptions,
        build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
    ) -> Result<WindowHandle<V>, String> {
        let bounds = match options.window_bounds {
            Some(WindowBounds::Windowed(b) | WindowBounds::Maximized(b) | WindowBounds::Fullscreen(b)) => b,
            None => Bounds { origin: point(px(0.0), px(0.0)), size: size(px(1024.0), px(700.0)) },
        };
        let title = options.titlebar.as_ref().and_then(|t| t.title.clone())
            .map(|t| t.to_string()).unwrap_or_else(|| "BUI".into());
        let id = self.windows.len();
        let mut w = Window::create(id, &title, bounds.size.width.0, bounds.size.height.0, self.headless)?;
        if let Some(n) = &w.native {
            unsafe {
                if let Some(WindowBounds::Maximized(_)) = options.window_bounds {
                    sys::BK_window_wm(*n, c"state".as_ptr(), 1.0, 0.0);
                }
                if options.is_resizable == Some(false) {
                    sys::BK_window_wm(*n, c"resizable".as_ptr(), 0.0, 0.0);
                }
                if let Some(m) = options.window_min_size {
                    sys::BK_window_wm(*n, c"minsize".as_ptr(), (m.width.0 * w.scale) as f64,
                                      (m.height.0 * w.scale) as f64);
                }
            }
        }
        let entity = build(&mut w, self);
        let view = entity.clone();
        w.view = Some(Rc::new(move |window: &mut Window, cx: &mut App| {
            view.update(cx, |v, cx| v.render(window, cx).into_any_element())
        }));
        w.dirty = true;
        self.windows.push(Some(Box::new(w)));
        if !self.headless {
            self.with_window(id, |w, app| w.rebuild(app));
            if let Some(n) = self.window(WindowId(id)).and_then(|w| w.native) {
                unsafe { sys::BK_window_show(n) };
            }
        }
        Ok(WindowHandle { id: WindowId(id), entity })
    }

    /// Mark every window for re-render.
    pub fn notify_all(&mut self) {
        for w in self.windows.iter_mut().flatten() {
            w.refresh();
        }
    }

    /// Stop the event loop.
    pub fn quit(&mut self) {
        unsafe { sys::BK_window_quit() };
    }

    /// Stop the current event from reaching ancestors.
    pub fn stop_propagation(&mut self) {
        self.propagate = false;
    }

    /// Activate the app (no-op: windows come to the front when shown).
    pub fn activate(&mut self, _ignoring_other_apps: bool) {}

    pub fn set_global<G: 'static>(&mut self, g: G) {
        self.globals.insert(TypeId::of::<G>(), Box::new(g));
    }
    pub fn global<G: 'static>(&self) -> &G {
        self.try_global().expect("global not set")
    }
    pub fn try_global<G: 'static>(&self) -> Option<&G> {
        self.globals.get(&TypeId::of::<G>()).and_then(|g| g.downcast_ref())
    }
    pub fn global_mut<G: 'static>(&mut self) -> &mut G {
        self.globals.get_mut(&TypeId::of::<G>()).and_then(|g| g.downcast_mut()).expect("global not set")
    }

    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(id.0)?.as_deref()
    }

    fn with_window<R>(&mut self, id: usize, f: impl FnOnce(&mut Window, &mut App) -> R) -> Option<R> {
        let mut w = self.windows.get_mut(id)?.take()?;
        let r = f(&mut w, self);
        self.windows[id] = Some(w);
        Some(r)
    }

    /// Handle queued kit events, then re-render what changed.
    fn drain(&mut self) {
        while let Some(ev) = QUEUE.with(|q| q.borrow_mut().pop_front()) {
            self.with_window(ev.window, |w, app| w.dispatch(ev, app));
        }
    }

    /// Re-render dirty windows now (headless) or soon (native: next loop turn).
    fn flush(&mut self) {
        self.drain();
        for id in 0..self.windows.len() {
            let Some(w) = self.windows[id].as_deref_mut() else { continue };
            if !w.dirty {
                continue;
            }
            match w.native {
                // animation: rebuild right before the next (vsync-paced) render
                Some(n) if w.animation_frame => {
                    w.animation_frame = false;
                    unsafe { sys::BK_window_request_frame(n, w.vsync as c_int) };
                }
                Some(n) if !w.timer_pending => {
                    w.timer_pending = true;
                    unsafe { sys::BK_window_timer(n, 0) };
                }
                Some(_) => {}
                None => {
                    self.with_window(id, |w, app| w.rebuild(app));
                }
            }
        }
    }

    fn shutdown(&mut self) {
        for w in self.windows.iter_mut().flatten() {
            w.destroy();
        }
        self.windows.clear();
    }

    // -- automation (tests, screenshots) --------------------------------------------------

    /// Render a window off-screen: (width, height, 0x00RRGGBB pixels) in device pixels.
    pub fn draw(&mut self, id: WindowId) -> (usize, usize, Vec<u32>) {
        self.flush();
        let Some(w) = self.windows.get_mut(id.0).and_then(|w| w.as_deref_mut()) else {
            return (0, 0, Vec::new());
        };
        let (pw, ph) = w.phys;
        // SAFETY: the raster is sized to the window; pixels are copied out.
        let px = unsafe {
            if w.raster.is_null() {
                w.raster = sys::BUI_raster_create(pw, ph);
            }
            let _s = crate::prof::span("kit render");
            sys::BK_render(w.ctx, w.raster, pw, ph, 1);
            let p = sys::BUI_raster_pixels(w.raster);
            std::slice::from_raw_parts(p, (pw * ph) as usize).to_vec()
        };
        self.flush(); // configure events from the render
        (pw as usize, ph as usize, px)
    }

    /// Bounds (logical pixels) of the element with `.id(id)`.
    pub fn element_bounds(&mut self, window: WindowId, id: &str) -> Option<Bounds<f32>> {
        self.flush();
        let w = self.window(window)?;
        let f = w.frame.as_ref()?;
        f.by_key().get(&format!("#{id}")).map(|&i| f.nodes[i].bounds)
    }

    fn input(&mut self, window: WindowId, f: impl FnOnce(*mut sys::BKContext, f32)) {
        self.flush();
        if let Some(w) = self.window(window) {
            if w.frame.is_none() {
                return;
            }
            f(w.ctx, w.scale);
        }
        self.flush();
    }

    /// Move the pointer to (x, y) in logical pixels.
    pub fn simulate_mouse_move(&mut self, window: WindowId, x: f32, y: f32) {
        self.input(window, |ctx, s| unsafe {
            sys::BK_motion(ctx, (x * s) as c_int, (y * s) as c_int, 0);
        });
    }

    /// Press and release the left button at (x, y).
    pub fn simulate_click(&mut self, window: WindowId, x: f32, y: f32) {
        self.simulate_mouse_move(window, x, y);
        self.input(window, |ctx, s| unsafe {
            sys::BK_press(ctx, (x * s) as c_int, (y * s) as c_int, 0);
        });
        self.input(window, |ctx, s| unsafe {
            sys::BK_release(ctx, (x * s) as c_int, (y * s) as c_int, sys::BK_BUTTON1);
        });
    }

    /// Scroll the wheel at (x, y): positive `lines` scroll the content up.
    pub fn simulate_scroll(&mut self, window: WindowId, x: f32, y: f32, lines: i32) {
        self.simulate_mouse_move(window, x, y);
        self.input(window, |ctx, s| unsafe {
            sys::BK_wheel(ctx, (x * s) as c_int, (y * s) as c_int, lines, 0);
        });
    }

    /// Type text / press a key (`keysym` as in Tk: "Return", "BackSpace", "a" ...).
    pub fn simulate_key(&mut self, window: WindowId, text: &str, keysym: &str) {
        let (t, k) = (CString::new(text).unwrap_or_default(), CString::new(keysym).unwrap_or_default());
        self.input(window, |ctx, _| unsafe {
            sys::BK_key(ctx, t.as_ptr(), k.as_ptr(), 0);
        });
    }
}

// ==========================================================================
// Kit callbacks

unsafe extern "C" fn on_emit(data: *mut c_void, node: *mut sys::BKNode, kind: c_int, ref_: *mut c_void,
                             a: c_double, b: c_double) {
    let text = if ref_.is_null() || kind < sys::BK_EV_PRESS {
        String::new()
    } else {
        // SAFETY: raw events carry a C string in `ref`.
        unsafe { CStr::from_ptr(ref_ as *const c_char) }.to_string_lossy().into_owned()
    };
    QUEUE.with(|q| q.borrow_mut().push_back(RawEvent { window: data as usize, node, kind, text, a, b }));
    with_running_app(|app| app.flush());
}

unsafe extern "C" fn on_timer(data: *mut c_void, _win: *mut sys::BKWindow, _id: c_int) {
    let id = data as usize;
    let ran = with_running_app(|app| {
        app.drain();
        app.with_window(id, |w, app| {
            w.timer_pending = false;
            if w.dirty {
                w.rebuild(app);
            }
        });
        app.flush();
    });
    if !ran {
        // the app is busy (re-entrant callback): try again next turn
        unsafe { sys::BK_window_timer(_win, 1) };
    }
}

unsafe extern "C" fn on_frame(data: *mut c_void, _win: *mut sys::BKWindow) {
    // BUI_PROFILE: wall time between frames, i.e. rebuild + kit render + present
    thread_local! { static LAST: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) }; }
    if crate::prof::on() {
        let now = std::time::Instant::now();
        if let Some(t) = LAST.with(|l| l.replace(Some(now))) {
            crate::prof::add("frame interval", (now - t).as_secs_f64() * 1000.0);
        }
    }
    let id = data as usize;
    with_running_app(|app| {
        app.drain();
        app.with_window(id, |w, app| {
            if w.dirty {
                w.rebuild(app);
            }
        });
        app.flush();
    });
}

unsafe extern "C" fn on_close(data: *mut c_void, _win: *mut sys::BKWindow) -> c_int {
    let id = data as usize;
    with_running_app(|app| {
        if let Some(w) = app.windows.get_mut(id).and_then(|w| w.as_deref_mut()) {
            w.closed = true;
        }
    });
    1
}

fn with_running_app(f: impl FnOnce(&mut App)) -> bool {
    let Some(mut app) = APP.with(|a| a.try_borrow_mut().ok().and_then(|mut a| a.take())) else {
        return false;
    };
    f(&mut app);
    APP.with(|a| *a.borrow_mut() = Some(app));
    true
}

unsafe extern "C" fn noop(_: *mut c_void) {}

// ==========================================================================
// Windows

type ViewFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// A window (gpui's `Window`): its kit context, layout and interaction state.
pub struct Window {
    id: usize,
    ctx: *mut sys::BKContext,
    canvas: *mut sys::BKNode,
    root: *mut sys::BKNode,
    opaque: bool,
    native: Option<*mut sys::BKWindow>,
    raster: *mut sys::BuiRaster,
    phys: (i32, i32),
    scale: f32,
    view: Option<ViewFn>,
    frame: Option<Frame>,
    state: UiState,
    widgets: Widgets,
    dirty: bool,
    timer_pending: bool,
    animation_frame: bool,
    vsync: bool,
    closed: bool,
    title: String,
    cursor: i32,
    pointer: (f32, f32),
    pressed: Vec<String>,
    press_button: MouseButton,
}

impl Window {
    fn create(id: usize, title: &str, w: f32, h: f32, headless: bool) -> Result<Window, String> {
        // SAFETY: building a fresh kit tree; pointers stay owned by this window.
        unsafe {
            let root = sys::BK_node_new(c"frame".as_ptr());
            let canvas = sys::BK_node_new(c"canvas".as_ptr());
            if root.is_null() || canvas.is_null() {
                return Err("kit nodes unavailable".into());
            }
            sys::BK_node_add(root, canvas, -1);
            sys::BK_geom_set_manager(canvas, sys::BK_GEOM_PLACE);
            sys::BK_geom_set(canvas, c"relwidth".as_ptr(), 1.0);
            sys::BK_geom_set(canvas, c"relheight".as_ptr(), 1.0);
            sys::BK_set_num(canvas, c"events".as_ptr(), 1.0);
            let ctx = sys::BK_context_new(root);
            let host = sys::BKHost {
                data: id as *mut c_void,
                request: Some(noop),
                set_cursor: None,
                warp: None,
                clipboard_get: None,
                clipboard_set: None,
                schedule: None,
                emit: Some(on_emit),
            };
            sys::BK_context_set_host(ctx, &host);
            let mut win = Window {
                id,
                ctx,
                canvas,
                root,
                opaque: false,
                native: None,
                raster: std::ptr::null_mut(),
                phys: (w.round() as i32, h.round() as i32),
                scale: 1.0,
                view: None,
                frame: None,
                state: UiState::default(),
                widgets: Widgets::new(),
                dirty: true,
                timer_pending: false,
                animation_frame: false,
                vsync: true,
                closed: false,
                title: title.to_owned(),
                cursor: 0,
                pointer: (-1.0, -1.0),
                pressed: Vec::new(),
                press_button: MouseButton::Left,
            };
            if headless {
                win.scale = sys::BK_scale().max(1.0);
                win.phys = ((w * win.scale).round() as i32, (h * win.scale).round() as i32);
            } else {
                let t = CString::new(title).unwrap_or_default();
                let n = sys::BK_window_new(ctx, t.as_ptr(), w.round() as c_int, h.round() as c_int);
                if n.is_null() {
                    return Err("could not create a native window".into());
                }
                let cb = sys::BKWindowCallbacks { data: id as *mut c_void, on_timer: Some(on_timer),
                                                  on_close: Some(on_close), on_frame: Some(on_frame) };
                sys::BK_window_set_callbacks(n, &cb);
                win.scale = sys::BK_scale().max(0.5);
                win.phys = ((w * win.scale).round() as i32, (h * win.scale).round() as i32);
                win.native = Some(n);
            }
            Ok(win)
        }
    }

    fn destroy(&mut self) {
        self.widgets.free_all();
        unsafe {
            if let Some(n) = self.native.take() {
                sys::BK_window_free(n);
            }
            if !self.raster.is_null() {
                sys::BUI_raster_free(self.raster);
                self.raster = std::ptr::null_mut();
            }
        }
    }

    /// Window id.
    pub fn window_handle(&self) -> WindowId {
        WindowId(self.id)
    }

    /// Re-render on the next turn.
    /// Render again on the next display refresh (gpui's
    /// `request_animation_frame`): call it from `render` to animate.
    pub fn request_animation_frame(&mut self) {
        self.animation_frame = true;
        self.dirty = true;
    }

    /// Pace animation frames to the display (default `true`). With `false`
    /// frames render as fast as the CPU allows (benchmarks).
    pub fn set_vsync(&mut self, on: bool) {
        self.vsync = on;
    }

    /// Frames presented to the native window so far.
    pub fn presented_frames(&self) -> u64 {
        self.native.map_or(0, |n| unsafe { sys::BK_window_frames(n) } as u64)
    }

    pub fn refresh(&mut self) {
        self.dirty = true;
    }

    /// Content size in logical pixels.
    pub fn viewport_size(&self) -> Size<Pixels> {
        size(px(self.phys.0 as f32 / self.scale), px(self.phys.1 as f32 / self.scale))
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        Bounds { origin: point(px(0.0), px(0.0)), size: self.viewport_size() }
    }

    /// Device pixels per logical pixel.
    pub fn scale_factor(&self) -> f32 {
        self.scale
    }

    pub fn set_window_title(&mut self, title: &str) {
        if self.title != title {
            self.title = title.to_owned();
            if let Some(n) = self.native {
                let t = CString::new(title).unwrap_or_default();
                unsafe { sys::BK_window_set_title(n, t.as_ptr()) };
            }
        }
    }

    /// Pointer position in logical pixels.
    pub fn mouse_position(&self) -> crate::geometry::Point<Pixels> {
        point(px(self.pointer.0), px(self.pointer.1))
    }

    /// The `.id()` of the focused element, if any.
    pub fn focused_id(&self) -> Option<&str> {
        self.state.focused.as_deref().and_then(|k| k.strip_prefix('#'))
    }

    /// Focus the element with `.id(id)` (keyboard events go to it).
    pub fn focus_id(&mut self, id: &str) {
        self.state.focused = Some(format!("#{id}"));
        self.dirty = true;
    }

    /// Is the element with `.id(id)` hovered?
    pub fn is_hovered(&self, id: &str) -> bool {
        self.state.hovered.contains(&format!("#{id}"))
    }

    /// Scroll offset of a scroll container with `.id(id)`.
    pub fn scroll_offset(&self, id: &str) -> (f32, f32) {
        self.state.scroll.get(&format!("#{id}")).copied().unwrap_or((0.0, 0.0))
    }

    pub fn set_scroll_offset(&mut self, id: &str, x: f32, y: f32) {
        self.state.scroll.insert(format!("#{id}"), (x, y));
        self.dirty = true;
    }

    fn viewport(&self) -> (f32, f32) {
        (self.phys.0 as f32 / self.scale, self.phys.1 as f32 / self.scale)
    }

    // -- render -----------------------------------------------------------------------------

    fn rebuild(&mut self, app: &mut App) {
        let Some(view) = self.view.clone() else { return };
        self.dirty = false;
        let root = {
            let _s = crate::prof::span("render");
            view(self, app)
        };
        let vp = self.viewport();
        let (canvas, scale) = (self.canvas, self.scale);
        let widgets = &mut self.widgets;
        let frame = layout(root, vp, &self.state, &mut |key, spec| widgets.ensure(canvas, key, spec, scale));
        let mut p = Painter::new();
        {
            let _s = crate::prof::span("paint");
            paint_frame(&frame, &mut p, vp);
        }
        {
            let _s = crate::prof::span("flush");
            p.flush(canvas, scale);
        }
        let _rest = crate::prof::span("widgets+hover");
        let mut at = HashMap::new();
        for n in &frame.nodes {
            if matches!(n.content, Content::Widget(_)) {
                at.insert(n.key.clone(), (n.bounds, n.clip, n.visible));
            }
        }
        self.widgets.place(&at, scale);
        // an opaque <html> background covers the window: the kit can skip its clear
        let opaque = frame.nodes.first().is_some_and(|h| h.style.background.a >= 1.0 && h.style.opacity >= 1.0);
        if opaque != self.opaque {
            self.opaque = opaque;
            unsafe { sys::BK_set_num(self.root, c"opaque".as_ptr(), opaque as i32 as f64) };
        }
        if let Some(t) = frame.title.clone() {
            self.set_window_title(&t);
        }
        self.frame = Some(frame);
        self.update_hover(app, false);
        drop(_rest);
        crate::prof::frame_done();
    }

    // -- events ------------------------------------------------------------------------------

    /// Element keys from the root to the deepest element under the pointer.
    fn path_at(&self, x: f32, y: f32) -> Vec<String> {
        let Some(f) = &self.frame else { return Vec::new() };
        f.hit_path(x, y).into_iter().map(|i| f.nodes[i].key.clone()).collect()
    }

    /// Listeners of `pick` from `key` up to the root (bubbling order).
    fn bubble<E: 'static>(&self, key: &str, pick: impl Fn(&Listeners) -> Vec<Listener<E>>)
        -> Vec<Listener<E>> {
        let Some(f) = &self.frame else { return Vec::new() };
        let mut out = Vec::new();
        let mut cur = f.by_key().get(key).copied();
        while let Some(i) = cur {
            if let Some(el) = &f.nodes[i].el {
                out.extend(pick(&el.listeners));
            }
            cur = f.nodes[i].parent;
        }
        out
    }

    fn fire<E: 'static>(&mut self, app: &mut App, listeners: Vec<Listener<E>>, e: &E) {
        app.propagate = true;
        for l in listeners {
            l(e, self, app);
            if !app.propagate {
                break;
            }
        }
        app.propagate = true;
    }

    fn update_hover(&mut self, app: &mut App, fire: bool) {
        let path = self.path_at(self.pointer.0, self.pointer.1);
        let now: std::collections::HashSet<String> = path.iter().cloned().collect();
        if now == self.state.hovered {
            return;
        }
        let entered: Vec<String> = now.difference(&self.state.hovered).cloned().collect();
        let left: Vec<String> = self.state.hovered.difference(&now).cloned().collect();
        let restyle = self.frame.as_ref().is_some_and(|f| {
            entered.iter().chain(&left).any(|k| f.by_key().get(k).is_some())
        });
        self.state.hovered = now;
        // cursor of the deepest element that sets one
        if let Some(f) = &self.frame {
            let cursor = path.iter().rev()
                .find_map(|k| f.by_key().get(k).and_then(|&i| f.nodes[i].style.cursor)).unwrap_or(0);
            if cursor != self.cursor {
                self.cursor = cursor;
                unsafe { sys::BK_set_num(self.canvas, c"cursor".as_ptr(), cursor as f64) };
            }
        }
        if fire {
            for (keys, on) in [(left, false), (entered, true)] {
                for k in keys {
                    let ls = self.frame.as_ref().and_then(|f| f.by_key().get(&k).map(|&i| f.nodes[i].clone_hover()))
                        .unwrap_or_default();
                    self.fire(app, ls, &on);
                }
            }
        }
        if restyle {
            self.dirty = true;
        }
    }

    fn pointer_event(&self, text: &str) -> (Modifiers, MouseButton) {
        let mut it = text.split_whitespace().map(|v| v.parse::<i32>().unwrap_or(0));
        let mods = it.next().unwrap_or(0);
        let button = match it.next().unwrap_or(1) {
            2 => MouseButton::Middle,
            3 => MouseButton::Right,
            _ => MouseButton::Left,
        };
        (Modifiers::from_kit(mods), button)
    }

    fn dispatch(&mut self, ev: RawEvent, app: &mut App) {
        if ev.node != self.canvas {
            self.widget_event(ev, app);
            return;
        }
        let (x, y) = (ev.a as f32 / self.scale, ev.b as f32 / self.scale);
        match ev.kind {
            sys::BK_EV_CONFIGURE => {
                let phys = (ev.a as i32, ev.b as i32);
                if phys != self.phys && phys.0 > 0 && phys.1 > 0 {
                    self.phys = phys;
                    self.dirty = true;
                }
            }
            sys::BK_EV_MOTION | sys::BK_EV_ENTER => {
                self.pointer = (x, y);
                self.update_hover(app, true);
                let (modifiers, _) = self.pointer_event(&ev.text);
                let pressed = (!self.pressed.is_empty()).then_some(self.press_button);
                let e = MouseMoveEvent { position: point(px(x), px(y)), pressed_button: pressed, modifiers };
                if let Some(deepest) = self.state_deepest() {
                    let ls = self.bubble(&deepest, |l| l.mouse_move.clone());
                    self.fire(app, ls, &e);
                }
            }
            sys::BK_EV_LEAVE => {
                self.pointer = (-1.0, -1.0);
                self.update_hover(app, true);
            }
            sys::BK_EV_PRESS | sys::BK_EV_DOUBLE => {
                self.pointer = (x, y);
                let (modifiers, button) = self.pointer_event(&ev.text);
                let path = self.path_at(x, y);
                self.press_button = button;
                self.state.active = path.iter().cloned().collect();
                self.state.focused = path.last().cloned();
                self.pressed = path.clone();
                self.dirty = true;
                let e = MouseDownEvent { button, position: point(px(x), px(y)), modifiers,
                                         click_count: if ev.kind == sys::BK_EV_DOUBLE { 2 } else { 1 } };
                if let Some(deepest) = path.last() {
                    let ls = self.bubble(deepest, |l| l.mouse_down.iter()
                        .filter(|(b, _)| b.is_none_or(|b| b == button)).map(|(_, f)| f.clone()).collect());
                    self.fire(app, ls, &e);
                }
            }
            sys::BK_EV_RELEASE => {
                let (modifiers, button) = self.pointer_event(&ev.text);
                let path = self.path_at(x, y);
                let up = MouseUpEvent { button, position: point(px(x), px(y)), modifiers };
                if let Some(deepest) = path.last() {
                    let ls = self.bubble(deepest, |l| l.mouse_up.iter()
                        .filter(|(b, _)| b.is_none_or(|b| b == button)).map(|(_, f)| f.clone()).collect());
                    self.fire(app, ls, &up);
                }
                // click: the deepest element under both press and release
                let common = self.pressed.iter().zip(&path).take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.clone()).last();
                self.pressed.clear();
                self.state.active.clear();
                self.dirty = true;
                if let (Some(target), MouseButton::Left) = (common, button) {
                    self.toggle_details(&target);
                    let down = MouseDownEvent { button, position: point(px(x), px(y)), modifiers, click_count: 1 };
                    let e = ClickEvent { down, up };
                    let ls = self.bubble(&target, |l| l.click.clone());
                    self.fire(app, ls, &e);
                }
            }
            sys::BK_EV_WHEEL => {
                let (modifiers, _) = self.pointer_event(&ev.text);
                let (px_, py_) = self.pointer;
                let e = ScrollWheelEvent { position: point(px(px_), px(py_)), delta: ev.a as f32, modifiers };
                let path = self.path_at(px_, py_);
                if let Some(deepest) = path.last() {
                    let ls = self.bubble(deepest, |l| l.scroll.clone());
                    self.fire(app, ls, &e);
                }
                self.scroll_by(&path, -(ev.a as f32) * 40.0, modifiers.shift);
            }
            sys::BK_EV_KEY | sys::BK_EV_KEYRELEASE => {
                let mut parts = ev.text.splitn(2, '\t');
                let ch = parts.next().unwrap_or("").to_owned();
                let key = parts.next().unwrap_or("").to_owned();
                let ks = Keystroke { key, key_char: (!ch.is_empty()).then_some(ch),
                                     modifiers: Modifiers::from_kit(ev.a as i32) };
                let target = self.state.focused.clone().or_else(|| self.state_deepest())
                    .unwrap_or_else(|| "html".into());
                if ev.kind == sys::BK_EV_KEY {
                    let ls = self.bubble(&target, |l| l.key_down.clone());
                    self.fire(app, ls, &KeyDownEvent { keystroke: ks });
                } else {
                    let ls = self.bubble(&target, |l| l.key_up.clone());
                    self.fire(app, ls, &KeyUpEvent { keystroke: ks });
                }
            }
            _ => {}
        }
    }

    fn state_deepest(&self) -> Option<String> {
        let path = self.path_at(self.pointer.0, self.pointer.1);
        path.last().cloned()
    }

    /// `<summary>` clicks open and close their `<details>`.
    fn toggle_details(&mut self, target: &str) {
        let Some(f) = &self.frame else { return };
        let mut cur = f.by_key().get(target).copied();
        while let Some(i) = cur {
            let n = &f.nodes[i];
            if n.el.as_ref().is_some_and(|e| e.tag == "summary") {
                if let Some(p) = n.parent.filter(|&p| f.nodes[p].el.as_ref().is_some_and(|e| e.tag == "details")) {
                    let key = f.nodes[p].key.clone();
                    let open = self.state.details_open.get(&key).copied()
                        .unwrap_or(f.nodes[p].attr("open").is_some());
                    self.state.details_open.insert(key, !open);
                    self.dirty = true;
                }
                return;
            }
            cur = n.parent;
        }
    }

    /// Scroll the innermost scroll container along `path` that can move.
    fn scroll_by(&mut self, path: &[String], dy: f32, horizontal: bool) {
        let Some(f) = &self.frame else { return };
        for key in path.iter().rev() {
            let Some(&i) = f.by_key().get(key) else { continue };
            let n = &f.nodes[i];
            let (rx, ry) = n.scroll_range;
            let (sx, sy) = n.scroll_pos;
            let (nx, ny) = if horizontal || ry <= 0.0 {
                ((sx + dy).clamp(0.0, rx), sy)
            } else {
                (sx, (sy + dy).clamp(0.0, ry))
            };
            if (nx, ny) != (sx, sy) {
                self.state.scroll.insert(key.clone(), (nx, ny));
                self.dirty = true;
                return;
            }
        }
    }

    fn widget_event(&mut self, ev: RawEvent, app: &mut App) {
        if self.widgets.syncing {
            return;
        }
        let Some(key) = self.widgets.key_of(ev.node) else { return };
        match ev.kind {
            sys::BK_EV_CHANGE => {
                let e = self.widgets.read(&key);
                let kind = self.widgets.nodes.get(&key).map(|w| w.spec.kind);
                match kind {
                    Some(WKind::Checkbox) => {
                        self.state.checked.insert(key.clone(), e.checked);
                    }
                    Some(WKind::Radio) if e.checked => {
                        self.state.checked.insert(key.clone(), true);
                        for other in self.widgets.radio_group(&key) {
                            self.state.checked.insert(other.clone(), false);
                            self.widgets.set_value(&other, 0.0);
                        }
                    }
                    _ => {}
                }
                let ls = self.bubble(&key, |l| l.input.clone());
                self.fire(app, ls, &e);
                let ls = self.bubble(&key, |l| l.change.clone());
                self.fire(app, ls, &e);
                self.dirty = true;
            }
            sys::BK_EV_CLICK => {
                let (x, y) = self.pointer;
                let down = MouseDownEvent { button: MouseButton::Left, position: point(px(x), px(y)),
                                            modifiers: Modifiers::default(), click_count: 1 };
                let up = MouseUpEvent { button: MouseButton::Left, position: point(px(x), px(y)),
                                        modifiers: Modifiers::default() };
                let ls = self.bubble(&key, |l| l.click.clone());
                self.fire(app, ls, &ClickEvent { down, up });
                self.dirty = true;
            }
            _ => {}
        }
    }
}

impl crate::layout::BoxNode {
    fn clone_hover(&self) -> Vec<Listener<bool>> {
        self.el.as_ref().map(|e| e.listeners.hover.clone()).unwrap_or_default()
    }
}

/// Marker so `Window` is not `Send` (it holds raw kit pointers).
#[allow(dead_code)]
struct NotSend(PhantomData<*const ()>);
