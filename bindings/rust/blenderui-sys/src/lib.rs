//! Raw bindings to the BUI C kit (`kit/include/BUI_kit.h`, `gpu/include/GPU_raster.h`).
//!
//! Everything here is `unsafe` and mirrors the C header one to one; the safe,
//! gpui-style API is the `blenderui` crate.

#![allow(non_camel_case_types, non_snake_case)]

use std::os::raw::{c_char, c_double, c_float, c_int, c_uchar, c_void};

/// A kit node (widget, frame, canvas ...).
#[repr(C)]
pub struct BKNode {
    _private: [u8; 0],
}
/// Input state, popups, focus, undo for one node tree.
#[repr(C)]
pub struct BKContext {
    _private: [u8; 0],
}
/// A native window (Win32) showing a context.
#[repr(C)]
pub struct BKWindow {
    _private: [u8; 0],
}
/// A CPU frame buffer (0x00RRGGBB pixels) with damage tracking.
#[repr(C)]
pub struct BuiRaster {
    _private: [u8; 0],
}

// Event kinds (BK_EV_*).
pub const BK_EV_CHANGE: c_int = 1;
pub const BK_EV_CLICK: c_int = 2;
pub const BK_EV_CALL: c_int = 3;
pub const BK_EV_RENAME: c_int = 4;
pub const BK_EV_ACTIVATE: c_int = 5;
pub const BK_EV_TOGGLE: c_int = 6;
pub const BK_EV_BROWSE: c_int = 7;
pub const BK_EV_PRESS: c_int = 8;
pub const BK_EV_RELEASE: c_int = 9;
pub const BK_EV_DOUBLE: c_int = 10;
pub const BK_EV_MOTION: c_int = 11;
pub const BK_EV_ENTER: c_int = 12;
pub const BK_EV_LEAVE: c_int = 13;
pub const BK_EV_KEY: c_int = 14;
pub const BK_EV_WHEEL: c_int = 15;
pub const BK_EV_SUBMIT: c_int = 16;
pub const BK_EV_VIEW: c_int = 17;
pub const BK_EV_SCROLL: c_int = 18;
pub const BK_EV_CONFIGURE: c_int = 19;
pub const BK_EV_KEYRELEASE: c_int = 20;

// Modifier bits for input calls.
pub const BK_SHIFT: c_int = 1;
pub const BK_CTRL: c_int = 2;
pub const BK_ALT: c_int = 4;
pub const BK_BUTTON1: c_int = 8;

// Geometry managers and anchors.
pub const BK_GEOM_NONE: c_int = 0;
pub const BK_GEOM_PACK: c_int = 1;
pub const BK_GEOM_GRID: c_int = 2;
pub const BK_GEOM_PLACE: c_int = 3;
pub const BK_ANCHOR_N: c_int = 1;
pub const BK_ANCHOR_S: c_int = 2;
pub const BK_ANCHOR_W: c_int = 4;
pub const BK_ANCHOR_E: c_int = 8;

// Canvas item types.
pub const BK_ITEM_RECT: c_int = 0;
pub const BK_ITEM_OVAL: c_int = 1;
pub const BK_ITEM_LINE: c_int = 2;
pub const BK_ITEM_POLYGON: c_int = 3;
pub const BK_ITEM_TEXT: c_int = 4;
pub const BK_ITEM_ARC: c_int = 5;
pub const BK_ITEM_IMAGE: c_int = 6;
pub const BK_ITEM_ROUNDRECT: c_int = 7;

// Font flags (GPU_raster.h).
pub const BUI_FONT_BOLD: c_int = 1;
pub const BUI_FONT_ITALIC: c_int = 2;
pub const BUI_FONT_UNDERLINE: c_int = 4;

/// One item of a canvas display list (`BKCanvasItem`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BKCanvasItem {
    pub id: c_int,
    pub type_: c_int,
    pub coords: *const c_float,
    pub ncoords: c_int,
    pub fill: [c_float; 4],
    pub outline: [c_float; 4],
    pub width: c_float,
    pub radius: c_float,
    pub size: c_float,
    pub start: c_float,
    pub extent: c_float,
    pub text: *const c_char,
    pub anchor: c_int,
    pub rgba: *const c_uchar,
    pub iw: c_int,
    pub ih: c_int,
    pub hidden: c_int,
    pub text_flags: c_int,
    pub clip_on: c_int,
    pub clip: [c_float; 4],
}

