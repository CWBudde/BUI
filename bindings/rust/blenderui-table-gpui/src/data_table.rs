//! [`DataTable`]: the element that shows a [`TableState`] (gpui-component's
//! `DataTable::new(&state)`).
//!
//! A frame builds **two** elements however many rows or columns there are:
//! a focus-tracking wrapper that takes key input, and [`TableElement`], a
//! custom gpui `Element` that lays out as one box, syncs the table with its
//! size in `prepaint`, and in `paint` draws the visible cells with
//! `Window::paint_quad` and shaped lines (via `blenderui_table::paint_table`)
//! and registers the pointer listeners. Cost per frame is O(visible cells).
//! Cells that need rich content opt in through
//! [`TableDelegate::render_td`](crate::TableDelegate::render_td).

use std::rc::Rc;

use blenderui_table::{
    paint_table, with_scratch, Button, Drag, Hit, Key, Mods, Palette, Point as CorePoint, Size, TableEvent, TableOptions,
};
use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, CursorStyle, DispatchPhase, Element, ElementId, Entity,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement as _, IntoElement, KeyDownEvent,
    LayoutId, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels,
    RenderOnce, ScrollEvent, Style, Styled as _, Window, div, point, px, relative, size,
};

use crate::delegate::TableDelegate;
use blenderui_paint_gpui::GpuiSink;
use crate::state::TableState;

type Listener = Box<dyn Fn(&TableEvent, &mut Window, &mut App)>;

/// The palette a table paints with when none is given: a global the app may set.
#[derive(Clone, Copy, Debug)]
pub struct TablePalette(pub Palette);

impl gpui::Global for TablePalette {}

/// A table element.
///
/// ```ignore
/// let table = cx.new(|cx| TableState::new(MyDelegate::new(), window, cx));
/// // in render:
/// div().size_full().child(DataTable::new(&table).stripe(true).on_select(|row, _, _| println!("{row}")))
/// ```
#[derive(IntoElement)]
pub struct DataTable<D: TableDelegate> {
    state: Entity<TableState<D>>,
    stripe: Option<bool>,
    bordered: Option<bool>,
    size: Option<Size>,
    header_h: Option<f32>,
    fixed_border: Option<bool>,
    scrollbars: Option<(bool, bool)>,
    unit: Option<f32>,
    text_size: Option<f32>,
    palette: Option<Palette>,
    listeners: Vec<Listener>,
}

impl<D: TableDelegate> DataTable<D> {
    #[must_use]
    pub fn new(state: &Entity<TableState<D>>) -> Self {
        Self {
            state: state.clone(),
            stripe: None,
            bordered: None,
            size: None,
            header_h: None,
            fixed_border: None,
            scrollbars: None,
            unit: None,
            text_size: None,
            palette: None,
            listeners: Vec::new(),
        }
    }

