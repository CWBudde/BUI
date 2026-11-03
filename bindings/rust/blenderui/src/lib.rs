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

pub mod color;
pub mod css;
pub mod element;
pub mod events;
pub mod geometry;
pub mod html;
pub(crate) mod layout;
pub mod paint;
pub(crate) mod prof;
pub mod style;
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

/// Everything an app needs: `use blenderui::prelude::*;`
pub mod prelude {
    pub use crate::css;
    pub use crate::color::*;
    pub use crate::element::*;
    pub use crate::events::*;
    pub use crate::geometry::{point, px, relative, rems, size, Bounds, Length, Pixels, Point, Rems, Size};
    pub use crate::html::*;
    pub use crate::paint::{Image, Painter};
    pub use crate::styled::{FontWeight, StyleBlock, Styled};
    pub use crate::window::*;
}