impl Default for BKCanvasItem {
    fn default() -> Self {
        // SAFETY: all-zero is a valid "empty" item in C (null pointers, no colors).
        unsafe { std::mem::zeroed() }
    }
}

/// Host callbacks of a context (`BKHost`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BKHost {
    pub data: *mut c_void,
    pub request: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub set_cursor: Option<unsafe extern "C" fn(data: *mut c_void, cursor: c_int)>,
    pub warp: Option<unsafe extern "C" fn(data: *mut c_void, x: c_int, y: c_int)>,
    pub clipboard_get: Option<unsafe extern "C" fn(data: *mut c_void) -> *mut c_char>,
    pub clipboard_set: Option<unsafe extern "C" fn(data: *mut c_void, text: *const c_char)>,
    pub schedule: Option<unsafe extern "C" fn(data: *mut c_void, ms: c_int, timer: c_int)>,
    pub emit: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            node: *mut BKNode,
            kind: c_int,
            ref_: *mut c_void,
            a: c_double,
            b: c_double,
        ),
    >,
}

/// Native window callbacks (`BKWindowCallbacks`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BKWindowCallbacks {
    pub data: *mut c_void,
    pub on_timer: Option<unsafe extern "C" fn(data: *mut c_void, win: *mut BKWindow, id: c_int)>,
    pub on_close: Option<unsafe extern "C" fn(data: *mut c_void, win: *mut BKWindow) -> c_int>,
    pub on_frame: Option<unsafe extern "C" fn(data: *mut c_void, win: *mut BKWindow)>,
}

