//! From an element tree to positioned boxes: cascade, box tree, taffy layout.
//!
//! * Block containers with inline content become wrapping flex lines of word
//!   boxes (so text, links and inline-blocks wrap like a browser's line
//!   boxes); inline elements own their words for painting and hit testing.
//! * Flex and grid containers get text runs as measured, wrapping leaves.
//! * Tables map onto flex rows and columns, lists paint their markers.
//!
//! Everything lives in flat arenas owned by the window and reused every
//! frame: boxes and element data in [`Frame`], computed styles in
//! [`StyleStore`] (shared between identical elements), the taffy tree, text
//! widths and images. Boxes refer to each other by index; nothing is
//! reference-counted.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use taffy::prelude::*;
use taffy::style::Overflow;

use crate::css::{Decl, Prop, Rule, Stylesheet, Subject};
use crate::element::{AnyElement, Div, Listeners, PaintFn, ParentElement, SharedString};
use crate::geometry::{rect, Bounds, Units};
use crate::html::{tag_kind, ua_stylesheet, TagKind};
use crate::paint::{Image, ImageCache};
use crate::prof::{span, Phase};
use crate::style::{apply, Computed, DisplayKind, Fnv, FnvMap, TextAlign, TextTransform, VAlign, WhiteSpace};
use crate::text::{self, TextCache};
use crate::widgets::WidgetSpec;

// ==========================================================================
// Keys

/// An element's identity across frames: a hash of its `.id()`, or of its
/// parent's key, tag and position.
pub(crate) type Key = u64;

fn fnv(parts: &[&[u8]]) -> u64 {
    let mut h = Fnv::default();
    for p in parts {
        h.write(p);
        h.write(&[0xff]);
    }
    h.finish()
}

/// The key of the element with `.id(id)`.
pub(crate) fn id_key(id: &str) -> Key {
    fnv(&[b"#", id.as_bytes()])
}

fn child_key(parent: Key, tag: &str, n: usize) -> Key {
    fnv(&[&parent.to_le_bytes(), tag.as_bytes(), &(n as u64).to_le_bytes()])
}

/// The key of the `<html>` element.
pub(crate) const ROOT_KEY: Key = 0x68746d6c; // "html"

/// Interaction state the cascade reads (`:hover`, `:active`, `:focus`, `details[open]`).
#[derive(Default)]
pub(crate) struct UiState {
    pub hovered: HashSet<Key>,
    pub active: HashSet<Key>,
    pub focused: Option<Key>,
    /// The focused element and its ancestors (`:focus-within`).
    pub focus_path: HashSet<Key>,
    /// The `.id()` of the focused element.
    pub focused_id: Option<String>,
    pub details_open: HashMap<Key, bool>,
    pub scroll: HashMap<Key, (f32, f32)>,
    pub checked: HashMap<Key, bool>,
}

// ==========================================================================
// Styles

/// A computed style in a [`StyleStore`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StyleId(u32);

/// The style everything inherits from at the root.
pub(crate) const ROOT_STYLE: StyleId = StyleId(0);

/// What a computed style depends on (style sharing).
struct StyleKey {
    parent: StyleId,
    viewport: (u32, u32),
    author: u64,
    rules: Vec<usize>,
    decls: Vec<Decl>,
    fixup: u8,
}

/// Computed styles, shared by every element with the same inputs and kept
/// across frames (browsers call this style sharing). Elements refer to them
/// by [`StyleId`].
pub(crate) struct StyleStore {
    styles: Vec<Computed>,
    cache: FnvMap<u64, Vec<(StyleKey, StyleId)>>,
}

impl Default for StyleStore {
    fn default() -> Self {
        StyleStore { styles: vec![Computed::default()], cache: Default::default() }
    }
}

impl std::ops::Index<StyleId> for StyleStore {
    type Output = Computed;
    fn index(&self, id: StyleId) -> &Computed {
        &self.styles[id.0 as usize]
    }
}

impl StyleStore {
    /// Before building a frame: drop everything when the store has grown
    /// large (ids from the previous frame are not used after this).
    fn begin_frame(&mut self) {
        if self.styles.len() > 20_000 {
            self.styles.truncate(1);
            self.cache.clear();
        }
    }

    fn add(&mut self, c: Computed) -> StyleId {
        self.styles.push(c);
        StyleId((self.styles.len() - 1) as u32)
    }
}

/// The author stylesheet (all `<style>` elements), reparsed only when its text changes.
#[derive(Default)]
pub(crate) struct AuthorSheet {
    hash: u64,
    sheet: Stylesheet,
}

// ==========================================================================
// Boxes

/// What a box holds.
pub(crate) enum Content {
    Element,
    /// A word (or pre-formatted line) inside a line container: bytes
    /// `start..end` of the text of box `src`.
    Word { src: u32, start: u32, end: u32 },
    /// A whole text run in a flex/grid container (wraps when measured).
    Text(String),
    LineBreak,
    Widget(WidgetSpec),
    Replaced(Replaced),
    /// An anonymous line container (inline content between blocks).
    Anon,
}

pub(crate) enum Replaced {
    Image(Arc<Image>),
    Painted(PaintFn),
    Placeholder(&'static str),
}

/// The element-level data kept from a `Div` after building (moved, not copied).
pub(crate) struct ElData {
    pub tag: &'static str,
    pub id: Option<SharedString>,
    pub attrs: Vec<(SharedString, SharedString)>,
    pub listeners: Option<Box<Listeners>>,
}

const NONE: u32 = u32::MAX;

pub(crate) struct BoxNode {
    pub parent: Option<usize>,
    /// Element and text children, linked (words / breaks / line boxes point
    /// at their parent but aren't linked into its list).
    first: u32,
    last: u32,
    next: u32,
    prev: u32,
    pub content: Content,
    /// Index into [`Frame::els`].
    el: u32,
    /// The element this box belongs to (itself, or the inline element of a word).
    pub owner: usize,
    pub style: StyleId,
    pub key: Key,
    /// What selectors can see of this element (its tag, id, attributes,
    /// position and state, chained through its parent and previous sibling):
    /// equal fingerprints match the same rules.
    fp: u64,
    pub taffy: Option<NodeId>,
    pub pos: (usize, usize),
    pub marker: Option<String>,
    // filled after layout
    pub bounds: Bounds<f32>,
    pub clip: Option<Bounds<f32>>,
    pub opacity: f32,
    pub visible: bool,
    pub scroll_range: (f32, f32),
    pub scroll_pos: (f32, f32),
    /// This element is one layout leaf measuring its lone text child (that box).
    pub fold: Option<usize>,
    /// This text run is laid out by its parent (`fold`), not by its own node.
    pub folded: bool,
}

/// A laid-out frame. The arenas are reused by the next frame.
#[derive(Default)]
pub(crate) struct Frame {
    pub nodes: Vec<BoxNode>,
    els: Vec<ElData>,
    pub title: Option<String>,
    by_key: std::cell::OnceCell<HashMap<Key, usize>>,
}

/// Measure contexts of taffy leaves. Compared across frames: an unchanged
/// context keeps the leaf's cached layout.
#[derive(PartialEq)]
pub(crate) enum Measure {
    /// Text of the box registered under `key` this frame (already
    /// whitespace-processed and transformed); `hash` identifies the text.
    Text { key: Key, hash: u64, size: f32, flags: i32, line: f32, nowrap: bool },
}

/// The persistent taffy tree's nodes by box key (with the frame they were
/// last used in). Reconciling instead of rebuilding lets taffy's cache skip
/// every subtree whose style, content and children didn't change.
#[derive(Default)]
struct Recon {
    map: FnvMap<Key, (NodeId, u32)>,
    stamp: u32,
    /// Nodes for duplicate keys this frame: removed next frame.
    extra: Vec<NodeId>,
}

/// Everything layout keeps across frames, owned by the window.
pub(crate) struct LayoutCaches {
    pub styles: StyleStore,
    pub text: TextCache,
    pub images: ImageCache,
    pub author: AuthorSheet,
    pub taffy: TaffyTree<Measure>,
    /// Box index of each taffy node (by slot), rebuilt in place each frame.
    box_of: Vec<u32>,
    /// Scratch for `finish`.
    acc: Vec<Option<Bounds<f32>>>,
    /// Matched rules by element fingerprint (+ viewport): selector matching
    /// runs once per distinct element, not every frame.
    matched: MatchCache,
    recon: Recon,
    /// Measured text leaves: key -> box index, this frame.
    text_box: FnvMap<Key, usize>,
}

/// Rule indices (`Rule::order`) in the UA and author sheets.
type RuleIds = smallvec::SmallVec<[u32; 8]>;

#[derive(Default)]
struct MatchCache {
    map: FnvMap<u64, (RuleIds, RuleIds)>,
    author: u64,
}

impl Default for LayoutCaches {
    fn default() -> Self {
        LayoutCaches {
            styles: StyleStore::default(),
            text: TextCache::default(),
            images: ImageCache::default(),
            author: AuthorSheet::default(),
            taffy: TaffyTree::new(),
            box_of: Vec::new(),
            acc: Vec::new(),
            matched: MatchCache::default(),
            recon: Recon::default(),
            text_box: FnvMap::default(),
        }
    }
}

struct Builder<'a> {
    nodes: Vec<BoxNode>,
    els: Vec<ElData>,
    taffy: &'a mut TaffyTree<Measure>,
    styles: &'a mut StyleStore,
    text: &'a mut TextCache,
    images: &'a mut ImageCache,
    matched: &'a mut MatchCache,
    recon: &'a mut Recon,
    text_box: &'a mut FnvMap<Key, usize>,
    /// Set when a node changed while building the current subtree.
    changed: bool,
    author: &'a Stylesheet,
    author_hash: u64,
    state: &'a UiState,
    viewport: (f32, f32),
    title: Option<String>,
    widgets: &'a mut dyn FnMut(Key, &WidgetSpec) -> (f32, f32),
}

