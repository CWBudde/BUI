//! gpui's `Styled` methods (tailwind names). Each method is CSS sugar: it
//! pushes a declaration that the one CSS property table applies.

use crate::color::Rgba;
use crate::css::{parse_declarations, tokenize, Decl, Prop, Tok, Value};
use crate::geometry::Length;


/// `prop!("padding")`: the property index, resolved at compile time.
macro_rules! prop {
    ($name:literal) => {
        const { Prop::named($name) }
    };
}

/// A fixed value, tokenized once for the whole program.
macro_rules! fixed_value {
    ($css:literal) => {{
        static V: std::sync::OnceLock<Box<[Tok]>> = std::sync::OnceLock::new();
        Value::Static(V.get_or_init(|| tokenize($css).into_boxed_slice()))
    }};
}

impl Rgba {
    /// CSS text: `rgba(r, g, b, a)`.
    pub fn to_css(self) -> String {
        format!("rgba({},{},{},{})", (self.r * 255.0).round(), (self.g * 255.0).round(),
                (self.b * 255.0).round(), self.a)
    }
}

/// What `.style()` accepts: `css!` output (compile time) or CSS text (run time).
pub trait StyleBlock {
    fn push_into(self, out: &mut Vec<Decl>);
}

impl<const N: usize> StyleBlock for [Decl; N] {
    fn push_into(self, out: &mut Vec<Decl>) {
        out.extend(self);
    }
}
impl StyleBlock for &str {
    fn push_into(self, out: &mut Vec<Decl>) {
        out.extend(parse_declarations(self));
    }
}
impl StyleBlock for String {
    fn push_into(self, out: &mut Vec<Decl>) {
        out.extend(parse_declarations(&self));
    }
}
impl StyleBlock for &String {
    fn push_into(self, out: &mut Vec<Decl>) {
        out.extend(parse_declarations(self));
    }
}

/// Font weights (gpui's `FontWeight`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: FontWeight = FontWeight(100);
    pub const EXTRA_LIGHT: FontWeight = FontWeight(200);
    pub const LIGHT: FontWeight = FontWeight(300);
    pub const NORMAL: FontWeight = FontWeight(400);
    pub const MEDIUM: FontWeight = FontWeight(500);
    pub const SEMIBOLD: FontWeight = FontWeight(600);
    pub const BOLD: FontWeight = FontWeight(700);
    pub const EXTRA_BOLD: FontWeight = FontWeight(800);
    pub const BLACK: FontWeight = FontWeight(900);
}

/// A spacing/sizing method with the whole tailwind scale:
/// `p(l)`, `p_0`, `p_0p5`, `p_1` ... `p_96`, `p_px`.
macro_rules! scale {
    ($name:ident: $($prop:literal),+) => {
        paste::paste! {
            fn $name(self, length: impl Into<Length>) -> Self {
                let v = Tok::length(length.into());
                self$(.decl(Decl::one(prop!($prop), v.clone())))+
            }
            fn [<$name _0>](self) -> Self { self.$name(Length::Px(0.0)) }
            fn [<$name _0p5>](self) -> Self { self.$name(Length::Rem(0.125)) }
            fn [<$name _1>](self) -> Self { self.$name(Length::Rem(0.25)) }
            fn [<$name _1p5>](self) -> Self { self.$name(Length::Rem(0.375)) }
            fn [<$name _2>](self) -> Self { self.$name(Length::Rem(0.5)) }
            fn [<$name _2p5>](self) -> Self { self.$name(Length::Rem(0.625)) }
            fn [<$name _3>](self) -> Self { self.$name(Length::Rem(0.75)) }
            fn [<$name _3p5>](self) -> Self { self.$name(Length::Rem(0.875)) }
            fn [<$name _4>](self) -> Self { self.$name(Length::Rem(1.0)) }
            fn [<$name _5>](self) -> Self { self.$name(Length::Rem(1.25)) }
            fn [<$name _6>](self) -> Self { self.$name(Length::Rem(1.5)) }
            fn [<$name _7>](self) -> Self { self.$name(Length::Rem(1.75)) }
            fn [<$name _8>](self) -> Self { self.$name(Length::Rem(2.0)) }
            fn [<$name _9>](self) -> Self { self.$name(Length::Rem(2.25)) }
            fn [<$name _10>](self) -> Self { self.$name(Length::Rem(2.5)) }
            fn [<$name _11>](self) -> Self { self.$name(Length::Rem(2.75)) }
            fn [<$name _12>](self) -> Self { self.$name(Length::Rem(3.0)) }
            fn [<$name _16>](self) -> Self { self.$name(Length::Rem(4.0)) }
            fn [<$name _20>](self) -> Self { self.$name(Length::Rem(5.0)) }
            fn [<$name _24>](self) -> Self { self.$name(Length::Rem(6.0)) }
            fn [<$name _32>](self) -> Self { self.$name(Length::Rem(8.0)) }
            fn [<$name _40>](self) -> Self { self.$name(Length::Rem(10.0)) }
            fn [<$name _48>](self) -> Self { self.$name(Length::Rem(12.0)) }
            fn [<$name _56>](self) -> Self { self.$name(Length::Rem(14.0)) }
            fn [<$name _64>](self) -> Self { self.$name(Length::Rem(16.0)) }
            fn [<$name _72>](self) -> Self { self.$name(Length::Rem(18.0)) }
            fn [<$name _80>](self) -> Self { self.$name(Length::Rem(20.0)) }
            fn [<$name _96>](self) -> Self { self.$name(Length::Rem(24.0)) }
            fn [<$name _px>](self) -> Self { self.$name(Length::Px(1.0)) }
            fn [<$name _auto>](self) -> Self { self.$name(Length::Auto) }
        }
    };
}

