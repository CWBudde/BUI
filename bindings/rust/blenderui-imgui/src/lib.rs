//! Dear ImGui on BUI for Rust: the real Dear ImGui (the version vendored in the BUI
//! repository) drawn by BUI's renderers, in Blender's look.
//!
//! - [`sys`]: raw FFI over the C API (`cimgui.h`), generated from `imgui.h` by
//!   `bindings/imgui/gen/gen.py` (no bindgen): every struct with its C++ layout, every
//!   enum value as a constant, `ImGui_Button`, `ImDrawList_AddLine`...
//! - the safe layer (re-exported here, also generated): `button(label)`,
//!   `button_ex(label, size)`, `slider_float(label, &mut v, min, max)`,
//!   `input_text(label, &mut String)`, methods on the structs
//!   (`get_window_draw_list().unwrap().add_line(p1, p2, col)`), constructors returning
//!   [`Owned`] (`ImGuiTextFilter::new()`).
//! - a runner on the BUI backend: [`run`] (a native window) and [`App`] (window or headless,
//!   with simulated input for tests).
//!
//! ```no_run
//! use blenderui_imgui as imgui;
//!
//! let mut value = 0.5f32;
//! imgui::run("Hello", [1280, 800], move || {
//!     imgui::show_demo_window();
//!     imgui::begin("Hello");
//!     imgui::slider_float("Value", &mut value, 0.0, 1.0);
//!     imgui::end();
//! });
//! ```
//!
//! Dear ImGui keeps one current context per thread of use: create apps and call imgui
//! functions from one thread. Pointers and references the functions return
//! (`get_io()`, draw lists...) are only valid while their context lives.
//!
//! SPDX-License-Identifier: GPL-2.0-or-later

use core::ffi::{c_char, c_int, c_void};
use std::any::Any;
use std::ffi::{CStr, CString};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::ptr::NonNull;

// links the native libraries (bui_cimgui, bui_imgui, the kit) built by blenderui-sys
extern crate blenderui_sys as _;

pub mod sys;
mod safe;

pub use safe::*;
pub use sys::*;

// ------------------------------------------------------------------------------------
// helpers used by the generated safe layer
// ------------------------------------------------------------------------------------

/// `&str` -> C string (cut at an interior NUL).
pub(crate) fn cs(s: &str) -> CString {
    match CString::new(s) {
        Ok(c) => c,
        Err(e) => {
            let n = e.nul_position();
            CString::new(&s.as_bytes()[..n]).unwrap_or_default()
        }
    }
}

/// A C string returned by imgui (static, or owned by the context) as `&str` ("" for NULL).
pub(crate) fn from_cstr(p: *const c_char) -> &'static str {
    if p.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(p) }.to_str().unwrap_or("")
}

/// The `char* buf` of `input_text*`: grown by Dear ImGui through the resize callback.
pub(crate) struct InputBuf {
    buf: Vec<u8>,
}

impl InputBuf {
    pub(crate) fn new(s: &str) -> Self {
        let mut buf = Vec::with_capacity(s.len() + 64);
        buf.extend_from_slice(s.as_bytes());
        buf.resize(s.len() + 64, 0);
        InputBuf { buf }
    }
    pub(crate) fn ptr(&mut self) -> *mut c_char {
        self.buf.as_mut_ptr() as *mut c_char
    }
    pub(crate) fn capacity(&self) -> usize {
        self.buf.len()
    }
    pub(crate) fn finish(&self, out: &mut String) {
        let n = self.buf.iter().position(|&b| b == 0).unwrap_or(self.buf.len());
        let s = String::from_utf8_lossy(&self.buf[..n]);
        if *out != s {
            *out = s.into_owned();
        }
    }
}

pub(crate) unsafe extern "C" fn input_text_resize(data: *mut ImGuiInputTextCallbackData) -> c_int {
    unsafe {
        let d = &mut *data;
        if d.EventFlag == ImGuiInputTextFlags_CallbackResize {
            let b = &mut *(d.UserData as *mut InputBuf);
            b.buf.resize(d.BufSize as usize + 1, 0);
            d.Buf = b.buf.as_mut_ptr() as *mut c_char;
        }
    }
    0
}

/// Types with a C++ destructor reachable from Rust (`X_Delete`).
pub trait Delete {
    /// # Safety
    /// `p` must come from the matching `X_New`.
    unsafe fn delete(p: *mut Self);
}

/// An imgui object created from Rust (`ImGuiTextFilter::new()`...): deleted on drop.
pub struct Owned<T: Delete>(NonNull<T>);

impl<T: Delete> Owned<T> {
    pub(crate) fn from_raw(p: *mut T) -> Self {
        Owned(NonNull::new(p).expect("imgui allocation failed"))
    }
    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }
}