/// Child box indices / layout nodes being assembled: on the stack for the usual few.
type Kids = smallvec::SmallVec<[usize; 8]>;
type NodeList = smallvec::SmallVec<[NodeId; 8]>;

/// The slot index of a taffy node (dense from 0 after `TaffyTree::clear`).
fn slot(id: NodeId) -> usize {
    (u64::from(id) & 0xffff_ffff) as usize
}

/// Children of box `i` in the arena `nodes`.
fn kids(nodes: &[BoxNode], i: usize) -> impl Iterator<Item = usize> + '_ {
    let mut k = nodes[i].first;
    std::iter::from_fn(move || {
        (k != NONE).then(|| {
            let cur = k as usize;
            k = nodes[cur].next;
            cur
        })
    })
}

fn el_attr<'a>(nodes: &[BoxNode], els: &'a [ElData], i: usize, name: &str) -> Option<&'a str> {
    let e = nodes[i].el;
    if e == NONE {
        return None;
    }
    els[e as usize].attrs.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v.as_ref())
}

// -- selector subject -------------------------------------------------------------------

#[derive(Clone)]
struct NodeRef<'b> {
    nodes: &'b [BoxNode],
    els: &'b [ElData],
    state: &'b UiState,
    i: usize,
}

impl NodeRef<'_> {
    fn el(&self) -> Option<&ElData> {
        let e = self.nodes[self.i].el;
        (e != NONE).then(|| &self.els[e as usize])
    }
    fn own_attr(&self, name: &str) -> Option<&str> {
        el_attr(self.nodes, self.els, self.i, name)
    }
}

impl Subject for NodeRef<'_> {
    fn tag(&self) -> &str {
        self.el().map(|e| e.tag).unwrap_or("")
    }
    fn id(&self) -> Option<&str> {
        self.el()?.id.as_deref()
    }
    fn has_class(&self, class: &str) -> bool {
        self.own_attr("class").is_some_and(|c| c.split_whitespace().any(|w| w == class))
    }
    fn attr(&self, name: &str) -> Option<&str> {
        let n = &self.nodes[self.i];
        if name == "open" && self.tag() == "details" {
            return self.state.details_open.get(&n.key).copied()
                .unwrap_or(self.own_attr("open").is_some()).then_some("");
        }
        self.own_attr(name)
    }
    fn state(&self, pseudo: &str) -> bool {
        let key = self.nodes[self.i].key;
        match pseudo {
            "hover" => self.state.hovered.contains(&key),
            "active" => self.state.active.contains(&key),
            "focus" | "focus-visible" => self.state.focused == Some(key),
            "focus-within" => self.state.focus_path.contains(&key),
            "checked" => self.state.checked.get(&key).copied().unwrap_or(self.own_attr("checked").is_some()),
            "disabled" => self.own_attr("disabled").is_some(),
            "enabled" => self.own_attr("disabled").is_none(),
            "link" | "any-link" => self.tag() == "a" && self.own_attr("href").is_some(),
            "required" => self.own_attr("required").is_some(),
            "optional" => self.own_attr("required").is_none(),
            "read-only" => self.own_attr("readonly").is_some(),
            "read-write" => self.own_attr("readonly").is_none(),
            "open" => self.attr("open").is_some(),
            "placeholder-shown" => {
                self.own_attr("placeholder").is_some() && self.own_attr("value").is_none_or(str::is_empty)
            }
            _ => false,
        }
    }
    fn parent(&self) -> Option<Self> {
        self.nodes[self.i].parent.map(|p| NodeRef { i: p, ..self.clone() })
    }
    fn prev_sibling(&self) -> Option<Self> {
        let mut k = self.nodes[self.i].prev;
        while k != NONE {
            if self.nodes[k as usize].el != NONE {
                return Some(NodeRef { i: k as usize, ..self.clone() });
            }
            k = self.nodes[k as usize].prev;
        }
        None
    }
    fn position(&self) -> (usize, usize) {
        self.nodes[self.i].pos
    }
    fn is_empty(&self) -> bool {
        self.nodes[self.i].first == NONE
    }
}

// -- building -------------------------------------------------------------------------------

fn collect_styles(el: &AnyElement, css: &mut String, title: &mut Option<String>) {
    if let AnyElement::Element(d) = el {
        let text = || d.children.iter().filter_map(|c| match c {
            AnyElement::Text(t) => Some(t.as_ref()),
            _ => None,
        }).collect::<String>();
        match d.tag {
            "style" => css.push_str(&text()),
            "title" if title.is_none() => *title = Some(text()),
            _ => {}
        }
        for c in &d.children {
            collect_styles(c, css, title);
        }
    }
}

