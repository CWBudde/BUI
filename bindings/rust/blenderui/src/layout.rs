//! From an element tree to positioned boxes: cascade, box tree, taffy layout.
//!
//! * Block containers with inline content become wrapping flex lines of word
//!   boxes (so text, links and inline-blocks wrap like a browser's line
//!   boxes); inline elements own their words for painting and hit testing.
//! * Flex and grid containers get text runs as measured, wrapping leaves.
//! * Tables map onto flex rows and columns, lists paint their markers.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use taffy::prelude::*;
use taffy::style::Overflow;

use crate::css::{Decl, Rule, Stylesheet, Subject};
use crate::element::{AnyElement, Div, Listeners, PaintFn, ParentElement};
use crate::geometry::{rect, Bounds, Units};
use crate::html::{tag_kind, ua_stylesheet, TagKind};
use crate::paint::Image;
use crate::style::{apply, Computed, DisplayKind, TextAlign, TextTransform, VAlign, WhiteSpace};
use crate::text;
use crate::widgets::WidgetSpec;

/// Interaction state the cascade reads (`:hover`, `:active`, `:focus`, `details[open]`).
#[derive(Default)]
pub(crate) struct UiState {
    pub hovered: HashSet<String>,
    pub active: HashSet<String>,
    pub focused: Option<String>,
    pub details_open: HashMap<String, bool>,
    pub scroll: HashMap<String, (f32, f32)>,
    pub checked: HashMap<String, bool>,
}

/// What a box holds.
pub(crate) enum Content {
    Element,
    /// A word (or pre-formatted line) inside a line container.
    Word(String),
    /// A whole text run in a flex/grid container (wraps when measured).
    Text(String),
    LineBreak,
    Widget(WidgetSpec),
    Replaced(Replaced),
    /// An anonymous line container (inline content between blocks).
    Anon,
}

