//! Elements: one [`Div`] type for every HTML tag, built gpui-style.

use std::borrow::Cow;
use std::rc::Rc;

use crate::css::Decl;
pub use crate::styled::Styled;
use crate::events::*;
use crate::window::{App, Window};

/// Shared, cheap-to-clone text (gpui's `SharedString`).
pub type SharedString = Cow<'static, str>;

/// An element identifier (gpui's `ElementId`): keeps widget and scroll
/// state across renders.
pub type ElementId = SharedString;

pub(crate) type Listener<E> = Rc<dyn Fn(&E, &mut Window, &mut App)>;

/// Custom painting (gpui's `canvas()` element).
pub(crate) type PaintFn = Rc<dyn Fn(crate::geometry::Bounds<f32>, &mut crate::paint::Painter)>;

#[derive(Clone, Default)]
pub(crate) struct Listeners {
    pub click: Vec<Listener<ClickEvent>>,
    pub mouse_down: Vec<(Option<MouseButton>, Listener<MouseDownEvent>)>,
    pub mouse_up: Vec<(Option<MouseButton>, Listener<MouseUpEvent>)>,
    pub mouse_move: Vec<Listener<MouseMoveEvent>>,
    pub hover: Vec<Listener<bool>>,
    pub scroll: Vec<Listener<ScrollWheelEvent>>,
    pub key_down: Vec<Listener<KeyDownEvent>>,
    pub key_up: Vec<Listener<KeyUpEvent>>,
    pub change: Vec<Listener<ChangeEvent>>,
    pub input: Vec<Listener<ChangeEvent>>,
}

/// An element: a tag, its attributes, styles, children and listeners.
/// `div()`, `span()`, `button()` ... all build one of these.
#[derive(Clone)]
pub struct Div {
    pub(crate) tag: &'static str,
    pub(crate) id: Option<ElementId>,
    pub(crate) attrs: Vec<(SharedString, SharedString)>,
    pub(crate) decls: Vec<Decl>,
    pub(crate) hover: Vec<Decl>,
    pub(crate) active: Vec<Decl>,
    pub(crate) focus: Vec<Decl>,
    pub(crate) children: Vec<AnyElement>,
    pub(crate) listeners: Listeners,
    pub(crate) paint: Option<PaintFn>,
    pub(crate) image: Option<Rc<crate::paint::Image>>,
}

/// Anything that can be a child: an element or a text run.
#[derive(Clone)]
pub enum AnyElement {
    Element(Box<Div>),
    Text(SharedString),
}

/// Convert into an element (gpui's `IntoElement`).
pub trait IntoElement {
    fn into_any_element(self) -> AnyElement;
}

impl IntoElement for Div {
    fn into_any_element(self) -> AnyElement {
        AnyElement::Element(Box::new(self))
    }
}
impl IntoElement for AnyElement {
    fn into_any_element(self) -> AnyElement {
        self
    }
}
impl IntoElement for &'static str {
    fn into_any_element(self) -> AnyElement {
        AnyElement::Text(Cow::Borrowed(self))
    }
}
impl IntoElement for String {
    fn into_any_element(self) -> AnyElement {
        AnyElement::Text(Cow::Owned(self))
    }
}
impl IntoElement for Cow<'static, str> {
    fn into_any_element(self) -> AnyElement {
        AnyElement::Text(self)
    }
}

impl Div {
    pub(crate) fn new(tag: &'static str) -> Div {
        Div {
            tag,
            id: None,
            attrs: Vec::new(),
            decls: Vec::new(),
            hover: Vec::new(),
            active: Vec::new(),
            focus: Vec::new(),
            children: Vec::new(),
            listeners: Listeners::default(),
            paint: None,
            image: None,
        }
    }