impl<'a> Builder<'a> {
    fn subject(&self, i: usize) -> NodeRef<'_> {
        NodeRef { nodes: &self.nodes, els: &self.els, state: self.state, i }
    }

    fn st(&self, i: usize) -> &Computed {
        &self.styles[self.nodes[i].style]
    }

    fn attr(&self, i: usize, name: &str) -> Option<&str> {
        el_attr(&self.nodes, &self.els, i, name)
    }

    /// The computed style of element `i`, shared with every element (this
    /// frame or earlier) that had the same parent style, matched rules and
    /// declarations. `fixup` adjusts `display` for widgets and flex/grid
    /// items before the result is shared.
    fn cascade(&mut self, i: usize, div: &Div, parent: StyleId, fixup: u8) -> StyleId {
        let _s = span(Phase::Cascade);
        let (ua, au) = self.matched_rules(i);
        let decls = self.declared(i, div, &ua, &au);
        let viewport = (self.viewport.0.to_bits(), self.viewport.1.to_bits());
        let rule_ids = || ua.iter().map(|r| *r as *const Rule as usize).chain(au.iter().map(|r| r.order));
        // probe without building a key: hash the inputs, compare in place
        let hash = {
            let mut h = Fnv::default();
            parent.hash(&mut h);
            viewport.hash(&mut h);
            self.author_hash.hash(&mut h);
            fixup.hash(&mut h);
            for r in rule_ids() {
                r.hash(&mut h);
            }
            for d in &decls {
                d.probe_hash(&mut h);
            }
            h.finish()
        };
        let same = |k: &StyleKey| {
            k.parent == parent
                && k.viewport == viewport
                && k.author == self.author_hash
                && k.fixup == fixup
                && k.rules.len() == ua.len() + au.len()
                && k.rules.iter().copied().eq(rule_ids())
                && k.decls.len() == decls.len()
                && k.decls.iter().zip(&decls).all(|(a, d)| a == *d)
        };
        if let Some(&(_, id)) = self.styles.cache.get(&hash).and_then(|b| b.iter().find(|(k, _)| same(k))) {
            return id;
        }
        let key = StyleKey {
            parent,
            viewport,
            author: self.author_hash,
            rules: rule_ids().collect(),
            decls: decls.iter().map(|d| (*d).clone()).collect(),
            fixup,
        };
        let mut c = self.compute(parent, &decls);
        match fixup {
            1 if c.display == DisplayKind::Inline => c.display = DisplayKind::InlineBlock,
            2 if c.display == DisplayKind::Inline => c.display = DisplayKind::Block,
            _ => {}
        }
        let id = self.styles.add(c);
        self.styles.cache.entry(hash).or_default().push((key, id));
        id
    }

    /// The UA and author rules matching element `i`: from the memo when an
    /// element with the same fingerprint was matched before.
    fn matched_rules(&mut self, i: usize) -> (crate::css::Matches<'static>, crate::css::Matches<'a>) {
        let ua_sheet = ua_stylesheet();
        let author: &'a Stylesheet = self.author;
        let memo = {
            let mut h = Fnv::default();
            self.nodes[i].fp.hash(&mut h);
            self.viewport.0.to_bits().hash(&mut h);
            self.viewport.1.to_bits().hash(&mut h);
            h.finish()
        };
        if let Some((u, a)) = self.matched.map.get(&memo) {
            return (u.iter().map(|&k| &ua_sheet.rules[k as usize]).collect(),
                    a.iter().map(|&k| &author.rules[k as usize]).collect());
        }
        let subj = self.subject(i);
        let ua = ua_sheet.matches(&subj, self.viewport);
        let au = author.matches(&subj, self.viewport);
        self.matched.map.insert(memo, (ua.iter().map(|r| r.order as u32).collect(),
                                       au.iter().map(|r| r.order as u32).collect()));
        (ua, au)
    }

    /// The fingerprint of element `i` (see [`BoxNode::fp`]).
    fn fingerprint(&self, i: usize) -> u64 {
        let n = &self.nodes[i];
        let mut h = Fnv::default();
        n.parent.map(|p| self.nodes[p].fp).unwrap_or(0).hash(&mut h);
        // the previous element sibling (text in between doesn't count)
        let mut k = n.prev;
        while k != NONE && self.nodes[k as usize].el == NONE {
            k = self.nodes[k as usize].prev;
        }
        (if k == NONE { 0 } else { self.nodes[k as usize].fp }).hash(&mut h);
        n.pos.hash(&mut h);
        if let Some(e) = (n.el != NONE).then(|| &self.els[n.el as usize]) {
            e.tag.hash(&mut h);
            e.id.as_deref().hash(&mut h);
            for (a, v) in &e.attrs {
                a.hash(&mut h);
                v.hash(&mut h);
            }
        }
        let st = self.state;
        let key = n.key;
        (st.hovered.contains(&key), st.active.contains(&key), st.focused == Some(key),
         st.focus_path.contains(&key), st.checked.get(&key).copied(), st.details_open.get(&key).copied())
            .hash(&mut h);
        h.finish()
    }

    /// Every declaration that applies to element `i`, in cascade order.
    fn declared<'d>(&self, i: usize, div: &'d Div, ua: &[&'d Rule], au: &[&'d Rule])
        -> smallvec::SmallVec<[&'d Decl; 24]> {
        let key = self.nodes[i].key;
        let mut decls = smallvec::SmallVec::new();
        decls.extend(ua.iter().flat_map(|r| r.decls.iter()));
        decls.extend(au.iter().flat_map(|r| r.decls.iter()));
        decls.extend(div.decls.iter());
        if let Some(st) = &div.states {
            if self.state.hovered.contains(&key) {
                decls.extend(st.hover.iter());
            }
            if self.state.active.contains(&key) {
                decls.extend(st.active.iter());
            }
            if self.state.focused == Some(key) {
                decls.extend(st.focus.iter());
            }
        }
        decls
    }

    fn compute(&self, parent: StyleId, decls: &[&Decl]) -> Computed {
        let parent = &self.styles[parent];
        let mut c = Computed::inherit(parent);
        const FONT_SIZE: Prop = Prop::named("font-size");
        const FONT: Prop = Prop::named("font");
        let font_first = |d: &&Decl| d.prop == FONT_SIZE || d.prop == FONT;
        for important in [false, true] {
            // font-size first: em lengths in the other properties depend on it
            for d in decls.iter().filter(|d| d.important == important && font_first(d)) {
                apply(&mut c, parent, self.units(parent.font_size), d);
            }
        }
        let u = self.units(c.font_size);
        for important in [false, true] {
            for d in decls.iter().filter(|d| d.important == important && !font_first(d)) {
                apply(&mut c, parent, u, d);
            }
        }
        c
    }

    fn units(&self, font_size: f32) -> Units {
        Units { font_size, viewport: self.viewport }
    }

    /// A new box. `link`: append it to its parent's child list (elements and
    /// text); words, breaks and line boxes only point at their parent.
    fn push(&mut self, parent: Option<usize>, content: Content, owner: Option<usize>, style: StyleId,
            key: Key, link: bool) -> usize {
        let i = self.nodes.len();
        let prev = match parent {
            Some(p) if link => self.nodes[p].last,
            _ => NONE,
        };
        self.nodes.push(BoxNode {
            parent,
            first: NONE,
            last: NONE,
            next: NONE,
            prev,
            content,
            el: NONE,
            owner: owner.unwrap_or(i),
            style,
            key,
            fp: 0,
            taffy: None,
            pos: (1, 1),
            marker: None,
            bounds: rect(0.0, 0.0, 0.0, 0.0),
            clip: None,
            opacity: 1.0,
            visible: true,
            scroll_range: (0.0, 0.0),
            scroll_pos: (0.0, 0.0),
            fold: None,
            folded: false,
        });
        if let (Some(p), true) = (parent, link) {
            if prev == NONE {
                self.nodes[p].first = i as u32;
            } else {
                self.nodes[prev as usize].next = i as u32;
            }
            self.nodes[p].last = i as u32;
        }
        i
    }

    /// Remove the last box (an element that turned out not to render).
    fn pop(&mut self) {
        let n = self.nodes.pop().expect("a box to pop");
        if let Some(p) = n.parent {
            let i = self.nodes.len() as u32;
            if self.nodes[p].last == i {
                self.nodes[p].last = n.prev;
                if n.prev == NONE {
                    self.nodes[p].first = NONE;
                } else {
                    self.nodes[n.prev as usize].next = NONE;
                }
            }
        }
        if n.el != NONE && n.el as usize == self.els.len() - 1 {
            self.els.pop();
        }
    }

    /// Build an element; returns its index (None when not rendered).
    fn element(&mut self, mut div: Div, parent: Option<usize>, key: Key, pos: (usize, usize)) -> Option<usize> {
        let parent_style = parent.map(|p| self.nodes[p].style).unwrap_or(ROOT_STYLE);
        let i = self.push(parent, Content::Element, None, parent_style, key, true);
        self.nodes[i].pos = pos;
        self.nodes[i].el = self.els.len() as u32;
        self.els.push(ElData {
            tag: div.tag,
            id: div.id.take(),
            attrs: std::mem::take(&mut div.attrs),
            listeners: std::mem::take(&mut div.listeners),
        });
        self.nodes[i].fp = self.fingerprint(i);
        let kind = tag_kind(div.tag);
        // widgets and replaced elements are atomic inlines; flex/grid items are blockified
        let fixup = if kind == TagKind::Widget || kind == TagKind::Replaced {
            1
        } else if parent.is_some_and(|p| matches!(self.st(p).display,
                                                  DisplayKind::Flex | DisplayKind::InlineFlex
                                                  | DisplayKind::Grid | DisplayKind::InlineGrid)) {
            2
        } else {
            0
        };
        let style = self.cascade(i, &div, parent_style, fixup);
        if self.styles[style].display == DisplayKind::None || kind == TagKind::Metadata {
            self.pop();
            return None;
        }
        self.nodes[i].style = style;
        // attributes moved into ElData: widgets read them back through the arena
        div.attrs = Vec::new();
        match kind {
            TagKind::Widget => {
                let spec = WidgetSpec::from_element(&div, self.els.last().map(|e| &e.attrs[..]).unwrap_or(&[]),
                                                    key, self.state);
                if let Some(spec) = spec {
                    self.nodes[i].content = Content::Widget(spec);
                } else {
                    self.children(i, div.children);
                }
            }
            TagKind::Replaced => {
                let r = if let Some(f) = div.paint.take() {
                    Replaced::Painted(f)
                } else if let Some(img) = div.image.take().or_else(|| {
                    self.attr(i, "src").map(str::to_owned).and_then(|s| self.images.load(&s))
                }) {
                    Replaced::Image(img)
                } else {
                    Replaced::Placeholder(div.tag)
                };
                self.nodes[i].content = Content::Replaced(r);
            }
            TagKind::Break => {
                self.nodes[i].content = Content::LineBreak;
            }
            TagKind::Box | TagKind::Metadata => {
                let mut children = div.children;
                if div.tag == "details" {
                    let open = self.subject(i).attr("open").is_some();
                    if !open {
                        children.retain(|c| matches!(c, AnyElement::Element(d) if d.tag == "summary"));
                    }
                }
                if div.tag == "q" {
                    let (open, close) = self.st(i).quotes.clone()
                        .unwrap_or_else(|| ("\u{201c}".into(), "\u{201d}".into()));
                    children.insert(0, AnyElement::Text(open.into()));
                    children.push(AnyElement::Text(close.into()));
                }
                self.children(i, children);
            }
        }
        Some(i)
    }

    /// Children of element `i`: elements and text, keyed by position.
    fn children(&mut self, i: usize, mut children: Vec<AnyElement>) {
        // list markers: painted outside the box, or inline text when `inside`
        if self.st(i).display == DisplayKind::ListItem {
            let ls = self.st(i).list_style;
            let tag = self.subject(i).tag().to_owned();
            let marker = if tag == "summary" {
                let details = self.nodes[i].parent.unwrap_or(i);
                Some(if self.subject(details).attr("open").is_some() { "\u{25be}" } else { "\u{25b8}" }.into())
            } else {
                let idx = self.nodes[i].pos.0 + self.nodes[i].parent
                    .and_then(|p| self.attr(p, "start"))
                    .and_then(|s| s.parse::<usize>().ok()).map(|s| s - 1).unwrap_or(0);
                ls.marker(idx)
            };
            match marker {
                Some(m) if self.st(i).marker_inside => {
                    children.insert(0, AnyElement::Text(format!("{m}\u{a0}").into()));
                }
                m => self.nodes[i].marker = m,
            }
        }
        let count = children.iter().filter(|c| matches!(c, AnyElement::Element(_))).count();
        let (mut k, mut n) = (0, 0);
        let parent_key = self.nodes[i].key;
        for child in children {
            n += 1;
            match child {
                AnyElement::Element(d) => {
                    k += 1;
                    let key = match &d.id {
                        Some(id) => id_key(id),
                        None => child_key(parent_key, d.tag, k),
                    };
                    self.element(d, Some(i), key, (k, count));
                }
                AnyElement::Text(t) => {
                    let style = self.nodes[i].style;
                    self.push(Some(i), Content::Text(t.into_owned()), Some(i), style,
                              child_key(parent_key, "#text", n), true);
                }
            }
        }
    }

    // -- taffy ------------------------------------------------------------------------------

    fn is_block_level(&self, i: usize) -> bool {
        match &self.nodes[i].content {
            Content::Element => !self.st(i).display.is_inline_level(),
            _ => false,
        }
    }

    fn base_style(&self, i: usize) -> Style {
        let c = self.st(i);
        let mut s = c.layout.clone();
        s.display = match c.display {
            DisplayKind::Flex | DisplayKind::InlineFlex => Display::Flex,
            DisplayKind::Grid | DisplayKind::InlineGrid => Display::Grid,
            DisplayKind::Table | DisplayKind::TableRowGroup => {
                s.flex_direction = FlexDirection::Column;
                let sp = if c.border_collapse { 0.0 } else { c.border_spacing.1 };
                s.gap.height = LengthPercentage::Length(sp);
                if c.display == DisplayKind::Table && !c.border_collapse {
                    let pad = |v: LengthPercentage, add: f32| match v {
                        LengthPercentage::Length(p) => LengthPercentage::Length(p + add),
                        other => other,
                    };
                    let (h, v) = c.border_spacing;
                    s.padding = Rect { left: pad(s.padding.left, h), right: pad(s.padding.right, h),
                                       top: pad(s.padding.top, v), bottom: pad(s.padding.bottom, v) };
                }
                Display::Flex
            }
            DisplayKind::TableRow => {
                s.flex_direction = FlexDirection::Row;
                s.gap.width = LengthPercentage::Length(if c.border_collapse { 0.0 } else { c.border_spacing.0 });
                Display::Flex
            }
            _ => Display::Block,
        };
        if c.display == DisplayKind::TableCell && s.size.width == Dimension::Auto {
            let span = self.attr(i, "colspan").and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
            s.flex_grow = span;
            s.flex_shrink = 1.0;
            s.flex_basis = Dimension::Percent(0.0);
        }
        s
    }

    /// Make line-container style: a wrapping flex row aligned per text-align.
    fn line_style(c: &Computed, mut s: Style) -> Style {
        s.display = Display::Flex;
        s.flex_direction = FlexDirection::Row;
        // nowrap / pre text arrives as unbreakable pieces; wrapping only
        // happens at forced breaks (<br>, newlines) and between inline boxes
        s.flex_wrap = FlexWrap::Wrap;
        s.align_items = Some(AlignItems::FlexEnd);
        s.align_content = Some(AlignContent::FlexStart);
        s.justify_content = Some(match c.text_align {
            TextAlign::Center => JustifyContent::Center,
            TextAlign::Right => JustifyContent::FlexEnd,
            _ => JustifyContent::FlexStart,
        });
        s
    }

    /// The taffy node for `key` with this style, measure context and
    /// children: last frame's node, updated only where something differs
    /// (which marks it dirty), or a new one.
    fn node(&mut self, key: Key, style: Style, ctx: Option<Measure>, children: &[NodeId]) -> NodeId {
        let stamp = self.recon.stamp;
        let reuse = match self.recon.map.get_mut(&key) {
            Some((id, st)) if *st != stamp => {
                *st = stamp;
                Some(*id)
            }
            _ => None,
        };
        let t = &mut *self.taffy;
        if let Some(id) = reuse {
            if t.style(id).map(|s| *s != style).unwrap_or(true) {
                let _ = t.set_style(id, style);
                self.changed = true;
            }
            let same_kids = t.child_count(id) == children.len()
                && children.iter().enumerate().all(|(k, c)| t.child_at_index(id, k).ok() == Some(*c));
            if !same_kids {
                let _ = t.set_children(id, children);
                self.changed = true;
            }
            if t.get_node_context(id) != ctx.as_ref() {
                let _ = t.set_node_context(id, ctx);
                self.changed = true;
            }
            return id;
        }
        let id = match ctx {
            Some(m) => t.new_leaf_with_context(style, m).unwrap(),
            None => t.new_with_children(style, children).unwrap(),
        };
        self.changed = true;
        match self.recon.map.entry(key) {
            std::collections::hash_map::Entry::Occupied(_) => self.recon.extra.push(id), // duplicate key
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert((id, stamp));
            }
        }
        id
    }

    fn leaf(&mut self, i: usize, style: Style) -> NodeId {
        let id = self.node(self.nodes[i].key, style, None, &[]);
        self.nodes[i].taffy = Some(id);
        id
    }

    /// Replace the text of box `k` by its whitespace-processed, transformed form.
    fn transform_text(&mut self, k: usize, c: StyleId) {
        let (tt, ws) = (self.styles[c].text_transform, self.styles[c].white_space);
        if let Content::Text(t) = &self.nodes[k].content {
            if let std::borrow::Cow::Owned(s) = transform(t, tt, ws) {
                self.nodes[k].content = Content::Text(s);
            }
        }
    }

    /// The measure context for the text of box `node`, registered under `key`.
    fn text_measure(&mut self, key: Key, node: usize, c: StyleId, nowrap: bool) -> Measure {
        self.text_box.insert(key, node);
        let hash = match &self.nodes[node].content {
            Content::Text(t) => fnv(&[t.as_bytes()]),
            _ => 0,
        };
        let c = &self.styles[c];
        Measure::Text { key, hash, size: c.font_size, flags: text::flags(c), line: c.line_px(), nowrap }
    }

    /// The taffy node for box `i` (and its subtree). Marks it dirty when
    /// anything below changed (taffy's own propagation follows parent links,
    /// which reconciling can leave stale).
    fn layout_node(&mut self, i: usize) -> NodeId {
        let outer = std::mem::replace(&mut self.changed, false);
        let id = self.layout_node_inner(i);
        if self.changed {
            let _ = self.taffy.mark_dirty(id);
        }
        self.changed |= outer;
        id
    }

    fn layout_node_inner(&mut self, i: usize) -> NodeId {
        let mut base = self.base_style(i);
        if i == 0 {
            // the root fills the viewport
            base.min_size = Size { width: Dimension::Length(self.viewport.0),
                                   height: Dimension::Length(self.viewport.1) };
        }
        let sid = self.nodes[i].style;
        match &self.nodes[i].content {
            Content::Widget(spec) => {
                let (w, h) = (self.widgets)(self.nodes[i].key, spec);
                let mut s = base;
                if s.size.width == Dimension::Auto {
                    s.size.width = Dimension::Length(w);
                }
                if s.size.height == Dimension::Auto {
                    s.size.height = Dimension::Length(h);
                }
                return self.leaf(i, s);
            }
            Content::Replaced(r) => {
                let (iw, ih) = match r {
                    Replaced::Image(img) => (img.width as f32, img.height as f32),
                    _ => (300.0, 150.0),
                };
                let attr = |n: &str| self.attr(i, n).and_then(|v| v.trim_end_matches("px").parse::<f32>().ok());
                let mut s = base;
                let (aw, ah) = (attr("width"), attr("height"));
                if s.size.width == Dimension::Auto {
                    s.size.width = Dimension::Length(aw.unwrap_or(match ah {
                        Some(h) if ih > 0.0 => h * iw / ih,
                        _ => iw,
                    }));
                }
                if s.size.height == Dimension::Auto {
                    s.size.height = Dimension::Length(ah.unwrap_or(match aw {
                        Some(w) if iw > 0.0 => w * ih / iw,
                        _ => ih,
                    }));
                }
                return self.leaf(i, s);
            }
            Content::LineBreak => {
                let s = Style { size: Size { width: Dimension::Percent(1.0), height: Dimension::Length(0.0) },
                                flex_shrink: 0.0, ..Default::default() };
                return self.leaf(i, s);
            }
            Content::Text(_) => {
                // a text run as a flex/grid item: measured, wrapping. The box
                // keeps the transformed text, so paint draws what was measured.
                self.transform_text(i, sid);
                let nowrap = matches!(self.styles[sid].white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
                let key = self.nodes[i].key;
                let m = self.text_measure(key, i, sid, nowrap);
                let id = self.node(key, Style::default(), Some(m), &[]);
                self.nodes[i].taffy = Some(id);
                return id;
            }
            _ => {}
        }
        if let Some(k) = self.foldable_text(i) {
            // one leaf for the box and its text: the text is measured as the
            // box's content and placed inside it after layout
            self.transform_text(k, sid);
            let nowrap = self.styles[sid].white_space == WhiteSpace::NoWrap;
            let key = self.nodes[i].key;
            let m = self.text_measure(key, k, sid, nowrap);
            let mut s = base;
            s.display = Display::Block;
            let id = self.node(key, s, Some(m), &[]);
            self.nodes[i].taffy = Some(id);
            self.nodes[i].fold = Some(k);
            self.nodes[k].folded = true;
            return id;
        }
        let c = self.styles[sid].clone_display();
        let flex_like = matches!(c.display, DisplayKind::Flex | DisplayKind::InlineFlex
                                 | DisplayKind::Grid | DisplayKind::InlineGrid
                                 | DisplayKind::Table | DisplayKind::TableRowGroup
                                 | DisplayKind::TableRow);
        let mut list: Kids = kids(&self.nodes, i).collect();
        let ordered = |b: &Builder, k: usize| {
            let st = b.st(k);
            if st.display == DisplayKind::TableCaption && st.caption_bottom { i32::MAX } else { st.order }
        };
        if list.iter().any(|&k| ordered(self, k) != 0) {
            list.sort_by_key(|&k| ordered(self, k));
        }
        let mut out: NodeList = NodeList::new();
        if flex_like {
            for k in list {
                if let Content::Text(t) = &self.nodes[k].content {
                    if t.trim().is_empty() && self.st(k).white_space == WhiteSpace::Normal {
                        continue;
                    }
                }
                self.child_nodes(k, &mut out);
            }
            let id = self.node(self.nodes[i].key, base, None, &out);
            self.nodes[i].taffy = Some(id);
            return id;
        }
        // block container: runs of inline content become line boxes
        if list.iter().all(|&k| !self.is_block_level(k)) {
            self.inline_items(&list, i, &mut out, true);
            let line = Self::line_style(&self.styles[sid], base);
            let id = self.node(self.nodes[i].key, line, None, &out);
            self.nodes[i].taffy = Some(id);
            return id;
        }
        let mut run: Kids = Kids::new();
        for k in list {
            if self.is_block_level(k) {
                self.flush_run(i, &mut run, &mut out);
                self.child_nodes(k, &mut out);
            } else {
                run.push(k);
            }
        }
        self.flush_run(i, &mut run, &mut out);
        let id = self.node(self.nodes[i].key, base, None, &out);
        self.nodes[i].taffy = Some(id);
        id
    }

    /// The lone text child of element `i`, when laying the pair out as one
    /// measured leaf gives the same result as two nodes (or a line of words).
    fn foldable_text(&self, i: usize) -> Option<usize> {
        let n = &self.nodes[i];
        if !matches!(n.content, Content::Element) || n.first == NONE || n.first != n.last {
            return None;
        }
        let k = n.first as usize;
        match &self.nodes[k].content {
            Content::Text(t) if !t.trim().is_empty() => {}
            _ => return None,
        }
        let c = self.st(i);
        let s = &c.layout;
        let display_ok = match c.display {
            DisplayKind::Block | DisplayKind::InlineBlock | DisplayKind::ListItem
            | DisplayKind::TableCell | DisplayKind::TableCaption => true,
            DisplayKind::Flex | DisplayKind::InlineFlex => {
                s.flex_wrap == FlexWrap::NoWrap
                    && matches!(s.flex_direction, FlexDirection::Row | FlexDirection::Column)
            }
            _ => false,
        };
        (display_ok
            && c.letter_spacing == 0.0
            && c.word_spacing == 0.0
            && c.text_indent == crate::geometry::Length::Px(0.0)
            && c.vertical_align == VAlign::Baseline
            && !c.small_caps
            && matches!(c.white_space, WhiteSpace::Normal | WhiteSpace::NoWrap)
            && s.overflow.x == Overflow::Visible
            && s.overflow.y == Overflow::Visible)
            .then_some(k)
    }

    fn child_nodes(&mut self, k: usize, out: &mut NodeList) {
        if self.st(k).display == DisplayKind::Contents {
            let list: Kids = kids(&self.nodes, k).collect();
            for c in list {
                self.child_nodes(c, out);
            }
            return;
        }
        out.push(self.layout_node(k));
    }

    /// An anonymous line box for inline content between blocks.
    fn flush_run(&mut self, parent: usize, run: &mut Kids, out: &mut NodeList) {
        let only_space = run.iter().all(|&k| matches!(&self.nodes[k].content,
                                                      Content::Text(t) if t.trim().is_empty()));
        if run.is_empty() || only_space {
            run.clear();
            return;
        }
        let style = self.nodes[parent].style;
        let key = child_key(self.nodes[parent].key, "#anon", out.len());
        let anon = self.push(Some(parent), Content::Anon, Some(parent), style, key, false);
        let mut line = NodeList::new();
        let items: Kids = std::mem::take(run);
        self.inline_items(&items, parent, &mut line, true);
        let ls = Self::line_style(&self.styles[style], Style::default());
        let id = self.node(key, ls, None, &line);
        if self.changed {
            let _ = self.taffy.mark_dirty(id);
        }
        self.nodes[anon].taffy = Some(id);
        out.push(id);
    }

    /// Flatten inline content into line items: words, atomic inline boxes, breaks.
    fn inline_items(&mut self, list: &[usize], owner: usize, line: &mut NodeList, first: bool) {
        let mut first_word = first;
        for &k in list {
            match &self.nodes[k].content {
                Content::Text(_) => {
                    let n = line.len();
                    self.words(k, owner, first_word, line);
                    if line.len() > n {
                        first_word = false;
                    }
                }
                Content::Element if matches!(self.st(k).display, DisplayKind::Inline | DisplayKind::Contents) => {
                    let inner: Kids = kids(&self.nodes, k).collect();
                    self.inline_items(&inner, k, line, first_word);
                    first_word = false;
                }
                _ => {
                    self.child_nodes(k, line);
                    first_word = false;
                }
            }
        }
    }

    /// Word boxes for text box `src`, with the style of its inline owner. The
    /// box keeps the processed text; words are byte ranges into it.
    fn words(&mut self, src: usize, owner: usize, first: bool, line: &mut NodeList) {
        let sid = self.nodes[owner].style;
        let c = &self.styles[sid];
        let (ws, tt, size, letter, word_sp) = (c.white_space, c.text_transform, c.font_size, c.letter_spacing, c.word_spacing);
        let flags = text::flags(c);
        let line_h = c.line_px();
        let indent = if first { c.text_indent.resolve(&self.units(size), 0.0) } else { 0.0 };
        // the processed text, kept in the box
        let Content::Text(raw) = std::mem::replace(&mut self.nodes[src].content, Content::Anon) else { return };
        let mut text = match transform(&raw, tt, ws) {
            std::borrow::Cow::Borrowed(_) => raw,
            std::borrow::Cow::Owned(s) => s,
        };
        if ws == WhiteSpace::NoWrap {
            text = collapse(&text);
        }
        // pieces: byte range, trailing space, forced break after
        let mut pieces: smallvec::SmallVec<[(usize, usize, bool, bool); 16]> = smallvec::SmallVec::new();
        let base = text.as_ptr() as usize;
        let range = |w: &str| (w.as_ptr() as usize - base, w.as_ptr() as usize - base + w.len());
        match ws {
            WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine => {
                let lines: Vec<&str> = text.split('\n').collect();
                for (n, l) in lines.iter().enumerate() {
                    let brk = n + 1 < lines.len();
                    if ws == WhiteSpace::Pre {
                        let (a, b) = range(l);
                        pieces.push((a, b, false, brk));
                    } else {
                        let words: Vec<&str> = l.split_whitespace().collect();
                        if words.is_empty() {
                            let (a, _) = range(l);
                            pieces.push((a, a, false, brk));
                        }
                        for (j, w) in words.iter().enumerate() {
                            let (a, b) = range(w);
                            pieces.push((a, b, j + 1 < words.len(), brk && j + 1 == words.len()));
                        }
                    }
                }
            }
            WhiteSpace::NoWrap => pieces.push((0, text.len(), false, false)),
            WhiteSpace::Normal => {
                let lead = text.starts_with(char::is_whitespace) && !first;
                let words: Vec<&str> = text.split_whitespace().collect();
                let trail = text.ends_with(char::is_whitespace);
                for (j, w) in words.iter().enumerate() {
                    let (mut a, b) = range(w);
                    if j == 0 && lead {
                        a -= 1; // one leading space, drawn with the word
                    }
                    pieces.push((a, b, j + 1 < words.len() || trail, false));
                }
            }
        }
        let space = self.text.width(" ", size, flags) + word_sp;
        let src_key = self.nodes[src].key;
        for (n, &(a, b, trailing, brk)) in pieces.iter().enumerate() {
            let piece = &text[a..b];
            // a leading space becomes a plain space for measuring and drawing
            let w = self.text.width(piece, size, flags)
                + letter * piece.chars().count() as f32
                + if trailing { space } else { 0.0 };
            let content = Content::Word { src: src as u32, start: a as u32, end: b as u32 };
            let node = self.push(Some(owner), content, Some(owner), sid, child_key(src_key, "#w", n), false);
            let s = Style {
                size: Size { width: Dimension::Length(w.ceil()), height: Dimension::Length(line_h) },
                flex_shrink: 0.0,
                margin: Rect { left: LengthPercentageAuto::Length(if n == 0 { indent } else { 0.0 }),
                               ..Rect::zero() },
                ..Default::default()
            };
            line.push(self.leaf(node, s));
            if brk {
                let br = self.push(Some(owner), Content::LineBreak, Some(owner), sid,
                                   child_key(src_key, "#br", n), false);
                line.push(self.leaf(br, Style {
                    size: Size { width: Dimension::Percent(1.0), height: Dimension::Length(0.0) },
                    flex_shrink: 0.0,
                    ..Default::default()
                }));
            }
        }
        self.nodes[src].content = Content::Text(text);
    }
}