pub(crate) enum Replaced {
    Image(Rc<Image>),
    Painted(PaintFn),
    Placeholder(&'static str),
}

/// The element-level data kept from a `Div` after building.
pub(crate) struct ElData {
    pub tag: &'static str,
    pub id: Option<String>,
    pub attrs: Vec<(String, String)>,
    pub listeners: Listeners,
}

pub(crate) struct BoxNode {
    pub parent: Option<usize>,
    pub kids: Vec<usize>,
    pub content: Content,
    pub el: Option<Rc<ElData>>,
    /// The element this box belongs to (itself, or the inline element of a word).
    pub owner: usize,
    pub style: Rc<Computed>,
    pub key: String,
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
}

impl BoxNode {
    pub(crate) fn attr(&self, name: &str) -> Option<&str> {
        self.el.as_ref()?.attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

/// A laid-out frame.
pub(crate) struct Frame {
    pub nodes: Vec<BoxNode>,
    pub title: Option<String>,
    by_key: std::cell::OnceCell<HashMap<String, usize>>,
}

enum Measure {
    /// The text of box `node` (already whitespace-processed and transformed).
    Text { node: usize, size: f32, flags: i32, line: f32, nowrap: bool },
}

struct Builder<'a> {
    nodes: Vec<BoxNode>,
    taffy: TaffyTree<Measure>,
    author: Stylesheet,
    author_hash: u64,
    state: &'a UiState,
    viewport: (f32, f32),
    title: Option<String>,
    widgets: &'a mut dyn FnMut(&str, &WidgetSpec) -> (f32, f32),
}

// -- style sharing ---------------------------------------------------------------------

/// What a computed style depends on. Holding the parent `Rc` and the value
/// `Arc`s keeps their addresses unique while the entry lives, so they are
/// compared by pointer.
struct StyleKey {
    parent: Rc<Computed>,
    viewport: (u32, u32),
    author: u64,
    rules: Vec<usize>,
    decls: Vec<(std::borrow::Cow<'static, str>, std::sync::Arc<[crate::css::Tok]>, bool)>,
    fixup: u8,
}

impl PartialEq for StyleKey {
    fn eq(&self, o: &Self) -> bool {
        Rc::ptr_eq(&self.parent, &o.parent)
            && self.viewport == o.viewport
            && self.author == o.author
            && self.fixup == o.fixup
            && self.rules == o.rules
            && self.decls.len() == o.decls.len()
            && self.decls.iter().zip(&o.decls).all(|(a, b)| {
                a.2 == b.2 && std::sync::Arc::ptr_eq(&a.1, &b.1) && a.0 == b.0
            })
    }
}
impl Eq for StyleKey {}

impl std::hash::Hash for StyleKey {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        (Rc::as_ptr(&self.parent) as usize).hash(h);
        self.viewport.hash(h);
        self.author.hash(h);
        self.fixup.hash(h);
        self.rules.hash(h);
        for (p, v, imp) in &self.decls {
            p.hash(h);
            (std::sync::Arc::as_ptr(v) as *const () as usize).hash(h);
            imp.hash(h);
        }
    }
}

thread_local! {
    static STYLES: RefCell<crate::style::FnvMap<StyleKey, Rc<Computed>>> = RefCell::new(Default::default());
}

// -- selector subject -------------------------------------------------------------------

#[derive(Clone)]
struct NodeRef<'b> {
    nodes: &'b [BoxNode],
    state: &'b UiState,
    i: usize,
}

impl Subject for NodeRef<'_> {
    fn tag(&self) -> &str {
        self.nodes[self.i].el.as_ref().map(|e| e.tag).unwrap_or("")
    }
    fn id(&self) -> Option<&str> {
        self.nodes[self.i].el.as_ref()?.id.as_deref()
    }
    fn has_class(&self, class: &str) -> bool {
        self.nodes[self.i].attr("class").is_some_and(|c| c.split_whitespace().any(|w| w == class))
    }
    fn attr(&self, name: &str) -> Option<&str> {
        let n = &self.nodes[self.i];
        if name == "open" && n.el.as_ref().is_some_and(|e| e.tag == "details") {
            return self.state.details_open.get(&n.key).copied()
                .unwrap_or(n.attr("open").is_some()).then_some("");
        }
        n.attr(name)
    }
    fn state(&self, pseudo: &str) -> bool {
        let n = &self.nodes[self.i];
        match pseudo {
            "hover" => self.state.hovered.contains(&n.key),
            "active" => self.state.active.contains(&n.key),
            "focus" | "focus-visible" => self.state.focused.as_deref() == Some(n.key.as_str()),
            "focus-within" => self.state.focused.as_deref().is_some_and(|f| f.starts_with(&n.key)),
            "checked" => self.state.checked.get(&n.key).copied().unwrap_or(n.attr("checked").is_some()),
            "disabled" => n.attr("disabled").is_some(),
            "enabled" => n.attr("disabled").is_none(),
            "link" | "any-link" => self.tag() == "a" && n.attr("href").is_some(),
            "required" => n.attr("required").is_some(),
            "optional" => n.attr("required").is_none(),
            "read-only" => n.attr("readonly").is_some(),
            "read-write" => n.attr("readonly").is_none(),
            "open" => self.attr("open").is_some(),
            "placeholder-shown" => n.attr("placeholder").is_some() && n.attr("value").is_none_or(str::is_empty),
            _ => false,
        }
    }
    fn parent(&self) -> Option<Self> {
        self.nodes[self.i].parent.map(|p| NodeRef { i: p, ..self.clone() })
    }
    fn prev_sibling(&self) -> Option<Self> {
        let p = self.nodes[self.i].parent?;
        let sibs = &self.nodes[p].kids;
        let k = sibs.iter().position(|&s| s == self.i)?;
        sibs[..k].iter().rev().find(|&&s| self.nodes[s].el.is_some())
            .map(|&s| NodeRef { i: s, ..self.clone() })
    }
    fn position(&self) -> (usize, usize) {
        self.nodes[self.i].pos
    }
    fn is_empty(&self) -> bool {
        self.nodes[self.i].kids.is_empty()
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

impl Builder<'_> {
    fn subject(&self, i: usize) -> NodeRef<'_> {
        NodeRef { nodes: &self.nodes, state: self.state, i }
    }

