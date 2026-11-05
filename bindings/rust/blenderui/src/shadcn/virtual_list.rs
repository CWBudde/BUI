//! A virtual list in shadcn's component style, with TanStack Virtual's names.
//! shadcn/ui has no virtual list, so this follows `@tanstack/react-virtual`:
//!
//! | TanStack Virtual | here |
//! |---|---|
//! | `count` | `count` ([`VirtualList`]); a [`ChatList`] has its `messages` |
//! | `estimateSize` | `estimate_size` (px of a one-line item before it is measured) |
//! | `overscan` | `overscan` (items beyond the viewport the source is asked for) |
//! | `getItemKey` | `get_item_key` (ascending keys) |
//! | `scrollToIndex(index, { align })` | [`ListHandle::scroll_to_index`] |
//! | `scrollToOffset(offset)` | [`ListHandle::scroll_to_offset`] |
//! | `measureElement` | [`ListHandle::measure_element`] (a height you know) |
//! | `onChange` / `range` | `on_range_change`, [`ListHandle::range`] |
//! | `getTotalSize()`, `scrollOffset` | [`ListHandle::view_info`] |
//!
//! The rows are not elements: the engine draws visible rows' wrapped text
//! straight from the source ([`blenderui_vlist`]), so a list of ten million
//! items costs what a screenful does. An item is therefore text
//! (`item(index, out)` writes it), not JSX.

use std::rc::Rc;
use std::sync::Arc;

pub use blenderui_vlist::{Align, Message};
pub use crate::vlist::ViewInfo;
use blenderui_vlist::{ChatList as ChatCore, CountSource, ItemSource, List, Options, RowList, RowStyle};

use crate::css::Decl;
use crate::hooks::{use_signal, Complete, Signal};
use crate::prelude::*;
use crate::vlist::{chat_list, theme_palette, variable_list, ListState};

/// Writes the text of item `index` into the buffer (`item`).
pub type ItemText = Box<dyn Fn(usize, &mut String)>;

/// A change to make to a list (what `scrollToIndex` and friends send).
#[derive(Clone, Debug, PartialEq)]
pub enum ListCommand {
    ScrollToIndex(usize, Align),
    ScrollToOffset(f64),
    ScrollToEnd,
    /// A height in px that is known for sure (`measureElement`).
    MeasureElement(usize, f64),
    /// A new message at the end ([`ChatList`] only).
    Push(Message),
    /// Replace the text of message `n` ([`ChatList`] only).
    Edit(usize, String),
    /// The items changed without the count changing: measure again.
    Invalidate,
}

/// A list the commands apply to.
trait Drive {
    fn apply(&mut self, cmd: ListCommand);
}

fn common<S: ItemSource, P: blenderui_vlist::Measurer + blenderui_vlist::ItemPainter + Clone>(list: &mut List<S, P>, cmd: ListCommand) {
    match cmd {
        ListCommand::ScrollToIndex(i, align) => list.view.jump_to(i, align),
        ListCommand::ScrollToOffset(px) => list.view.scroll_to_offset(px),
        ListCommand::ScrollToEnd => list.view.scroll_to_end(),
        ListCommand::MeasureElement(i, px) => list.view.set_known_height(i, px),
        ListCommand::Invalidate => {
            let style = list.style.clone();
            list.restyle(style);
        }
        ListCommand::Push(_) | ListCommand::Edit(..) => {}
    }
}

impl Drive for RowList {
    fn apply(&mut self, cmd: ListCommand) {
        common(self, cmd);
    }
}

impl Drive for ChatCore {
    fn apply(&mut self, cmd: ListCommand) {
        match cmd {
            ListCommand::Push(m) => self.push(m),
            ListCommand::Edit(i, text) => self.edit(i, &text),
            other => common(self, other),
        }
    }
}

/// The list behind a handle.
trait ListOps {
    fn run(&self, cmd: ListCommand, cx: &mut App);
    fn info(&self, cx: &App) -> ViewInfo;
}

struct ListLink<S: ItemSource + 'static, P: blenderui_vlist::Measurer + blenderui_vlist::ItemPainter + 'static> {
    state: Entity<ListState<S, P>>,
}