impl Computed {
    /// The few fields `layout_node` reads after building children (avoids
    /// holding a borrow of the style store across recursion).
    fn clone_display(&self) -> DisplayOnly {
        DisplayOnly { display: self.display }
    }
}

struct DisplayOnly {
    display: DisplayKind,
}

fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for w in s.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    out
}

/// Whitespace processing and `text-transform`; borrows when nothing changes.
fn transform(s: &str, t: TextTransform, ws: WhiteSpace) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    let collapse_ws = (ws == WhiteSpace::Normal || ws == WhiteSpace::NoWrap) && s.contains(['\n', '\t', '\r']);
    if !collapse_ws && t == TextTransform::None {
        return Cow::Borrowed(s);
    }
    let s = if collapse_ws { s.replace(['\n', '\t', '\r'], " ") } else { s.to_string() };
    Cow::Owned(match t {
        TextTransform::None => s,
        TextTransform::Upper => s.to_uppercase(),
        TextTransform::Lower => s.to_lowercase(),
        TextTransform::Capitalize => s
            .split(' ')
            .map(|w| {
                let mut c = w.chars();
                c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
            })
            .collect::<Vec<String>>()
            .join(" "),
    })
}

/// Build, cascade and lay out `root` in a `viewport` (logical pixels),
/// reusing the arenas of `recycle` (the previous frame) and `caches`.
pub(crate) fn layout(root: AnyElement, viewport: (f32, f32), state: &UiState, caches: &mut LayoutCaches,
                     recycle: Frame, widgets: &mut dyn FnMut(Key, &WidgetSpec) -> (f32, f32)) -> Frame {
    let mut css = String::new();
    let mut title = None;
    collect_styles(&root, &mut css, &mut title);
    let css_hash = fnv(&[css.as_bytes()]);
    if css_hash != caches.author.hash {
        caches.author = AuthorSheet { hash: css_hash, sheet: Stylesheet::parse(&css) };
    }
    caches.styles.begin_frame();
    // the taffy tree persists: drop last frame's duplicates, start a new stamp
    for id in caches.recon.extra.drain(..) {
        let _ = caches.taffy.remove(id);
    }
    caches.recon.stamp = caches.recon.stamp.wrapping_add(1);
    caches.text_box.clear();
    let Frame { mut nodes, mut els, .. } = recycle;
    nodes.clear();
    els.clear();
    if caches.matched.author != caches.author.hash || caches.matched.map.len() > 50_000 {
        caches.matched.map.clear();
        caches.matched.author = caches.author.hash;
    }
    let LayoutCaches { styles, text, images, author, taffy, box_of, acc, matched, recon, text_box } = caches;
    let mut b = Builder {
        nodes,
        els,
        taffy,
        styles,
        text,
        images,
        matched,
        recon,
        text_box,
        changed: false,
        author: &author.sheet,
        author_hash: author.hash,
        state,
        viewport,
        title,
        widgets,
    };
    // the document: <html><body> unless the app built them itself
    let root_div = match root {
        AnyElement::Element(d) if d.tag == "html" => d,
        AnyElement::Element(d) if d.tag == "body" => crate::html::html().child(d),
        other => {
            let mut body = crate::html::body();
            body.children.push(other);
            crate::html::html().child(body)
        }
    };
    let s = span(Phase::Build);
    let root = match b.element(root_div, None, ROOT_KEY, (1, 1)) {
        Some(r) => r,
        None => b.push(None, Content::Element, None, ROOT_STYLE, ROOT_KEY, false),
    };
    drop(s);
    let s = span(Phase::Boxes);
    let root_id = b.layout_node(root);
    drop(s);
    let s = span(Phase::Taffy);
    // nodes not used this frame
    let stamp = b.recon.stamp;
    let taffy_ref = &mut *b.taffy;
    b.recon.map.retain(|_, (id, st)| {
        if *st == stamp {
            return true;
        }
        let _ = taffy_ref.remove(*id);
        false
    });
    let avail = Size { width: AvailableSpace::Definite(viewport.0),
                       height: AvailableSpace::Definite(viewport.1) };
    let Builder { nodes, els, taffy, styles, text: tc, title, text_box, .. } = b;
    taffy
        .compute_layout_with_measure(root_id, avail, |known, available, _id, ctx, _style| {
            match ctx {
                Some(Measure::Text { key, size, flags, line, nowrap, .. }) => {
                    let text = match text_box.get(key).map(|&k| &nodes[k].content) {
                        Some(Content::Text(t)) => t.as_str(),
                        _ => "",
                    };
                    if !text.contains([' ', '\n']) {
                        // one word: no wrapping to do
                        let w = tc.width(text, *size, *flags);
                        return Size { width: known.width.unwrap_or(w.ceil()),
                                      height: known.height.unwrap_or(*line) };
                    }
                    let max_w = known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(w) => w,
                        AvailableSpace::MinContent => 0.0,
                        AvailableSpace::MaxContent => f32::INFINITY,
                    });
                    let lines = tc.wrap(text, *size, *flags, if *nowrap { f32::INFINITY } else { max_w });
                    let w = lines.iter().map(|l| tc.width(l, *size, *flags)).fold(0.0, f32::max);
                    Size { width: known.width.unwrap_or(w.ceil()),
                           height: known.height.unwrap_or(*line * lines.len().max(1) as f32) }
                }
                None => Size::ZERO,
            }
        })
        .unwrap();
    drop(s);
    let _s = span(Phase::Place);
    let mut frame = Frame { nodes, els, title, by_key: Default::default() };
    box_of.clear();
    for (i, n) in frame.nodes.iter().enumerate() {
        if let Some(t) = n.taffy {
            let s = slot(t);
            if s >= box_of.len() {
                box_of.resize(s + 1, NONE);
            }
            box_of[s] = i as u32;
        }
    }
    let mut walk = Walk { t: taffy, box_of, state, vp: viewport, styles, text: tc };
    walk.place(&mut frame.nodes, root_id, 0.0, 0.0, None);
    finish(&mut frame.nodes, styles, acc);
    frame
}