/// Fractions for sizes: `w_full`, `w_1_2`, `w_2_3` ...
macro_rules! fractions {
    ($name:ident) => {
        paste::paste! {
            fn [<$name _full>](self) -> Self { self.$name(Length::Percent(100.0)) }
            fn [<$name _1_2>](self) -> Self { self.$name(Length::Percent(50.0)) }
            fn [<$name _1_3>](self) -> Self { self.$name(Length::Percent(100.0 / 3.0)) }
            fn [<$name _2_3>](self) -> Self { self.$name(Length::Percent(200.0 / 3.0)) }
            fn [<$name _1_4>](self) -> Self { self.$name(Length::Percent(25.0)) }
            fn [<$name _3_4>](self) -> Self { self.$name(Length::Percent(75.0)) }
            fn [<$name _1_5>](self) -> Self { self.$name(Length::Percent(20.0)) }
            fn [<$name _2_5>](self) -> Self { self.$name(Length::Percent(40.0)) }
            fn [<$name _3_5>](self) -> Self { self.$name(Length::Percent(60.0)) }
            fn [<$name _4_5>](self) -> Self { self.$name(Length::Percent(80.0)) }
            fn [<$name _1_6>](self) -> Self { self.$name(Length::Percent(100.0 / 6.0)) }
            fn [<$name _5_6>](self) -> Self { self.$name(Length::Percent(500.0 / 6.0)) }
        }
    };
}

/// Methods that set one fixed declaration: `flex() => display: flex`. The
/// property index is resolved at compile time and the value parsed once.
macro_rules! fixed {
    ($($name:ident => $prop:literal : $value:literal),* $(,)?) => {
        $(fn $name(self) -> Self {
            self.decl(Decl { prop: prop!($prop), value: fixed_value!($value), important: false })
        })*
    };
}

/// Style methods shared by elements and `.hover(|s| ...)` refinements.
pub trait Styled: Sized {
    fn decls_mut(&mut self) -> &mut Vec<Decl>;

    /// One CSS declaration, parsed at run time:
    /// `.css("grid-template-columns", "1fr 2fr")`.
    fn css(mut self, property: &'static str, value: impl AsRef<str>) -> Self {
        self.decls_mut().push(Decl::new(property, value.as_ref()));
        self
    }

    /// A declaration built by the caller (typed values, no parsing).
    fn decl(mut self, d: Decl) -> Self {
        self.decls_mut().push(d);
        self
    }

    /// A CSS declaration block: `.style(css!("display: flex; gap: 4px"))`
    /// (checked and resolved at compile time) or `.style("...")` (parsed
    /// at run time, for text built at run time).
    fn style(mut self, block: impl StyleBlock) -> Self {
        block.push_into(self.decls_mut());
        self
    }