impl<S: ItemSource + 'static, P: blenderui_vlist::Measurer + blenderui_vlist::ItemPainter + 'static> ListOps for ListLink<S, P>
where
    List<S, P>: Drive,
{
    fn run(&self, cmd: ListCommand, cx: &mut App) {
        self.state.update(cx, |s, _| s.apply(cmd));
    }

    fn info(&self, cx: &App) -> ViewInfo {
        self.state.read(cx).view_info()
    }
}

/// `useVirtualizer`'s result: a `Copy` handle to a list.
///
/// ```ignore
/// let list = use_virtual_list();
/// rsx! { <VirtualList list={list} count={1_000_000} item={|i: usize, out: &mut String| out.push_str(&i.to_string())} /> }
/// // in a handler:
/// list.scroll_to_index(500, Align::Center, cx);
/// ```
#[derive(Clone, Copy)]
pub struct ListHandle {
    view: Signal<ViewInfo>,
    ops: Signal<Option<Rc<dyn ListOps>>>,
}

/// A list handle for the calling component (the same one on every render).
#[track_caller]
#[must_use]
pub fn use_virtual_list() -> ListHandle {
    ListHandle { view: use_signal(ViewInfo::default()), ops: use_signal(None) }
}

impl ListHandle {
    /// Send `cmd` to the list (nothing happens before its first frame).
    pub fn dispatch(self, cmd: ListCommand, cx: &mut App) {
        if let Some(ops) = self.ops.get() {
            ops.run(cmd, cx);
            cx.notify_all();
        }
    }

    /// `scrollToIndex(index, { align })`.
    pub fn scroll_to_index(self, index: usize, align: Align, cx: &mut App) {
        self.dispatch(ListCommand::ScrollToIndex(index, align), cx);
    }

    /// `scrollToOffset(offset)`, px from the top.
    pub fn scroll_to_offset(self, offset: f64, cx: &mut App) {
        self.dispatch(ListCommand::ScrollToOffset(offset), cx);
    }

    /// Scroll to the end of the list.
    pub fn scroll_to_end(self, cx: &mut App) {
        self.dispatch(ListCommand::ScrollToEnd, cx);
    }

    /// `measureElement`: item `index` is `height` px tall.
    pub fn measure_element(self, index: usize, height: f64, cx: &mut App) {
        self.dispatch(ListCommand::MeasureElement(index, height), cx);
    }

    /// Items changed without the count changing: measure again.
    pub fn invalidate(self, cx: &mut App) {
        self.dispatch(ListCommand::Invalidate, cx);
    }

    /// A new message at the end of a [`ChatList`].
    pub fn push(self, message: Message, cx: &mut App) {
        self.dispatch(ListCommand::Push(message), cx);
    }

    /// The items in the viewport as of the last range change (`range`).
    #[must_use]
    pub fn range(self) -> std::ops::Range<usize> {
        self.view.with(|v| v.range.clone())
    }

    /// The item count as of the last range change.
    #[must_use]
    pub fn count(self) -> usize {
        self.view.with(|v| v.count)
    }

    /// The list now: range, count, `scrollOffset` and `getTotalSize()`.
    #[must_use]
    pub fn view_info(self, cx: &App) -> Option<ViewInfo> {
        self.ops.get().map(|o| o.info(cx))
    }
}

/// What every list component takes besides its content.
pub struct ListExtras {
    base: Div,
    handle: Option<ListHandle>,
    options: Options,
    on_range_change: Option<Rc<dyn Fn(&ViewInfo)>>,
}

impl ListExtras {
    fn new() -> Self {
        Self { base: div().relative().overflow_hidden().size_full(), handle: None, options: Options::default(), on_range_change: None }
    }
}

/// The props every list component shares, written once.
pub trait ListProps: Sized {
    #[doc(hidden)]
    fn extras_mut(&mut self) -> &mut ListExtras;

    /// A handle from [`use_virtual_list`], to scroll and read the list.
    #[must_use]
    fn list(mut self, handle: ListHandle) -> Self {
        self.extras_mut().handle = Some(handle);
        self
    }