/// Positions follow the taffy tree (words live in line containers, which may
/// be anonymous); translate and scroll offsets shift whole subtrees.
struct Walk<'a> {
    t: &'a TaffyTree<Measure>,
    box_of: &'a [u32],
    state: &'a UiState,
    vp: (f32, f32),
    styles: &'a StyleStore,
    text: &'a mut TextCache,
}

impl Walk<'_> {
    fn place(&mut self, nodes: &mut [BoxNode], id: NodeId, ox: f32, oy: f32, clip: Option<Bounds<f32>>) {
        let l = match self.t.layout(id) {
            Ok(l) => *l,
            Err(_) => return,
        };
        let (mut x, mut y) = (ox + l.location.x, oy + l.location.y);
        let (w, h) = (l.size.width, l.size.height);
        let mut child_clip = clip;
        let (mut sx, mut sy) = (0.0, 0.0);
        if let Some(i) = self.box_of.get(slot(id)).filter(|&&b| b != NONE).map(|&b| b as usize) {
            let st = &self.styles[nodes[i].style];
            let u = Units { font_size: st.font_size, viewport: self.vp };
            x += st.translate.0.resolve(&u, w);
            y += st.translate.1.resolve(&u, h);
            if matches!(nodes[i].content, Content::Word { .. }) {
                y += match st.vertical_align {
                    VAlign::Sub => st.font_size * 0.25,
                    VAlign::Super => -st.font_size * 0.35,
                    VAlign::Baseline => 0.0,
                };
            }
            nodes[i].bounds = rect(x, y, w, h);
            nodes[i].clip = clip;
            let ov = st.layout.overflow;
            if ov.x != Overflow::Visible || ov.y != Overflow::Visible {
                let b = l.border;
                let pad = rect(x + b.left, y + b.top, (w - b.left - b.right).max(0.0),
                               (h - b.top - b.bottom).max(0.0));
                child_clip = Some(match clip {
                    Some(c) => c.intersect(&pad),
                    None => pad,
                });
                if ov.x == Overflow::Scroll || ov.y == Overflow::Scroll {
                    let axis = |o: Overflow, content: f32, view: f32| {
                        if o == Overflow::Scroll { (content - view).max(0.0) } else { 0.0 }
                    };
                    let range = (axis(ov.x, l.content_size.width + l.padding.right, pad.size.width),
                                 axis(ov.y, l.content_size.height + l.padding.bottom, pad.size.height));
                    nodes[i].scroll_range = range;
                    let (ax, ay) = self.state.scroll.get(&nodes[i].key).copied().unwrap_or((0.0, 0.0));
                    sx = ax.clamp(0.0, range.0);
                    sy = ay.clamp(0.0, range.1);
                    nodes[i].scroll_pos = (sx, sy);
                }
            }
            if let Some(k) = nodes[i].fold {
                place_folded_text(nodes, i, k, rect(x, y, w, h), &l, self.styles, self.text);
            }
        }
        let n = self.t.child_count(id);
        for c in 0..n {
            let k = self.t.child_at_index(id, c).unwrap();
            self.place(nodes, k, x - sx, y - sy, child_clip);
        }
    }
}