    /// The HTML tag name.
    pub fn tag(&self) -> &'static str {
        self.tag
    }

    /// Give the element an identity (keeps state such as scroll position and
    /// form values across renders).
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        let id = id.into();
        self.attrs.push(("id".into(), id.clone()));
        self.id = Some(id);
        self
    }

    /// Set an HTML attribute (`href`, `type`, `value`, `placeholder` ...).
    pub fn attr(mut self, name: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        let name = name.into();
        self.attrs.retain(|(n, _)| *n != name);
        self.attrs.push((name, value.into()));
        self
    }

    /// Add CSS classes (matched by stylesheets).
    pub fn class(mut self, classes: impl Into<SharedString>) -> Self {
        let add = classes.into();
        match self.attrs.iter_mut().find(|(n, _)| n == "class") {
            Some((_, v)) => *v = format!("{v} {add}").into(),
            None => self.attrs.push(("class".into(), add)),
        }
        self
    }

    pub(crate) fn get_attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v.as_ref())
    }

    // -- common HTML attributes, typed ---------------------------------------------
    pub fn href(self, url: impl Into<SharedString>) -> Self {
        self.attr("href", url)
    }
    pub fn value(self, v: impl Into<SharedString>) -> Self {
        self.attr("value", v)
    }
    pub fn placeholder(self, v: impl Into<SharedString>) -> Self {
        self.attr("placeholder", v)
    }
    pub fn input_type(self, t: impl Into<SharedString>) -> Self {
        self.attr("type", t)
    }
    pub fn name(self, n: impl Into<SharedString>) -> Self {
        self.attr("name", n)
    }
    pub fn checked(self, on: bool) -> Self {
        if on { self.attr("checked", "") } else { self.remove_attr("checked") }
    }
    pub fn disabled(self, on: bool) -> Self {
        if on { self.attr("disabled", "") } else { self.remove_attr("disabled") }
    }
    pub fn selected(self, on: bool) -> Self {
        if on { self.attr("selected", "") } else { self.remove_attr("selected") }
    }
    pub fn open(self, on: bool) -> Self {
        if on { self.attr("open", "") } else { self.remove_attr("open") }
    }
    pub fn min(self, v: f64) -> Self {
        self.attr("min", v.to_string())
    }
    pub fn max(self, v: f64) -> Self {
        self.attr("max", v.to_string())
    }
    pub fn step(self, v: f64) -> Self {
        self.attr("step", v.to_string())
    }
    pub fn src(self, path: impl Into<SharedString>) -> Self {
        self.attr("src", path)
    }
    pub fn alt(self, text: impl Into<SharedString>) -> Self {
        self.attr("alt", text)
    }
    pub fn title(self, text: impl Into<SharedString>) -> Self {
        self.attr("title", text)
    }
    pub fn colspan(self, n: u32) -> Self {
        self.attr("colspan", n.to_string())
    }

    fn remove_attr(mut self, name: &str) -> Self {
        self.attrs.retain(|(n, _)| n != name);
        self
    }

    /// Image pixels for `<img>`: RGBA, `width * height * 4` bytes.
    pub fn image_rgba(mut self, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        self.image = Some(Rc::new(crate::paint::Image { width, height, rgba }));
        self
    }

    // -- events ----------------------------------------------------------------------------
    /// Click (press and release on this element); bubbles to ancestors.
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.click.push(Rc::new(f));
        self
    }
    pub fn on_mouse_down(mut self, button: MouseButton,
                         f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.mouse_down.push((Some(button), Rc::new(f)));
        self
    }
    pub fn on_any_mouse_down(mut self,
                             f: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.mouse_down.push((None, Rc::new(f)));
        self
    }
    pub fn on_mouse_up(mut self, button: MouseButton,
                       f: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.mouse_up.push((Some(button), Rc::new(f)));
        self
    }
    pub fn on_mouse_move(mut self,
                         f: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.mouse_move.push(Rc::new(f));
        self
    }
    /// Called with `true` when the pointer enters, `false` when it leaves.
    pub fn on_hover(mut self, f: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.hover.push(Rc::new(f));
        self
    }
    pub fn on_scroll_wheel(mut self,
                           f: impl Fn(&ScrollWheelEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.scroll.push(Rc::new(f));
        self
    }
    pub fn on_key_down(mut self, f: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.key_down.push(Rc::new(f));
        self
    }
    pub fn on_key_up(mut self, f: impl Fn(&KeyUpEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.key_up.push(Rc::new(f));
        self
    }
    /// Form controls: the committed value changed (`change` in HTML).
    pub fn on_change(mut self, f: impl Fn(&ChangeEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.change.push(Rc::new(f));
        self
    }
    /// Form controls: every edit (`input` in HTML).
    pub fn on_input(mut self, f: impl Fn(&ChangeEvent, &mut Window, &mut App) + 'static) -> Self {
        self.listeners.input.push(Rc::new(f));
        self
    }

    // -- state styles (gpui's .hover(|s| ...), CSS :hover / :active / :focus) ----------------
    pub fn hover(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.hover.extend(f(StyleRefinement::default()).decls);
        self
    }
    pub fn active(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.active.extend(f(StyleRefinement::default()).decls);
        self
    }
    pub fn focus(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.focus.extend(f(StyleRefinement::default()).decls);
        self
    }
}

/// A set of style declarations (the argument of `.hover(|s| ...)`).
#[derive(Clone, Default)]
pub struct StyleRefinement {
    pub(crate) decls: Vec<Decl>,
}

impl Styled for Div {
    fn decls_mut(&mut self) -> &mut Vec<Decl> {
        &mut self.decls
    }
}
impl Styled for StyleRefinement {
    fn decls_mut(&mut self) -> &mut Vec<Decl> {
        &mut self.decls
    }
}

/// Children (gpui's `ParentElement`).
pub trait ParentElement: Sized {
    fn children_mut(&mut self) -> &mut Vec<AnyElement>;

    fn child(mut self, child: impl IntoElement) -> Self {
        self.children_mut().push(child.into_any_element());
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children_mut().extend(children.into_iter().map(IntoElement::into_any_element));
        self
    }
}

impl ParentElement for Div {
    fn children_mut(&mut self) -> &mut Vec<AnyElement> {
        &mut self.children
    }
}

/// Conditional building (gpui's `FluentBuilder`).
pub trait FluentBuilder: Sized {
    fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        if condition { then(self) } else { self }
    }
    fn when_some<T>(self, option: Option<T>, then: impl FnOnce(Self, T) -> Self) -> Self {
        match option {
            Some(v) => then(self, v),
            None => self,
        }
    }
    fn when_none<T>(self, option: &Option<T>, then: impl FnOnce(Self) -> Self) -> Self {
        if option.is_none() { then(self) } else { self }
    }
    fn map<U>(self, f: impl FnOnce(Self) -> U) -> U {
        f(self)
    }
}

impl FluentBuilder for Div {}
impl FluentBuilder for StyleRefinement {}

/// gpui's `canvas(prepaint, paint)`: an element that paints itself.
pub fn canvas(paint: impl Fn(crate::geometry::Bounds<f32>, &mut crate::paint::Painter) + 'static)
    -> Div {
    let mut d = Div::new("canvas");
    d.paint = Some(Rc::new(paint));
    d
}