    /// `estimateSize`: the height in px an item of one line is assumed to
    /// have until it is measured. Read when the list is created.
    #[must_use]
    fn estimate_size(mut self, px: f32) -> Self {
        self.extras_mut().options.estimate_size = Some(px);
        self
    }

    /// `overscan`: items beyond the viewport the source is asked for. Read
    /// when the list is created.
    #[must_use]
    fn overscan(mut self, items: usize) -> Self {
        self.extras_mut().options.overscan = items;
        self
    }

    /// `onChange`: the visible range or the count changed.
    #[must_use]
    fn on_range_change(mut self, f: impl Fn(&ViewInfo) + 'static) -> Self {
        self.extras_mut().on_range_change = Some(Rc::new(f));
        self
    }

    /// Stay at the end while new items arrive (the chat default), or not.
    #[must_use]
    fn stick_to_bottom(mut self, on: bool) -> Self {
        self.extras_mut().options.stick_to_bottom = on;
        self
    }
}

macro_rules! styled_list {
    ($($ty:ident<$($g:ident),*>)*) => {$(
        impl<$($g),*> Styled for $ty<$($g),*> {
            fn decls_mut(&mut self) -> &mut Vec<Decl> {
                self.extras.base.decls_mut()
            }
        }
        impl<$($g),*> ListProps for $ty<$($g),*> {
            fn extras_mut(&mut self) -> &mut ListExtras {
                &mut self.extras
            }
        }
    )*};
}

/// A virtualized list of `count` text items (TanStack Virtual's model).
///
/// ```ignore
/// // rsx!
/// rsx! { <VirtualList count={10_000_000} item={|i: usize, out: &mut String| out.push_str("row")} overscan={8} h={px(400.0)} /> }
/// // builder
/// VirtualList::new(10_000_000, |i, out| out.push_str("row")).overscan(8).h(px(400.0))
/// ```
pub struct VirtualList<C = usize, I = ItemText> {
    count: C,
    item: I,
    key: Option<Box<dyn Fn(usize) -> u64>>,
    extras: ListExtras,
}

styled_list! { VirtualList<C, I> }

impl VirtualList<(), ()> {
    /// `count` items; item `i`'s text is what `item(i, out)` pushes to `out`.
    pub fn new(count: usize, item: impl Fn(usize, &mut String) + 'static) -> VirtualList {
        Self::__builder().count(count).item(item)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __builder() -> Self {
        Self { count: (), item: (), key: None, extras: ListExtras::new() }
    }
}

impl<I> VirtualList<(), I> {
    /// `count`.
    pub fn count(self, count: usize) -> VirtualList<usize, I> {
        VirtualList { count, item: self.item, key: self.key, extras: self.extras }
    }
}

impl<C> VirtualList<C, ()> {
    /// The text of each item: `item(index, out)` pushes it to `out`.
    pub fn item(self, item: impl Fn(usize, &mut String) + 'static) -> VirtualList<C, ItemText> {
        VirtualList { count: self.count, item: Box::new(item), key: self.key, extras: self.extras }
    }
}

impl<C, I> VirtualList<C, I> {
    /// `getItemKey`: a stable key per index, ascending with the index.
    #[must_use]
    pub fn get_item_key(mut self, key: impl Fn(usize) -> u64 + 'static) -> Self {
        self.key = Some(Box::new(key));
        self
    }
}

impl Complete for VirtualList {
    type Props = Self;
    fn finish(self) -> Self {
        self
    }
}

type ListSlot<S, P> = Signal<Option<Entity<ListState<S, P>>>>;

/// Publish range changes to the handle and the user's callback.
fn view_changed(handle: ListHandle, user: Option<Rc<dyn Fn(&ViewInfo)>>) -> impl Fn(&ViewInfo) + 'static {
    move |info: &ViewInfo| {
        if handle.view.with(|v| v != info) {
            handle.view.set(info.clone());
        }
        if let Some(f) = &user {
            f(info);
        }
    }
}