/// Put a folded text run where its own flex item / line box would have gone:
/// inside the content box, aligned like a lone flex item (or by `text-align`
/// in a block, which paint applies across the full width).
fn place_folded_text(nodes: &mut [BoxNode], i: usize, k: usize, b: Bounds<f32>, l: &taffy::Layout,
                     styles: &StyleStore, tc: &mut TextCache) {
    let c = &styles[nodes[i].style];
    let cb = rect(b.origin.x + l.border.left + l.padding.left, b.origin.y + l.border.top + l.padding.top,
                  (b.size.width - l.border.left - l.border.right - l.padding.left - l.padding.right).max(0.0),
                  (b.size.height - l.border.top - l.border.bottom - l.padding.top - l.padding.bottom).max(0.0));
    let Content::Text(t) = &nodes[k].content else { return };
    let flags = text::flags(c);
    let line = c.line_px();
    let (tw, th) = if !t.contains([' ', '\n']) {
        (tc.width(t, c.font_size, flags).ceil(), line)
    } else {
        let max = if c.white_space == WhiteSpace::NoWrap { f32::INFINITY } else { cb.size.width + 0.5 };
        let lines = tc.wrap(t, c.font_size, flags, max);
        let w = lines.iter().map(|s| tc.width(s, c.font_size, flags)).fold(0.0, f32::max);
        (w.ceil(), line * lines.len().max(1) as f32)
    };
    let tw = if c.white_space == WhiteSpace::NoWrap { tw } else { tw.min(cb.size.width) };
    let s = &c.layout;
    // offset of a box of `size` in `room` for a start / center / end choice
    let at = |room: f32, size: f32, pos: u8| match pos {
        1 => (room - size) / 2.0,
        2 => room - size,
        _ => 0.0,
    };
    let justify = match s.justify_content {
        Some(JustifyContent::Center | JustifyContent::SpaceAround | JustifyContent::SpaceEvenly) => 1,
        Some(JustifyContent::End | JustifyContent::FlexEnd) => 2,
        _ => 0,
    };
    let align = match s.align_items {
        Some(AlignItems::Center) => 1,
        Some(AlignItems::End | AlignItems::FlexEnd) => 2,
        _ => 0,
    };
    let stretch = matches!(s.align_items, None | Some(AlignItems::Stretch));
    let tb = match c.display {
        DisplayKind::Flex | DisplayKind::InlineFlex if s.flex_direction == FlexDirection::Row => {
            rect(cb.origin.x + at(cb.size.width, tw, justify), cb.origin.y + at(cb.size.height, th, align), tw, th)
        }
        DisplayKind::Flex | DisplayKind::InlineFlex => {
            let w = if stretch { cb.size.width } else { tw };
            rect(cb.origin.x + at(cb.size.width, w, align), cb.origin.y + at(cb.size.height, th, justify), w, th)
        }
        _ => rect(cb.origin.x, cb.origin.y, cb.size.width, th), // block: text-align at paint
    };
    nodes[k].bounds = tb;
}

