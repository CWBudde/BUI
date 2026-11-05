//! A variable-height virtualized list for gpui, chat-grade, from 100 items to
//! 10M+: the framework-independent `blenderui-vlist` core (a Fenwick-over-
//! blocks height index, anchor-based scrolling, wrapped-text measuring, item
//! sources) painted straight into the window's scene by one custom gpui
//! `Element`: a frame builds a constant number of elements however many items
//! there are, and paints only the visible ones.
//!
//! * [`ListState`]: the entity around a [`List`] (items, style, view);
//! * [`VariableList`] ([`variable_list`], [`chat_list`]): the element, with
//!   pointer, wheel and keyboard input.
//!
//! ```ignore
//! use blenderui_vlist_gpui::*;
//!
//! let chat = cx.new(|cx| ListState::new(ChatList::from_messages(msgs, Palette::blender_dark(), 600.0, 400.0, Options::default()), cx));
//! // in render
//! div().size_full().child(chat_list(&chat))
//! ```

mod element;
mod state;

pub use blenderui_vlist::{
    Align, Budget, ChatList, ChatStyle, Hit, ItemPainter, ItemSource, List, ListView, Measurer, Message, Nav, Options, Palette, Record,
    VecSource, log, synth,
};
pub use element::{VariableList, chat_list, variable_list};
pub use state::ListState;
