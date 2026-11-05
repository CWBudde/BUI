//! [`VariableList`]: the element that shows a [`ListState`].
//!
//! A frame builds **two** elements however many items there are: a
//! focus-tracking wrapper that takes key input, and [`ListElement`], a custom
//! gpui `Element` that lays out as one box, lays the list's frame out in
//! `prepaint` (measuring what is unmeasured, with the shaper) and in `paint`
//! draws the placed items with `Window::paint_quad` and shaped lines (via
//! `blenderui_vlist::List::paint`) and registers the pointer listeners.

use std::rc::Rc;

use blenderui_vlist::{ChatStyle, Hit, ItemPainter, ItemSource, List, Measurer, VecSource};
use gpui::{
    App, Bounds, ContentMask, DispatchPhase, Element, ElementId, Entity, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement as _, IntoElement, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement as _, Pixels, RenderOnce, ScrollDelta, ScrollEvent, Style, Styled as _,
    Window, div, px, relative,
};

use crate::state::ListState;

type Listener<S, P> = Rc<dyn Fn(usize, &mut List<S, P>)>;

/// A line of text in px, for keyboard and wheel steps.
const LINE_PX: f64 = 19.0;

/// A variable-height list over `state`.
#[derive(IntoElement)]
pub struct VariableList<S: ItemSource + 'static, P: Measurer + ItemPainter + 'static> {
    state: Entity<ListState<S, P>>,
    on_item_click: Option<Listener<S, P>>,
}

/// The element for a list state.
pub fn variable_list<S: ItemSource, P: Measurer + ItemPainter>(state: &Entity<ListState<S, P>>) -> VariableList<S, P> {
    VariableList { state: state.clone(), on_item_click: None }
}

/// A chat: a [`VariableList`] over a chat where a click selects a whole message.
#[must_use]
pub fn chat_list(state: &Entity<ListState<VecSource, ChatStyle>>) -> VariableList<VecSource, ChatStyle> {
    variable_list(state).on_item_click(|i, list| list.toggle_selected(i))
}

impl<S: ItemSource, P: Measurer + ItemPainter> VariableList<S, P> {
    /// An item was clicked (item index). The closure may change the list.
    #[must_use]
    pub fn on_item_click(mut self, f: impl Fn(usize, &mut List<S, P>) + 'static) -> Self {
        self.on_item_click = Some(Rc::new(f));
        self
    }
}

impl<S: ItemSource, P: Measurer + ItemPainter> RenderOnce for VariableList<S, P> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { state, on_item_click } = self;
        let focus = gpui::Focusable::focus_handle(state.read(cx), cx);
        let key_state = state.clone();
        div()
            .id(("blenderui-variable-list", state.entity_id()))
            .size_full()
            .relative()
            .overflow_hidden()
            .track_focus(&focus)
            .key_context("VariableList")
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                let handled = key_state.update(cx, |s, _| s.key(&e.keystroke.key, LINE_PX));
                if handled {
                    window.refresh();
                    cx.stop_propagation();
                }
            })
            .child(ListElement { state, on_item_click })
    }
}

/// What `prepaint` found for `paint`.
struct Prepaint {
    hitbox: Hitbox,
}

/// The custom element: see the module docs.
struct ListElement<S: ItemSource + 'static, P: Measurer + ItemPainter + 'static> {
    state: Entity<ListState<S, P>>,
    on_item_click: Option<Listener<S, P>>,
}

impl<S: ItemSource, P: Measurer + ItemPainter> IntoElement for ListElement<S, P> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl<S: ItemSource, P: Measurer + ItemPainter> Element for ListElement<S, P> {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
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
        self.state.update(cx, |s, cx| s.sync(bounds, window, cx));
        Prepaint { hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal) }
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
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.state.update(cx, |s, cx| s.paint_into(window, cx, bounds.origin));
        });
        self.register_input(bounds, &pre.hitbox, window);
    }
}

impl<S: ItemSource, P: Measurer + ItemPainter> ListElement<S, P> {
    /// Pointer and wheel listeners, for this frame.
    fn register_input(&self, bounds: Bounds<Pixels>, hitbox: &Hitbox, window: &mut Window) {
        let origin = bounds.origin;
        let (state, click, hb) = (self.state.clone(), self.on_item_click.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || e.button != MouseButton::Left || !hb.is_hovered(window) {
                return;
            }
            let focus = gpui::Focusable::focus_handle(state.read(cx), cx);
            window.focus(&focus, cx);
            let (x, y) = (f32::from(e.position.x - origin.x), f32::from(e.position.y - origin.y));
            state.update(cx, |s, _| {
                if let Hit::Item(i) = s.pointer_down(x, y)
                    && let Some(f) = &click
                {
                    f(i, s);
                }
            });
            window.refresh();
        });
        let (state, hb) = (self.state.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || !(state.read(cx).dragging() || hb.is_hovered(window)) {
                return;
            }
            let y = f32::from(e.position.y - origin.y);
            let moved = state.update(cx, |s, _| if e.pressed_button.is_none() { s.pointer_up(); false } else { s.pointer_move(y) });
            if moved {
                window.refresh();
            }
        });
        let state = self.state.clone();
        window.on_mouse_event(move |e: &MouseUpEvent, phase, _, cx| {
            if phase == DispatchPhase::Bubble && e.button == MouseButton::Left {
                state.update(cx, |s, _| s.pointer_up());
            }
        });
        let (state, hb) = (self.state.clone(), hitbox.clone());
        window.on_mouse_event(move |e: &ScrollEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || !hb.is_hovered(window) {
                return;
            }
            let precise = matches!(e.delta, ScrollDelta::Pixels(_));
            let d = e.delta.pixel_delta(px(LINE_PX as f32));
            // content follows the wheel: a negative delta scrolls down
            let dy = -f64::from(f32::from(d.y));
            state.update(cx, |s, _| s.view.wheel(dy, precise));
            window.request_animation_frame();
            cx.stop_propagation();
        });
    }
}