/// Inherited opacity / visibility (parents precede children in the arena),
/// and bounds of boxes without a layout node (inline elements): the union of
/// their pieces (children follow parents, so a reverse pass sees them first).
fn finish(nodes: &mut [BoxNode], styles: &StyleStore, acc: &mut Vec<Option<Bounds<f32>>>) {
    for i in 0..nodes.len() {
        let (po, pv, pc) = match nodes[i].parent {
            Some(p) => (nodes[p].opacity, nodes[p].visible, nodes[p].clip),
            None => (1.0, true, None),
        };
        let st = &styles[nodes[i].style];
        let own = matches!(nodes[i].content, Content::Element | Content::Widget(_) | Content::Replaced(_));
        nodes[i].opacity = if own { po * st.opacity } else { po };
        nodes[i].visible = pv && st.visible;
        if nodes[i].taffy.is_none() {
            nodes[i].clip = pc;
        }
    }
    acc.clear();
    acc.resize(nodes.len(), None);
    for i in (0..nodes.len()).rev() {
        if nodes[i].taffy.is_none() {
            if let Some(a) = acc[i] {
                nodes[i].bounds = a;
            }
        }
        let Some(p) = nodes[i].parent else { continue };
        if nodes[p].taffy.is_some() {
            continue;
        }
        let cb = nodes[i].bounds;
        if cb.size.width <= 0.0 && cb.size.height <= 0.0 {
            continue;
        }
        acc[p] = Some(match acc[p] {
            None => cb,
            Some(a) => {
                let x0 = a.origin.x.min(cb.origin.x);
                let y0 = a.origin.y.min(cb.origin.y);
                let x1 = (a.origin.x + a.size.width).max(cb.origin.x + cb.size.width);
                let y1 = (a.origin.y + a.size.height).max(cb.origin.y + cb.size.height);
                rect(x0, y0, x1 - x0, y1 - y0)
            }
        });
    }
}

