//! # blenderui
//!
//! Blender's UI for Rust with gpui's syntax, every HTML element and CSS.
//!
//! ```no_run
//! use blenderui::prelude::*;
//!
//! struct Counter {
//!     count: i32,
//! }
//!
//! impl Render for Counter {
//!     fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!         div()
//!             .flex()
//!             .flex_col()
//!             .gap_2()
//!             .p_4()
//!             .child(h1().child(format!("Count: {}", self.count)))
//!             .child(button().child("Add one").on_click(cx.listener(|this, _, _, cx| {
//!                 this.count += 1;
//!                 cx.notify();
//!             })))
//!     }
//! }
//!
//! fn main() {
//!     Application::new().run(|cx: &mut App| {
//!         cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Counter { count: 0 }))
//!             .unwrap();
//!     });
//! }
//! ```
//!
//! * **Elements**: one [`Div`] type for all 113 HTML elements ([`html`] has a
//!   constructor per tag), with `.child()`, `.on_click()`, `.id()`, `.attr()`.
//! * **Styling**: gpui / Tailwind methods ([`Styled`]), any CSS property with
//!   `.css("prop", "value")` / `.style("a: b; c: d")`, and `<style>` sheets
//!   with selectors, combinators, pseudo-classes and `@media`.
//! * **Layout**: block, inline (wrapping line boxes), flexbox, grid, tables,
//!   positioning and scrolling via [taffy](https://docs.rs/taffy).
//! * **Rendering**: Blender's widget drawing; form controls are Blender's own
//!   widgets (number fields, sliders, dropdowns, text fields ...).

// lets `css!` (which names `::blenderui::...`) work inside this crate too
extern crate self as blenderui;

/// A CSS declaration block checked and resolved at compile time:
/// `div().style(css!("display: flex; gap: 4px"))`. An unknown property is a
/// compile error:
///
/// ```compile_fail
/// use blenderui::prelude::*;
/// let _ = div().style(css!("colr: red"));
/// ```
pub use blenderui_macros::css;

/// Tailwind CSS v4 classes checked at compile time:
/// `div().class(tw!("p-4 md:flex hover:bg-sky-500/50"))`. Plain
/// `.class("...")` works too (classes compile on first use); `tw!` turns a
/// typo into a compile error:
///
/// ```compile_fail
/// use blenderui::prelude::*;
/// let _ = div().class(tw!("p-4 flx"));
/// ```
pub use blenderui_macros::tw;

/// JSX-style markup lowered to builder calls (no runtime parsing):
///
/// ```
/// use blenderui::prelude::*;
/// let name = "Ada";
/// let items = vec!["a", "b"];
/// let _ = rsx! {
///     <div class="flex gap-2 p-4" onClick={|| println!("hi")}>
///         <h1 style="color: red">"Hello, " {name}</h1>
///         <input type="checkbox" checked={true} />
///         <ul>{for it in items { <li>{it}</li> }}</ul>
///         {if name.len() > 2 { <p>long name</p> }}
///     </div>
/// };
/// ```
///
/// Attributes with `{expr}` call the builder method of that name, so any
/// method works (`w={px(10.)}`, `hover={|s| s.bg(red())}`); text values
/// become HTML attributes. Components: `<Card title="x">..</Card>` builds
/// `Card { title: "x".into(), children: vec![..] }`.
pub use blenderui_macros::rsx;
/// [`rsx!`] under its zopra / Leptos name.
pub use blenderui_macros::rsx as view;

/// `#[derive(Props)]` on a struct component: `rsx!` may leave out its
/// `Option<T>` fields and `children`, and names missing required ones.
pub use blenderui_macros::Props;

pub mod anim;
pub mod calc;
pub mod components;
pub mod effects;
pub mod color;
mod color4;
pub mod css;
pub mod element;
pub mod events;
pub mod geometry;
pub mod html;
pub(crate) mod layout;
pub mod paint;
pub(crate) mod prof;
mod selector;
pub mod hooks;
pub mod style;
pub use blenderui_tailwind as tailwind;
mod tree;
mod vars;
pub mod styled;
pub(crate) mod text;
pub(crate) mod widgets;
pub mod window;

pub use blenderui_sys as sys;

pub use color::*;
pub use element::*;
pub use events::*;
pub use geometry::{point, px, relative, rems, size, Bounds, Length, Pixels, Point, Rems, Size};
pub use paint::{Image, Painter};
pub use styled::{FontWeight, StyleBlock, Styled};
pub use window::*;

/// Whether `class` names also work as Tailwind CSS v4 utilities (`p-4`,
/// `md:hover:bg-sky-500/50` ...), compiled on first use. On by default;
/// turn it off when your own class names collide with Tailwind's.
pub fn set_tailwind(on: bool) {
    layout::TAILWIND.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Registers a TrueType/OpenType font under `family` (as bold and/or italic
/// when set), e.g. bundled with `include_bytes!`. Required on wasm, where
/// there are no system fonts; elsewhere it adds to them. Returns false when
/// the data isn't a font.
pub fn add_font(family: &str, bold: bool, italic: bool, data: &[u8]) -> bool {
    let fam = std::ffi::CString::new(family).unwrap_or_default();
    text::register_family(family);
    let flags = if bold { 1 } else { 0 } | if italic { 2 } else { 0 };
    let len = data.len().min(i32::MAX as usize) as i32;
    // SAFETY: the kit copies the bytes; both pointers are valid for the call.
    unsafe { sys::BUI_text_add_font(fam.as_ptr(), flags, data.as_ptr().cast(), len) != 0 }
}

/// How the GPU renderer draws text. `true`: Slug, glyph outlines evaluated
/// in the pixel shader (sharp at any size; the default with the `slug`
/// feature). `false`: a per-glyph bitmap atlas like Blender's (less GPU
/// work). Takes effect on the next frame; `BUI_SLUG=1`/`0` overrides.
pub fn set_slug(on: bool) {
    // SAFETY: sets a global flag.
    unsafe { sys::BUI_gpu_set_slug(on as std::os::raw::c_int) }
}

/// Whether this platform has native windows (Windows today); elsewhere
/// apps run headless or through a host such as the Tk fallback.
pub fn native_windows() -> bool {
    // SAFETY: a constant query.
    unsafe { sys::BK_window_supported() != 0 }
}

/// Whether GPU text is drawn with Slug (see [`set_slug`]).
pub fn slug() -> bool {
    // SAFETY: reads a global flag.
    unsafe { sys::BUI_gpu_slug() != 0 }
}

/// Everything an app needs: `use blenderui::prelude::*;`
pub mod prelude {
    pub use crate::{css, rsx, tw, view, use_effect};
    pub use crate::hooks::{
        create_signal, provide_context, signal, try_use_context, use_context, use_memo, use_signal, use_state,
        Children, Effect, Memo, ReadSignal, RwSignal, Signal, WriteSignal,
    };
    pub use blenderui_macros::{component, Props};
    pub use crate::color::*;
    pub use crate::components::{
        Button, Checkbox, ColorSwatch, Dropdown, NumberField, NumberSlider, ProgressBar, Radio, TextArea, TextField,
    };
    pub use crate::element::*;
    pub use crate::events::*;
    pub use crate::geometry::{point, px, relative, rems, size, Bounds, Length, Pixels, Point, Rems, Size};
    pub use crate::html::*;
    pub use crate::paint::{Image, Painter};
    pub use crate::styled::{FontWeight, StyleBlock, Styled};
    pub use crate::window::*;
}