impl<T: Delete> core::ops::Deref for Owned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T: Delete> core::ops::DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<T: Delete> Drop for Owned<T> {
    fn drop(&mut self) {
        unsafe { T::delete(self.0.as_ptr()) }
    }
}

impl From<[f32; 2]> for ImVec2 {
    fn from(v: [f32; 2]) -> Self {
        ImVec2 { x: v[0], y: v[1] }
    }
}
impl From<(f32, f32)> for ImVec2 {
    fn from(v: (f32, f32)) -> Self {
        ImVec2 { x: v.0, y: v.1 }
    }
}
impl From<[f32; 4]> for ImVec4 {
    fn from(v: [f32; 4]) -> Self {
        ImVec4 { x: v[0], y: v[1], z: v[2], w: v[3] }
    }
}
impl From<(f32, f32, f32, f32)> for ImVec4 {
    fn from(v: (f32, f32, f32, f32)) -> Self {
        ImVec4 { x: v.0, y: v.1, z: v.2, w: v.3 }
    }
}
impl From<ImTextureID> for ImTextureRef {
    /// A user texture (for BUI: a `BUI_texture_create` id).
    fn from(id: ImTextureID) -> Self {
        ImTextureRef { _TexData: core::ptr::null_mut(), _TexID: id }
    }
}

/// `IM_COL32(r, g, b, a)`: a packed color.
pub const fn im_col32(r: u8, g: u8, b: u8, a: u8) -> ImU32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}

// ------------------------------------------------------------------------------------
// the BUI backend (cimgui_impl_bui.h)
// ------------------------------------------------------------------------------------

/// Modifier bits for [`App::key`].
pub const BK_SHIFT: i32 = 1;
pub const BK_CTRL: i32 = 2;
pub const BK_ALT: i32 = 4;

#[repr(C)]
struct RawApp {
    _opaque: [u8; 0],
}

/// Backend options (`cImGui_ImplBUI_Config`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Blender's theme colors and metrics (true).
    pub blender_style: bool,
    /// Blender's UI font, Inter (true).
    pub blender_font: bool,
    /// Font size in pixels at scale 1; 0: Blender's UI text size.
    pub font_size: f32,
    /// Follow DPI changes (true).
    pub follow_scale: bool,
    /// Frames after an input event (3).
    pub settle_frames: c_int,
}

impl Default for Config {
    fn default() -> Self {
        unsafe { cImGui_ImplBUI_DefaultConfig() }
    }
}

type FrameFn = unsafe extern "C" fn(user: *mut c_void);

unsafe extern "C" {
    fn cImGui_ImplBUI_DefaultConfig() -> Config;
    fn cImGui_ImplBUI_CreateApp(title: *const c_char, w: c_int, h: c_int, frame: FrameFn, user: *mut c_void, cfg: *const Config) -> *mut RawApp;
    fn cImGui_ImplBUI_CreateHeadlessApp(w: c_int, h: c_int, frame: FrameFn, user: *mut c_void, cfg: *const Config) -> *mut RawApp;
    fn cImGui_ImplBUI_RunApp(app: *mut RawApp);
    fn cImGui_ImplBUI_CloseApp(app: *mut RawApp);
    fn cImGui_ImplBUI_DestroyApp(app: *mut RawApp);
    fn cImGui_ImplBUI_IsHeadless(app: *mut RawApp) -> bool;
    fn cImGui_ImplBUI_Render(app: *mut RawApp, n: c_int) -> c_int;
    fn cImGui_ImplBUI_Pixels(app: *mut RawApp, w: *mut c_int, h: *mut c_int) -> *const u32;
    fn cImGui_ImplBUI_SavePNG(app: *mut RawApp, path: *const c_char) -> bool;
    fn cImGui_ImplBUI_MouseMove(app: *mut RawApp, x: f32, y: f32);
    fn cImGui_ImplBUI_MouseDown(app: *mut RawApp, x: f32, y: f32, button: c_int);
    fn cImGui_ImplBUI_MouseUp(app: *mut RawApp, x: f32, y: f32, button: c_int);
    fn cImGui_ImplBUI_Scroll(app: *mut RawApp, x: f32, y: f32, dx: f32, dy: f32);
    fn cImGui_ImplBUI_KeyDown(app: *mut RawApp, keysym: *const c_char, mods: c_int);
    fn cImGui_ImplBUI_KeyUp(app: *mut RawApp, keysym: *const c_char, mods: c_int);
    fn cImGui_ImplBUI_Key(app: *mut RawApp, keysym: *const c_char, mods: c_int);
    fn cImGui_ImplBUI_TypeText(app: *mut RawApp, utf8: *const c_char);
    fn cImGui_ImplBUI_FrameCount() -> c_int;
    fn cImGui_ImplBUI_StyleBlender(dst: *mut ImGuiStyle, scale: f32);
    fn cImGui_ImplBUI_GetScale() -> f32;
    fn cImGui_ImplBUI_HasBlenderWidgets() -> bool;
    fn cImGui_ImplBUI_SetBlenderWidgets(on: bool);
    fn cImGui_ImplBUI_GetBlenderWidgets() -> bool;
    fn cImGui_ImplBUI_SetScale(scale: f32);
}

