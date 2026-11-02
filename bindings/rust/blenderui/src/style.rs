//! Computed style and the one table that applies every CSS property.
//!
//! Layout properties land directly in a `taffy::Style`; paint and text
//! properties in [`Computed`]. Shorthands call the longhand handlers, so
//! each property has exactly one implementation.

use std::collections::HashMap;
use std::sync::OnceLock;

use taffy::geometry::MinMax;
use taffy::prelude::*;
use taffy::style::Overflow;

use crate::color::{parse_color, Rgba};
use crate::css::{Decl, Tok};
use crate::geometry::{Length, Units};

// ==========================================================================
// Computed style

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayKind {
    None,
    Block,
    Inline,
    InlineBlock,
    Flex,
    InlineFlex,
    Grid,
    InlineGrid,
    Contents,
    ListItem,
    Table,
    TableRowGroup,
    TableRow,
    TableCell,
    TableCaption,
}

impl DisplayKind {
    pub fn is_inline_level(self) -> bool {
        matches!(self, DisplayKind::Inline | DisplayKind::InlineBlock | DisplayKind::InlineFlex
                 | DisplayKind::InlineGrid)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WhiteSpace {
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextTransform {
    None,
    Upper,
    Lower,
    Capitalize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Normal,
    Factor(f32),
    Px(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ListStyle {
    None,
    Disc,
    Circle,
    Square,
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VAlign {
    Baseline,
    Sub,
    Super,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub radial: bool,
    pub angle_deg: f32,
    pub stops: Vec<(Rgba, Option<f32>)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Rgba,
    pub inset: bool,
}

/// The style of one element after the cascade.
#[derive(Clone, Debug)]
pub struct Computed {
    pub display: DisplayKind,
    pub layout: Style,
    pub order: i32,
    // paint (not inherited)
    pub background: Rgba,
    pub gradient: Option<Gradient>,
    pub border_color: [Rgba; 4],
    pub border_drawn: [bool; 4],
    pub radius: [Length; 4],
    pub shadows: Vec<Shadow>,
    pub outline: (f32, Rgba, f32),
    pub opacity: f32,
    pub z_index: Option<i32>,
    pub translate: (Length, Length),
    pub scale: f32,
    // inherited
    pub visible: bool,
    pub cursor: Option<i32>,
    pub pointer_events: bool,
    pub color: Rgba,
    pub accent: Option<Rgba>,
    pub font_size: f32,
    pub weight: u16,
    pub italic: bool,
    pub underline: bool,
    pub line_through: bool,
    pub overline: bool,
    pub text_align: TextAlign,
    pub line_height: LineHeight,
    pub white_space: WhiteSpace,
    pub text_transform: TextTransform,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub text_indent: Length,
    pub list_style: ListStyle,
    pub ellipsis: bool,
    pub small_caps: bool,
    pub vertical_align: VAlign,
    pub font_family: String,
    // tables, lists, quotes and text effects (inherited)
    pub border_spacing: (f32, f32),
    pub border_collapse: bool,
    pub caption_bottom: bool,
    pub marker_inside: bool,
    /// `quotes`: None = the default curly quotes.
    pub quotes: Option<(String, String)>,
    pub text_shadow: Option<Shadow>,
    pub decoration_color: Option<Rgba>,
}

impl Default for Computed {
    fn default() -> Self {
        Computed {
            display: DisplayKind::Inline,
            layout: Style { display: Display::Block, ..Default::default() },
            order: 0,
            background: Rgba::TRANSPARENT,
            gradient: None,
            border_color: [Rgba::TRANSPARENT; 4],
            border_drawn: [true; 4],
            radius: [Length::Px(0.0); 4],
            shadows: Vec::new(),
            outline: (0.0, Rgba::TRANSPARENT, 0.0),
            opacity: 1.0,
            z_index: None,
            translate: (Length::Px(0.0), Length::Px(0.0)),
            scale: 1.0,
            visible: true,
            cursor: None,
            pointer_events: true,
            color: crate::color::rgb(0xe6e6e6),
            accent: None,
            font_size: 13.0,
            weight: 400,
            italic: false,
            underline: false,
            line_through: false,
            overline: false,
            text_align: TextAlign::Left,
            line_height: LineHeight::Normal,
            white_space: WhiteSpace::Normal,
            text_transform: TextTransform::None,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            text_indent: Length::Px(0.0),
            list_style: ListStyle::Disc,
            ellipsis: false,
            small_caps: false,
            vertical_align: VAlign::Baseline,
            font_family: String::new(),
            border_spacing: (0.0, 0.0),
            border_collapse: false,
            caption_bottom: false,
            marker_inside: false,
            quotes: None,
            text_shadow: None,
            decoration_color: None,
        }
    }
}

impl Computed {
    /// A child's starting style: inherited properties from `parent`, the rest initial.
    pub fn inherit(parent: &Computed) -> Computed {
        Computed {
            visible: parent.visible,
            cursor: parent.cursor,
            pointer_events: parent.pointer_events,
            color: parent.color,
            accent: parent.accent,
            font_size: parent.font_size,
            weight: parent.weight,
            italic: parent.italic,
            text_align: parent.text_align,
            line_height: parent.line_height,
            white_space: parent.white_space,
            text_transform: parent.text_transform,
            letter_spacing: parent.letter_spacing,
            word_spacing: parent.word_spacing,
            text_indent: parent.text_indent,
            list_style: parent.list_style,
            small_caps: parent.small_caps,
            font_family: parent.font_family.clone(),
            // text-decoration propagates to inline descendants in CSS
            underline: parent.underline,
            line_through: parent.line_through,
            overline: parent.overline,
            decoration_color: parent.decoration_color,
            border_spacing: parent.border_spacing,
            border_collapse: parent.border_collapse,
            caption_bottom: parent.caption_bottom,
            marker_inside: parent.marker_inside,
            quotes: parent.quotes.clone(),
            text_shadow: parent.text_shadow,
            ..Computed::default()
        }
    }

    /// Line box height in pixels.
    pub fn line_px(&self) -> f32 {
        match self.line_height {
            LineHeight::Normal => (self.font_size * 1.3).round(),
            LineHeight::Factor(f) => (self.font_size * f).round(),
            LineHeight::Px(p) => p,
        }
    }

    pub fn bold(&self) -> bool {
        self.weight >= 600
    }
}

// ==========================================================================
// Applying declarations

/// What a handler sees: the style being computed, its parent, units.
pub struct Ctx<'a> {
    pub c: &'a mut Computed,
    pub parent: &'a Computed,
    pub u: Units,
}

type Handler = fn(&mut Ctx, &[Tok]) -> bool;

/// Apply one declaration; false when the property or value isn't understood.
pub fn apply(c: &mut Computed, parent: &Computed, u: Units, d: &Decl) -> bool {
    let mut ctx = Ctx { c, parent, u };
    let name = d.prop.as_ref();
    if let [Tok::Ident(k)] = &*d.value {
        if matches!(k.as_str(), "inherit" | "initial" | "unset" | "revert" | "revert-layer") {
            return global_keyword(&mut ctx, name, k);
        }
    }
    if name.starts_with("--") {
        return true; // custom properties are accepted (no var() substitution)
    }
    match handlers().get(name) {
        Some(h) => h(&mut ctx, &d.value),
        None => ACCEPTED.contains(&name),
    }
}

/// Standard properties that parse and are accepted but have no effect in this
/// renderer (floats, bidi, generated content, background images, animation ...).
pub const ACCEPTED: &[&str] = &[
    "background-attachment", "background-position", "background-repeat", "background-size",
    "background-clip", "background-origin", "clear", "float", "clip", "clip-path", "content",
    "counter-increment", "counter-reset", "counter-set", "direction", "unicode-bidi",
    "writing-mode", "empty-cells", "list-style-image", "table-layout", "grid-template-areas",
    "object-fit", "object-position", "filter", "backdrop-filter", "mix-blend-mode", "isolation",
    "transition", "transition-property", "transition-duration", "transition-timing-function",
    "transition-delay", "animation", "animation-name", "animation-duration", "animation-delay",
    "animation-iteration-count", "animation-direction", "animation-fill-mode",
    "animation-timing-function", "animation-play-state", "will-change", "contain",
    "user-select", "resize", "appearance", "caret-color", "scroll-behavior", "scroll-snap-type",
    "scroll-snap-align", "overscroll-behavior", "touch-action", "hyphens", "overflow-wrap",
    "word-wrap", "word-break", "tab-size", "font-feature-settings", "font-kerning",
    "font-stretch", "text-rendering", "-webkit-font-smoothing", "-moz-osx-font-smoothing",
    "orphans", "widows", "page-break-before", "page-break-after", "page-break-inside",
    "break-before", "break-after", "break-inside", "columns", "column-count", "column-width",
    "column-rule", "rotate", "transform-origin", "perspective", "backface-visibility",
    "box-decoration-break", "image-rendering", "shape-outside", "text-underline-offset",
    "text-decoration-thickness", "text-decoration-style", "text-justify", "line-break",
];

/// Implemented or accepted.
pub fn is_supported(name: &str) -> bool {
    is_implemented(name) || ACCEPTED.contains(&name) || name.starts_with("--")
}

fn global_keyword(ctx: &mut Ctx, name: &str, keyword: &str) -> bool {
    let src = if keyword == "inherit" || (keyword == "unset" && is_inherited(name)) {
        ctx.parent.clone()
    } else {
        Computed::default()
    };
    macro_rules! copy {
        ($($prop:literal => $($field:ident).+),* $(,)?) => {
            match name {
                $($prop => { ctx.c.$($field).+ = src.$($field).+.clone(); true })*
                _ => handlers().contains_key(name),
            }
        };
    }
    copy! {
        "color" => color, "font-size" => font_size, "font-weight" => weight,
        "font-style" => italic, "text-align" => text_align, "line-height" => line_height,
        "white-space" => white_space, "text-transform" => text_transform,
        "letter-spacing" => letter_spacing, "word-spacing" => word_spacing,
        "visibility" => visible, "cursor" => cursor, "list-style-type" => list_style,
        "background-color" => background, "opacity" => opacity, "display" => display,
        "width" => layout.size.width, "height" => layout.size.height,
        "font-family" => font_family, "accent-color" => accent,
    }
}

fn is_inherited(name: &str) -> bool {
    INHERITED.contains(&name)
}

const INHERITED: &[&str] = &[
    "border-spacing", "border-collapse", "caption-side", "list-style-position", "quotes",
    "text-shadow", "empty-cells",
    "color", "cursor", "font", "font-family", "font-size", "font-style", "font-variant",
    "font-weight", "letter-spacing", "line-height", "list-style", "list-style-type",
    "text-align", "text-indent", "text-transform", "visibility", "white-space", "word-spacing",
    "accent-color", "caret-color", "pointer-events",
];

/// FNV-1a: property names are short, so this beats the default SipHash.
#[derive(Default, Clone, Copy)]
pub(crate) struct Fnv(pub u64);

impl std::hash::Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf29ce484222325 } else { self.0 };
        for b in bytes {
            h = (h ^ *b as u64).wrapping_mul(0x100000001b3);
        }
        self.0 = h;
    }
}

pub(crate) type FnvMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<Fnv>>;

fn handlers() -> &'static FnvMap<&'static str, Handler> {
    static MAP: OnceLock<FnvMap<&'static str, Handler>> = OnceLock::new();
    MAP.get_or_init(|| HANDLERS.iter().copied().collect())
}

/// Names of every property that changes layout or rendering.
pub fn implemented_properties() -> impl Iterator<Item = &'static str> {
    HANDLERS.iter().map(|(n, _)| *n)
}

pub fn is_implemented(name: &str) -> bool {
    handlers().contains_key(name)
}

// -- value helpers -----------------------------------------------------------------

pub(crate) fn length(t: &Tok) -> Option<Length> {
    Some(match t {
        Tok::Num(v) if *v == 0.0 => Length::Px(0.0),
        Tok::Pct(p) => Length::Percent(*p),
        Tok::Ident(s) if s == "auto" => Length::Auto,
        Tok::Dim(v, unit) => match unit.as_str() {
            "px" => Length::Px(*v),
            "rem" => Length::Rem(*v),
            "em" => Length::Em(*v),
            "ex" | "ch" => Length::Em(*v * 0.5),
            "vw" => Length::Vw(*v),
            "vh" => Length::Vh(*v),
            "vmin" | "vmax" => Length::Vw(*v),
            "pt" => Length::Px(*v * 4.0 / 3.0),
            "pc" => Length::Px(*v * 16.0),
            "in" => Length::Px(*v * 96.0),
            "cm" => Length::Px(*v * 96.0 / 2.54),
            "mm" => Length::Px(*v * 96.0 / 25.4),
            "q" => Length::Px(*v * 96.0 / 101.6),
            _ => return None,
        },
        Tok::Func(f, args) if matches!(f.as_str(), "calc" | "min" | "max" | "clamp") => {
            return calc(f, args)
        }
        _ => return None,
    })
}

/// `calc()` and friends: sums of pixels, or of percentages (a mix keeps the
/// percentage part, the closest a single taffy length can express).
fn calc(f: &str, args: &[Vec<Tok>]) -> Option<Length> {
    let eval = |toks: &[Tok]| -> Option<Length> {
        let (mut px, mut pct, mut sign, mut any_pct) = (0.0f32, 0.0f32, 1.0f32, false);
        for t in toks {
            match t {
                Tok::Delim('+') => sign = 1.0,
                Tok::Delim('-') => sign = -1.0,
                Tok::Pct(p) => {
                    pct += sign * p;
                    any_pct = true;
                }
                other => {
                    let l = length(other)?;
                    px += sign * l.to_px(&Units { font_size: 16.0, viewport: (0.0, 0.0) })?;
                }
            }
        }
        Some(if any_pct { Length::Percent(pct) } else { Length::Px(px) })
    };
    match f {
        "calc" => eval(args.first()?),
        "min" | "max" => args.iter().filter_map(|a| eval(a)).next(),
        _ => args.get(1).and_then(|a| eval(a)),
    }
}

pub(crate) fn tok_text(t: &Tok) -> String {
    match t {
        Tok::Num(v) => format!("{v}"),
        Tok::Dim(v, u) => format!("{v}{u}"),
        Tok::Pct(v) => format!("{v}%"),
        Tok::Ident(s) => s.clone(),
        Tok::Hash(h) => format!("#{h}"),
        Tok::Str(s) => s.clone(),
        Tok::Func(n, args) => format!(
            "{n}({})",
            args.iter()
                .map(|a| a.iter().map(tok_text).collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Tok::Comma => ",".into(),
        Tok::Slash => "/".into(),
        Tok::Delim(c) => c.to_string(),
    }
}

pub(crate) fn color(t: &Tok, current: Rgba) -> Option<Rgba> {
    match t {
        Tok::Ident(s) if s == "currentcolor" => Some(current),
        Tok::Ident(s) if matches!(s.as_str(), "canvas" | "field") => Some(crate::color::rgb(0x282828)),
        Tok::Ident(s) if matches!(s.as_str(), "canvastext" | "fieldtext" | "buttontext") => {
            Some(crate::color::rgb(0xe6e6e6))
        }
        Tok::Ident(s) if matches!(s.as_str(), "linktext" | "highlight") => {
            Some(crate::color::rgb(0x4772b3))
        }
        Tok::Hash(_) | Tok::Ident(_) | Tok::Func(..) => parse_color(&tok_text(t)),
        _ => None,
    }
}

fn number(t: &Tok) -> Option<f32> {
    match t {
        Tok::Num(v) => Some(*v),
        Tok::Pct(p) => Some(*p / 100.0),
        _ => None,
    }
}

fn kw(v: &[Tok]) -> Option<&str> {
    v.first().and_then(Tok::ident)
}

fn px(ctx: &Ctx, l: Length) -> f32 {
    l.to_px(&ctx.u).unwrap_or(0.0)
}

fn lp(ctx: &Ctx, l: Length) -> LengthPercentage {
    match l {
        Length::Percent(p) => LengthPercentage::Percent(p / 100.0),
        Length::Auto => LengthPercentage::Length(0.0),
        other => LengthPercentage::Length(px(ctx, other)),
    }
}

fn lpa(ctx: &Ctx, l: Length) -> LengthPercentageAuto {
    match l {
        Length::Percent(p) => LengthPercentageAuto::Percent(p / 100.0),
        Length::Auto => LengthPercentageAuto::Auto,
        other => LengthPercentageAuto::Length(px(ctx, other)),
    }
}

fn dim(ctx: &Ctx, l: Length) -> Dimension {
    match l {
        Length::Percent(p) => Dimension::Percent(p / 100.0),
        Length::Auto => Dimension::Auto,
        other => Dimension::Length(px(ctx, other)),
    }
}

fn dim_kw(ctx: &Ctx, v: &[Tok]) -> Option<Dimension> {
    match kw(v) {
        Some("none") | Some("max-content") | Some("min-content") | Some("fit-content") => {
            Some(Dimension::Auto)
        }
        _ => v.first().and_then(length).map(|l| dim(ctx, l)),
    }
}

/// The 1–4 value box shorthand: top right bottom left.
fn four<T: Copy>(v: &[T]) -> Option<[T; 4]> {
    Some(match v.len() {
        1 => [v[0]; 4],
        2 => [v[0], v[1], v[0], v[1]],
        3 => [v[0], v[1], v[2], v[1]],
        4 => [v[0], v[1], v[2], v[3]],
        _ => return None,
    })
}

fn lengths(v: &[Tok]) -> Option<Vec<Length>> {
    v.iter().filter(|t| !matches!(t, Tok::Slash)).map(length).collect()
}

// side index: 0 top, 1 right, 2 bottom, 3 left
fn set_margin(ctx: &mut Ctx, side: usize, l: Length) {
    let v = lpa(ctx, l);
    let m = &mut ctx.c.layout.margin;
    match side {
        0 => m.top = v,
        1 => m.right = v,
        2 => m.bottom = v,
        _ => m.left = v,
    }
}

fn set_padding(ctx: &mut Ctx, side: usize, l: Length) {
    let v = lp(ctx, l);
    let p = &mut ctx.c.layout.padding;
    match side {
        0 => p.top = v,
        1 => p.right = v,
        2 => p.bottom = v,
        _ => p.left = v,
    }
}

fn set_inset(ctx: &mut Ctx, side: usize, l: Length) {
    let v = lpa(ctx, l);
    let i = &mut ctx.c.layout.inset;
    match side {
        0 => i.top = v,
        1 => i.right = v,
        2 => i.bottom = v,
        _ => i.left = v,
    }
}

fn set_border_width(ctx: &mut Ctx, side: usize, w: f32) {
    let v = LengthPercentage::Length(w);
    let b = &mut ctx.c.layout.border;
    match side {
        0 => b.top = v,
        1 => b.right = v,
        2 => b.bottom = v,
        _ => b.left = v,
    }
}

fn border_width_tok(ctx: &Ctx, t: &Tok) -> Option<f32> {
    match t.ident() {
        Some("thin") => Some(1.0),
        Some("medium") => Some(3.0),
        Some("thick") => Some(5.0),
        _ => length(t).map(|l| px(ctx, l)),
    }
}

const BORDER_STYLES: &[&str] = &["none", "hidden", "solid", "dashed", "dotted", "double",
                                 "groove", "ridge", "inset", "outset"];

/// `border[-side]: <width> || <style> || <color>` for the given sides.
fn border_sides(ctx: &mut Ctx, v: &[Tok], sides: &[usize]) -> bool {
    let (mut w, mut style, mut col) = (None, None, None);
    for t in v {
        if let Some(s) = t.ident().filter(|s| BORDER_STYLES.contains(s)) {
            style = Some(s.to_string());
        } else if let Some(c) = color(t, ctx.c.color).filter(|_| !matches!(t, Tok::Dim(..))) {
            col = Some(c);
        } else if let Some(px) = border_width_tok(ctx, t) {
            w = Some(px);
        } else {
            return false;
        }
    }
    let none = matches!(style.as_deref(), Some("none") | Some("hidden"));
    for &s in sides {
        let width = if none { 0.0 } else { w.unwrap_or(if style.is_some() { 3.0 } else { 0.0 }) };
        set_border_width(ctx, s, width);
        ctx.c.border_color[s] = col.unwrap_or(ctx.c.color);
        ctx.c.border_drawn[s] = !none;
    }
    true
}

fn per_side(v: &[Tok], mut f: impl FnMut(usize, &Tok) -> bool) -> bool {
    match four(&v.iter().collect::<Vec<_>>()) {
        Some(sides) => (0..4).all(|i| f(i, sides[i])),
        None => false,
    }
}

fn align(v: &str) -> Option<AlignItems> {
    Some(match v {
        "start" | "self-start" => AlignItems::Start,
        "end" | "self-end" => AlignItems::End,
        "flex-start" | "left" => AlignItems::FlexStart,
        "flex-end" | "right" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "baseline" | "first" | "last" => AlignItems::Baseline,
        "stretch" | "normal" => AlignItems::Stretch,
        _ => return None,
    })
}

fn align_content(v: &str) -> Option<AlignContent> {
    Some(match v {
        "start" => AlignContent::Start,
        "end" => AlignContent::End,
        "flex-start" | "left" => AlignContent::FlexStart,
        "flex-end" | "right" => AlignContent::FlexEnd,
        "center" => AlignContent::Center,
        "stretch" | "normal" => AlignContent::Stretch,
        "space-between" => AlignContent::SpaceBetween,
        "space-around" => AlignContent::SpaceAround,
        "space-evenly" => AlignContent::SpaceEvenly,
        _ => return None,
    })
}

fn kw_align(v: &[Tok]) -> Option<AlignItems> {
    v.iter().filter_map(Tok::ident).filter(|s| *s != "safe" && *s != "unsafe").next_back()
        .and_then(align)
}

fn kw_align_content(v: &[Tok]) -> Option<AlignContent> {
    v.iter().filter_map(Tok::ident).filter(|s| *s != "safe" && *s != "unsafe").next_back()
        .and_then(align_content)
}

// -- grid ----------------------------------------------------------------------------

fn track_min(ctx: &Ctx, t: &Tok) -> Option<MinTrackSizingFunction> {
    Some(match t.ident() {
        Some("auto") => MinTrackSizingFunction::Auto,
        Some("min-content") => MinTrackSizingFunction::MinContent,
        Some("max-content") => MinTrackSizingFunction::MaxContent,
        _ => match t {
            Tok::Dim(_, u) if u == "fr" => MinTrackSizingFunction::Auto,
            _ => MinTrackSizingFunction::Fixed(lp(ctx, length(t)?)),
        },
    })
}

fn track_max(ctx: &Ctx, t: &Tok) -> Option<MaxTrackSizingFunction> {
    Some(match t.ident() {
        Some("auto") => MaxTrackSizingFunction::Auto,
        Some("min-content") => MaxTrackSizingFunction::MinContent,
        Some("max-content") => MaxTrackSizingFunction::MaxContent,
        _ => match t {
            Tok::Dim(v, u) if u == "fr" => MaxTrackSizingFunction::Fraction(*v),
            _ => MaxTrackSizingFunction::Fixed(lp(ctx, length(t)?)),
        },
    })
}

fn track(ctx: &Ctx, t: &Tok) -> Option<NonRepeatedTrackSizingFunction> {
    match t {
        Tok::Func(f, args) if f == "minmax" && args.len() == 2 => Some(MinMax {
            min: track_min(ctx, args[0].first()?)?,
            max: track_max(ctx, args[1].first()?)?,
        }),
        Tok::Func(f, args) if f == "fit-content" => Some(MinMax {
            min: MinTrackSizingFunction::Auto,
            max: MaxTrackSizingFunction::FitContent(lp(ctx, length(args.first()?.first()?)?)),
        }),
        _ => Some(MinMax { min: track_min(ctx, t)?, max: track_max(ctx, t)? }),
    }
}

fn track_list(ctx: &Ctx, v: &[Tok]) -> Option<Vec<TrackSizingFunction>> {
    if kw(v) == Some("none") {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    for t in v {
        match t {
            Tok::Func(f, args) if f == "repeat" && args.len() >= 2 => {
                let rep = match args[0].first()? {
                    Tok::Num(n) => GridTrackRepetition::Count(*n as u16),
                    Tok::Ident(s) if s == "auto-fill" => GridTrackRepetition::AutoFill,
                    Tok::Ident(s) if s == "auto-fit" => GridTrackRepetition::AutoFit,
                    _ => return None,
                };
                let tracks = args[1..].iter().flatten().map(|t| track(ctx, t))
                    .collect::<Option<Vec<_>>>()?;
                out.push(TrackSizingFunction::Repeat(rep, tracks));
            }
            Tok::Delim('[') | Tok::Delim(']') => {}
            _ => out.push(TrackSizingFunction::Single(track(ctx, t)?)),
        }
    }
    Some(out)
}

fn grid_line(v: &[Tok]) -> Option<Line<GridPlacement>> {
    let place = |toks: &[Tok]| -> Option<GridPlacement> {
        match toks {
            [Tok::Ident(s)] if s == "auto" => Some(GridPlacement::Auto),
            [Tok::Num(n)] => Some(taffy::style_helpers::line(*n as i16)),
            [Tok::Ident(s), Tok::Num(n)] if s == "span" => {
                Some(taffy::style_helpers::span(*n as u16))
            }
            [Tok::Ident(_)] => Some(GridPlacement::Auto), // named lines/areas
            _ => None,
        }
    };
    let parts: Vec<&[Tok]> = v.split(|t| matches!(t, Tok::Slash)).collect();
    let start = place(parts[0])?;
    let end = match parts.get(1) {
        Some(p) => place(p)?,
        None => GridPlacement::Auto,
    };
    Some(Line { start, end })
}

// -- backgrounds, shadows, fonts -----------------------------------------------------------

fn gradient(ctx: &Ctx, t: &Tok) -> Option<Gradient> {
    let Tok::Func(f, args) = t else { return None };
    let radial = f.contains("radial");
    if !f.contains("gradient") {
        return None;
    }
    let mut angle = 180.0;
    let mut first = 0;
    if let Some(a) = args.first() {
        match a.first() {
            Some(Tok::Dim(v, u)) if u == "deg" || u == "turn" || u == "rad" => {
                angle = match u.as_str() {
                    "turn" => v * 360.0,
                    "rad" => v.to_degrees(),
                    _ => *v,
                };
                first = 1;
            }
            Some(Tok::Ident(s)) if s == "to" => {
                let dirs: Vec<&str> = a[1..].iter().filter_map(Tok::ident).collect();
                angle = match dirs.as_slice() {
                    ["top"] => 0.0,
                    ["right"] => 90.0,
                    ["left"] => 270.0,
                    ["bottom"] => 180.0,
                    [a, b] if (*a == "top" || *b == "top") => {
                        if dirs.contains(&"right") { 45.0 } else { 315.0 }
                    }
                    _ => if dirs.contains(&"right") { 135.0 } else { 225.0 },
                };
                first = 1;
            }
            Some(Tok::Ident(s)) if radial && !parse_color(s).is_some() => first = 1,
            _ => {}
        }
    }
    let stops: Vec<(Rgba, Option<f32>)> = args[first..]
        .iter()
        .filter_map(|stop| {
            let c = stop.iter().find_map(|t| color(t, ctx.c.color))?;
            let pos = stop.iter().find_map(|t| match t {
                Tok::Pct(p) => Some(p / 100.0),
                _ => None,
            });
            Some((c, pos))
        })
        .collect();
    (!stops.is_empty()).then_some(Gradient { radial, angle_deg: angle, stops })
}

fn shadows(ctx: &Ctx, v: &[Tok]) -> Option<Vec<Shadow>> {
    if kw(v) == Some("none") {
        return Some(Vec::new());
    }
    v.split(|t| matches!(t, Tok::Comma))
        .map(|part| {
            let mut nums = Vec::new();
            let mut c = Rgba { a: 0.35, ..Rgba::default() };
            let mut inset = false;
            for t in part {
                if t.ident() == Some("inset") {
                    inset = true;
                } else if let Some(l) = length(t) {
                    nums.push(px(ctx, l));
                } else {
                    c = color(t, ctx.c.color)?;
                }
            }
            if nums.len() < 2 {
                return None;
            }
            Some(Shadow {
                x: nums[0],
                y: nums[1],
                blur: *nums.get(2).unwrap_or(&0.0),
                spread: *nums.get(3).unwrap_or(&0.0),
                color: c,
                inset,
            })
        })
        .collect()
}

fn font_size(ctx: &Ctx, t: &Tok) -> Option<f32> {
    let base = ctx.parent.font_size;
    Some(match t.ident() {
        Some("xx-small") => 9.0,
        Some("x-small") => 10.0,
        Some("small") => 13.0 * 0.89,
        Some("medium") => 13.0,
        Some("large") => 13.0 * 1.2,
        Some("x-large") => 13.0 * 1.5,
        Some("xx-large") => 13.0 * 2.0,
        Some("xxx-large") => 13.0 * 3.0,
        Some("smaller") => base / 1.2,
        Some("larger") => base * 1.2,
        _ => match length(t)? {
            Length::Percent(p) => base * p / 100.0,
            Length::Em(e) => base * e,
            other => other.to_px(&ctx.u)?,
        },
    })
}

fn font_weight(ctx: &Ctx, t: &Tok) -> Option<u16> {
    Some(match t {
        Tok::Num(n) => *n as u16,
        Tok::Ident(s) => match s.as_str() {
            "normal" => 400,
            "bold" => 700,
            "bolder" => (ctx.parent.weight + 300).min(900),
            "lighter" => ctx.parent.weight.saturating_sub(300).max(100),
            _ => return None,
        },
        _ => return None,
    })
}

fn line_height(t: &Tok, ctx: &Ctx) -> Option<LineHeight> {
    Some(match t {
        Tok::Ident(s) if s == "normal" => LineHeight::Normal,
        Tok::Num(n) => LineHeight::Factor(*n),
        Tok::Pct(p) => LineHeight::Factor(p / 100.0),
        _ => LineHeight::Px(match length(t)? {
            Length::Em(e) => e * ctx.c.font_size,
            other => other.to_px(&ctx.u)?,
        }),
    })
}

fn display(ctx: &mut Ctx, v: &[Tok]) -> bool {
    let words: Vec<&str> = v.iter().filter_map(Tok::ident).collect();
    let k = match words.as_slice() {
        ["none"] => DisplayKind::None,
        ["block"] | ["block", "flow"] | ["flow-root"] | ["block", "flow-root"] => DisplayKind::Block,
        ["inline"] | ["inline", "flow"] | ["ruby"] | ["ruby-text"] | ["ruby-base"] => {
            DisplayKind::Inline
        }
        ["inline-block"] | ["inline", "flow-root"] => DisplayKind::InlineBlock,
        ["flex"] | ["block", "flex"] => DisplayKind::Flex,
        ["inline-flex"] | ["inline", "flex"] => DisplayKind::InlineFlex,
        ["grid"] | ["block", "grid"] => DisplayKind::Grid,
        ["inline-grid"] | ["inline", "grid"] => DisplayKind::InlineGrid,
        ["contents"] => DisplayKind::Contents,
        ["list-item"] | ["block", "list-item"] => DisplayKind::ListItem,
        ["table"] | ["inline-table"] => DisplayKind::Table,
        ["table-row-group"] | ["table-header-group"] | ["table-footer-group"] => {
            DisplayKind::TableRowGroup
        }
        ["table-row"] => DisplayKind::TableRow,
        ["table-cell"] | ["table-column"] | ["table-column-group"] => DisplayKind::TableCell,
        ["table-caption"] => DisplayKind::TableCaption,
        _ => return false,
    };
    ctx.c.display = k;
    true
}

const CURSORS: &[(&str, i32)] = &[
    ("auto", -1), ("default", 0), ("pointer", 5), ("text", 3), ("vertical-text", 3),
    ("crosshair", 4), ("move", 6), ("all-scroll", 6), ("grab", 6), ("grabbing", 6),
    ("ew-resize", 1), ("e-resize", 1), ("w-resize", 1), ("col-resize", 1),
    ("ns-resize", 2), ("n-resize", 2), ("s-resize", 2), ("row-resize", 2),
    ("none", 7), ("not-allowed", 0), ("wait", 0), ("progress", 0), ("help", 0),
    ("context-menu", 0), ("cell", 4), ("copy", 0), ("alias", 0), ("no-drop", 0),
    ("nesw-resize", 6), ("nwse-resize", 6), ("ne-resize", 6), ("nw-resize", 6),
    ("se-resize", 6), ("sw-resize", 6), ("zoom-in", 4), ("zoom-out", 4),
];

fn roman(mut n: usize, upper: bool) -> String {
    let table = [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"),
                 (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")];
    let mut s = String::new();
    for (v, r) in table {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    if upper { s.to_uppercase() } else { s }
}

impl ListStyle {
    /// The marker text for the `index`-th (1-based) item.
    pub fn marker(self, index: usize) -> Option<String> {
        Some(match self {
            ListStyle::None => return None,
            ListStyle::Disc => "\u{2022}".into(),
            ListStyle::Circle => "\u{25e6}".into(),
            ListStyle::Square => "\u{25aa}".into(),
            ListStyle::Decimal => format!("{index}."),
            ListStyle::LowerAlpha => format!("{}.", (b'a' + ((index - 1) % 26) as u8) as char),
            ListStyle::UpperAlpha => format!("{}.", (b'A' + ((index - 1) % 26) as u8) as char),
            ListStyle::LowerRoman => format!("{}.", roman(index, false)),
            ListStyle::UpperRoman => format!("{}.", roman(index, true)),
        })
    }
}

fn list_style(s: &str) -> Option<ListStyle> {
    Some(match s {
        "none" => ListStyle::None,
        "disc" => ListStyle::Disc,
        "circle" => ListStyle::Circle,
        "square" => ListStyle::Square,
        "decimal" | "decimal-leading-zero" => ListStyle::Decimal,
        "lower-alpha" | "lower-latin" => ListStyle::LowerAlpha,
        "upper-alpha" | "upper-latin" => ListStyle::UpperAlpha,
        "lower-roman" => ListStyle::LowerRoman,
        "upper-roman" => ListStyle::UpperRoman,
        _ => return None,
    })
}

fn overflow(s: &str) -> Option<Overflow> {
    Some(match s {
        "visible" => Overflow::Visible,
        "hidden" => Overflow::Hidden,
        "clip" => Overflow::Clip,
        "scroll" | "auto" | "overlay" => Overflow::Scroll,
        _ => return None,
    })
}

/// Run another property's handler (shorthands, logical aliases).
fn delegate(ctx: &mut Ctx, name: &str, v: &[Tok]) -> bool {
    handlers().get(name).is_some_and(|h| h(ctx, v))
}

macro_rules! side_handlers {
    ($setter:ident, $top:literal, $right:literal, $bottom:literal, $left:literal) => {
        [
            ($top, (|c: &mut Ctx, v: &[Tok]| v.first().and_then(length).map(|l| $setter(c, 0, l)).is_some()) as Handler),
            ($right, (|c: &mut Ctx, v: &[Tok]| v.first().and_then(length).map(|l| $setter(c, 1, l)).is_some()) as Handler),
            ($bottom, (|c: &mut Ctx, v: &[Tok]| v.first().and_then(length).map(|l| $setter(c, 2, l)).is_some()) as Handler),
            ($left, (|c: &mut Ctx, v: &[Tok]| v.first().and_then(length).map(|l| $setter(c, 3, l)).is_some()) as Handler),
        ]
    };
}

static HANDLERS: &[(&str, Handler)] = &{
    const fn h(n: &'static str, f: Handler) -> (&'static str, Handler) {
        (n, f)
    }
    // The table: [box model, positioning, flexbox, grid, alignment, overflow,
    // background, border, effects, text, interaction]. Every entry changes
    // layout or rendering.
    let margin = side_handlers!(set_margin, "margin-top", "margin-right", "margin-bottom", "margin-left");
    let padding = side_handlers!(set_padding, "padding-top", "padding-right", "padding-bottom", "padding-left");
    let inset = side_handlers!(set_inset, "top", "right", "bottom", "left");
    [
        // -- box model --------------------------------------------------------------
        h("display", display),
        h("box-sizing", |c, v| {
            c.c.layout.box_sizing = match kw(v) {
                Some("content-box") => BoxSizing::ContentBox,
                Some("border-box") => BoxSizing::BorderBox,
                _ => return false,
            };
            true
        }),
        h("width", |c, v| dim_kw(c, v).map(|d| c.c.layout.size.width = d).is_some()),
        h("height", |c, v| dim_kw(c, v).map(|d| c.c.layout.size.height = d).is_some()),
        h("min-width", |c, v| dim_kw(c, v).map(|d| c.c.layout.min_size.width = d).is_some()),
        h("min-height", |c, v| dim_kw(c, v).map(|d| c.c.layout.min_size.height = d).is_some()),
        h("max-width", |c, v| dim_kw(c, v).map(|d| c.c.layout.max_size.width = d).is_some()),
        h("max-height", |c, v| dim_kw(c, v).map(|d| c.c.layout.max_size.height = d).is_some()),
        h("inline-size", |c, v| delegate(c, "width", v)),
        h("block-size", |c, v| delegate(c, "height", v)),
        h("min-inline-size", |c, v| delegate(c, "min-width", v)),
        h("min-block-size", |c, v| delegate(c, "min-height", v)),
        h("max-inline-size", |c, v| delegate(c, "max-width", v)),
        h("max-block-size", |c, v| delegate(c, "max-height", v)),
        h("aspect-ratio", |c, v| {
            let nums: Vec<f32> = v.iter().filter_map(number).collect();
            c.c.layout.aspect_ratio = match (kw(v), nums.as_slice()) {
                (Some("auto"), _) => None,
                (_, [w, h]) if *h != 0.0 => Some(w / h),
                (_, [r]) => Some(*r),
                _ => return false,
            };
            true
        }),
        margin[0], margin[1], margin[2], margin[3],
        h("margin", |c, v| lengths(v).and_then(|l| four(&l)).map(|s| {
            (0..4).for_each(|i| set_margin(c, i, s[i]))
        }).is_some()),
        h("margin-inline-start", |c, v| delegate(c, "margin-left", v)),
        h("margin-inline-end", |c, v| delegate(c, "margin-right", v)),
        h("margin-block-start", |c, v| delegate(c, "margin-top", v)),
        h("margin-block-end", |c, v| delegate(c, "margin-bottom", v)),
        h("margin-inline", |c, v| pair(c, v, "margin-left", "margin-right")),
        h("margin-block", |c, v| pair(c, v, "margin-top", "margin-bottom")),
        padding[0], padding[1], padding[2], padding[3],
        h("padding", |c, v| lengths(v).and_then(|l| four(&l)).map(|s| {
            (0..4).for_each(|i| set_padding(c, i, s[i]))
        }).is_some()),
        h("padding-inline-start", |c, v| delegate(c, "padding-left", v)),
        h("padding-inline-end", |c, v| delegate(c, "padding-right", v)),
        h("padding-block-start", |c, v| delegate(c, "padding-top", v)),
        h("padding-block-end", |c, v| delegate(c, "padding-bottom", v)),
        h("padding-inline", |c, v| pair(c, v, "padding-left", "padding-right")),
        h("padding-block", |c, v| pair(c, v, "padding-top", "padding-bottom")),
        // -- positioning ------------------------------------------------------------
        h("position", |c, v| {
            c.c.layout.position = match kw(v) {
                Some("static") | Some("relative") | Some("sticky") => Position::Relative,
                Some("absolute") | Some("fixed") => Position::Absolute,
                _ => return false,
            };
            true
        }),
        inset[0], inset[1], inset[2], inset[3],
        h("inset", |c, v| lengths(v).and_then(|l| four(&l)).map(|s| {
            (0..4).for_each(|i| set_inset(c, i, s[i]))
        }).is_some()),
        h("inset-inline-start", |c, v| delegate(c, "left", v)),
        h("inset-inline-end", |c, v| delegate(c, "right", v)),
        h("inset-block-start", |c, v| delegate(c, "top", v)),
        h("inset-block-end", |c, v| delegate(c, "bottom", v)),
        h("inset-inline", |c, v| pair(c, v, "left", "right")),
        h("inset-block", |c, v| pair(c, v, "top", "bottom")),
        h("z-index", |c, v| {
            c.c.z_index = match v.first() {
                Some(Tok::Num(n)) => Some(*n as i32),
                Some(Tok::Ident(s)) if s == "auto" => None,
                _ => return false,
            };
            true
        }),
        // -- flexbox --------------------------------------------------------------------
        h("flex-direction", |c, v| {
            c.c.layout.flex_direction = match kw(v) {
                Some("row") => FlexDirection::Row,
                Some("row-reverse") => FlexDirection::RowReverse,
                Some("column") => FlexDirection::Column,
                Some("column-reverse") => FlexDirection::ColumnReverse,
                _ => return false,
            };
            true
        }),
        h("flex-wrap", |c, v| {
            c.c.layout.flex_wrap = match kw(v) {
                Some("nowrap") => FlexWrap::NoWrap,
                Some("wrap") => FlexWrap::Wrap,
                Some("wrap-reverse") => FlexWrap::WrapReverse,
                _ => return false,
            };
            true
        }),
        h("flex-flow", |c, v| v.iter().all(|t| {
            let one = std::slice::from_ref(t);
            delegate(c, "flex-direction", one) || delegate(c, "flex-wrap", one)
        })),
        h("flex-grow", |c, v| v.first().and_then(number).map(|n| c.c.layout.flex_grow = n).is_some()),
        h("flex-shrink", |c, v| v.first().and_then(number).map(|n| c.c.layout.flex_shrink = n).is_some()),
        h("flex-basis", |c, v| match kw(v) {
            Some("content") => {
                c.c.layout.flex_basis = Dimension::Auto;
                true
            }
            _ => dim_kw(c, v).map(|d| c.c.layout.flex_basis = d).is_some(),
        }),
        h("flex", |c, v| {
            let l = &mut c.c.layout;
            match (kw(v), v) {
                (Some("none"), _) => (l.flex_grow, l.flex_shrink, l.flex_basis) = (0.0, 0.0, Dimension::Auto),
                (Some("auto"), _) => (l.flex_grow, l.flex_shrink, l.flex_basis) = (1.0, 1.0, Dimension::Auto),
                (Some("initial"), _) => (l.flex_grow, l.flex_shrink, l.flex_basis) = (0.0, 1.0, Dimension::Auto),
                (_, [Tok::Num(g)]) => (l.flex_grow, l.flex_shrink, l.flex_basis) = (*g, 1.0, Dimension::Percent(0.0)),
                (_, [Tok::Num(g), Tok::Num(s)]) => (l.flex_grow, l.flex_shrink, l.flex_basis) = (*g, *s, Dimension::Percent(0.0)),
                (_, [Tok::Num(g), Tok::Num(s), b]) => {
                    let Some(len) = length(b) else { return false };
                    let d = dim(c, len);
                    let l = &mut c.c.layout;
                    (l.flex_grow, l.flex_shrink, l.flex_basis) = (*g, *s, d);
                }
                (_, [Tok::Num(g), b]) => {
                    let Some(len) = length(b) else { return false };
                    let d = dim(c, len);
                    let l = &mut c.c.layout;
                    (l.flex_grow, l.flex_shrink, l.flex_basis) = (*g, 1.0, d);
                }
                (_, [b]) => {
                    let Some(len) = length(b) else { return false };
                    let d = dim(c, len);
                    let l = &mut c.c.layout;
                    (l.flex_grow, l.flex_shrink, l.flex_basis) = (1.0, 1.0, d);
                }
                _ => return false,
            }
            true
        }),
        h("order", |c, v| match v.first() {
            Some(Tok::Num(n)) => {
                c.c.order = *n as i32;
                true
            }
            _ => false,
        }),
        // -- alignment ------------------------------------------------------------------
        h("align-items", |c, v| kw_align(v).map(|a| c.c.layout.align_items = Some(a)).is_some()),
        h("align-self", |c, v| match kw(v) {
            Some("auto") => {
                c.c.layout.align_self = None;
                true
            }
            _ => kw_align(v).map(|a| c.c.layout.align_self = Some(a)).is_some(),
        }),
        h("justify-items", |c, v| kw_align(v).map(|a| c.c.layout.justify_items = Some(a)).is_some()),
        h("justify-self", |c, v| match kw(v) {
            Some("auto") => {
                c.c.layout.justify_self = None;
                true
            }
            _ => kw_align(v).map(|a| c.c.layout.justify_self = Some(a)).is_some(),
        }),
        h("align-content", |c, v| kw_align_content(v).map(|a| c.c.layout.align_content = Some(a)).is_some()),
        h("justify-content", |c, v| kw_align_content(v).map(|a| c.c.layout.justify_content = Some(a)).is_some()),
        h("place-items", |c, v| two_kw(c, v, "align-items", "justify-items")),
        h("place-content", |c, v| two_kw(c, v, "align-content", "justify-content")),
        h("place-self", |c, v| two_kw(c, v, "align-self", "justify-self")),
        h("gap", |c, v| {
            let Some(l) = lengths(v) else { return false };
            let (r, col) = match l.as_slice() {
                [a] => (*a, *a),
                [a, b] => (*a, *b),
                _ => return false,
            };
            c.c.layout.gap = Size { width: lp(c, col), height: lp(c, r) };
            true
        }),
        h("row-gap", |c, v| v.first().and_then(length).map(|l| c.c.layout.gap.height = lp(c, l)).is_some()),
        h("column-gap", |c, v| match kw(v) {
            Some("normal") => true,
            _ => v.first().and_then(length).map(|l| c.c.layout.gap.width = lp(c, l)).is_some(),
        }),
        h("grid-gap", |c, v| delegate(c, "gap", v)),
        h("grid-row-gap", |c, v| delegate(c, "row-gap", v)),
        h("grid-column-gap", |c, v| delegate(c, "column-gap", v)),
        // -- grid -------------------------------------------------------------------------
        h("grid-template-columns", |c, v| track_list(c, v).map(|t| c.c.layout.grid_template_columns = t).is_some()),
        h("grid-template-rows", |c, v| track_list(c, v).map(|t| c.c.layout.grid_template_rows = t).is_some()),
        h("grid-template", |c, v| {
            let parts: Vec<&[Tok]> = v.split(|t| matches!(t, Tok::Slash)).collect();
            match parts.as_slice() {
                [rows, cols] => delegate(c, "grid-template-rows", rows)
                    && delegate(c, "grid-template-columns", cols),
                _ => delegate(c, "grid-template-rows", v),
            }
        }),
        h("grid", |c, v| {
            // <rows> / <columns> | auto-flow [dense] <rows> / <columns> | <rows> / auto-flow <columns>
            let parts: Vec<&[Tok]> = v.split(|t| matches!(t, Tok::Slash)).collect();
            let [rows, cols] = parts.as_slice() else { return false };
            let flow = |p: &[Tok]| p.iter().any(|t| t.ident() == Some("auto-flow"));
            let dense = v.iter().any(|t| t.ident() == Some("dense"));
            let strip = |p: &[Tok]| p.iter().filter(|t| !matches!(t.ident(), Some("auto-flow" | "dense")))
                .cloned().collect::<Vec<Tok>>();
            if flow(rows) {
                c.c.layout.grid_auto_flow = if dense { GridAutoFlow::RowDense } else { GridAutoFlow::Row };
                let auto = strip(rows);
                (auto.is_empty() || delegate(c, "grid-auto-rows", &auto))
                    && delegate(c, "grid-template-columns", cols)
            } else if flow(cols) {
                c.c.layout.grid_auto_flow = if dense { GridAutoFlow::ColumnDense } else { GridAutoFlow::Column };
                let auto = strip(cols);
                (auto.is_empty() || delegate(c, "grid-auto-columns", &auto))
                    && delegate(c, "grid-template-rows", rows)
            } else {
                delegate(c, "grid-template", v)
            }
        }),
        h("grid-auto-columns", |c, v| v.iter().map(|t| track(c, t)).collect::<Option<Vec<_>>>()
            .map(|t| c.c.layout.grid_auto_columns = t).is_some()),
        h("grid-auto-rows", |c, v| v.iter().map(|t| track(c, t)).collect::<Option<Vec<_>>>()
            .map(|t| c.c.layout.grid_auto_rows = t).is_some()),
        h("grid-auto-flow", |c, v| {
            let words: Vec<&str> = v.iter().filter_map(Tok::ident).collect();
            let dense = words.contains(&"dense");
            c.c.layout.grid_auto_flow = match (words.contains(&"column"), dense) {
                (true, true) => GridAutoFlow::ColumnDense,
                (true, false) => GridAutoFlow::Column,
                (false, true) => GridAutoFlow::RowDense,
                (false, false) => GridAutoFlow::Row,
            };
            true
        }),
        h("grid-row", |c, v| grid_line(v).map(|l| c.c.layout.grid_row = l).is_some()),
        h("grid-column", |c, v| grid_line(v).map(|l| c.c.layout.grid_column = l).is_some()),
        h("grid-row-start", |c, v| grid_line(v).map(|l| c.c.layout.grid_row.start = l.start).is_some()),
        h("grid-row-end", |c, v| grid_line(v).map(|l| c.c.layout.grid_row.end = l.start).is_some()),
        h("grid-column-start", |c, v| grid_line(v).map(|l| c.c.layout.grid_column.start = l.start).is_some()),
        h("grid-column-end", |c, v| grid_line(v).map(|l| c.c.layout.grid_column.end = l.start).is_some()),
        h("grid-area", |c, v| {
            let parts: Vec<&[Tok]> = v.split(|t| matches!(t, Tok::Slash)).collect();
            let get = |i: usize| parts.get(i).and_then(|p| grid_line(p)).map(|l| l.start);
            match (get(0), get(1), get(2), get(3)) {
                (Some(rs), cs, re, ce) => {
                    c.c.layout.grid_row = Line { start: rs, end: re.unwrap_or(GridPlacement::Auto) };
                    c.c.layout.grid_column = Line {
                        start: cs.unwrap_or(GridPlacement::Auto),
                        end: ce.unwrap_or(GridPlacement::Auto),
                    };
                    true
                }
                _ => false,
            }
        }),
        // -- overflow ---------------------------------------------------------------------
        h("overflow-x", |c, v| kw(v).and_then(overflow).map(|o| c.c.layout.overflow.x = o).is_some()),
        h("overflow-y", |c, v| kw(v).and_then(overflow).map(|o| c.c.layout.overflow.y = o).is_some()),
        h("overflow", |c, v| {
            let o: Option<Vec<Overflow>> = v.iter().filter_map(Tok::ident).map(overflow).collect();
            match o.as_deref() {
                Some([a]) => c.c.layout.overflow = taffy::Point { x: *a, y: *a },
                Some([a, b]) => c.c.layout.overflow = taffy::Point { x: *a, y: *b },
                _ => return false,
            }
            true
        }),
        h("overflow-inline", |c, v| delegate(c, "overflow-x", v)),
        h("overflow-block", |c, v| delegate(c, "overflow-y", v)),
        h("scrollbar-width", |c, v| {
            c.c.layout.scrollbar_width = match kw(v) {
                Some("none") => 0.0,
                Some("thin") => 6.0,
                Some("auto") => 8.0,
                _ => return v.first().and_then(length).map(|l| c.c.layout.scrollbar_width = px(c, l)).is_some(),
            };
            true
        }),
        h("text-overflow", |c, v| {
            c.c.ellipsis = kw(v) == Some("ellipsis");
            true
        }),
        // -- background -------------------------------------------------------------------
        h("background-color", |c, v| v.first().and_then(|t| color(t, c.c.color)).map(|col| c.c.background = col).is_some()),
        h("background-image", |c, v| match kw(v) {
            Some("none") => {
                c.c.gradient = None;
                true
            }
            _ => v.first().and_then(|t| gradient(c, t)).map(|g| c.c.gradient = Some(g)).is_some(),
        }),
        h("background", |c, v| {
            let mut ok = false;
            for t in v {
                if let Some(g) = gradient(c, t) {
                    c.c.gradient = Some(g);
                    ok = true;
                } else if let Some(col) = color(t, c.c.color) {
                    c.c.background = col;
                    ok = true;
                } else if t.ident() == Some("none") {
                    c.c.background = Rgba::TRANSPARENT;
                    c.c.gradient = None;
                    ok = true;
                }
            }
            ok
        }),
        // -- border -------------------------------------------------------------------------
        h("border", |c, v| border_sides(c, v, &[0, 1, 2, 3])),
        h("border-top", |c, v| border_sides(c, v, &[0])),
        h("border-right", |c, v| border_sides(c, v, &[1])),
        h("border-bottom", |c, v| border_sides(c, v, &[2])),
        h("border-left", |c, v| border_sides(c, v, &[3])),
        h("border-inline", |c, v| border_sides(c, v, &[1, 3])),
        h("border-block", |c, v| border_sides(c, v, &[0, 2])),
        h("border-inline-start", |c, v| border_sides(c, v, &[3])),
        h("border-inline-end", |c, v| border_sides(c, v, &[1])),
        h("border-block-start", |c, v| border_sides(c, v, &[0])),
        h("border-block-end", |c, v| border_sides(c, v, &[2])),
        h("border-width", |c, v| per_side(v, |i, t| border_width_tok(c, t).map(|w| set_border_width(c, i, w)).is_some())),
        h("border-top-width", |c, v| v.first().and_then(|t| border_width_tok(c, t)).map(|w| set_border_width(c, 0, w)).is_some()),
        h("border-right-width", |c, v| v.first().and_then(|t| border_width_tok(c, t)).map(|w| set_border_width(c, 1, w)).is_some()),
        h("border-bottom-width", |c, v| v.first().and_then(|t| border_width_tok(c, t)).map(|w| set_border_width(c, 2, w)).is_some()),
        h("border-left-width", |c, v| v.first().and_then(|t| border_width_tok(c, t)).map(|w| set_border_width(c, 3, w)).is_some()),
        h("border-color", |c, v| per_side(v, |i, t| color(t, c.c.color).map(|col| c.c.border_color[i] = col).is_some())),
        h("border-top-color", |c, v| v.first().and_then(|t| color(t, c.c.color)).map(|col| c.c.border_color[0] = col).is_some()),
        h("border-right-color", |c, v| v.first().and_then(|t| color(t, c.c.color)).map(|col| c.c.border_color[1] = col).is_some()),
        h("border-bottom-color", |c, v| v.first().and_then(|t| color(t, c.c.color)).map(|col| c.c.border_color[2] = col).is_some()),
        h("border-left-color", |c, v| v.first().and_then(|t| color(t, c.c.color)).map(|col| c.c.border_color[3] = col).is_some()),
        h("border-style", |c, v| per_side(v, |i, t| border_style(c, i, t))),
        h("border-top-style", |c, v| v.first().is_some_and(|t| border_style(c, 0, t))),
        h("border-right-style", |c, v| v.first().is_some_and(|t| border_style(c, 1, t))),
        h("border-bottom-style", |c, v| v.first().is_some_and(|t| border_style(c, 2, t))),
        h("border-left-style", |c, v| v.first().is_some_and(|t| border_style(c, 3, t))),
        h("border-radius", |c, v| {
            let first: Vec<Tok> = v.split(|t| matches!(t, Tok::Slash)).next().unwrap_or(&[]).to_vec();
            lengths(&first).and_then(|l| four(&l)).map(|s| c.c.radius = s).is_some()
        }),
        h("border-top-left-radius", |c, v| v.first().and_then(length).map(|l| c.c.radius[0] = l).is_some()),
        h("border-top-right-radius", |c, v| v.first().and_then(length).map(|l| c.c.radius[1] = l).is_some()),
        h("border-bottom-right-radius", |c, v| v.first().and_then(length).map(|l| c.c.radius[2] = l).is_some()),
        h("border-bottom-left-radius", |c, v| v.first().and_then(length).map(|l| c.c.radius[3] = l).is_some()),
        h("border-start-start-radius", |c, v| delegate(c, "border-top-left-radius", v)),
        h("border-start-end-radius", |c, v| delegate(c, "border-top-right-radius", v)),
        h("border-end-start-radius", |c, v| delegate(c, "border-bottom-left-radius", v)),
        h("border-end-end-radius", |c, v| delegate(c, "border-bottom-right-radius", v)),
        h("outline", |c, v| {
            let (mut w, mut col, mut ok) = (c.c.outline.0.max(3.0), c.c.color, true);
            for t in v {
                if t.ident().is_some_and(|s| s == "none") {
                    w = 0.0;
                } else if t.ident().is_some_and(|s| BORDER_STYLES.contains(&s) || s == "auto") {
                } else if let Some(cc) = color(t, c.c.color).filter(|_| !matches!(t, Tok::Dim(..))) {
                    col = cc;
                } else if let Some(px) = border_width_tok(c, t) {
                    w = px;
                } else {
                    ok = false;
                }
            }
            c.c.outline.0 = w;
            c.c.outline.1 = col;
            ok
        }),
        h("outline-width", |c, v| v.first().and_then(|t| border_width_tok(c, t)).map(|w| c.c.outline.0 = w).is_some()),
        h("outline-color", |c, v| v.first().and_then(|t| color(t, c.c.color)).map(|col| c.c.outline.1 = col).is_some()),
        h("outline-style", |c, v| {
            if kw(v) == Some("none") {
                c.c.outline.0 = 0.0;
            }
            true
        }),
        h("outline-offset", |c, v| v.first().and_then(length).map(|l| c.c.outline.2 = px(c, l)).is_some()),
        // -- effects ------------------------------------------------------------------------
        h("opacity", |c, v| v.first().and_then(number).map(|n| c.c.opacity = n.clamp(0.0, 1.0)).is_some()),
        h("visibility", |c, v| {
            c.c.visible = match kw(v) {
                Some("visible") => true,
                Some("hidden") | Some("collapse") => false,
                _ => return false,
            };
            true
        }),
        h("box-shadow", |c, v| shadows(c, v).map(|s| c.c.shadows = s).is_some()),
        h("transform", |c, v| {
            let mut ok = true;
            for t in v {
                match t {
                    Tok::Func(f, args) => {
                        let arg = |i: usize| args.get(i).and_then(|a| a.first()).and_then(length);
                        match f.as_str() {
                            "translate" => {
                                c.c.translate = (arg(0).unwrap_or_default(), arg(1).unwrap_or(Length::Px(0.0)))
                            }
                            "translatex" => c.c.translate.0 = arg(0).unwrap_or_default(),
                            "translatey" => c.c.translate.1 = arg(0).unwrap_or_default(),
                            "scale" => {
                                c.c.scale = args.first().and_then(|a| a.first()).and_then(number).unwrap_or(1.0)
                            }
                            _ => ok = false, // rotate/skew/matrix: not drawn
                        }
                    }
                    Tok::Ident(s) if s == "none" => {
                        c.c.translate = (Length::Px(0.0), Length::Px(0.0));
                        c.c.scale = 1.0;
                    }
                    _ => ok = false,
                }
            }
            ok
        }),
        h("translate", |c, v| {
            let l = lengths(v).unwrap_or_default();
            c.c.translate = (*l.first().unwrap_or(&Length::Px(0.0)), *l.get(1).unwrap_or(&Length::Px(0.0)));
            !l.is_empty()
        }),
        // -- text ---------------------------------------------------------------------------
        h("color", |c, v| v.first().and_then(|t| color(t, c.parent.color)).map(|col| c.c.color = col).is_some()),
        h("font-size", |c, v| v.first().and_then(|t| font_size(c, t)).map(|s| c.c.font_size = s).is_some()),
        h("font-weight", |c, v| v.first().and_then(|t| font_weight(c, t)).map(|w| c.c.weight = w).is_some()),
        h("font-style", |c, v| {
            c.c.italic = matches!(kw(v), Some("italic") | Some("oblique"));
            true
        }),
        h("font-family", |c, v| {
            c.c.font_family = v.iter().map(tok_text).collect::<Vec<_>>().join(" ");
            true
        }),
        h("font-variant", |c, v| {
            c.c.small_caps = kw(v) == Some("small-caps");
            true
        }),
        h("font-variant-caps", |c, v| delegate(c, "font-variant", v)),
        h("font", |c, v| {
            // [style] [variant] [weight] size[/line-height] family
            let mut i = 0;
            while i < v.len() {
                let t = &v[i];
                if let Some(s) = t.ident() {
                    match s {
                        "italic" | "oblique" => c.c.italic = true,
                        "small-caps" => c.c.small_caps = true,
                        "bold" | "bolder" | "lighter" => c.c.weight = font_weight(c, t).unwrap_or(400),
                        "normal" => {}
                        _ => break,
                    }
                } else if let Tok::Num(n) = t {
                    c.c.weight = *n as u16;
                } else if let Some(s) = font_size(c, t) {
                    c.c.font_size = s;
                    if matches!(v.get(i + 1), Some(Tok::Slash)) {
                        if let Some(lh) = v.get(i + 2).and_then(|t| line_height(t, c)) {
                            c.c.line_height = lh;
                        }
                        i += 2;
                    }
                    i += 1;
                    c.c.font_family = v[i..].iter().map(tok_text).collect::<Vec<_>>().join(" ");
                    return true;
                } else {
                    return false;
                }
                i += 1;
            }
            false
        }),
        h("line-height", |c, v| v.first().and_then(|t| line_height(t, c)).map(|l| c.c.line_height = l).is_some()),
        h("text-align", |c, v| {
            c.c.text_align = match kw(v) {
                Some("left") | Some("start") => TextAlign::Left,
                Some("right") | Some("end") => TextAlign::Right,
                Some("center") | Some("-webkit-center") => TextAlign::Center,
                Some("justify") => TextAlign::Justify,
                _ => return false,
            };
            true
        }),
        h("text-decoration", |c, v| {
            let words: Vec<&str> = v.iter().filter_map(Tok::ident).collect();
            c.c.underline = words.contains(&"underline");
            c.c.line_through = words.contains(&"line-through");
            c.c.overline = words.contains(&"overline");
            true
        }),
        h("text-decoration-line", |c, v| delegate(c, "text-decoration", v)),
        h("white-space", |c, v| {
            c.c.white_space = match kw(v) {
                Some("normal") => WhiteSpace::Normal,
                Some("nowrap") => WhiteSpace::NoWrap,
                Some("pre") => WhiteSpace::Pre,
                Some("pre-wrap") | Some("break-spaces") => WhiteSpace::PreWrap,
                Some("pre-line") => WhiteSpace::PreLine,
                _ => return false,
            };
            true
        }),
        h("text-wrap", |c, v| {
            if kw(v) == Some("nowrap") {
                c.c.white_space = WhiteSpace::NoWrap;
            }
            true
        }),
        h("text-transform", |c, v| {
            c.c.text_transform = match kw(v) {
                Some("none") => TextTransform::None,
                Some("uppercase") => TextTransform::Upper,
                Some("lowercase") => TextTransform::Lower,
                Some("capitalize") => TextTransform::Capitalize,
                _ => return false,
            };
            true
        }),
        h("letter-spacing", |c, v| match kw(v) {
            Some("normal") => {
                c.c.letter_spacing = 0.0;
                true
            }
            _ => v.first().and_then(length).map(|l| c.c.letter_spacing = px(c, l)).is_some(),
        }),
        h("word-spacing", |c, v| match kw(v) {
            Some("normal") => {
                c.c.word_spacing = 0.0;
                true
            }
            _ => v.first().and_then(length).map(|l| c.c.word_spacing = px(c, l)).is_some(),
        }),
        h("text-indent", |c, v| v.first().and_then(length).map(|l| c.c.text_indent = l).is_some()),
        h("vertical-align", |c, v| {
            c.c.vertical_align = match kw(v) {
                Some("sub") => VAlign::Sub,
                Some("super") => VAlign::Super,
                _ => VAlign::Baseline,
            };
            true
        }),
        h("list-style-type", |c, v| kw(v).and_then(list_style).map(|l| c.c.list_style = l).is_some()),
        h("list-style-position", |c, v| match kw(v) {
            Some(p @ ("inside" | "outside")) => {
                c.c.marker_inside = p == "inside";
                true
            }
            _ => false,
        }),
        h("quotes", |c, v| {
            let strs: Vec<&str> = v.iter().filter_map(|t| match t {
                Tok::Str(s) => Some(s.as_str()),
                _ => None,
            }).collect();
            match (kw(v), strs.as_slice()) {
                (Some("none"), _) => c.c.quotes = Some((String::new(), String::new())),
                (Some("auto"), _) => c.c.quotes = None,
                (_, [open, close, ..]) => c.c.quotes = Some((open.to_string(), close.to_string())),
                _ => return false,
            }
            true
        }),
        h("text-shadow", |c, v| {
            if kw(v) == Some("none") {
                c.c.text_shadow = None;
                return true;
            }
            let first: Vec<Tok> = v.split(|t| matches!(t, Tok::Comma)).next().unwrap_or(&[]).to_vec();
            let lens: Vec<f32> = first.iter().filter_map(|t| length(t).and_then(|l| l.to_px(&c.u))).collect();
            let col = first.iter()
                .find_map(|t| color(t, c.c.color).filter(|_| !matches!(t, Tok::Dim(..) | Tok::Num(_))))
                .unwrap_or(c.c.color);
            match lens.as_slice() {
                [x, y, rest @ ..] => {
                    c.c.text_shadow = Some(Shadow { x: *x, y: *y, blur: rest.first().copied().unwrap_or(0.0),
                                                    spread: 0.0, color: col, inset: false });
                    true
                }
                _ => false,
            }
        }),
        h("text-decoration-color", |c, v| v.first().and_then(|t| color(t, c.c.color))
            .map(|col| c.c.decoration_color = Some(col)).is_some()),
        // -- tables ----------------------------------------------------------------------------
        h("border-spacing", |c, v| {
            let l: Option<Vec<f32>> = v.iter().map(|t| length(t).and_then(|l| l.to_px(&c.u))).collect();
            match l.as_deref() {
                Some([a]) => c.c.border_spacing = (*a, *a),
                Some([a, b]) => c.c.border_spacing = (*a, *b),
                _ => return false,
            }
            true
        }),
        h("border-collapse", |c, v| match kw(v) {
            Some(k @ ("collapse" | "separate")) => {
                c.c.border_collapse = k == "collapse";
                true
            }
            _ => false,
        }),
        h("caption-side", |c, v| match kw(v) {
            Some(k @ ("top" | "bottom" | "block-start" | "block-end")) => {
                c.c.caption_bottom = matches!(k, "bottom" | "block-end");
                true
            }
            _ => false,
        }),
        h("list-style", |c, v| v.iter().filter_map(Tok::ident).find_map(list_style)
            .map(|l| c.c.list_style = l).is_some()),
        // -- interaction --------------------------------------------------------------------
        h("cursor", |c, v| {
            let name = v.iter().filter_map(Tok::ident).next_back();
            match name.and_then(|n| CURSORS.iter().find(|(k, _)| *k == n)) {
                Some((_, -1)) => c.c.cursor = None,
                Some((_, id)) => c.c.cursor = Some(*id),
                None => return false,
            }
            true
        }),
        h("pointer-events", |c, v| {
            c.c.pointer_events = kw(v) != Some("none");
            true
        }),
        h("accent-color", |c, v| match kw(v) {
            Some("auto") => {
                c.c.accent = None;
                true
            }
            _ => v.first().and_then(|t| color(t, c.c.color)).map(|col| c.c.accent = Some(col)).is_some(),
        }),
    ]
};

fn pair(c: &mut Ctx, v: &[Tok], a: &str, b: &str) -> bool {
    let (x, y) = match v {
        [one] => (one.clone(), one.clone()),
        [x, y] => (x.clone(), y.clone()),
        _ => return false,
    };
    delegate(c, a, &[x]) && delegate(c, b, &[y])
}

fn two_kw(c: &mut Ctx, v: &[Tok], a: &str, b: &str) -> bool {
    let (x, y) = match v {
        [one] => (one.clone(), one.clone()),
        [x, y] => (x.clone(), y.clone()),
        _ => return false,
    };
    delegate(c, a, &[x]) && delegate(c, b, &[y])
}

fn border_style(c: &mut Ctx, side: usize, t: &Tok) -> bool {
    match t.ident() {
        Some(s) if BORDER_STYLES.contains(&s) => {
            let none = s == "none" || s == "hidden";
            c.c.border_drawn[side] = !none;
            if none {
                set_border_width(c, side, 0.0);
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::parse_declarations;

    fn computed(css: &str) -> Computed {
        let parent = Computed::default();
        let mut c = Computed::inherit(&parent);
        let u = Units { font_size: 13.0, viewport: (800.0, 600.0) };
        for d in parse_declarations(css) {
            assert!(apply(&mut c, &parent, u, &d), "not applied: {}", d.prop);
        }
        c
    }

    #[test]
    fn box_model_and_flex() {
        let c = computed("display: flex; padding: 4px 8px; margin: 0 auto; flex: 1; gap: 2px 6px;
                          width: 50%; border: 2px solid #f00; border-radius: 4px");
        assert_eq!(c.display, DisplayKind::Flex);
        assert_eq!(c.layout.padding.left, LengthPercentage::Length(8.0));
        assert_eq!(c.layout.margin.left, LengthPercentageAuto::Auto);
        assert_eq!(c.layout.flex_grow, 1.0);
        assert_eq!(c.layout.size.width, Dimension::Percent(0.5));
        assert_eq!(c.layout.border.top, LengthPercentage::Length(2.0));
        assert_eq!(c.border_color[0], crate::color::rgb(0xff0000));
    }

    #[test]
    fn grid_and_text() {
        let c = computed("grid-template-columns: repeat(3, 1fr) 100px; grid-column: 1 / span 2;
                          font: italic bold 20px/1.5 serif; text-decoration: underline;
                          background: linear-gradient(to right, red, blue); box-shadow: 0 2px 4px #000");
        assert_eq!(c.layout.grid_template_columns.len(), 2);
        assert!(c.italic && c.bold() && c.underline && c.font_size == 20.0);
        assert_eq!(c.gradient.as_ref().unwrap().stops.len(), 2);
        assert_eq!(c.shadows.len(), 1);
    }
}