impl Frame {
    /// Element boxes by key, built on first use (events and tests need it,
    /// rendering doesn't).
    pub(crate) fn by_key(&self) -> &HashMap<Key, usize> {
        self.by_key.get_or_init(|| {
            self.nodes.iter().enumerate().filter(|(_, n)| n.el != NONE).map(|(i, n)| (n.key, i)).collect()
        })
    }

    /// The element data of box `i`.
    pub(crate) fn el(&self, i: usize) -> Option<&ElData> {
        let e = self.nodes[i].el;
        (e != NONE).then(|| &self.els[e as usize])
    }

    pub(crate) fn attr(&self, i: usize, name: &str) -> Option<&str> {
        el_attr(&self.nodes, &self.els, i, name)
    }

    /// The text a box draws: a text run's text, or a word's slice of it.
    pub(crate) fn text(&self, i: usize) -> &str {
        match &self.nodes[i].content {
            Content::Text(t) => t,
            Content::Word { src, start, end } => match &self.nodes[*src as usize].content {
                Content::Text(t) => &t[*start as usize..*end as usize],
                _ => "",
            },
            _ => "",
        }
    }

    /// Element boxes from the root down to the topmost box under (x, y).
    pub(crate) fn hit_path(&self, x: f32, y: f32, styles: &StyleStore) -> Vec<usize> {
        let mut best: Option<(i32, usize)> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            if !n.visible || n.taffy.is_none() || !styles[n.style].pointer_events {
                continue;
            }
            if n.clip.is_some_and(|c| !c.contains(x, y)) || !n.bounds.contains(x, y) {
                continue;
            }
            // later boxes paint over earlier ones; z-index lifts a subtree
            let z = self.z_of(i, styles);
            if best.is_none_or(|(bz, _)| z >= bz) {
                best = Some((z, i));
            }
        }
        let mut path = Vec::new();
        let mut cur = best.map(|(_, b)| self.nodes[b].owner);
        while let Some(i) = cur {
            if self.nodes[i].el != NONE {
                path.push(i);
            }
            cur = self.nodes[i].parent;
        }
        path.reverse();
        path
    }

    /// The effective stacking level of a box (its nearest z-index).
    pub(crate) fn z_of(&self, mut i: usize, styles: &StyleStore) -> i32 {
        loop {
            if let Some(z) = styles[self.nodes[i].style].z_index {
                return z;
            }
            match self.nodes[i].parent {
                Some(p) => i = p,
                None => return 0,
            }
        }
    }
}