    /// Tint every second row (Blender's `draw_alternating_row_overlay`).
    #[must_use]
    pub const fn stripe(mut self, on: bool) -> Self {
        self.stripe = Some(on);
        self
    }
    #[must_use]
    pub const fn bordered(mut self, on: bool) -> Self {
        self.bordered = Some(on);
        self
    }
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
    #[must_use]
    pub fn header_row_height(mut self, h: impl Into<Pixels>) -> Self {
        self.header_h = Some(f32::from(h.into()));
        self
    }
    #[must_use]
    pub const fn fixed_cols_border(mut self, on: bool) -> Self {
        self.fixed_border = Some(on);
        self
    }
    #[must_use]
    pub const fn scrollbar_visible(mut self, vertical: bool, horizontal: bool) -> Self {
        self.scrollbars = Some((vertical, horizontal));
        self
    }
    /// The UI unit in logical pixels: a medium row is one unit tall and the
    /// text is 0.6 of it (Blender's `UI_UNIT_Y`, 20 there; 24 by default here).
    #[must_use]
    pub fn unit(mut self, unit: impl Into<Pixels>) -> Self {
        self.unit = Some(f32::from(unit.into()));
        self
    }
    /// The text size (default: 0.6 of the unit).
    #[must_use]
    pub fn text_size(mut self, size: impl Into<Pixels>) -> Self {
        self.text_size = Some(f32::from(size.into()));
        self
    }
    /// The colors to paint with (default: the [`TablePalette`] global, else Blender's dark theme).
    #[must_use]
    pub const fn palette(mut self, palette: Palette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// Every table event.
    #[must_use]
    pub fn on_event(mut self, f: impl Fn(&TableEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.push(Box::new(f));
        self
    }
    /// A row was selected (source row index).
    #[must_use]
    pub fn on_select(self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_event(move |e, w, cx| {
            if let TableEvent::SelectRow(r) = e {
                f(*r, w, cx);
            }
        })
    }
    /// A row was double clicked.
    #[must_use]
    pub fn on_row_double_click(self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_event(move |e, w, cx| {
            if let TableEvent::DoubleClickedRow(r) = e {
                f(*r, w, cx);
            }
        })
    }
    /// A header click sorted a column (source column index).
    #[must_use]
    pub fn on_sort(self, f: impl Fn(usize, blenderui_table::ColumnSort, &mut Window, &mut App) + 'static) -> Self {
        self.on_event(move |e, w, cx| {
            if let TableEvent::Sorted(c, s) = e {
                f(*c, *s, w, cx);
            }
        })
    }
    /// A column was resized (all widths, view order).
    #[must_use]
    pub fn on_column_resize(self, f: impl Fn(&[Pixels], &mut Window, &mut App) + 'static) -> Self {
        self.on_event(move |e, w, cx| {
            if let TableEvent::ColumnWidthsChanged(v) = e {
                let widths: Vec<Pixels> = v.iter().map(|&w| px(w)).collect();
                f(&widths, w, cx);
            }
        })
    }
    /// A right click on a row (`None`: on empty space).
    #[must_use]
    pub fn on_context_menu(self, f: impl Fn(Option<usize>, &mut Window, &mut App) + 'static) -> Self {
        self.on_event(move |e, w, cx| {
            if let TableEvent::RightClickedRow(r) = e {
                f(*r, w, cx);
            }
        })
    }
}

/// Hand the state's pending events to the listeners.
fn dispatch<D: TableDelegate>(state: &Entity<TableState<D>>, ls: &[Listener], window: &mut Window, cx: &mut App) {
    let events = state.update(cx, |s, _| s.take_events());
    for e in &events {
        for l in ls {
            l(e, window, cx);
        }
    }
}

const fn mods(m: Modifiers) -> Mods {
    // the platform key (command) is the control key of table shortcuts on macOS
    Mods { shift: m.shift, control: m.control || m.platform, alt: m.alt }
}

fn local(pos: gpui::Point<Pixels>, origin: gpui::Point<Pixels>) -> CorePoint {
    CorePoint { x: f32::from(pos.x - origin.x), y: f32::from(pos.y - origin.y) }
}

impl<D: TableDelegate> RenderOnce for DataTable<D> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { state, stripe, bordered, size, header_h, fixed_border, scrollbars, unit, text_size, palette, listeners } = self;
        let focus = gpui::Focusable::focus_handle(state.read(cx), cx);
        let ls: Rc<[Listener]> = listeners.into();
        let key_state = state.clone();
        let key_ls = ls.clone();
        let opts = TableOptionsOverride { stripe, bordered, size, header_h, text_size, fixed_border, scrollbars };
        div()
            .id(("blenderui-data-table", state.entity_id()))
            .size_full()
            .relative()
            .overflow_hidden()
            .track_focus(&focus)
            .key_context("DataTable")
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                let key = Key::parse(&e.keystroke.key);
                let m = mods(e.keystroke.modifiers);
                let handled = key_state.update(cx, |s, cx| s.key_down(key, m, window, cx));
                dispatch(&key_state, &key_ls, window, cx);
                if handled {
                    cx.stop_propagation();
                }
            })
            .child(TableElement { state, opts, unit, palette, listeners: ls })
    }
}