    // -- spacing and sizing --------------------------------------------------------------
    scale!(p: "padding");
    scale!(px: "padding-left", "padding-right");
    scale!(py: "padding-top", "padding-bottom");
    scale!(pt: "padding-top");
    scale!(pr: "padding-right");
    scale!(pb: "padding-bottom");
    scale!(pl: "padding-left");
    scale!(m: "margin");
    scale!(mx: "margin-left", "margin-right");
    scale!(my: "margin-top", "margin-bottom");
    scale!(mt: "margin-top");
    scale!(mr: "margin-right");
    scale!(mb: "margin-bottom");
    scale!(ml: "margin-left");
    scale!(gap: "gap");
    scale!(gap_x: "column-gap");
    scale!(gap_y: "row-gap");
    scale!(w: "width");
    scale!(h: "height");
    scale!(size: "width", "height");
    scale!(min_w: "min-width");
    scale!(min_h: "min-height");
    scale!(max_w: "max-width");
    scale!(max_h: "max-height");
    scale!(top: "top");
    scale!(right: "right");
    scale!(bottom: "bottom");
    scale!(left: "left");
    scale!(inset: "inset");
    scale!(flex_basis: "flex-basis");
    scale!(rounded: "border-radius");
    fractions!(w);
    fractions!(h);
    fractions!(size);
    fractions!(min_w);
    fractions!(min_h);
    fractions!(max_w);
    fractions!(max_h);
    fractions!(top);
    fractions!(right);
    fractions!(bottom);
    fractions!(left);
    fractions!(inset);
    fractions!(flex_basis);

    // -- display, position, overflow ----------------------------------------------------------
    fixed! {
        block => "display": "block", flex => "display": "flex", grid => "display": "grid",
        inline => "display": "inline", inline_block => "display": "inline-block",
        inline_flex => "display": "inline-flex", contents => "display": "contents",
        hidden => "display": "none",
        visible => "visibility": "visible", invisible => "visibility": "hidden",
        relative => "position": "relative", absolute => "position": "absolute",
        fixed_position => "position": "fixed",
        overflow_hidden => "overflow": "hidden", overflow_visible => "overflow": "visible",
        overflow_scroll => "overflow": "scroll", overflow_x_hidden => "overflow-x": "hidden",
        overflow_y_hidden => "overflow-y": "hidden", overflow_x_scroll => "overflow-x": "scroll",
        overflow_y_scroll => "overflow-y": "scroll", overflow_clip => "overflow": "clip",
        box_border => "box-sizing": "border-box", box_content => "box-sizing": "content-box",
    }

    // -- flexbox ----------------------------------------------------------------------------
    fixed! {
        flex_row => "flex-direction": "row", flex_col => "flex-direction": "column",
        flex_row_reverse => "flex-direction": "row-reverse",
        flex_col_reverse => "flex-direction": "column-reverse",
        flex_wrap => "flex-wrap": "wrap", flex_wrap_reverse => "flex-wrap": "wrap-reverse",
        flex_nowrap => "flex-wrap": "nowrap",
        flex_1 => "flex": "1 1 0%", flex_auto => "flex": "auto", flex_initial => "flex": "initial",
        flex_none => "flex": "none", flex_grow => "flex-grow": "1", flex_grow_0 => "flex-grow": "0",
        flex_shrink => "flex-shrink": "1", flex_shrink_0 => "flex-shrink": "0",
        items_start => "align-items": "flex-start", items_end => "align-items": "flex-end",
        items_center => "align-items": "center", items_baseline => "align-items": "baseline",
        items_stretch => "align-items": "stretch",
        self_start => "align-self": "flex-start", self_end => "align-self": "flex-end",
        self_center => "align-self": "center", self_flex_start => "align-self": "flex-start",
        self_flex_end => "align-self": "flex-end", self_baseline => "align-self": "baseline",
        self_stretch => "align-self": "stretch",
        justify_start => "justify-content": "flex-start", justify_end => "justify-content": "flex-end",
        justify_center => "justify-content": "center",
        justify_between => "justify-content": "space-between",
        justify_around => "justify-content": "space-around",
        justify_evenly => "justify-content": "space-evenly",
        justify_stretch => "justify-content": "stretch",
        content_normal => "align-content": "normal", content_center => "align-content": "center",
        content_start => "align-content": "flex-start", content_end => "align-content": "flex-end",
        content_between => "align-content": "space-between",
        content_around => "align-content": "space-around",
        content_evenly => "align-content": "space-evenly", content_stretch => "align-content": "stretch",
    }

    fn order(self, n: i32) -> Self {
        self.decl(Decl::one(prop!("order"), Tok::Num(n as f32)))
    }