impl IntoElement for VirtualList {
    fn into_any_element(self) -> AnyElement {
        let Self { count, item, key, extras: ListExtras { base, handle, options, on_range_change } } = self;
        let own = use_virtual_list();
        let slot: ListSlot<CountSource, RowStyle> = use_signal(None);
        let handle = handle.unwrap_or(own);
        base.defer(move |_, cx, el| {
            let state = match slot.get() {
                Some(state) => {
                    state.update(cx, |s, _| {
                        let old = s.source.set_count(count);
                        s.source.set_text(item);
                        s.source.set_key(key);
                        let bytes = s.source.avg_bytes();
                        // an index error cannot happen below 2^40 items
                        if count > old {
                            let _ = s.view.appended(count - old, (count - old) as u64 * bytes);
                        } else if count < old {
                            s.view.removed(count, old - count, (old - count) as u64 * bytes);
                        }
                    });
                    state
                }
                None => {
                    let mut source = CountSource::new(count, item);
                    source.set_key(key);
                    let list = List::new(source, RowStyle::new(theme_palette()), 400.0, 300.0, options).expect("a count below 2^40 fits the index");
                    let state = cx.new(|cx| ListState::new(list, cx));
                    slot.set(Some(state));
                    state
                }
            };
            handle.ops.update_quiet(|o| *o = Some(Rc::new(ListLink { state })));
            let select = |i: usize, list: &mut RowList| {
                let id = list.source.id_at(i);
                list.style.selected = if list.style.selected == id { None } else { id };
            };
            let taken = std::mem::replace(el, Div::new("div"));
            *el = taken.child(variable_list(&state).on_item_click(select).on_view_change(view_changed(handle, on_range_change)));
        })
        .into_any_element()
    }
}

/// A chat: a [`VirtualList`] of messages (avatars, runs, day dividers, whole-message
/// selection by click). `messages` is compared by identity: pass the same `Arc`
/// to keep the list, a new one to start another conversation; append with
/// [`ListHandle::push`].
///
/// ```ignore
/// rsx! { <ChatList messages={messages} h={px(400.0)} /> }
/// ChatList::new(messages).h(px(400.0))
/// ```
pub struct ChatList<M = Arc<[Message]>> {
    messages: M,
    extras: ListExtras,
}

styled_list! { ChatList<M> }

impl ChatList<()> {
    /// A chat of `messages`.
    pub fn new(messages: impl Into<Arc<[Message]>>) -> ChatList {
        Self::__builder().messages(messages)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __builder() -> Self {
        let mut extras = ListExtras::new();
        extras.options.start_at_end = true;
        Self { messages: (), extras }
    }

    /// `messages`.
    pub fn messages(self, messages: impl Into<Arc<[Message]>>) -> ChatList {
        ChatList { messages: messages.into(), extras: self.extras }
    }
}

impl Complete for ChatList {
    type Props = Self;
    fn finish(self) -> Self {
        self
    }
}

impl IntoElement for ChatList {
    fn into_any_element(self) -> AnyElement {
        let Self { messages, extras: ListExtras { base, handle, options, on_range_change } } = self;
        let own = use_virtual_list();
        let slot: ListSlot<blenderui_vlist::VecSource, blenderui_vlist::ChatStyle> = use_signal(None);
        let shown: Signal<Option<Arc<[Message]>>> = use_signal(None);
        let handle = handle.unwrap_or(own);
        base.defer(move |_, cx, el| {
            let fresh = shown.with(|s| s.as_ref().is_none_or(|s| !Arc::ptr_eq(s, &messages)));
            let existing = slot.get().filter(|_| !fresh);
            let state = existing.unwrap_or_else(|| {
                shown.update_quiet(|s| *s = Some(Arc::clone(&messages)));
                let list = ChatCore::from_messages(messages.to_vec(), theme_palette(), 400.0, 300.0, options);
                let state = cx.new(|cx| ListState::new(list, cx));
                slot.set(Some(state));
                state
            });
            handle.ops.update_quiet(|o| *o = Some(Rc::new(ListLink { state })));
            let taken = std::mem::replace(el, Div::new("div"));
            *el = taken.child(chat_list(&state).on_view_change(view_changed(handle, on_range_change)));
        })
        .into_any_element()
    }
}