/// The options a [`DataTable`] builder overrides on the state's own.
#[derive(Clone, Copy)]
struct TableOptionsOverride {
    stripe: Option<bool>,
    bordered: Option<bool>,
    size: Option<Size>,
    header_h: Option<f32>,
    text_size: Option<f32>,
    fixed_border: Option<bool>,
    scrollbars: Option<(bool, bool)>,
}

impl TableOptionsOverride {
    fn apply(self, mut o: TableOptions) -> TableOptions {
        o.stripe = self.stripe.unwrap_or(o.stripe);
        o.bordered = self.bordered.unwrap_or(o.bordered);
        o.size = self.size.unwrap_or(o.size);
        o.fixed_cols_border = self.fixed_border.unwrap_or(o.fixed_cols_border);
        o.scrollbars = self.scrollbars.unwrap_or(o.scrollbars);
        o.header_row_height = self.header_h.or(o.header_row_height);
        o.text_size = self.text_size.or(o.text_size);
        o
    }
}

/// The default UI unit (a medium row's height) in logical pixels.
const DEFAULT_UNIT: f32 = 24.0;

/// A rich cell built for this frame.
struct CustomCell {
    element: AnyElement,
    clip: Bounds<Pixels>,
}

/// What `prepaint` found for `paint`.
struct Prepaint {
    hitbox: Hitbox,
    custom: Vec<CustomCell>,
}

/// The custom element: see the module docs.
struct TableElement<D: TableDelegate> {
    state: Entity<TableState<D>>,
    opts: TableOptionsOverride,
    unit: Option<f32>,
    palette: Option<Palette>,
    listeners: Rc<[Listener]>,
}

impl<D: TableDelegate> IntoElement for TableElement<D> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl<D: TableDelegate> Element for TableElement<D> {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        &mut (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Prepaint {
        let opts = self.opts;
        let unit = self.unit.unwrap_or(DEFAULT_UNIT);
        let scale = window.scale_factor();
        let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let has_custom = self.state.update(cx, |s, cx| {
            s.opts = opts.apply(s.opts);
            if (s.text_scale - scale).abs() > f32::EPSILON {
                s.text.clear();
                s.text_scale = scale;
            }
            s.sync(w, h, unit, scale, window, cx);
            let want = {
                let v = s.visible_range();
                2 * (v.rows().len() + 1) * (v.cols().len() + 2) + 256
            };
            s.text.ensure_capacity(want);
            s.delegate().has_custom_cells()
        });
        let custom = if has_custom { self.build_custom_cells(bounds, window, cx) } else { Vec::new() };
        Prepaint { hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal), custom }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        &mut (): &mut (),
        pre: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let palette = self.palette.unwrap_or_else(|| cx.try_global::<TablePalette>().map_or_else(Palette::blender_dark, |p| p.0));
        let font = window.text_style().font();
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.state.update(cx, |s, cx| {
                s.text.next_generation();
                let st = &mut *s;
                let (core, text) = (&st.core, &mut st.text);
                let mut sink = GpuiSink::new(window, cx, bounds.origin, text, &font, palette.text);
                with_scratch(|scratch| {
                    paint_table(core, core.delegate(), &palette, core.delegate().empty_text(), scratch, &mut sink);
                });
            });
            for c in &mut pre.custom {
                window.with_content_mask(Some(ContentMask { bounds: c.clip }), |window| c.element.paint(window, cx));
            }
        });
        self.set_cursor(bounds, &pre.hitbox, window, cx);
        self.register_input(bounds, &pre.hitbox, window);
    }
}