/// Blender's theme (colors and metrics at `scale`) on `style` (None: the current style).
pub fn style_blender(style: Option<&mut ImGuiStyle>, scale: f32) {
    unsafe { cImGui_ImplBUI_StyleBlender(style.map_or(core::ptr::null_mut(), |s| s as *mut _), scale) }
}

/// Built with Blender's widgets (feature `blender-widgets`, CMake BUI_IMGUI_BLENDER_WIDGETS):
/// ImGui's widgets drawn and handled as Blender's.
pub fn has_blender_widgets() -> bool {
    unsafe { cImGui_ImplBUI_HasBlenderWidgets() }
}

/// Whether Blender's widgets are in use (on by default when built with them).
pub fn blender_widgets() -> bool {
    unsafe { cImGui_ImplBUI_GetBlenderWidgets() }
}

/// Switches Blender's widgets on or off at runtime (no effect when not built with them).
pub fn set_blender_widgets(on: bool) {
    unsafe { cImGui_ImplBUI_SetBlenderWidgets(on) }
}

/// BUI's UI scale (DPI).
pub fn ui_scale() -> f32 {
    unsafe { cImGui_ImplBUI_GetScale() }
}

/// Sets BUI's UI scale (1 = Blender's 20 px widget unit).
pub fn set_ui_scale(scale: f32) {
    unsafe { cImGui_ImplBUI_SetScale(scale) }
}

struct FrameState<F> {
    frame: F,
    app: *mut RawApp,
    panic: Option<Box<dyn Any + Send>>,
}

unsafe extern "C" fn trampoline<F: FnMut()>(user: *mut c_void) {
    let st = unsafe { &mut *(user as *mut FrameState<F>) };
    if st.panic.is_some() {
        return; // empty frames until the caller sees the panic
    }
    if let Err(p) = catch_unwind(AssertUnwindSafe(|| (st.frame)())) {
        st.panic = Some(p);
        unsafe { cImGui_ImplBUI_CloseApp(st.app) };
    }
}

/// Dear ImGui in a BUI window, or headless: `frame` runs inside each ImGui frame.
///
/// Creates an ImGui context when none is current (destroyed with the app). A panic in
/// `frame` closes the window and resumes from [`App::run`] / [`App::render`].
pub struct App<F: FnMut()> {
    raw: *mut RawApp,
    state: Box<FrameState<F>>,
    own_ctx: *mut ImGuiContext,
}

impl<F: FnMut()> App<F> {
    /// A native window (headless when there is no window system: see [`App::is_headless`]).
    pub fn new(title: &str, size: [i32; 2], frame: F) -> Self {
        Self::create(Some(title), size, Config::default(), frame)
    }

    /// No window: render with [`App::render`] into an offscreen raster of `size`.
    pub fn headless(size: [i32; 2], frame: F) -> Self {
        Self::create(None, size, Config::default(), frame)
    }

    /// Either, with backend options.
    pub fn with_config(title: Option<&str>, size: [i32; 2], config: Config, frame: F) -> Self {
        Self::create(title, size, config, frame)
    }

    fn create(title: Option<&str>, size: [i32; 2], config: Config, frame: F) -> Self {
        let mut own_ctx = core::ptr::null_mut();
        unsafe {
            if ImGui_GetCurrentContext().is_null() {
                own_ctx = ImGui_CreateContext();
                ImGui_SetCurrentContext(own_ctx);
                (*ImGui_GetIO()).IniFilename = core::ptr::null();
            }
        }
        let mut state = Box::new(FrameState { frame, app: core::ptr::null_mut(), panic: None });
        let user = &mut *state as *mut FrameState<F> as *mut c_void;
        let raw = unsafe {
            match title {
                Some(t) => {
                    let t = cs(t);
                    cImGui_ImplBUI_CreateApp(t.as_ptr(), size[0], size[1], trampoline::<F>, user, &config)
                }
                None => cImGui_ImplBUI_CreateHeadlessApp(size[0], size[1], trampoline::<F>, user, &config),
            }
        };
        state.app = raw;
        App { raw, state, own_ctx }
    }