    /// The computed style of element `i`, shared with every earlier element
    /// that had the same parent style, matched rules and declarations
    /// (browsers call this style sharing). `fixup` adjusts `display` for
    /// widgets and flex/grid items before the result is cached.
    fn cascade(&self, i: usize, div: &Div, parent: &Rc<Computed>, fixup: u8) -> Rc<Computed> {
        let _s = crate::prof::span("  cascade");
        let subj = self.subject(i);
        let ua = ua_stylesheet().matching(&subj, self.viewport);
        let au = self.author.matching(&subj, self.viewport);
        let decls = self.declared(i, div, &ua, &au);
        let key = StyleKey {
            parent: parent.clone(),
            viewport: (self.viewport.0.to_bits(), self.viewport.1.to_bits()),
            author: self.author_hash,
            rules: ua.iter().map(|r| *r as *const Rule as usize).chain(au.iter().map(|r| r.order)).collect(),
            decls: decls.iter().map(|d| (d.prop.clone(), d.value.clone(), d.important)).collect(),
            fixup,
        };
        if let Some(hit) = STYLES.with(|c| c.borrow().get(&key).cloned()) {
            return hit;
        }
        let mut c = self.compute(parent, &decls);
        match fixup {
            1 if c.display == DisplayKind::Inline => c.display = DisplayKind::InlineBlock,
            2 if c.display == DisplayKind::Inline => c.display = DisplayKind::Block,
            _ => {}
        }
        let c = Rc::new(c);
        STYLES.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.len() > 20_000 {
                cache.clear();
            }
            cache.insert(key, c.clone());
        });
        c
    }

    /// Every declaration that applies to element `i`, in cascade order.
    fn declared<'d>(&self, i: usize, div: &'d Div, ua: &[&'d Rule], au: &[&'d Rule]) -> Vec<&'d Decl> {
        let key = &self.nodes[i].key;
        let mut decls: Vec<&Decl> = Vec::new();
        decls.extend(ua.iter().flat_map(|r| r.decls.iter()));
        decls.extend(au.iter().flat_map(|r| r.decls.iter()));
        decls.extend(div.decls.iter());
        if self.state.hovered.contains(key) {
            decls.extend(div.hover.iter());
        }
        if self.state.active.contains(key) {
            decls.extend(div.active.iter());
        }
        if self.state.focused.as_deref() == Some(key.as_str()) {
            decls.extend(div.focus.iter());
        }
        decls
    }

    fn compute(&self, parent: &Computed, decls: &[&Decl]) -> Computed {
        let mut c = Computed::inherit(parent);
        let font_first = |d: &&Decl| d.prop == "font-size" || d.prop == "font";
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

    fn push(&mut self, parent: Option<usize>, content: Content, owner: Option<usize>,
            style: Rc<Computed>, key: String) -> usize {
        let i = self.nodes.len();
        self.nodes.push(BoxNode {
            parent,
            kids: Vec::new(),
            content,
            el: None,
            owner: owner.unwrap_or(i),
            style,
            key,
            taffy: None,
            pos: (1, 1),
            marker: None,
            bounds: rect(0.0, 0.0, 0.0, 0.0),
            clip: None,
            opacity: 1.0,
            visible: true,
            scroll_range: (0.0, 0.0),
            scroll_pos: (0.0, 0.0),
        });
        if let Some(p) = parent {
            self.nodes[p].kids.push(i);
        }
        i
    }

    /// Build an element; returns its index (None when not rendered).
    fn element(&mut self, mut div: Div, parent: Option<usize>, key: String, pos: (usize, usize))
        -> Option<usize> {
        let parent_style = parent.map(|p| self.nodes[p].style.clone())
            .unwrap_or_else(|| root_parent());
        let i = self.push(parent, Content::Element, None, parent_style.clone(), key);
        self.nodes[i].pos = pos;
        self.nodes[i].el = Some(Rc::new(ElData {
            tag: div.tag,
            id: div.id.as_ref().map(|s| s.to_string()),
            attrs: div.attrs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            listeners: std::mem::take(&mut div.listeners),
        }));
        let kind = tag_kind(div.tag);
        // widgets and replaced elements are atomic inlines; flex/grid items are blockified
        let fixup = if kind == TagKind::Widget || kind == TagKind::Replaced {
            1
        } else if parent.is_some_and(|p| matches!(self.nodes[p].style.display,
                                                  DisplayKind::Flex | DisplayKind::InlineFlex
                                                  | DisplayKind::Grid | DisplayKind::InlineGrid)) {
            2
        } else {
            0
        };
        let style = self.cascade(i, &div, &parent_style, fixup);
        if style.display == DisplayKind::None || kind == TagKind::Metadata {
            self.nodes.pop();
            if let Some(p) = parent {
                self.nodes[p].kids.pop();
            }
            return None;
        }
        self.nodes[i].style = style;
        match kind {
            TagKind::Widget => {
                let spec = WidgetSpec::from_element(&div, &self.nodes[i].key, self.state);
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
                    div.get_attr("src").and_then(crate::paint::load_image)
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
                    let (open, close) = self.nodes[i].style.quotes.clone()
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
        if self.nodes[i].style.display == DisplayKind::ListItem {
            let ls = self.nodes[i].style.list_style;
            let tag = self.nodes[i].el.as_ref().map(|e| e.tag).unwrap_or("");
            let marker = if tag == "summary" {
                let details = self.nodes[i].parent.unwrap_or(i);
                Some(if self.subject(details).attr("open").is_some() { "\u{25be}" } else { "\u{25b8}" }.into())
            } else {
                let idx = self.nodes[i].pos.0 + self.nodes[i].parent
                    .and_then(|p| self.nodes[p].attr("start"))
                    .and_then(|s| s.parse::<usize>().ok()).map(|s| s - 1).unwrap_or(0);
                ls.marker(idx)
            };
            match marker {
                Some(m) if self.nodes[i].style.marker_inside => {
                    children.insert(0, AnyElement::Text(format!("{m}\u{a0}").into()));
                }
                m => self.nodes[i].marker = m,
            }
        }
        let count = children.iter().filter(|c| matches!(c, AnyElement::Element(_))).count();
        let mut k = 0;
        for child in children {
            match child {
                AnyElement::Element(d) => {
                    k += 1;
                    let key = match &d.id {
                        Some(id) => format!("#{id}"),
                        None => format!("{}/{}{}", self.nodes[i].key, d.tag, k),
                    };
                    self.element(*d, Some(i), key, (k, count));
                }
                AnyElement::Text(t) => {
                    let key = format!("{}/t{}", self.nodes[i].key, self.nodes[i].kids.len());
                    let style = self.nodes[i].style.clone();
                    self.push(Some(i), Content::Text(t.into_owned()), Some(i), style, key);
                }
            }
        }
    }

    // -- taffy ------------------------------------------------------------------------------

    fn is_block_level(&self, i: usize) -> bool {
        match &self.nodes[i].content {
            Content::Element => !self.nodes[i].style.display.is_inline_level(),
            Content::Widget(_) | Content::Replaced(_) => false,
            _ => false,
        }
    }

    fn base_style(&self, i: usize) -> Style {
        let c = &self.nodes[i].style;
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
            let span = self.nodes[i].attr("colspan").and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
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

    fn leaf(&mut self, i: usize, style: Style) -> NodeId {
        let id = self.taffy.new_leaf(style).unwrap();
        self.nodes[i].taffy = Some(id);
        id
    }

    /// The taffy node for box `i` (and its subtree).
    fn layout_node(&mut self, i: usize) -> NodeId {
        let base = self.base_style(i);
        let c = self.nodes[i].style.clone();
        match &self.nodes[i].content {
            Content::Widget(spec) => {
                let (w, h) = (self.widgets)(&self.nodes[i].key, spec);
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
                let attr = |n: &str| self.nodes[i].attr(n).and_then(|v| v.trim_end_matches("px").parse::<f32>().ok());
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
            Content::Text(t) => {
                // a text run as a flex/grid item: measured, wrapping. The box
                // keeps the transformed text, so paint draws what was measured.
                if let std::borrow::Cow::Owned(s) = transform(t, c.text_transform, c.white_space) {
                    self.nodes[i].content = Content::Text(s);
                }
                let m = Measure::Text {
                    node: i,
                    size: c.font_size,
                    flags: text::flags(&c),
                    line: c.line_px(),
                    nowrap: matches!(c.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre),
                };
                let id = self.taffy.new_leaf_with_context(Style::default(), m).unwrap();
                self.nodes[i].taffy = Some(id);
                return id;
            }
            _ => {}
        }
        let flex_like = matches!(c.display, DisplayKind::Flex | DisplayKind::InlineFlex
                                 | DisplayKind::Grid | DisplayKind::InlineGrid
                                 | DisplayKind::Table | DisplayKind::TableRowGroup
                                 | DisplayKind::TableRow);
        let kids = self.nodes[i].kids.clone();
        let mut kids = kids;
        kids.sort_by_key(|&k| {
            let st = &self.nodes[k].style;
            if st.display == DisplayKind::TableCaption && st.caption_bottom { i32::MAX } else { st.order }
        });
        let mut out: Vec<NodeId> = Vec::new();
        if flex_like {
            for k in kids {
                if let Content::Text(t) = &self.nodes[k].content {
                    if t.trim().is_empty() && self.nodes[k].style.white_space == WhiteSpace::Normal {
                        continue;
                    }
                }
                out.extend(self.child_nodes(k));
            }
            let id = self.taffy.new_with_children(base, &out).unwrap();
            self.nodes[i].taffy = Some(id);
            return id;
        }
        // block container: runs of inline content become line boxes
        let all_inline = kids.iter().all(|&k| !self.is_block_level(k));
        if all_inline {
            let mut line = Vec::new();
            self.inline_items(&kids, i, &mut line, true);
            let id = self.taffy.new_with_children(Self::line_style(&c, base), &line).unwrap();
            self.nodes[i].taffy = Some(id);
            return id;
        }
        let mut run: Vec<usize> = Vec::new();
        for k in kids {
            if self.is_block_level(k) {
                self.flush_run(i, &mut run, &mut out);
                out.extend(self.child_nodes(k));
            } else {
                run.push(k);
            }
        }
        self.flush_run(i, &mut run, &mut out);
        let id = self.taffy.new_with_children(base, &out).unwrap();
        self.nodes[i].taffy = Some(id);
        id
    }

    fn child_nodes(&mut self, k: usize) -> Vec<NodeId> {
        if self.nodes[k].style.display == DisplayKind::Contents {
            let kids = self.nodes[k].kids.clone();
            return kids.into_iter().flat_map(|c| self.child_nodes(c)).collect();
        }
        vec![self.layout_node(k)]
    }

    /// An anonymous line box for inline content between blocks.
    fn flush_run(&mut self, parent: usize, run: &mut Vec<usize>, out: &mut Vec<NodeId>) {
        let only_space = run.iter().all(|&k| matches!(&self.nodes[k].content,
                                                      Content::Text(t) if t.trim().is_empty()));
        if run.is_empty() || only_space {
            run.clear();
            return;
        }
        let style = self.nodes[parent].style.clone();
        let key = format!("{}/anon{}", self.nodes[parent].key, out.len());
        let anon = self.push(None, Content::Anon, Some(parent), style.clone(), key);
        self.nodes[anon].parent = Some(parent);
        let mut line = Vec::new();
        let items: Vec<usize> = std::mem::take(run);
        self.inline_items(&items, parent, &mut line, true);
        let id = self.taffy.new_with_children(Self::line_style(&style, Style::default()), &line).unwrap();
        self.nodes[anon].taffy = Some(id);
        out.push(id);
    }

    /// Flatten inline content into line items: words, atomic inline boxes, breaks.
    fn inline_items(&mut self, kids: &[usize], owner: usize, line: &mut Vec<NodeId>, first: bool) {
        let mut first_word = first;
        for &k in kids {
            match &self.nodes[k].content {
                Content::Text(_) => {
                    let Content::Text(t) = std::mem::replace(&mut self.nodes[k].content, Content::Anon) else {
                        unreachable!()
                    };
                    let words = self.words(k, owner, &t, first_word);
                    self.nodes[k].content = Content::Text(t);
                    if !words.is_empty() {
                        first_word = false;
                    }
                    line.extend(words);
                }
                Content::Element if self.nodes[k].style.display == DisplayKind::Inline
                    || self.nodes[k].style.display == DisplayKind::Contents => {
                    let kids = self.nodes[k].kids.clone();
                    self.inline_items(&kids, k, line, first_word);
                    first_word = false;
                }
                _ => {
                    line.extend(self.child_nodes(k));
                    first_word = false;
                }
            }
        }
    }

    /// Word boxes for a text node, with the style of its inline owner.
    fn words(&mut self, text_node: usize, owner: usize, t: &str, first: bool) -> Vec<NodeId> {
        let c = self.nodes[owner].style.clone();
        let text = transform(t, c.text_transform, c.white_space);
        let flags = text::flags(&c);
        let line_h = c.line_px();
        let space = text::width(" ", c.font_size, flags) + c.word_spacing;
        let mut pieces: Vec<(String, bool, bool)> = Vec::new(); // text, trailing space, forced break after
        match c.white_space {
            WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine => {
                let lines: Vec<&str> = text.split('\n').collect();
                for (n, l) in lines.iter().enumerate() {
                    let brk = n + 1 < lines.len();
                    if c.white_space == WhiteSpace::Pre {
                        pieces.push((l.to_string(), false, brk));
                    } else {
                        let ws: Vec<&str> = l.split_whitespace().collect();
                        if ws.is_empty() {
                            pieces.push((String::new(), false, brk));
                        }
                        for (j, w) in ws.iter().enumerate() {
                            pieces.push((w.to_string(), j + 1 < ws.len(), brk && j + 1 == ws.len()));
                        }
                    }
                }
            }
            WhiteSpace::NoWrap => pieces.push((collapse(&text), false, false)),
            WhiteSpace::Normal => {
                let lead = text.starts_with(char::is_whitespace) && !first;
                let ws: Vec<&str> = text.split_whitespace().collect();
                let trail = text.ends_with(char::is_whitespace);
                for (j, w) in ws.iter().enumerate() {
                    let space_after = j + 1 < ws.len() || trail;
                    let word = if j == 0 && lead { format!(" {w}") } else { w.to_string() };
                    pieces.push((word, space_after, false));
                }
            }
        }
        let indent = if first { c.text_indent.resolve(&self.units(c.font_size), 0.0) } else { 0.0 };
        let mut ids = Vec::new();
        for (n, (word, trailing, brk)) in pieces.into_iter().enumerate() {
            let w = text::width(&word, c.font_size, flags)
                + c.letter_spacing * word.chars().count() as f32
                + if trailing { space } else { 0.0 };
            let key = format!("{}/w{}", self.nodes[text_node].key, n);
            let node = self.push(Some(owner), Content::Word(word), Some(owner), c.clone(), key);
            self.nodes[node].parent = Some(owner);
            let s = Style {
                size: Size { width: Dimension::Length(w.ceil()), height: Dimension::Length(line_h) },
                flex_shrink: 0.0,
                margin: Rect { left: LengthPercentageAuto::Length(if n == 0 { indent } else { 0.0 }),
                               ..Rect::zero() },
                ..Default::default()
            };
            ids.push(self.leaf(node, s));
            if brk {
                let b = self.push(Some(owner), Content::LineBreak, Some(owner), c.clone(),
                                  format!("{}/br{}", self.nodes[text_node].key, n));
                ids.push(self.leaf(b, Style {
                    size: Size { width: Dimension::Percent(1.0), height: Dimension::Length(0.0) },
                    flex_shrink: 0.0,
                    ..Default::default()
                }));
            }
        }
        ids
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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

/// Build, cascade and lay out `root` in a `viewport` (logical pixels).
pub(crate) fn layout(root: AnyElement, viewport: (f32, f32), state: &UiState,
                     widgets: &mut dyn FnMut(&str, &WidgetSpec) -> (f32, f32)) -> Frame {
    let mut css = String::new();
    let mut title = None;
    collect_styles(&root, &mut css, &mut title);
    let mut b = Builder {
        nodes: Vec::new(),
        taffy: TaffyTree::new(),
        author: Stylesheet::parse(&css),
        author_hash: {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            css.hash(&mut h);
            h.finish()
        },
        state,
        viewport,
        title,
        widgets,
    };
    // the document: <html><body> unless the app built them itself
    let root_div = match root {
        AnyElement::Element(d) if d.tag == "html" => *d,
        AnyElement::Element(d) if d.tag == "body" => crate::html::html().child(*d),
        other => {
            let mut body = crate::html::body();
            body.children.push(other);
            crate::html::html().child(body)
        }
    };
    let span = crate::prof::span("build");
    let root = match b.element(root_div, None, "html".into(), (1, 1)) {
        Some(r) => r,
        None => b.push(None, Content::Element, None, root_parent(), "html".into()),
    };
    drop(span);
    let span = crate::prof::span("boxes");
    let root_id = b.layout_node(root);
    drop(span);
    let span = crate::prof::span("taffy");
    // the root fills the viewport
    let mut rs = b.taffy.style(root_id).unwrap().clone();
    rs.min_size = Size { width: Dimension::Length(viewport.0), height: Dimension::Length(viewport.1) };
    b.taffy.set_style(root_id, rs).unwrap();
    let avail = Size { width: AvailableSpace::Definite(viewport.0),
                       height: AvailableSpace::Definite(viewport.1) };
    let nodes = &b.nodes;
    b.taffy
        .compute_layout_with_measure(root_id, avail, |known, available, _id, ctx, _style| {
            match ctx {
                Some(Measure::Text { node, size, flags, line, nowrap }) => {
                    let text = match &nodes[*node].content {
                        Content::Text(t) => t.as_str(),
                        _ => "",
                    };
                    if !text.contains([' ', '\n']) {
                        // one word: no wrapping to do
                        let w = text::width(text, *size, *flags);
                        return Size { width: known.width.unwrap_or(w.ceil()),
                                      height: known.height.unwrap_or(*line) };
                    }
                    let max_w = known.width.unwrap_or(match available.width {
                        AvailableSpace::Definite(w) => w,
                        AvailableSpace::MinContent => 0.0,
                        AvailableSpace::MaxContent => f32::INFINITY,
                    });
                    let lines = text::wrap(text, *size, *flags,
                                           if *nowrap { f32::INFINITY } else { max_w });
                    let w = lines.iter().map(|l| text::width(l, *size, *flags)).fold(0.0, f32::max);
                    Size { width: known.width.unwrap_or(w.ceil()),
                           height: known.height.unwrap_or(*line * lines.len().max(1) as f32) }
                }
                None => Size::ZERO,
            }
        })
        .unwrap();
    drop(span);
    let _span = crate::prof::span("place");
    let mut frame = Frame { nodes: b.nodes, title: b.title, by_key: Default::default() };
    let by_taffy: HashMap<NodeId, usize> = frame.nodes.iter().enumerate()
        .filter_map(|(i, n)| n.taffy.map(|t| (t, i))).collect();
    let mut walk = Walk { t: &b.taffy, by_taffy: &by_taffy, state, vp: viewport };
    walk.place(&mut frame.nodes, root_id, 0.0, 0.0, None);
    finish(&mut frame.nodes);
    frame
}

/// Positions follow the taffy tree (words live in line containers, which may
/// be anonymous); translate and scroll offsets shift whole subtrees.
struct Walk<'a> {
    t: &'a TaffyTree<Measure>,
    by_taffy: &'a HashMap<NodeId, usize>,
    state: &'a UiState,
    vp: (f32, f32),
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
        if let Some(&i) = self.by_taffy.get(&id) {
            let st = nodes[i].style.clone();
            let u = Units { font_size: st.font_size, viewport: self.vp };
            x += st.translate.0.resolve(&u, w);
            y += st.translate.1.resolve(&u, h);
            if matches!(nodes[i].content, Content::Word(_)) {
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
        }
        let kids = self.t.children(id).unwrap_or_default();
        for k in kids {
            self.place(nodes, k, x - sx, y - sy, child_clip);
        }
    }
}

/// Inherited opacity / visibility (parents precede children in the arena),
/// and bounds of inline elements: the union of their pieces (children follow
/// parents, so a reverse pass sees them first).
fn finish(nodes: &mut [BoxNode]) {
    for i in 0..nodes.len() {
        let (po, pv, pc) = match nodes[i].parent {
            Some(p) => (nodes[p].opacity, nodes[p].visible, nodes[p].clip),
            None => (1.0, true, None),
        };
        let own = matches!(nodes[i].content, Content::Element | Content::Widget(_) | Content::Replaced(_));
        nodes[i].opacity = if own { po * nodes[i].style.opacity } else { po };
        nodes[i].visible = pv && nodes[i].style.visible;
        if nodes[i].taffy.is_none() {
            nodes[i].clip = pc;
        }
    }
    for i in (0..nodes.len()).rev() {
        if nodes[i].taffy.is_some() || nodes[i].kids.is_empty() {
            continue;
        }
        let mut acc: Option<Bounds<f32>> = None;
        for &c in &nodes[i].kids {
            let cb = nodes[c].bounds;
            if cb.size.width <= 0.0 && cb.size.height <= 0.0 {
                continue;
            }
            acc = Some(match acc {
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
        if let Some(a) = acc {
            nodes[i].bounds = a;
        }
    }
}

impl Frame {
    /// Element boxes by key, built on first use (events and tests need it,
    /// rendering doesn't).
    pub(crate) fn by_key(&self) -> &HashMap<String, usize> {
        self.by_key.get_or_init(|| {
            self.nodes.iter().enumerate().filter(|(_, n)| n.el.is_some())
                .map(|(i, n)| (n.key.clone(), i)).collect()
        })
    }

    /// Element boxes from the root down to the topmost box under (x, y).
    pub(crate) fn hit_path(&self, x: f32, y: f32) -> Vec<usize> {
        let mut best: Option<(i32, usize)> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            if !n.visible || !n.style.pointer_events || n.taffy.is_none() {
                continue;
            }
            if n.clip.is_some_and(|c| !c.contains(x, y)) || !n.bounds.contains(x, y) {
                continue;
            }
            // later boxes paint over earlier ones; z-index lifts a subtree
            let z = self.z_of(i);
            if best.is_none_or(|(bz, _)| z >= bz) {
                best = Some((z, i));
            }
        }
        let mut path = Vec::new();
        let mut cur = best.map(|(_, b)| self.nodes[b].owner);
        while let Some(i) = cur {
            if self.nodes[i].el.is_some() {
                path.push(i);
            }
            cur = self.nodes[i].parent;
        }
        path.reverse();
        path
    }

    /// The effective stacking level of a box (its nearest z-index).
    pub(crate) fn z_of(&self, mut i: usize) -> i32 {
        loop {
            if let Some(z) = self.nodes[i].style.z_index {
                return z;
            }
            match self.nodes[i].parent {
                Some(p) => i = p,
                None => return 0,
            }
        }
    }
}

/// The style the root element inherits from: one shared value, so the root's
/// style (and through it everything below) can be reused across frames.
fn root_parent() -> Rc<Computed> {
    thread_local! { static ROOT: Rc<Computed> = Rc::new(Computed::default()); }
    ROOT.with(Rc::clone)
}