unsafe extern "C" {
    // look
    pub fn BK_set_scale(scale: c_float);
    pub fn BK_scale() -> c_float;
    pub fn BK_unit() -> c_int;
    pub fn BK_set_font(family: *const c_char);

    // nodes
    pub fn BK_node_new(type_: *const c_char) -> *mut BKNode;
    pub fn BK_node_free(node: *mut BKNode);
    pub fn BK_node_add(parent: *mut BKNode, child: *mut BKNode, index: c_int) -> c_int;
    pub fn BK_node_remove(parent: *mut BKNode, child: *mut BKNode);
    pub fn BK_node_rect(node: *const BKNode, r_rect: *mut c_int);
    pub fn BK_node_natural(node: *mut BKNode, r_size: *mut c_int);
    pub fn BK_node_set_image(node: *mut BKNode, rgba: *const c_uchar, w: c_int, h: c_int);

    // properties
    pub fn BK_set_num(node: *mut BKNode, key: *const c_char, value: c_double) -> c_int;
    pub fn BK_set_str(node: *mut BKNode, key: *const c_char, value: *const c_char) -> c_int;
    pub fn BK_set_color(node: *mut BKNode, key: *const c_char, rgba: *const c_float) -> c_int;
    pub fn BK_set_strs(
        node: *mut BKNode,
        key: *const c_char,
        values: *const *const c_char,
        n: c_int,
    ) -> c_int;
    pub fn BK_get_num(node: *mut BKNode, key: *const c_char) -> c_double;
    pub fn BK_get_str(node: *mut BKNode, key: *const c_char) -> *const c_char;
    pub fn BK_get_color(node: *mut BKNode, key: *const c_char, r_rgba: *mut c_float) -> c_int;

    // geometry managers
    pub fn BK_geom_set_manager(child: *mut BKNode, manager: c_int);
    pub fn BK_geom_set(child: *mut BKNode, key: *const c_char, value: c_double);

    // canvas display list
    pub fn BK_canvas_set_item(canvas: *mut BKNode, item: *const BKCanvasItem);
    pub fn BK_canvas_add_item(canvas: *mut BKNode, item: *const BKCanvasItem);
    pub fn BK_canvas_delete(canvas: *mut BKNode, id: c_int);
    pub fn BK_canvas_count(canvas: *mut BKNode) -> c_int;

    // contexts
    pub fn BK_context_new(root: *mut BKNode) -> *mut BKContext;
    pub fn BK_context_free(ctx: *mut BKContext);
    pub fn BK_context_set_host(ctx: *mut BKContext, host: *const BKHost);
    pub fn BK_render(ctx: *mut BKContext, r: *mut BuiRaster, w: c_int, h: c_int, full: c_int)
        -> c_int;
    pub fn BK_needs_render(ctx: *mut BKContext) -> c_int;
    pub fn BK_press(ctx: *mut BKContext, x: c_int, y: c_int, mods: c_int) -> c_int;
    pub fn BK_release(ctx: *mut BKContext, x: c_int, y: c_int, mods: c_int) -> c_int;
    pub fn BK_motion(ctx: *mut BKContext, x: c_int, y: c_int, mods: c_int) -> c_int;
    pub fn BK_wheel(ctx: *mut BKContext, x: c_int, y: c_int, delta: c_int, mods: c_int) -> c_int;
    pub fn BK_key(ctx: *mut BKContext, text: *const c_char, keysym: *const c_char, mods: c_int)
        -> c_int;
    pub fn BK_key_release(ctx: *mut BKContext, keysym: *const c_char, mods: c_int) -> c_int;
    pub fn BK_button(ctx: *mut BKContext, x: c_int, y: c_int, button: c_int, release: c_int,
                     mods: c_int) -> c_int;
    pub fn BK_context_menu(ctx: *mut BKContext, x: c_int, y: c_int, mods: c_int) -> c_int;
    pub fn BK_timer(ctx: *mut BKContext, timer: c_int);
    pub fn BK_cursor(ctx: *mut BKContext) -> c_int;
    pub fn BK_focus_node(ctx: *mut BKContext, n: *mut BKNode);
    pub fn BK_clipboard_get(ctx: *mut BKContext) -> *mut c_char;
    pub fn BK_clipboard_set(ctx: *mut BKContext, text: *const c_char);

    // native windows
    pub fn BK_window_supported() -> c_int;
    pub fn BK_window_new(ctx: *mut BKContext, title: *const c_char, width: c_int, height: c_int)
        -> *mut BKWindow;
    pub fn BK_window_free(win: *mut BKWindow);
    pub fn BK_window_set_callbacks(win: *mut BKWindow, cb: *const BKWindowCallbacks);
    pub fn BK_window_show(win: *mut BKWindow);
    pub fn BK_window_close(win: *mut BKWindow);
    pub fn BK_window_is_open(win: *mut BKWindow) -> c_int;
    pub fn BK_window_set_title(win: *mut BKWindow, title: *const c_char);
    pub fn BK_window_set_size(win: *mut BKWindow, width: c_int, height: c_int);
    pub fn BK_window_get_size(win: *mut BKWindow, r_width: *mut c_int, r_height: *mut c_int);
    pub fn BK_window_raster(win: *mut BKWindow) -> *mut BuiRaster;
    pub fn BK_window_frames(win: *mut BKWindow) -> c_int;
    pub fn BK_window_render_now(win: *mut BKWindow);
    pub fn BK_window_timer(win: *mut BKWindow, ms: c_int) -> c_int;
    pub fn BK_window_cancel_timer(win: *mut BKWindow, id: c_int);
    pub fn BK_window_request_frame(win: *mut BKWindow, vsync: c_int);
    pub fn BK_window_wm(win: *mut BKWindow, key: *const c_char, a: c_double, b: c_double);
    pub fn BK_window_wm_get(win: *mut BKWindow, key: *const c_char, r_a: *mut c_double,
                            r_b: *mut c_double) -> c_int;
    pub fn BK_window_run();
    pub fn BK_window_pump() -> c_int;
    pub fn BK_window_quit();

    // raster + text (gpu/)
    pub fn BUI_raster_create(width: c_int, height: c_int) -> *mut BuiRaster;
    pub fn BUI_raster_free(r: *mut BuiRaster);
    pub fn BUI_raster_width(r: *const BuiRaster) -> c_int;
    pub fn BUI_raster_height(r: *const BuiRaster) -> c_int;
    pub fn BUI_raster_pixels(r: *const BuiRaster) -> *const u32;
    pub fn BUI_text_measure(family: *const c_char, flags: c_int, size: c_float,
                            utf8: *const c_char, len: c_int, r_width: *mut c_float) -> c_int;
}