impl<D: TableDelegate> TableElement<D> {
    /// Build the elements of the visible custom cells (the escape hatch).
    fn build_custom_cells(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Vec<CustomCell> {
        let mut out = Vec::new();
        let (g, rows, cols) = {
            let s = self.state.read(cx);
            (s.geo(), s.visible_range().rows().clone(), s.visible_range().cols().clone())
        };
        let pinned = self.state.read(cx).layout().pinned();
        for v in (0..pinned).chain(cols) {
            for r in rows.clone() {
                let (row, col, x, y, w) = {
                    let s = self.state.read(cx);
                    (s.row_at(r), s.layout().src(v), s.col_x(&g, v), s.row_top(&g, r), s.layout().width(v))
                };
                if !self.state.read(cx).delegate().is_custom_cell(row, col) {
                    continue;
                }
                let mut element = self.state.update(cx, |s, cx| s.delegate_mut().render_td(row, col, window, cx));
                let origin = bounds.origin + point(px(x), px(y));
                let cell = size(px(w), px(g.row_h));
                element.prepaint_as_root(origin, size(AvailableSpace::Definite(cell.width), AvailableSpace::Definite(cell.height)), window, cx);
                out.push(CustomCell { element, clip: Bounds { origin, size: cell } });
            }
        }
        out
    }

    /// The column-resize cursor over a header border, or while resizing.
    fn set_cursor(&self, bounds: Bounds<Pixels>, hitbox: &Hitbox, window: &mut Window, cx: &mut App) {
        let s = self.state.read(cx);
        let resizing = matches!(s.drag(), Drag::Resize { .. });
        let hot = hitbox.is_hovered(window) && matches!(s.hit(local(window.mouse_position(), bounds.origin)), Hit::Border(_));
        if resizing || hot {
            window.set_cursor_style(CursorStyle::ResizeLeftRight, hitbox);
        }
    }

    /// Pointer and wheel listeners, for this frame.
    fn register_input(&self, bounds: Bounds<Pixels>, hitbox: &Hitbox, window: &mut Window) {
        let origin = bounds.origin;
        for button in [MouseButton::Left, MouseButton::Right] {
            let (state, ls, hitbox) = (self.state.clone(), self.listeners.clone(), hitbox.clone());
            window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || e.button != button || !hitbox.is_hovered(window) {
                    return;
                }
                let focus = gpui::Focusable::focus_handle(state.read(cx), cx);
                window.focus(&focus, cx);
                let b = if button == MouseButton::Left { Button::Left } else { Button::Right };
                let p = local(e.position, origin);
                state.update(cx, |s, cx| s.pointer_down(p, b, mods(e.modifiers), e.click_count, window, cx));
                dispatch(&state, &ls, window, cx);
            });
        }
        let (state, ls, hb) = (self.state.clone(), self.listeners.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            let dragging = state.read(cx).dragging();
            if !dragging && !hb.is_hovered(window) {
                return;
            }
            let p = local(e.position, origin);
            state.update(cx, |s, cx| {
                if e.pressed_button.is_none() && dragging {
                    s.pointer_up(window, cx);
                } else if !s.pointer_move(p, window, cx) {
                    // the cursor changes over a border without any state change
                    let over = matches!(s.hit(p), Hit::Border(_));
                    if over != s.over_border {
                        s.over_border = over;
                        window.refresh();
                    }
                }
            });
            dispatch(&state, &ls, window, cx);
        });
        let (state, ls) = (self.state.clone(), self.listeners.clone());
        window.on_mouse_event(move |e: &MouseUpEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || e.button != MouseButton::Left || !state.read(cx).dragging() {
                return;
            }
            state.update(cx, |s, cx| s.pointer_up(window, cx));
            dispatch(&state, &ls, window, cx);
        });
        let (state, hb) = (self.state.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &ScrollEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || !hb.is_hovered(window) {
                return;
            }
            let row_h = px(state.read(cx).geo().row_h);
            let d = e.delta.pixel_delta(row_h);
            // content follows the wheel: a negative delta scrolls down
            let (dx, dy) = (-f64::from(f32::from(d.x)), -f64::from(f32::from(d.y)));
            let sideways = e.modifiers.shift && dx == 0.0;
            let (dx, dy) = if sideways { (dy, 0.0) } else { (dx, dy) };
            let moved = state.update(cx, |s, _| s.scroll_by_px(dx, dy, window));
            if moved {
                cx.stop_propagation();
            }
        });
    }
}