    // -- grid -------------------------------------------------------------------------------
    fn grid_cols(self, n: u16) -> Self {
        self.css("grid-template-columns", format!("repeat({n}, minmax(0, 1fr))"))
    }
    fn grid_rows(self, n: u16) -> Self {
        self.css("grid-template-rows", format!("repeat({n}, minmax(0, 1fr))"))
    }
    fn col_span(self, n: u16) -> Self {
        self.css("grid-column", format!("span {n}"))
    }
    fn col_span_full(self) -> Self {
        self.decl(Decl { prop: prop!("grid-column"), value: fixed_value!("1 / -1"), important: false })
    }
    fn col_start(self, n: i16) -> Self {
        self.decl(Decl::one(prop!("grid-column-start"), Tok::Num(n as f32)))
    }
    fn col_end(self, n: i16) -> Self {
        self.decl(Decl::one(prop!("grid-column-end"), Tok::Num(n as f32)))
    }
    fn row_span(self, n: u16) -> Self {
        self.css("grid-row", format!("span {n}"))
    }
    fn row_span_full(self) -> Self {
        self.decl(Decl { prop: prop!("grid-row"), value: fixed_value!("1 / -1"), important: false })
    }
    fn row_start(self, n: i16) -> Self {
        self.decl(Decl::one(prop!("grid-row-start"), Tok::Num(n as f32)))
    }
    fn row_end(self, n: i16) -> Self {
        self.decl(Decl::one(prop!("grid-row-end"), Tok::Num(n as f32)))
    }

    // -- colors -------------------------------------------------------------------------------
    fn bg(self, color: impl Into<Rgba>) -> Self {
        self.decl(Decl::one(prop!("background-color"), Tok::Color(color.into())))
    }
    fn text_color(self, color: impl Into<Rgba>) -> Self {
        self.decl(Decl::one(prop!("color"), Tok::Color(color.into())))
    }
    fn border_color(self, color: impl Into<Rgba>) -> Self {
        self.decl(Decl::one(prop!("border-color"), Tok::Color(color.into())))
    }
    fn outline_color(self, color: impl Into<Rgba>) -> Self {
        self.decl(Decl::one(prop!("outline-color"), Tok::Color(color.into())))
    }
    fn accent_color(self, color: impl Into<Rgba>) -> Self {
        self.decl(Decl::one(prop!("accent-color"), Tok::Color(color.into())))
    }
    fn opacity(self, value: f32) -> Self {
        self.decl(Decl::one(prop!("opacity"), Tok::Num(value)))
    }

    // -- borders ----------------------------------------------------------------------------
    fixed! {
        border_0 => "border-width": "0", border_1 => "border-width": "1px",
        border_2 => "border-width": "2px", border_4 => "border-width": "4px",
        border_8 => "border-width": "8px",
        border_t_1 => "border-top-width": "1px", border_r_1 => "border-right-width": "1px",
        border_b_1 => "border-bottom-width": "1px", border_l_1 => "border-left-width": "1px",
        border_t_2 => "border-top-width": "2px", border_r_2 => "border-right-width": "2px",
        border_b_2 => "border-bottom-width": "2px", border_l_2 => "border-left-width": "2px",
        border_x_1 => "border-inline": "1px solid", border_y_1 => "border-block": "1px solid",
        border_solid => "border-style": "solid", border_dashed => "border-style": "dashed",
        border_dotted => "border-style": "dotted", border_none => "border-style": "none",
        rounded_none => "border-radius": "0", rounded_xs => "border-radius": "0.125rem",
        rounded_sm => "border-radius": "0.25rem", rounded_md => "border-radius": "0.375rem",
        rounded_lg => "border-radius": "0.5rem", rounded_xl => "border-radius": "0.75rem",
        rounded_2xl => "border-radius": "1rem", rounded_3xl => "border-radius": "1.5rem",
        rounded_full => "border-radius": "9999px",
        rounded_t_md => "border-top-left-radius": "0.375rem",
        rounded_b_md => "border-bottom-left-radius": "0.375rem",
    }