    fn rethrow(&mut self) {
        if let Some(p) = self.state.panic.take() {
            resume_unwind(p);
        }
    }

    /// BUI's message loop until the window closes (returns at once when headless).
    pub fn run(&mut self) {
        unsafe { cImGui_ImplBUI_RunApp(self.raw) };
        self.rethrow();
    }

    /// Closes the window (ends [`App::run`]).
    pub fn close(&mut self) {
        unsafe { cImGui_ImplBUI_CloseApp(self.raw) }
    }

    pub fn is_headless(&self) -> bool {
        unsafe { cImGui_ImplBUI_IsHeadless(self.raw) }
    }

    /// Requests `n` frames and renders them offscreen; returns the frames run so far.
    pub fn render(&mut self, n: i32) -> i32 {
        let r = unsafe { cImGui_ImplBUI_Render(self.raw, n) };
        self.rethrow();
        r
    }

    /// The last rendered frame: 0xAARRGGBB pixels, width, height.
    pub fn pixels(&self) -> Option<(&[u32], usize, usize)> {
        let (mut w, mut h) = (0, 0);
        let p = unsafe { cImGui_ImplBUI_Pixels(self.raw, &mut w, &mut h) };
        if p.is_null() {
            return None;
        }
        let (w, h) = (w as usize, h as usize);
        Some((unsafe { core::slice::from_raw_parts(p, w * h) }, w, h))
    }

    /// Writes the last rendered frame as a PNG.
    pub fn save_png(&self, path: &str) -> bool {
        let p = cs(path);
        unsafe { cImGui_ImplBUI_SavePNG(self.raw, p.as_ptr()) }
    }

    pub fn mouse_move(&mut self, x: f32, y: f32) {
        unsafe { cImGui_ImplBUI_MouseMove(self.raw, x, y) };
        self.rethrow();
    }
    /// button: 0 left, 1 right, 2 middle.
    pub fn mouse_down(&mut self, x: f32, y: f32, button: i32) {
        unsafe { cImGui_ImplBUI_MouseDown(self.raw, x, y, button) };
        self.rethrow();
    }
    pub fn mouse_up(&mut self, x: f32, y: f32, button: i32) {
        unsafe { cImGui_ImplBUI_MouseUp(self.raw, x, y, button) };
        self.rethrow();
    }
    pub fn click(&mut self, x: f32, y: f32) {
        self.mouse_down(x, y, 0);
        self.mouse_up(x, y, 0);
    }
    /// Wheel notches: dy > 0 scrolls up.
    pub fn scroll(&mut self, x: f32, y: f32, dx: f32, dy: f32) {
        unsafe { cImGui_ImplBUI_Scroll(self.raw, x, y, dx, dy) };
        self.rethrow();
    }
    /// Press and release a key by X11 name ("Return", "BackSpace", "a"); mods: BK_SHIFT | BK_CTRL | BK_ALT.
    pub fn key(&mut self, keysym: &str, mods: i32) {
        let k = cs(keysym);
        unsafe { cImGui_ImplBUI_Key(self.raw, k.as_ptr(), mods) };
        self.rethrow();
    }
    pub fn key_down(&mut self, keysym: &str, mods: i32) {
        let k = cs(keysym);
        unsafe { cImGui_ImplBUI_KeyDown(self.raw, k.as_ptr(), mods) };
        self.rethrow();
    }
    pub fn key_up(&mut self, keysym: &str, mods: i32) {
        let k = cs(keysym);
        unsafe { cImGui_ImplBUI_KeyUp(self.raw, k.as_ptr(), mods) };
        self.rethrow();
    }
    /// Typed text (as the window hosts send characters).
    pub fn type_text(&mut self, text: &str) {
        let t = cs(text);
        unsafe { cImGui_ImplBUI_TypeText(self.raw, t.as_ptr()) };
        self.rethrow();
    }
    /// ImGui frames run by the backend so far (current context).
    pub fn frame_count(&self) -> i32 {
        unsafe { cImGui_ImplBUI_FrameCount() }
    }
}

impl<F: FnMut()> Drop for App<F> {
    fn drop(&mut self) {
        unsafe {
            cImGui_ImplBUI_DestroyApp(self.raw);
            if !self.own_ctx.is_null() {
                ImGui_DestroyContextEx(self.own_ctx);
            }
        }
    }
}

/// Opens a BUI window titled `title` and calls `frame` inside each ImGui frame until the
/// window closes (Blender's style and font). Returns at once without a window system.
pub fn run(title: &str, size: [i32; 2], frame: impl FnMut()) {
    let mut app = App::new(title, size, frame);
    app.run();
}