    // -- shadows ------------------------------------------------------------------------------
    fixed! {
        shadow_none => "box-shadow": "none",
        shadow_2xs => "box-shadow": "0 1px rgba(0,0,0,0.05)",
        shadow_xs => "box-shadow": "0 1px 2px rgba(0,0,0,0.05)",
        shadow_sm => "box-shadow": "0 1px 3px rgba(0,0,0,0.1)",
        shadow_md => "box-shadow": "0 4px 6px -1px rgba(0,0,0,0.1)",
        shadow_lg => "box-shadow": "0 10px 15px -3px rgba(0,0,0,0.1)",
        shadow_xl => "box-shadow": "0 20px 25px -5px rgba(0,0,0,0.1)",
        shadow_2xl => "box-shadow": "0 25px 50px -12px rgba(0,0,0,0.25)",
    }

    // -- text ---------------------------------------------------------------------------------
    fixed! {
        text_xs => "font-size": "0.75rem", text_sm => "font-size": "0.875rem",
        text_base => "font-size": "1rem", text_lg => "font-size": "1.125rem",
        text_xl => "font-size": "1.25rem", text_2xl => "font-size": "1.5rem",
        text_3xl => "font-size": "1.875rem", text_4xl => "font-size": "2.25rem",
        text_5xl => "font-size": "3rem", text_6xl => "font-size": "3.75rem",
        font_thin => "font-weight": "100", font_light => "font-weight": "300",
        font_normal => "font-weight": "400", font_medium => "font-weight": "500",
        font_semibold => "font-weight": "600", font_bold => "font-weight": "700",
        font_extrabold => "font-weight": "800", font_black => "font-weight": "900",
        italic => "font-style": "italic", not_italic => "font-style": "normal",
        underline => "text-decoration": "underline", line_through => "text-decoration": "line-through",
        overline => "text-decoration": "overline", no_underline => "text-decoration": "none",
        text_left => "text-align": "left", text_center => "text-align": "center",
        text_right => "text-align": "right", text_justify => "text-align": "justify",
        uppercase => "text-transform": "uppercase", lowercase => "text-transform": "lowercase",
        capitalize => "text-transform": "capitalize", normal_case => "text-transform": "none",
        whitespace_normal => "white-space": "normal", whitespace_nowrap => "white-space": "nowrap",
        whitespace_pre => "white-space": "pre", whitespace_pre_wrap => "white-space": "pre-wrap",
        whitespace_pre_line => "white-space": "pre-line",
        text_ellipsis => "text-overflow": "ellipsis",
        leading_none => "line-height": "1", leading_tight => "line-height": "1.25",
        leading_snug => "line-height": "1.375", leading_normal => "line-height": "1.5",
        leading_relaxed => "line-height": "1.625", leading_loose => "line-height": "2",
    }

    /// Single line, clipped with an ellipsis.
    fn truncate(self) -> Self {
        self.decl(Decl::one(prop!("overflow"), Tok::Ident("hidden".into())))
            .decl(Decl::one(prop!("white-space"), Tok::Ident("nowrap".into())))
            .decl(Decl::one(prop!("text-overflow"), Tok::Ident("ellipsis".into())))
    }
    fn text_size(self, size: impl Into<Length>) -> Self {
        self.decl(Decl::one(prop!("font-size"), Tok::length(size.into())))
    }
    fn line_height(self, h: impl Into<Length>) -> Self {
        self.decl(Decl::one(prop!("line-height"), Tok::length(h.into())))
    }
    fn font_weight(self, w: FontWeight) -> Self {
        self.decl(Decl::one(prop!("font-weight"), Tok::Num(w.0 as f32)))
    }
    fn font_family(self, family: impl AsRef<str>) -> Self {
        self.css("font-family", family)
    }

    // -- interaction ----------------------------------------------------------------------------
    fixed! {
        cursor_default => "cursor": "default", cursor_pointer => "cursor": "pointer",
        cursor_text => "cursor": "text", cursor_move => "cursor": "move",
        cursor_not_allowed => "cursor": "not-allowed", cursor_context_menu => "cursor": "context-menu",
        cursor_crosshair => "cursor": "crosshair", cursor_grab => "cursor": "grab",
        cursor_grabbing => "cursor": "grabbing", cursor_ew_resize => "cursor": "ew-resize",
        cursor_ns_resize => "cursor": "ns-resize", cursor_col_resize => "cursor": "col-resize",
        cursor_row_resize => "cursor": "row-resize", cursor_none => "cursor": "none",
        pointer_events_none => "pointer-events": "none",
    }

    fn z_index(self, z: i32) -> Self {
        self.decl(Decl::one(prop!("z-index"), Tok::Num(z as f32)))
    }
}
