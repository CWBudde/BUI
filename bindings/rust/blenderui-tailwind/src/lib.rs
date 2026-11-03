//! Tailwind CSS v4: class names to CSS, like Tailwind's JIT engine.
//!
//! `compile("md:hover:bg-sky-500/50")` gives the rule for that class:
//! variants (state, pseudo-element, `group-*` / `peer-*`, `has-*`, `data-*`,
//! `aria-*`, `nth-*`, breakpoints, container queries, `dark`, `motion-*`,
//! `supports-*`, arbitrary `[&_p]` ...), utilities with the v4 theme,
//! arbitrary values (`w-[calc(100%-2rem)]`), CSS variable shorthand
//! (`bg-(--brand)`), negatives, opacity modifiers and `!important`.
//!
//! Self-contained (std only) so the `tw!` macro can check classes at
//! compile time with the same code.

mod theme;
use theme::THEME;

/// A compiled class: the CSS (inside `@layer utilities`) and its sort key
/// (Tailwind's order: variants, then property order).
#[derive(Clone, Debug, PartialEq)]
pub struct Compiled {
    pub css: String,
    pub sort: (u32, u32),
}

fn theme(name: &str) -> Option<&'static str> {
    THEME.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

/// The rule for `class`, or None when it isn't a Tailwind class.
pub fn compile(class: &str) -> Option<Compiled> {
    let parts = split_variants(class)?;
    let (variants, utility) = parts.split_at(parts.len() - 1);
    let mut utility = utility[0];
    let mut important = false;
    if let Some(u) = utility.strip_suffix('!') {
        utility = u;
        important = true;
    } else if let Some(u) = utility.strip_prefix('!') {
        utility = u;
        important = true;
    }
    let mut out = utility_decls(utility)?;
    let mut sel = format!(".{}", escape(class));
    let mut wrappers: Vec<String> = Vec::new();
    let mut vweight = 0u32;
    let mut pseudo_el = false;
    // variants apply right to left: the leftmost wraps everything
    for v in variants.iter().rev() {
        let (s, w, weight) = variant(v, &sel)?;
        if let Some(s) = s {
            pseudo_el |= s.contains("::before") || s.contains("::after");
            sel = s;
        }
        if let Some(w) = w {
            wrappers.push(w);
        }
        vweight = vweight.max(weight);
    }
    if pseudo_el && !out.decls.iter().any(|(p, _)| p == "content") {
        out.decls.insert(0, ("content".into(), "''".into()));
    }
    let bang = if important { " !important" } else { "" };
    let body: String = out.decls.iter().map(|(p, v)| format!("{p}: {v}{bang}; ")).collect();
    let mut css = match &out.child {
        Some(child) => format!("{} {{ {body}}}", child.replace('&', &sel)),
        None => format!("{sel} {{ {body}}}"),
    };
    for w in wrappers.iter() {
        css = format!("{w} {{ {css} }}");
    }
    let mut full = format!("@layer utilities {{ {css} }}");
    if let Some(k) = out.keyframes {
        full.push('\n');
        full.push_str(k);
    }
    let prank = out.decls.first().map(|(p, _)| prop_rank(p)).unwrap_or(0);
    Some(Compiled { css: full, sort: (vweight, prank) })
}

/// Split `a:b:[&>p]:c` at top-level colons.
fn split_variants(class: &str) -> Option<Vec<&str>> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    for (k, c) in class.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ':' if depth == 0 => {
                parts.push(&class[start..k]);
                start = k + 1;
            }
            _ => {}
        }
    }
    parts.push(&class[start..]);
    (!parts.iter().any(|p| p.is_empty())).then_some(parts)
}

/// Escape a class name for use in a selector.
pub fn escape(class: &str) -> String {
    let mut out = String::with_capacity(class.len() + 8);
    for (k, c) in class.chars().enumerate() {
        if c.is_ascii_alphanumeric() && !(k == 0 && c.is_ascii_digit()) || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('\\');
            out.push(c);
        }
    }
    out
}

/// `[a_b]` contents: underscores are spaces (`\_` stays an underscore).
fn arbitrary(s: &str) -> Option<String> {
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let mut out = String::with_capacity(inner.len());
    let mut it = inner.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' if it.peek() == Some(&'_') => {
                out.push('_');
                it.next();
            }
            '_' => out.push(' '),
            c => out.push(c),
        }
    }
    Some(out)
}

/// `(--x)` -> `var(--x)`.
fn css_var(s: &str) -> Option<String> {
    let inner = s.strip_prefix('(')?.strip_suffix(')')?;
    let inner = inner.split_once(':').map(|(_, v)| v).unwrap_or(inner);
    inner.starts_with("--").then(|| format!("var({inner})"))
}

/// An arbitrary or variable value, with an optional type hint (`[length:..]`).
fn free(s: &str) -> Option<(Option<String>, String)> {
    if let Some(v) = css_var(s) {
        let hint = s.trim_start_matches('(').split_once(':').map(|(h, _)| h.to_string());
        return Some((hint, v));
    }
    let a = arbitrary(s)?;
    for hint in ["length", "color", "number", "percentage", "url", "image", "position", "size", "family-name",
                 "absolute-size", "relative-size", "line-width", "bg-size", "integer", "angle", "ratio"] {
        if let Some(v) = a.strip_prefix(hint).and_then(|r| r.strip_prefix(':')) {
            return Some((Some(hint.to_string()), v.to_string()));
        }
    }
    Some((None, a))
}

fn is_color_value(v: &str) -> bool {
    let v = v.trim();
    v.starts_with('#') || v.starts_with("rgb") || v.starts_with("hsl") || v.starts_with("oklch")
        || v.starts_with("oklab") || v.starts_with("lab(") || v.starts_with("lch(") || v.starts_with("hwb(")
        || v.starts_with("color(") || v.starts_with("color-mix(") || v.starts_with("light-dark(")
        || v == "transparent" || v == "currentColor" || v == "currentcolor"
        || NAMED_COLORS.contains(&v.to_ascii_lowercase().as_str())
}

const NAMED_COLORS: &[&str] = &[
    "black", "white", "red", "green", "blue", "yellow", "orange", "purple", "pink", "gray", "grey", "silver",
    "maroon", "navy", "teal", "olive", "lime", "aqua", "fuchsia", "cyan", "magenta", "brown", "gold", "indigo",
    "violet", "coral", "salmon", "tomato", "crimson", "khaki", "beige", "ivory", "lavender", "plum", "orchid",
    "tan", "chocolate", "sienna", "peru", "wheat", "linen", "azure", "snow", "mintcream", "honeydew",
    "rebeccapurple", "darkred", "darkgreen", "darkblue", "lightgray", "lightgrey", "darkgray", "darkgrey",
];

/// A spacing value: `4` -> 1rem, `px`, fractions, keywords, arbitrary.
fn spacing(v: &str) -> Option<String> {
    if v == "px" {
        return Some("1px".into());
    }
    if let Some((_, a)) = free(v) {
        return Some(a);
    }
    if let Ok(n) = v.parse::<f32>() {
        if n >= 0.0 && (n * 4.0).fract() == 0.0 {
            let base = theme("spacing")?;
            return Some(if n == 0.0 { "0".into() } else { format!("calc({base} * {})", trim_num(n)) });
        }
    }
    None
}

fn fraction(v: &str) -> Option<String> {
    let (a, b) = v.split_once('/')?;
    let (a, b) = (a.parse::<f32>().ok()?, b.parse::<f32>().ok()?);
    (b != 0.0).then(|| format!("{}%", trim_num(a / b * 100.0)))
}

fn trim_num(n: f32) -> String {
    let s = format!("{n:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Theme colors (`sky-500`), keywords and arbitrary colors, with an
/// optional opacity modifier (`/50`, `/[0.3]`).
fn color(v: &str) -> Option<String> {
    let (base, alpha) = split_modifier(v);
    let c = match base {
        "transparent" => "transparent".to_string(),
        "current" => "currentColor".to_string(),
        "inherit" => "inherit".to_string(),
        "black" | "white" => theme(&format!("color-{base}"))?.to_string(),
        _ => match free(base) {
            Some((hint, a)) if hint.as_deref().is_none_or(|h| h == "color") => {
                if hint.is_none() && !is_color_value(&a) && !a.starts_with("var(") {
                    return None;
                }
                a
            }
            Some(_) => return None,
            None => theme(&format!("color-{base}"))?.to_string(),
        },
    };
    Some(match alpha.and_then(opacity) {
        Some(p) => format!("color-mix(in oklab, {c} {p}, transparent)"),
        None => c,
    })
}

/// `50` -> `50%`, `[0.3]` -> `30%`, `(--a)` -> var.
fn opacity(v: &str) -> Option<String> {
    if let Ok(n) = v.parse::<f32>() {
        return Some(format!("{}%", trim_num(n)));
    }
    let (_, a) = free(v)?;
    Some(match a.parse::<f32>() {
        Ok(n) if n <= 1.0 => format!("{}%", trim_num(n * 100.0)),
        _ => a,
    })
}

/// `bg-red-500/50` -> (`red-500`, `50`); not inside brackets.
fn split_modifier(v: &str) -> (&str, Option<&str>) {
    let mut depth = 0;
    for (k, c) in v.char_indices().rev() {
        match c {
            ']' | ')' => depth += 1,
            '[' | '(' => depth -= 1,
            '/' if depth == 0 => return (&v[..k], Some(&v[k + 1..])),
            _ => {}
        }
    }
    (v, None)
}

/// Utility output: declarations, an optional child selector (`& > :not(:last-child)`)
/// and keyframes it needs.
struct Out {
    decls: Vec<(String, String)>,
    child: Option<String>,
    keyframes: Option<&'static str>,
}

fn decls(list: &[(&str, &str)]) -> Option<Out> {
    Some(Out { decls: list.iter().map(|(p, v)| (p.to_string(), v.to_string())).collect(), child: None, keyframes: None })
}

fn one(p: &str, v: String) -> Option<Out> {
    Some(Out { decls: vec![(p.to_string(), v)], child: None, keyframes: None })
}

fn many(props: &[&str], v: String) -> Option<Out> {
    Some(Out { decls: props.iter().map(|p| (p.to_string(), v.clone())).collect(), child: None, keyframes: None })
}

/// Static utilities: exact class name -> declarations.
const STATIC: &[(&str, &[(&str, &str)])] = &[
    // layout
    ("block", &[("display", "block")]), ("inline-block", &[("display", "inline-block")]),
    ("inline", &[("display", "inline")]), ("flex", &[("display", "flex")]),
    ("inline-flex", &[("display", "inline-flex")]), ("grid", &[("display", "grid")]),
    ("inline-grid", &[("display", "inline-grid")]), ("table", &[("display", "table")]),
    ("inline-table", &[("display", "inline-table")]), ("table-row", &[("display", "table-row")]),
    ("table-cell", &[("display", "table-cell")]), ("table-caption", &[("display", "table-caption")]),
    ("table-row-group", &[("display", "table-row-group")]), ("table-header-group", &[("display", "table-header-group")]),
    ("table-footer-group", &[("display", "table-footer-group")]), ("table-column", &[("display", "table-column")]),
    ("table-column-group", &[("display", "table-column-group")]),
    ("contents", &[("display", "contents")]), ("list-item", &[("display", "list-item")]),
    ("hidden", &[("display", "none")]), ("flow-root", &[("display", "flow-root")]),
    ("sr-only", &[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("padding", "0"),
                  ("margin", "-1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap"),
                  ("border-width", "0")]),
    ("not-sr-only", &[("position", "static"), ("width", "auto"), ("height", "auto"), ("padding", "0"),
                      ("margin", "0"), ("overflow", "visible"), ("clip-path", "none"), ("white-space", "normal")]),
    ("static", &[("position", "static")]), ("fixed", &[("position", "fixed")]),
    ("absolute", &[("position", "absolute")]), ("relative", &[("position", "relative")]),
    ("sticky", &[("position", "sticky")]),
    ("visible", &[("visibility", "visible")]), ("invisible", &[("visibility", "hidden")]),
    ("collapse", &[("visibility", "collapse")]),
    ("isolate", &[("isolation", "isolate")]), ("isolation-auto", &[("isolation", "auto")]),
    ("box-border", &[("box-sizing", "border-box")]), ("box-content", &[("box-sizing", "content-box")]),
    ("float-left", &[("float", "left")]), ("float-right", &[("float", "right")]),
    ("float-start", &[("float", "inline-start")]), ("float-end", &[("float", "inline-end")]),
    ("float-none", &[("float", "none")]),
    ("clear-left", &[("clear", "left")]), ("clear-right", &[("clear", "right")]),
    ("clear-both", &[("clear", "both")]), ("clear-none", &[("clear", "none")]),
    ("clear-start", &[("clear", "inline-start")]), ("clear-end", &[("clear", "inline-end")]),
    ("container", &[("width", "100%")]),
    ("truncate", &[("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]),
    // flex & grid
    ("flex-row", &[("flex-direction", "row")]), ("flex-row-reverse", &[("flex-direction", "row-reverse")]),
    ("flex-col", &[("flex-direction", "column")]), ("flex-col-reverse", &[("flex-direction", "column-reverse")]),
    ("flex-wrap", &[("flex-wrap", "wrap")]), ("flex-wrap-reverse", &[("flex-wrap", "wrap-reverse")]),
    ("flex-nowrap", &[("flex-wrap", "nowrap")]),
    ("flex-auto", &[("flex", "auto")]), ("flex-initial", &[("flex", "0 auto")]), ("flex-none", &[("flex", "none")]),
    ("grow", &[("flex-grow", "1")]), ("shrink", &[("flex-shrink", "1")]),
    ("grid-flow-row", &[("grid-auto-flow", "row")]), ("grid-flow-col", &[("grid-auto-flow", "column")]),
    ("grid-flow-dense", &[("grid-auto-flow", "dense")]), ("grid-flow-row-dense", &[("grid-auto-flow", "row dense")]),
    ("grid-flow-col-dense", &[("grid-auto-flow", "column dense")]),
    ("col-auto", &[("grid-column", "auto")]), ("row-auto", &[("grid-row", "auto")]),
    ("col-span-full", &[("grid-column", "1 / -1")]), ("row-span-full", &[("grid-row", "1 / -1")]),
    ("auto-cols-auto", &[("grid-auto-columns", "auto")]), ("auto-cols-min", &[("grid-auto-columns", "min-content")]),
    ("auto-cols-max", &[("grid-auto-columns", "max-content")]), ("auto-cols-fr", &[("grid-auto-columns", "minmax(0, 1fr)")]),
    ("auto-rows-auto", &[("grid-auto-rows", "auto")]), ("auto-rows-min", &[("grid-auto-rows", "min-content")]),
    ("auto-rows-max", &[("grid-auto-rows", "max-content")]), ("auto-rows-fr", &[("grid-auto-rows", "minmax(0, 1fr)")]),
    ("grid-cols-none", &[("grid-template-columns", "none")]), ("grid-rows-none", &[("grid-template-rows", "none")]),
    ("grid-cols-subgrid", &[("grid-template-columns", "subgrid")]), ("grid-rows-subgrid", &[("grid-template-rows", "subgrid")]),
    // typography
    ("italic", &[("font-style", "italic")]), ("not-italic", &[("font-style", "normal")]),
    ("underline", &[("text-decoration-line", "underline")]), ("overline", &[("text-decoration-line", "overline")]),
    ("line-through", &[("text-decoration-line", "line-through")]), ("no-underline", &[("text-decoration-line", "none")]),
    ("uppercase", &[("text-transform", "uppercase")]), ("lowercase", &[("text-transform", "lowercase")]),
    ("capitalize", &[("text-transform", "capitalize")]), ("normal-case", &[("text-transform", "none")]),
    ("text-ellipsis", &[("text-overflow", "ellipsis")]), ("text-clip", &[("text-overflow", "clip")]),
    ("text-wrap", &[("text-wrap", "wrap")]), ("text-nowrap", &[("text-wrap", "nowrap")]),
    ("text-balance", &[("text-wrap", "balance")]), ("text-pretty", &[("text-wrap", "pretty")]),
    ("antialiased", &[("-webkit-font-smoothing", "antialiased")]), ("subpixel-antialiased", &[("-webkit-font-smoothing", "auto")]),
    ("ordinal", &[("font-variant-numeric", "ordinal")]), ("slashed-zero", &[("font-variant-numeric", "slashed-zero")]),
    ("lining-nums", &[("font-variant-numeric", "lining-nums")]), ("oldstyle-nums", &[("font-variant-numeric", "oldstyle-nums")]),
    ("proportional-nums", &[("font-variant-numeric", "proportional-nums")]), ("tabular-nums", &[("font-variant-numeric", "tabular-nums")]),
    ("diagonal-fractions", &[("font-variant-numeric", "diagonal-fractions")]), ("stacked-fractions", &[("font-variant-numeric", "stacked-fractions")]),
    ("normal-nums", &[("font-variant-numeric", "normal")]),
    ("break-normal", &[("overflow-wrap", "normal"), ("word-break", "normal")]), ("break-all", &[("word-break", "break-all")]),
    ("break-keep", &[("word-break", "keep-all")]), ("break-words", &[("overflow-wrap", "break-word")]),
    ("wrap-break-word", &[("overflow-wrap", "break-word")]), ("wrap-anywhere", &[("overflow-wrap", "anywhere")]), ("wrap-normal", &[("overflow-wrap", "normal")]),
    ("hyphens-none", &[("hyphens", "none")]), ("hyphens-manual", &[("hyphens", "manual")]), ("hyphens-auto", &[("hyphens", "auto")]),
    ("list-inside", &[("list-style-position", "inside")]), ("list-outside", &[("list-style-position", "outside")]),
    ("list-none", &[("list-style-type", "none")]), ("list-disc", &[("list-style-type", "disc")]),
    ("list-decimal", &[("list-style-type", "decimal")]),
    ("whitespace-normal", &[("white-space", "normal")]), ("whitespace-nowrap", &[("white-space", "nowrap")]),
    ("whitespace-pre", &[("white-space", "pre")]), ("whitespace-pre-line", &[("white-space", "pre-line")]),
    ("whitespace-pre-wrap", &[("white-space", "pre-wrap")]), ("whitespace-break-spaces", &[("white-space", "break-spaces")]),
    ("decoration-solid", &[("text-decoration-style", "solid")]), ("decoration-double", &[("text-decoration-style", "double")]),
    ("decoration-dotted", &[("text-decoration-style", "dotted")]), ("decoration-dashed", &[("text-decoration-style", "dashed")]),
    ("decoration-wavy", &[("text-decoration-style", "wavy")]),
    ("decoration-from-font", &[("text-decoration-thickness", "from-font")]), ("decoration-auto", &[("text-decoration-thickness", "auto")]),
    ("align-baseline", &[("vertical-align", "baseline")]), ("align-top", &[("vertical-align", "top")]),
    ("align-middle", &[("vertical-align", "middle")]), ("align-bottom", &[("vertical-align", "bottom")]),
    ("align-text-top", &[("vertical-align", "text-top")]), ("align-text-bottom", &[("vertical-align", "text-bottom")]),
    ("align-sub", &[("vertical-align", "sub")]), ("align-super", &[("vertical-align", "super")]),
    // backgrounds
    ("bg-fixed", &[("background-attachment", "fixed")]), ("bg-local", &[("background-attachment", "local")]),
    ("bg-scroll", &[("background-attachment", "scroll")]),
    ("bg-clip-border", &[("background-clip", "border-box")]), ("bg-clip-padding", &[("background-clip", "padding-box")]),
    ("bg-clip-content", &[("background-clip", "content-box")]), ("bg-clip-text", &[("background-clip", "text")]),
    ("bg-origin-border", &[("background-origin", "border-box")]), ("bg-origin-padding", &[("background-origin", "padding-box")]),
    ("bg-origin-content", &[("background-origin", "content-box")]),
    ("bg-repeat", &[("background-repeat", "repeat")]), ("bg-no-repeat", &[("background-repeat", "no-repeat")]),
    ("bg-repeat-x", &[("background-repeat", "repeat-x")]), ("bg-repeat-y", &[("background-repeat", "repeat-y")]),
    ("bg-repeat-round", &[("background-repeat", "round")]), ("bg-repeat-space", &[("background-repeat", "space")]),
    ("bg-auto", &[("background-size", "auto")]), ("bg-cover", &[("background-size", "cover")]),
    ("bg-contain", &[("background-size", "contain")]), ("bg-none", &[("background-image", "none")]),
    // borders
    ("border-solid", &[("border-style", "solid")]), ("border-dashed", &[("border-style", "dashed")]),
    ("border-dotted", &[("border-style", "dotted")]), ("border-double", &[("border-style", "double")]),
    ("border-hidden", &[("border-style", "hidden")]), ("border-none", &[("border-style", "none")]),
    ("outline-hidden", &[("outline", "2px solid transparent"), ("outline-offset", "2px")]),
    ("outline-none", &[("outline-style", "none")]), ("outline-solid", &[("outline-style", "solid")]),
    ("outline-dashed", &[("outline-style", "dashed")]), ("outline-dotted", &[("outline-style", "dotted")]),
    ("outline-double", &[("outline-style", "double")]),
    ("border-collapse", &[("border-collapse", "collapse")]), ("border-separate", &[("border-collapse", "separate")]),
    ("table-auto", &[("table-layout", "auto")]), ("table-fixed", &[("table-layout", "fixed")]),
    ("caption-top", &[("caption-side", "top")]), ("caption-bottom", &[("caption-side", "bottom")]),
    // effects & interactivity
    ("pointer-events-none", &[("pointer-events", "none")]), ("pointer-events-auto", &[("pointer-events", "auto")]),
    ("resize-none", &[("resize", "none")]), ("resize", &[("resize", "both")]), ("resize-x", &[("resize", "horizontal")]),
    ("resize-y", &[("resize", "vertical")]),
    ("select-none", &[("user-select", "none")]), ("select-text", &[("user-select", "text")]),
    ("select-all", &[("user-select", "all")]), ("select-auto", &[("user-select", "auto")]),
    ("appearance-none", &[("appearance", "none")]), ("appearance-auto", &[("appearance", "auto")]),
    ("scroll-auto", &[("scroll-behavior", "auto")]), ("scroll-smooth", &[("scroll-behavior", "smooth")]),
    ("snap-start", &[("scroll-snap-align", "start")]), ("snap-end", &[("scroll-snap-align", "end")]),
    ("snap-center", &[("scroll-snap-align", "center")]), ("snap-align-none", &[("scroll-snap-align", "none")]),
    ("snap-normal", &[("scroll-snap-stop", "normal")]), ("snap-always", &[("scroll-snap-stop", "always")]),
    ("snap-none", &[("scroll-snap-type", "none")]), ("snap-x", &[("scroll-snap-type", "x mandatory")]),
    ("snap-y", &[("scroll-snap-type", "y mandatory")]), ("snap-both", &[("scroll-snap-type", "both mandatory")]),
    ("transform-none", &[("transform", "none")]), ("transform-gpu", &[("will-change", "transform")]),
    ("transform-cpu", &[("will-change", "auto")]),
    ("transform-3d", &[("transform-style", "preserve-3d")]), ("transform-flat", &[("transform-style", "flat")]),
    ("backface-visible", &[("backface-visibility", "visible")]), ("backface-hidden", &[("backface-visibility", "hidden")]),
    ("transition", &[("transition-property", "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, opacity, box-shadow, transform, translate, scale, rotate, filter, backdrop-filter"),
                     ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)"), ("transition-duration", "150ms")]),
    ("transition-all", &[("transition-property", "all"), ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)"),
                         ("transition-duration", "150ms")]),
    ("transition-colors", &[("transition-property", "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke"),
                            ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)"), ("transition-duration", "150ms")]),
    ("transition-opacity", &[("transition-property", "opacity"), ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)"),
                             ("transition-duration", "150ms")]),
    ("transition-shadow", &[("transition-property", "box-shadow"), ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)"),
                            ("transition-duration", "150ms")]),
    ("transition-transform", &[("transition-property", "transform, translate, scale, rotate"),
                               ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)"), ("transition-duration", "150ms")]),
    ("transition-none", &[("transition-property", "none")]),
    ("ease-linear", &[("transition-timing-function", "linear")]),
    ("ease-in", &[("transition-timing-function", "cubic-bezier(0.4, 0, 1, 1)")]),
    ("ease-out", &[("transition-timing-function", "cubic-bezier(0, 0, 0.2, 1)")]),
    ("ease-in-out", &[("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)")]),
    ("ease-initial", &[("transition-timing-function", "initial")]),
    ("animate-none", &[("animation", "none")]),
    ("filter-none", &[("filter", "none")]), ("backdrop-filter-none", &[("backdrop-filter", "none")]),
    ("grayscale", &[("filter", "grayscale(100%)")]), ("invert", &[("filter", "invert(100%)")]),
    ("sepia", &[("filter", "sepia(100%)")]),
    ("backdrop-grayscale", &[("backdrop-filter", "grayscale(100%)")]), ("backdrop-invert", &[("backdrop-filter", "invert(100%)")]),
    ("backdrop-sepia", &[("backdrop-filter", "sepia(100%)")]),
    ("field-sizing-content", &[("field-sizing", "content")]), ("field-sizing-fixed", &[("field-sizing", "fixed")]),
    ("scheme-normal", &[("color-scheme", "normal")]), ("scheme-dark", &[("color-scheme", "dark")]),
    ("scheme-light", &[("color-scheme", "light")]), ("scheme-light-dark", &[("color-scheme", "light dark")]),
    ("scheme-only-dark", &[("color-scheme", "only dark")]), ("scheme-only-light", &[("color-scheme", "only light")]),
    ("forced-color-adjust-auto", &[("forced-color-adjust", "auto")]), ("forced-color-adjust-none", &[("forced-color-adjust", "none")]),
    ("object-contain", &[("object-fit", "contain")]), ("object-cover", &[("object-fit", "cover")]),
    ("object-fill", &[("object-fit", "fill")]), ("object-none", &[("object-fit", "none")]),
    ("object-scale-down", &[("object-fit", "scale-down")]),
    ("overflow-auto", &[("overflow", "auto")]), ("overflow-hidden", &[("overflow", "hidden")]),
    ("overflow-clip", &[("overflow", "clip")]), ("overflow-visible", &[("overflow", "visible")]),
    ("overflow-scroll", &[("overflow", "scroll")]),
    ("overflow-x-auto", &[("overflow-x", "auto")]), ("overflow-y-auto", &[("overflow-y", "auto")]),
    ("overflow-x-hidden", &[("overflow-x", "hidden")]), ("overflow-y-hidden", &[("overflow-y", "hidden")]),
    ("overflow-x-clip", &[("overflow-x", "clip")]), ("overflow-y-clip", &[("overflow-y", "clip")]),
    ("overflow-x-visible", &[("overflow-x", "visible")]), ("overflow-y-visible", &[("overflow-y", "visible")]),
    ("overflow-x-scroll", &[("overflow-x", "scroll")]), ("overflow-y-scroll", &[("overflow-y", "scroll")]),
    ("overscroll-auto", &[("overscroll-behavior", "auto")]), ("overscroll-contain", &[("overscroll-behavior", "contain")]),
    ("overscroll-none", &[("overscroll-behavior", "none")]),
    ("bg-blend-normal", &[("background-blend-mode", "normal")]),
    ("mix-blend-normal", &[("mix-blend-mode", "normal")]), ("mix-blend-plus-darker", &[("mix-blend-mode", "plus-darker")]),
    ("mix-blend-plus-lighter", &[("mix-blend-mode", "plus-lighter")]),
    ("content-none", &[("content", "none")]),
    ("break-before-auto", &[("break-before", "auto")]), ("break-after-auto", &[("break-after", "auto")]),
    ("break-inside-auto", &[("break-inside", "auto")]), ("break-inside-avoid", &[("break-inside", "avoid")]),
    ("box-decoration-clone", &[("box-decoration-break", "clone")]), ("box-decoration-slice", &[("box-decoration-break", "slice")]),
    ("touch-auto", &[("touch-action", "auto")]), ("touch-none", &[("touch-action", "none")]),
    ("touch-manipulation", &[("touch-action", "manipulation")]),
    ("will-change-auto", &[("will-change", "auto")]), ("will-change-scroll", &[("will-change", "scroll-position")]),
    ("will-change-contents", &[("will-change", "contents")]), ("will-change-transform", &[("will-change", "transform")]),
    ("rounded-none", &[("border-radius", "0")]), ("rounded-full", &[("border-radius", "calc(infinity * 1px)")]),
    ("shadow-none", &[("box-shadow", "0 0 #0000")]), ("inset-shadow-none", &[("box-shadow", "0 0 #0000")]),
    ("text-shadow-none", &[("text-shadow", "none")]), ("drop-shadow-none", &[("filter", "drop-shadow(0 0 #0000)")]),
];

const BLEND: &[&str] = &["multiply", "screen", "overlay", "darken", "lighten", "color-dodge", "color-burn",
                         "hard-light", "soft-light", "difference", "exclusion", "hue", "saturation", "color", "luminosity"];

const CURSORS: &[&str] = &["auto", "default", "pointer", "wait", "text", "move", "help", "not-allowed", "none",
    "context-menu", "progress", "cell", "crosshair", "vertical-text", "alias", "copy", "no-drop", "grab", "grabbing",
    "all-scroll", "col-resize", "row-resize", "n-resize", "e-resize", "s-resize", "w-resize", "ne-resize", "nw-resize",
    "se-resize", "sw-resize", "ew-resize", "ns-resize", "nesw-resize", "nwse-resize", "zoom-in", "zoom-out"];

/// Sizes for width / height: spacing, fractions, keywords, containers.
fn size(v: &str, axis: char) -> Option<String> {
    Some(match v {
        "auto" => "auto".into(),
        "full" => "100%".into(),
        "screen" => if axis == 'w' { "100vw".into() } else { "100vh".into() },
        "dvw" | "svw" | "lvw" | "dvh" | "svh" | "lvh" => format!("100{v}"),
        "min" => "min-content".into(),
        "max" => "max-content".into(),
        "fit" => "fit-content".into(),
        "lh" => "1lh".into(),
        "none" => "none".into(),
        "prose" => "65ch".into(),
        _ if axis == 'w' && theme(&format!("container-{v}")).is_some() => theme(&format!("container-{v}"))?.into(),
        _ if v.starts_with("screen-") => theme(&format!("breakpoint-{}", &v[7..]))?.into(),
        _ => fraction(v).or_else(|| spacing(v))?,
    })
}

fn sides(prefix: &str) -> Option<&'static [&'static str]> {
    Some(match prefix {
        "" => &[""],
        "x" => &["-inline"],
        "y" => &["-block"],
        "s" => &["-inline-start"],
        "e" => &["-inline-end"],
        "t" => &["-top"],
        "r" => &["-right"],
        "b" => &["-bottom"],
        "l" => &["-left"],
        "bs" => &["-block-start"],
        "be" => &["-block-end"],
        _ => return None,
    })
}

/// The declarations for one utility (no variants).
fn utility_decls(u: &str) -> Option<Out> {
    // arbitrary property: [mask-type:luminance]
    if u.starts_with('[') && u.ends_with(']') {
        let a = arbitrary(u)?;
        let (p, v) = a.split_once(':')?;
        return one(p.trim(), v.trim().to_string());
    }
    if let Some((_, d)) = STATIC.iter().find(|(n, _)| *n == u) {
        return decls(d);
    }
    let (neg, u) = match u.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, u),
    };
    let negate = |v: String| if neg { format!("calc({v} * -1)") } else { v };
    // longest matching root first
    let (root, val) = split_root(u)?;
    let v = val;
    let out = match root {
        // ---- spacing
        "p" | "px" | "py" | "ps" | "pe" | "pt" | "pr" | "pb" | "pl" | "pbs" | "pbe" if !neg => {
            let s = sides(&root[1..])?;
            let val = spacing(v)?;
            let props: Vec<String> = s.iter().map(|x| format!("padding{x}")).collect();
            Out { decls: props.into_iter().map(|p| (p, val.clone())).collect(), child: None, keyframes: None }
        }
        "m" | "mx" | "my" | "ms" | "me" | "mt" | "mr" | "mb" | "ml" | "mbs" | "mbe" => {
            let s = sides(&root[1..])?;
            let val = if v == "auto" { "auto".into() } else { negate(spacing(v)?) };
            Out { decls: s.iter().map(|x| (format!("margin{x}"), val.clone())).collect(), child: None, keyframes: None }
        }
        "space-x" | "space-y" => {
            let val = negate(spacing(v)?);
            let p = if root == "space-x" { "margin-inline-end" } else { "margin-block-end" };
            Out { decls: vec![(p.into(), val)], child: Some("& > :not(:last-child)".into()), keyframes: None }
        }
        "gap" | "gap-x" | "gap-y" if !neg => {
            let p = match root { "gap" => "gap", "gap-x" => "column-gap", _ => "row-gap" };
            one(p, spacing(v)?)?
        }
        // ---- sizing
        "w" | "min-w" | "max-w" if !neg => {
            let p = match root { "w" => "width", "min-w" => "min-width", _ => "max-width" };
            one(p, size(v, 'w')?)?
        }
        "h" | "min-h" | "max-h" if !neg => {
            let p = match root { "h" => "height", "min-h" => "min-height", _ => "max-height" };
            one(p, size(v, 'h')?)?
        }
        "size" if !neg => many(&["width", "height"], size(v, 'w')?)?,
        "inline" | "min-inline" | "max-inline" if !neg => {
            let p = match root { "inline" => "inline-size", "min-inline" => "min-inline-size", _ => "max-inline-size" };
            one(p, size(v, 'w')?)?
        }
        "block" | "min-block" | "max-block" if !neg => {
            let p = match root { "block" => "block-size", "min-block" => "min-block-size", _ => "max-block-size" };
            one(p, size(v, 'h')?)?
        }
        // ---- position
        "inset" | "inset-x" | "inset-y" | "start" | "end" | "top" | "right" | "bottom" | "left" => {
            let val = match v {
                "auto" => "auto".into(),
                "full" => negate("100%".into()),
                _ => negate(fraction(v).or_else(|| spacing(v))?),
            };
            let props: &[&str] = match root {
                "inset" => &["inset"],
                "inset-x" => &["inset-inline"],
                "inset-y" => &["inset-block"],
                "start" => &["inset-inline-start"],
                "end" => &["inset-inline-end"],
                r => std::slice::from_ref(match r { "top" => &"top", "right" => &"right", "bottom" => &"bottom", _ => &"left" }),
            };
            many(props, val)?
        }
        "z" => one("z-index", if v == "auto" { "auto".into() } else { negate(int_or_free(v)?) })?,
        "order" => one("order", match v {
            "first" => "-9999".into(),
            "last" => "9999".into(),
            "none" => "0".into(),
            _ => negate(int_or_free(v)?),
        })?,
        // ---- flex & grid
        "basis" if !neg => one("flex-basis", match v {
            "auto" => "auto".into(),
            "full" => "100%".into(),
            _ => match theme(&format!("container-{v}")) {
                Some(c) => c.into(),
                None => fraction(v).or_else(|| spacing(v))?,
            },
        })?,
        "flex" if !neg => one("flex", match v {
            _ if v.contains('/') && !v.starts_with('[') => format!("{} 1 0%", fraction(v)?),
            _ => num_or_free(v)?,
        })?,
        "grow" | "shrink" if !neg => one(if root == "grow" { "flex-grow" } else { "flex-shrink" }, num_or_free(v)?)?,
        "grid-cols" | "grid-rows" if !neg => {
            let p = if root == "grid-cols" { "grid-template-columns" } else { "grid-template-rows" };
            one(p, match v.parse::<u32>() {
                Ok(n) => format!("repeat({n}, minmax(0, 1fr))"),
                Err(_) => free(v)?.1,
            })?
        }
        "col-span" | "row-span" if !neg => {
            let p = if root == "col-span" { "grid-column" } else { "grid-row" };
            one(p, format!("span {} / span {}", int_or_free(v)?, int_or_free(v)?))?
        }
        "col-start" | "col-end" | "row-start" | "row-end" => {
            let p = match root {
                "col-start" => "grid-column-start",
                "col-end" => "grid-column-end",
                "row-start" => "grid-row-start",
                _ => "grid-row-end",
            };
            one(p, if v == "auto" { "auto".into() } else { negate(int_or_free(v)?) })?
        }
        "col" | "row" if !neg => one(if root == "col" { "grid-column" } else { "grid-row" }, int_or_free(v)?)?,
        "auto-cols" | "auto-rows" if !neg => one(if root == "auto-cols" { "grid-auto-columns" } else { "grid-auto-rows" }, free(v)?.1)?,
        "justify" | "justify-items" | "justify-self" | "content" | "items" | "self" | "place-content" | "place-items"
        | "place-self" if !neg && align_kw(root, v).is_some() => {
            let p = match root {
                "justify" => "justify-content",
                "justify-items" => "justify-items",
                "justify-self" => "justify-self",
                "content" => "align-content",
                "items" => "align-items",
                "self" => "align-self",
                r => r,
            };
            one(p, align_kw(root, v)?)?
        }
        // ---- typography
        "font" if !neg => match v {
            "sans" | "serif" | "mono" => one("font-family", theme(&format!("font-{v}"))?.into())?,
            _ => match theme(&format!("font-weight-{v}")) {
                Some(w) => one("font-weight", w.into())?,
                None => {
                    let (hint, a) = free(v)?;
                    if hint.as_deref() == Some("family-name") || a.parse::<f32>().is_err() && !a.starts_with("var(") {
                        one("font-family", a)?
                    } else {
                        one("font-weight", a)?
                    }
                }
            },
        },
        "text" if !neg => {
            let (base, lh) = split_modifier(v);
            if let Some(sz) = theme(&format!("text-{base}")) {
                let line = match lh {
                    Some(l) => leading(l)?,
                    None => theme(&format!("text-{base}--line-height")).unwrap_or("1.5").to_string(),
                };
                decls(&[("font-size", sz), ("line-height", &line)])?
            } else if matches!(v, "left" | "center" | "right" | "justify" | "start" | "end") {
                one("text-align", v.into())?
            } else if let Some(c) = color(v) {
                one("color", c)?
            } else {
                let (hint, a) = free(base)?;
                match hint.as_deref() {
                    Some("length") | Some("absolute-size") | Some("relative-size") => one("font-size", a)?,
                    _ if a.ends_with("px") || a.ends_with("rem") || a.ends_with("em") || a.contains("calc") => one("font-size", a)?,
                    _ => one("color", a)?,
                }
            }
        }
        "tracking" => one("letter-spacing", match theme(&format!("tracking-{v}")) {
            Some(t) => negate(t.into()),
            None => negate(free(v)?.1),
        })?,
        "leading" if !neg => one("line-height", leading(v)?)?,
        "line-clamp" if !neg => match v {
            "none" => decls(&[("overflow", "visible"), ("display", "block"), ("-webkit-box-orient", "horizontal"),
                              ("-webkit-line-clamp", "unset")])?,
            _ => {
                let n = int_or_free(v)?;
                decls(&[("overflow", "hidden"), ("display", "-webkit-box"), ("-webkit-box-orient", "vertical"),
                        ("-webkit-line-clamp", &n), ("line-clamp", &n)])?
            }
        },
        "indent" => one("text-indent", negate(spacing(v)?))?,
        "decoration" if !neg => match color(v) {
            Some(c) => one("text-decoration-color", c)?,
            None => one("text-decoration-thickness", px_or_free(v)?)?,
        },
        "underline-offset" => one("text-underline-offset", if v == "auto" { "auto".into() } else { negate(px_or_free(v)?) })?,
        "list" if !neg => one("list-style-type", free(v)?.1)?,
        "list-image" if !neg => one("list-style-image", if v == "none" { "none".into() } else { free(v)?.1 })?,
        "content" if !neg => one("content", free(v)?.1)?,
        "align" if !neg => one("vertical-align", free(v)?.1)?,
        // ---- backgrounds
        "bg" if !neg => {
            if let Some(c) = color(v) {
                one("background-color", c)?
            } else if let Some(dir) = v.strip_prefix("linear-to-").or_else(|| v.strip_prefix("gradient-to-")) {
                let to = dir_words(dir)?;
                one("background-image", format!("linear-gradient(to {to}, var(--tw-gradient-stops))"))?
            } else if let Some(rest) = v.strip_prefix("linear-") {
                let a = if let Ok(n) = rest.parse::<f32>() { format!("{n}deg") } else { free(rest)?.1 };
                one("background-image", format!("linear-gradient({a}, var(--tw-gradient-stops))"))?
            } else if v == "radial" {
                one("background-image", "radial-gradient(var(--tw-gradient-stops))".into())?
            } else if let Some(rest) = v.strip_prefix("radial-") {
                one("background-image", format!("radial-gradient({}, var(--tw-gradient-stops))", free(rest)?.1))?
            } else if v == "conic" {
                one("background-image", "conic-gradient(var(--tw-gradient-stops))".into())?
            } else if let Some(rest) = v.strip_prefix("conic-") {
                let a = if let Ok(n) = rest.parse::<f32>() { format!("from {n}deg") } else { free(rest)?.1 };
                one("background-image", format!("conic-gradient({a}, var(--tw-gradient-stops))"))?
            } else if let Some(p) = position_kw(v) {
                one("background-position", p)?
            } else {
                let (hint, a) = free(v)?;
                match hint.as_deref() {
                    Some("length") | Some("size") | Some("bg-size") => one("background-size", a)?,
                    Some("position") => one("background-position", a)?,
                    _ if a.starts_with("url(") || a.contains("gradient(") || hint.as_deref() == Some("image") || hint.as_deref() == Some("url") => {
                        one("background-image", a)?
                    }
                    _ => one("background-color", a)?,
                }
            }
        }
        "from" | "via" | "to" if !neg => {
            if let Some(c) = color(v) {
                let stops = match root {
                    "from" => format!("{c}, var(--tw-gradient-via, {c}), var(--tw-gradient-to, transparent)"),
                    "via" => format!("var(--tw-gradient-from, transparent), {c}, var(--tw-gradient-to, transparent)"),
                    _ => format!("var(--tw-gradient-from, transparent), var(--tw-gradient-via, {c}), {c}"),
                };
                Out { decls: vec![(format!("--tw-gradient-{root}"), c), ("--tw-gradient-stops".into(), stops)],
                      child: None, keyframes: None }
            } else {
                one(&format!("--tw-gradient-{root}-position"), pct_or_free(v)?)?
            }
        }
        "bg-blend" if !neg && BLEND.contains(&v) => one("background-blend-mode", v.into())?,
        "mix-blend" if !neg && BLEND.contains(&v) => one("mix-blend-mode", v.into())?,
        // ---- borders
        "border" | "border-x" | "border-y" | "border-s" | "border-e" | "border-t" | "border-r" | "border-b"
        | "border-l" | "border-bs" | "border-be" if !neg => {
            let s = sides(root.strip_prefix("border-").unwrap_or(""))?;
            match border_width(v) {
                Some(w) => Out { decls: s.iter().map(|x| (format!("border{x}-width"), w.clone())).collect(),
                                 child: None, keyframes: None },
                None => {
                    let c = color(v)?;
                    Out { decls: s.iter().map(|x| (format!("border{x}-color"), c.clone())).collect(), child: None, keyframes: None }
                }
            }
        }
        "rounded" | "rounded-s" | "rounded-e" | "rounded-t" | "rounded-r" | "rounded-b" | "rounded-l" | "rounded-ss"
        | "rounded-se" | "rounded-ee" | "rounded-es" | "rounded-tl" | "rounded-tr" | "rounded-br" | "rounded-bl" if !neg => {
            let r = radius(v)?;
            let props: &[&str] = match root {
                "rounded" => &["border-radius"],
                "rounded-s" => &["border-start-start-radius", "border-end-start-radius"],
                "rounded-e" => &["border-start-end-radius", "border-end-end-radius"],
                "rounded-t" => &["border-top-left-radius", "border-top-right-radius"],
                "rounded-r" => &["border-top-right-radius", "border-bottom-right-radius"],
                "rounded-b" => &["border-bottom-right-radius", "border-bottom-left-radius"],
                "rounded-l" => &["border-top-left-radius", "border-bottom-left-radius"],
                "rounded-ss" => &["border-start-start-radius"],
                "rounded-se" => &["border-start-end-radius"],
                "rounded-ee" => &["border-end-end-radius"],
                "rounded-es" => &["border-end-start-radius"],
                "rounded-tl" => &["border-top-left-radius"],
                "rounded-tr" => &["border-top-right-radius"],
                "rounded-br" => &["border-bottom-right-radius"],
                _ => &["border-bottom-left-radius"],
            };
            many(props, r)?
        }
        "divide-x" | "divide-y" if !neg => {
            let w = border_width(v)?;
            let p = if root == "divide-x" { "border-inline-start-width" } else { "border-top-width" };
            Out { decls: vec![(p.into(), w)], child: Some("& > :not(:last-child) ~ *".into()), keyframes: None }
        }
        "divide" if !neg => Out { decls: vec![("border-color".into(), color(v)?)],
                                  child: Some("& > :not(:last-child)".into()), keyframes: None },
        "outline" if !neg => match border_width(v) {
            Some(w) => decls(&[("outline-style", "solid"), ("outline-width", &w)])?,
            None => one("outline-color", color(v)?)?,
        },
        "outline-offset" => one("outline-offset", negate(px_or_free(v)?))?,
        "ring" | "inset-ring" if !neg => {
            let inset = if root == "inset-ring" { "inset " } else { "" };
            match border_width(v) {
                Some(w) => one("box-shadow", format!("{inset}0 0 0 {w} var(--tw-ring-color, currentColor)"))?,
                None => one("--tw-ring-color", color(v)?)?,
            }
        }
        "ring-offset" if !neg => match border_width(v) {
            Some(w) => one("--tw-ring-offset-width", w)?,
            None => one("--tw-ring-offset-color", color(v)?)?,
        },
        // ---- effects
        "shadow" | "inset-shadow" | "drop-shadow" | "text-shadow" if !neg => {
            let (base, alpha) = split_modifier(v);
            let key = format!("{root}-{base}");
            let theme_shadow = theme(&key).or_else(|| (base == "DEFAULT").then(|| match root {
                "shadow" => "0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)",
                "drop-shadow" => "0 1px 2px rgb(0 0 0 / 0.1), 0 1px 1px rgb(0 0 0 / 0.06)",
                "inset-shadow" => "inset 0 2px 4px rgb(0 0 0 / 0.05)",
                _ => "0px 1px 1px rgb(0 0 0 / 0.2)",
            }));
            if let Some(s) = theme_shadow {
                // `shadow-lg/50`: the shadow color's alpha becomes 50%
                let s = match alpha.and_then(opacity) {
                    Some(a) => with_shadow_alpha(s, &a),
                    None => s.to_string(),
                };
                match root {
                    "drop-shadow" => one("filter", s.split(", ").map(|d| format!("drop-shadow({d})")).collect::<Vec<_>>().join(" "))?,
                    "text-shadow" => one("text-shadow", s)?,
                    _ => one("box-shadow", s)?,
                }
            } else if let Some(c) = color(v) {
                one(&format!("--tw-{root}-color"), c)?
            } else {
                let a = free(v)?.1;
                match root {
                    "drop-shadow" => one("filter", format!("drop-shadow({a})"))?,
                    "text-shadow" => one("text-shadow", a)?,
                    "inset-shadow" => one("box-shadow", format!("inset {a}"))?,
                    _ => one("box-shadow", a)?,
                }
            }
        }
        "opacity" if !neg => one("opacity", opacity(v)?)?,
        "blur" | "backdrop-blur" if !neg => {
            let b = match v {
                "none" => "0".to_string(),
                "DEFAULT" => "8px".to_string(),
                _ => theme(&format!("blur-{v}")).map(String::from).or_else(|| free(v).map(|f| f.1))?,
            };
            one(if root == "blur" { "filter" } else { "backdrop-filter" }, format!("blur({b})"))?
        }
        "brightness" | "contrast" | "saturate" | "backdrop-brightness" | "backdrop-contrast" | "backdrop-saturate" if !neg => {
            let f = root.trim_start_matches("backdrop-");
            let p = if root.starts_with("backdrop") { "backdrop-filter" } else { "filter" };
            one(p, format!("{f}({})", pct_num(v)?))?
        }
        "grayscale" | "invert" | "sepia" | "backdrop-grayscale" | "backdrop-invert" | "backdrop-sepia" | "backdrop-opacity" if !neg => {
            let f = root.trim_start_matches("backdrop-");
            let p = if root.starts_with("backdrop") { "backdrop-filter" } else { "filter" };
            one(p, format!("{f}({})", pct_num(v)?))?
        }
        "hue-rotate" | "backdrop-hue-rotate" => {
            let p = if root.starts_with("backdrop") { "backdrop-filter" } else { "filter" };
            one(p, format!("hue-rotate({})", negate(deg(v)?)))?
        }
        // ---- transforms
        "scale" | "scale-x" | "scale-y" | "scale-z" => {
            let s = negate(pct_num(v)?);
            match root {
                "scale" => one("scale", s)?,
                "scale-x" => one("scale", format!("{s} 1"))?,
                "scale-y" => one("scale", format!("1 {s}"))?,
                _ => one("scale", format!("1 1 {s}"))?,
            }
        }
        "rotate" | "rotate-x" | "rotate-y" | "rotate-z" => {
            let d = negate(deg(v)?);
            match root {
                "rotate" => one("rotate", d)?,
                "rotate-x" => one("rotate", format!("x {d}"))?,
                "rotate-y" => one("rotate", format!("y {d}"))?,
                _ => one("rotate", format!("z {d}"))?,
            }
        }
        "translate" | "translate-x" | "translate-y" | "translate-z" => {
            let t = negate(match v { "full" => "100%".into(), "px" => "1px".into(), _ => fraction(v).or_else(|| spacing(v))? });
            match root {
                "translate" => one("translate", format!("{t} {t}"))?,
                "translate-x" => one("translate", format!("{t} 0"))?,
                "translate-y" => one("translate", format!("0 {t}"))?,
                _ => one("translate", format!("0 0 {t}"))?,
            }
        }
        "skew" | "skew-x" | "skew-y" => {
            let d = negate(deg(v)?);
            match root {
                "skew" => one("transform", format!("skewX({d}) skewY({d})"))?,
                "skew-x" => one("transform", format!("skewX({d})"))?,
                _ => one("transform", format!("skewY({d})"))?,
            }
        }
        "origin" | "perspective-origin" if !neg => one(if root == "origin" { "transform-origin" } else { "perspective-origin" },
                                                        position_kw(v).or_else(|| free(v).map(|f| f.1))?)?,
        "perspective" if !neg => one("perspective", match v {
            "none" => "none".into(),
            _ => theme(&format!("perspective-{v}")).map(String::from).or_else(|| free(v).map(|f| f.1))?,
        })?,
        "transform" if !neg => one("transform", free(v)?.1)?,
        // ---- transitions & animation
        "transition" if !neg => decls(&[("transition-property", &free(v)?.1),
                                         ("transition-timing-function", "cubic-bezier(0.4, 0, 0.2, 1)"),
                                         ("transition-duration", "150ms")])?,
        "duration" | "delay" if !neg => {
            let ms = match v.parse::<f32>() {
                Ok(n) => format!("{}ms", trim_num(n)),
                Err(_) if v == "initial" => "initial".into(),
                Err(_) => free(v)?.1,
            };
            one(if root == "duration" { "transition-duration" } else { "transition-delay" }, ms)?
        }
        "ease" if !neg => one("transition-timing-function", free(v)?.1)?,
        "animate" if !neg => {
            let (anim, kf): (String, &'static str) = match v {
                "spin" => (theme("animate-spin")?.into(), "@keyframes spin { to { transform: rotate(360deg) } }"),
                "ping" => (theme("animate-ping")?.into(),
                           "@keyframes ping { 75%, 100% { transform: scale(2); opacity: 0 } }"),
                "pulse" => (theme("animate-pulse")?.into(), "@keyframes pulse { 50% { opacity: 0.5 } }"),
                "bounce" => (theme("animate-bounce")?.into(),
                             "@keyframes bounce { 0%, 100% { transform: translateY(-25%); animation-timing-function: cubic-bezier(0.8, 0, 1, 1) } 50% { transform: none; animation-timing-function: cubic-bezier(0, 0, 0.2, 1) } }"),
                _ => (free(v)?.1, ""),
            };
            Out { decls: vec![("animation".into(), anim)], child: None, keyframes: (!kf.is_empty()).then_some(kf) }
        }
        // ---- interactivity, svg, misc
        "cursor" if !neg => one("cursor", if CURSORS.contains(&v) { v.into() } else { free(v)?.1 })?,
        "accent" if !neg => one("accent-color", if v == "auto" { "auto".into() } else { color(v)? })?,
        "caret" if !neg => one("caret-color", color(v)?)?,
        "fill" if !neg => one("fill", if v == "none" { "none".into() } else { color(v)? })?,
        "stroke" if !neg => match color(v) {
            Some(c) => one("stroke", c)?,
            None => one("stroke-width", num_or_free(v)?)?,
        },
        "aspect" if !neg => one("aspect-ratio", match v {
            "auto" => "auto".into(),
            "square" => "1 / 1".into(),
            "video" => theme("aspect-video")?.into(),
            _ if v.contains('/') && !v.starts_with('[') => v.replace('/', " / "),
            _ => free(v)?.1,
        })?,
        "columns" if !neg => one("columns", match v {
            "auto" => "auto".into(),
            _ if v.parse::<u32>().is_ok() => v.into(),
            _ => theme(&format!("container-{v}")).map(String::from).or_else(|| free(v).map(|f| f.1))?,
        })?,
        "object" if !neg => one("object-position", position_kw(v).or_else(|| free(v).map(|f| f.1))?)?,
        "scroll-m" | "scroll-mx" | "scroll-my" | "scroll-ms" | "scroll-me" | "scroll-mt" | "scroll-mr" | "scroll-mb"
        | "scroll-ml" => {
            let s = sides(&root[8..])?;
            let val = negate(spacing(v)?);
            Out { decls: s.iter().map(|x| (format!("scroll-margin{x}"), val.clone())).collect(), child: None, keyframes: None }
        }
        "scroll-p" | "scroll-px" | "scroll-py" | "scroll-ps" | "scroll-pe" | "scroll-pt" | "scroll-pr" | "scroll-pb"
        | "scroll-pl" if !neg => {
            let s = sides(&root[8..])?;
            let val = spacing(v)?;
            Out { decls: s.iter().map(|x| (format!("scroll-padding{x}"), val.clone())).collect(), child: None, keyframes: None }
        }
        "will-change" if !neg => one("will-change", free(v)?.1)?,
        "touch" if !neg => one("touch-action", v.replace("pan-", "pan-"))?,
        "border-spacing" | "border-spacing-x" | "border-spacing-y" if !neg => one("border-spacing", spacing(v)?)?,
        "break-before" | "break-after" | "break-inside" if !neg => one(root, v.into())?,
        "mask" if !neg => one("mask-image", free(v)?.1)?,
        "outline-style" => return None,
        _ => return None,
    };
    Some(out)
}

/// Split `p-4` into (`p`, `4`) using the longest known root.
fn split_root(u: &str) -> Option<(&str, &str)> {
    const ROOTS: &[&str] = &[
        "backdrop-hue-rotate", "backdrop-brightness", "backdrop-contrast", "backdrop-grayscale", "backdrop-saturate",
        "backdrop-opacity", "backdrop-invert", "backdrop-sepia", "backdrop-blur", "perspective-origin",
        "underline-offset", "outline-offset", "border-spacing-x", "border-spacing-y", "border-spacing", "inset-shadow",
        "drop-shadow", "text-shadow", "justify-items", "justify-self", "place-content", "place-items", "place-self",
        "ring-offset", "inset-ring", "will-change", "line-clamp", "list-image", "hue-rotate", "break-before",
        "break-after", "break-inside", "min-inline", "max-inline", "min-block", "max-block", "translate-x",
        "translate-y", "translate-z", "grid-cols", "grid-rows", "col-start", "row-start", "auto-cols", "auto-rows",
        "brightness", "grayscale", "translate", "perspective", "col-span", "row-span", "col-end", "row-end",
        "rounded-ss", "rounded-se", "rounded-ee", "rounded-es", "rounded-tl", "rounded-tr", "rounded-br",
        "rounded-bl", "rounded-s", "rounded-e", "rounded-t", "rounded-r", "rounded-b", "rounded-l", "rounded",
        "border-x", "border-y", "border-s", "border-e", "border-t", "border-r", "border-b", "border-l",
        "border-bs", "border-be", "divide-x", "divide-y", "scroll-mx", "scroll-my", "scroll-ms", "scroll-me",
        "scroll-mt", "scroll-mr", "scroll-mb", "scroll-ml", "scroll-px", "scroll-py", "scroll-ps", "scroll-pe",
        "scroll-pt", "scroll-pr", "scroll-pb", "scroll-pl", "scroll-m", "scroll-p", "rotate-x", "rotate-y",
        "rotate-z", "scale-x", "scale-y", "scale-z", "skew-x", "skew-y", "space-x", "space-y", "bg-blend",
        "mix-blend", "contrast", "saturate", "duration", "tracking", "leading", "opacity", "content", "columns",
        "inset-x", "inset-y", "transform", "transition", "animate", "decoration", "outline", "origin", "invert",
        "sepia", "divide", "border", "shadow", "rotate", "scale", "skew", "basis", "order", "accent", "caret",
        "cursor", "stroke", "aspect", "object", "indent", "gap-x", "gap-y", "min-w", "max-w", "min-h", "max-h",
        "inline", "block", "inset", "start", "items", "justify", "self", "align", "delay", "ease", "blur", "ring",
        "fill", "font", "text", "list", "mask", "touch", "size", "flex", "grow", "shrink", "from", "via", "top",
        "right", "bottom", "left", "end", "col", "row", "gap", "pbs", "pbe", "mbs", "mbe", "bg", "to", "z",
        "px", "py", "ps", "pe", "pt", "pr", "pb", "pl", "mx", "my", "ms", "me", "mt", "mr", "mb", "ml", "p", "m",
        "w", "h",
    ];
    for r in ROOTS {
        if let Some(rest) = u.strip_prefix(r) {
            if let Some(v) = rest.strip_prefix('-') {
                if !v.is_empty() {
                    return Some((r, v));
                }
            }
            if rest.is_empty() {
                // bare roots with a default value
                return match *r {
                    "border" | "border-x" | "border-y" | "border-s" | "border-e" | "border-t" | "border-r"
                    | "border-b" | "border-l" | "outline" | "ring" | "inset-ring" | "divide-x" | "divide-y" => Some((r, "1")),
                    "rounded" | "shadow" | "drop-shadow" | "blur" | "backdrop-blur" | "inset-shadow" => Some((r, "DEFAULT")),
                    "grow" | "shrink" => Some((r, "1")),
                    _ => None,
                };
            }
        }
    }
    None
}

/// Every `rgb(0 0 0 / x)` in a theme shadow with alpha `a`.
fn with_shadow_alpha(s: &str, a: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(k) = rest.find("rgb(0 0 0 / ") {
        out.push_str(&rest[..k + 12]);
        out.push_str(a);
        let after = &rest[k + 12..];
        let end = after.find(')').unwrap_or(after.len());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn int_or_free(v: &str) -> Option<String> {
    if v.parse::<i32>().is_ok() {
        return Some(v.into());
    }
    free(v).map(|f| f.1)
}

fn num_or_free(v: &str) -> Option<String> {
    if v.parse::<f32>().is_ok() {
        return Some(v.into());
    }
    free(v).map(|f| f.1)
}

fn px_or_free(v: &str) -> Option<String> {
    if let Ok(n) = v.parse::<f32>() {
        return Some(format!("{}px", trim_num(n)));
    }
    free(v).map(|f| f.1)
}

fn pct_or_free(v: &str) -> Option<String> {
    if let Some(p) = v.strip_suffix('%') {
        return p.parse::<f32>().ok().map(|n| format!("{}%", trim_num(n)));
    }
    free(v).map(|f| f.1)
}

/// `150` -> `150%` (filters, scale), arbitrary as is.
fn pct_num(v: &str) -> Option<String> {
    if v == "DEFAULT" {
        return Some("100%".into());
    }
    if let Ok(n) = v.parse::<f32>() {
        return Some(format!("{}%", trim_num(n)));
    }
    free(v).map(|f| f.1)
}

fn deg(v: &str) -> Option<String> {
    if let Ok(n) = v.parse::<f32>() {
        return Some(format!("{}deg", trim_num(n)));
    }
    free(v).map(|f| f.1)
}

fn leading(v: &str) -> Option<String> {
    if v == "none" {
        return Some("1".into());
    }
    if let Some(l) = theme(&format!("leading-{v}")) {
        return Some(l.into());
    }
    spacing(v)
}

fn radius(v: &str) -> Option<String> {
    match v {
        "DEFAULT" => Some("0.25rem".into()),
        "none" => Some("0".into()),
        "full" => Some("calc(infinity * 1px)".into()),
        _ => theme(&format!("radius-{v}")).map(String::from).or_else(|| free(v).map(|f| f.1)),
    }
}

fn border_width(v: &str) -> Option<String> {
    if let Ok(n) = v.parse::<f32>() {
        return Some(format!("{}px", trim_num(n)));
    }
    match free(v) {
        Some((Some(h), a)) if h == "length" || h == "line-width" => Some(a),
        Some((None, a)) if a.ends_with("px") || a.ends_with("rem") || a.ends_with("em") || a == "thin"
                           || a == "medium" || a == "thick" => Some(a),
        _ => None,
    }
}

fn dir_words(d: &str) -> Option<&'static str> {
    Some(match d {
        "t" => "top",
        "tr" => "top right",
        "r" => "right",
        "br" => "bottom right",
        "b" => "bottom",
        "bl" => "bottom left",
        "l" => "left",
        "tl" => "top left",
        _ => return None,
    })
}

fn position_kw(v: &str) -> Option<String> {
    Some(match v {
        "center" => "center",
        "top" => "top",
        "bottom" => "bottom",
        "left" => "left",
        "right" => "right",
        "top-left" | "left-top" => "left top",
        "top-right" | "right-top" => "right top",
        "bottom-left" | "left-bottom" => "left bottom",
        "bottom-right" | "right-bottom" => "right bottom",
        _ => return None,
    }.into())
}

fn align_kw(root: &str, v: &str) -> Option<String> {
    let base = v.trim_end_matches("-safe");
    let kw = match base {
        "start" => "flex-start",
        "end" => "flex-end",
        "center" => "center",
        "between" => "space-between",
        "around" => "space-around",
        "evenly" => "space-evenly",
        "stretch" => "stretch",
        "baseline" => "baseline",
        "baseline-last" => "last baseline",
        "normal" => "normal",
        "auto" => "auto",
        _ => return None,
    };
    // content alignment and flex item alignment use flex-start / flex-end;
    // justify-items, justify-self and place-items / place-self use start / end
    let kw = match (root, base) {
        ("justify-items" | "justify-self" | "place-items" | "place-self", "start") => "start",
        ("justify-items" | "justify-self" | "place-items" | "place-self", "end") => "end",
        _ => kw,
    };
    Some(if v.ends_with("-safe") { format!("safe {kw}") } else { kw.to_string() })
}

/// One variant: (new selector, wrapper at-rule, sort weight).
fn variant(v: &str, sel: &str) -> Option<(Option<String>, Option<String>, u32)> {
    let s = |suffix: &str| Some((Some(format!("{sel}{suffix}")), None, 10));
    let pseudo = |p: &str| -> Option<&'static str> {
        Some(match p {
            "hover" => ":hover",
            "focus" => ":focus",
            "focus-visible" => ":focus-visible",
            "focus-within" => ":focus-within",
            "active" => ":active",
            "visited" => ":visited",
            "target" => ":target",
            "first" => ":first-child",
            "last" => ":last-child",
            "only" => ":only-child",
            "odd" => ":nth-child(odd)",
            "even" => ":nth-child(even)",
            "first-of-type" => ":first-of-type",
            "last-of-type" => ":last-of-type",
            "only-of-type" => ":only-of-type",
            "empty" => ":empty",
            "disabled" => ":disabled",
            "enabled" => ":enabled",
            "checked" => ":checked",
            "indeterminate" => ":indeterminate",
            "default" => ":default",
            "optional" => ":optional",
            "required" => ":required",
            "valid" => ":valid",
            "invalid" => ":invalid",
            "user-valid" => ":user-valid",
            "user-invalid" => ":user-invalid",
            "in-range" => ":in-range",
            "out-of-range" => ":out-of-range",
            "placeholder-shown" => ":placeholder-shown",
            "autofill" => ":autofill",
            "read-only" => ":read-only",
            "read-write" => ":read-write",
            "open" => ":is([open], :popover-open, :open)",
            "inert" => ":is([inert], [inert] *)",
            "details-content" => "::details-content",
            _ => return None,
        })
    };
    if let Some(p) = pseudo(v) {
        return s(p);
    }
    let pe = |p: &str| -> Option<&'static str> {
        Some(match p {
            "before" => "::before",
            "after" => "::after",
            "placeholder" => "::placeholder",
            "file" => "::file-selector-button",
            "marker" => "::marker",
            "selection" => "::selection",
            "first-line" => "::first-line",
            "first-letter" => "::first-letter",
            "backdrop" => "::backdrop",
            _ => return None,
        })
    };
    if let Some(p) = pe(v) {
        return Some((Some(format!("{sel}{p}")), None, 60));
    }
    let bp = |name: &str| theme(&format!("breakpoint-{name}"));
    let media = |q: String, w: u32| Some((None, Some(format!("@media {q}")), w));
    match v {
        "*" => return Some((Some(format!(":is({sel} > *)")), None, 1)),
        "**" => return Some((Some(format!(":is({sel} *)")), None, 1)),
        "dark" => return media("(prefers-color-scheme: dark)".into(), 500),
        "motion-safe" => return media("(prefers-reduced-motion: no-preference)".into(), 300),
        "motion-reduce" => return media("(prefers-reduced-motion: reduce)".into(), 300),
        "contrast-more" => return media("(prefers-contrast: more)".into(), 300),
        "contrast-less" => return media("(prefers-contrast: less)".into(), 300),
        "print" => return media("print".into(), 510),
        "portrait" => return media("(orientation: portrait)".into(), 300),
        "landscape" => return media("(orientation: landscape)".into(), 300),
        "forced-colors" => return media("(forced-colors: active)".into(), 300),
        "inverted-colors" => return media("(inverted-colors: inverted)".into(), 300),
        "pointer-fine" => return media("(pointer: fine)".into(), 300),
        "pointer-coarse" => return media("(pointer: coarse)".into(), 300),
        "pointer-none" => return media("(pointer: none)".into(), 300),
        "any-pointer-fine" => return media("(any-pointer: fine)".into(), 300),
        "any-pointer-coarse" => return media("(any-pointer: coarse)".into(), 300),
        "noscript" => return media("(scripting: none)".into(), 300),
        "rtl" => return Some((Some(format!("{sel}:where(:dir(rtl), [dir=rtl], [dir=rtl] *)")), None, 20)),
        "ltr" => return Some((Some(format!("{sel}:where(:dir(ltr), [dir=ltr], [dir=ltr] *)")), None, 20)),
        "starting" => return Some((None, Some("@starting-style".into()), 600)),
        _ => {}
    }
    // breakpoints: sm md lg xl 2xl (ascending weight so larger wins), max-*, min-[..]
    for (k, name) in ["sm", "md", "lg", "xl", "2xl"].iter().enumerate() {
        if v == *name {
            return media(format!("(width >= {})", bp(name)?), 400 + k as u32 * 2);
        }
        if v.strip_prefix("max-") == Some(name) {
            return media(format!("(width < {})", bp(name)?), 399 - k as u32 * 2);
        }
    }
    if let Some(a) = v.strip_prefix("min-").and_then(arbitrary) {
        return media(format!("(width >= {a})"), 420);
    }
    if let Some(a) = v.strip_prefix("max-").and_then(arbitrary) {
        return media(format!("(width < {a})"), 380);
    }
    // container queries: @sm, @max-md, @min-[..], @[..], @md/name
    if let Some(c) = v.strip_prefix('@') {
        let (c, name) = match c.split_once('/') {
            Some((c, n)) => (c, format!("{n} ")),
            None => (c, String::new()),
        };
        let (op, size_name) = match c.strip_prefix("max-") {
            Some(r) => ("<", r),
            None => (">=", c.strip_prefix("min-").unwrap_or(c)),
        };
        let size = match arbitrary(size_name) {
            Some(a) => a,
            None => theme(&format!("container-{size_name}"))?.to_string(),
        };
        return Some((None, Some(format!("@container {name}(width {op} {size})")), 450));
    }
    // supports-[..], supports-grid
    if let Some(rest) = v.strip_prefix("supports-") {
        let cond = match arbitrary(rest) {
            Some(a) if a.contains(':') || a.starts_with('(') => if a.starts_with('(') { a } else { format!("({a})") },
            Some(a) => format!("({a}: var(--tw))"),
            None => format!("(display: {rest})"),
        };
        return Some((None, Some(format!("@supports {cond}")), 350));
    }
    // arbitrary variant [&>p] / [@media(...)]
    if v.starts_with('[') {
        let a = arbitrary(v)?;
        if a.starts_with('@') {
            return Some((None, Some(a), 700));
        }
        return Some((Some(if a.contains('&') { a.replace('&', sel) } else { format!("{sel}{a}") }), None, 30));
    }
    // data-*, aria-*
    if let Some(d) = v.strip_prefix("data-") {
        let attr = match arbitrary(d) {
            Some(a) => format!("[data-{a}]"),
            None => format!("[data-{d}]"),
        };
        return s(&attr);
    }
    if let Some(d) = v.strip_prefix("aria-") {
        let attr = match arbitrary(d) {
            Some(a) => format!("[aria-{a}]"),
            None => format!("[aria-{d}=\"true\"]"),
        };
        return s(&attr);
    }
    // nth-*
    for (name, pc) in [("nth-last-of-type-", "nth-last-of-type"), ("nth-of-type-", "nth-of-type"),
                       ("nth-last-", "nth-last-child"), ("nth-", "nth-child")] {
        if let Some(n) = v.strip_prefix(name) {
            let n = arbitrary(n).unwrap_or_else(|| n.to_string());
            return s(&format!(":{pc}({n})"));
        }
    }
    // group-*, peer-* (optionally named: group-hover/item), has-*, not-*, in-*
    let inner_sel = |x: &str| -> Option<String> {
        if let Some(p) = pseudo(x) {
            return Some(p.to_string());
        }
        if let Some(a) = arbitrary(x) {
            return Some(if a.contains('&') { a.replace('&', "") } else { a });
        }
        if let Some(d) = x.strip_prefix("data-") {
            return Some(arbitrary(d).map(|a| format!("[data-{a}]")).unwrap_or_else(|| format!("[data-{d}]")));
        }
        if let Some(d) = x.strip_prefix("aria-") {
            return Some(arbitrary(d).map(|a| format!("[aria-{a}]")).unwrap_or_else(|| format!("[aria-{d}=\"true\"]")));
        }
        if let Some(h) = x.strip_prefix("has-") {
            let hs = pseudo(h).map(|p| format!("*{p}")).or_else(|| arbitrary(h))?;
            return Some(format!(":has({hs})"));
        }
        None
    };
    for (prefix, comb) in [("group-", " "), ("peer-", " ~ ")] {
        if let Some(rest) = v.strip_prefix(prefix) {
            let (state, name) = match rest.rsplit_once('/') {
                Some((s, n)) if !s.contains('[') || s.ends_with(']') => (s, format!("\\/{n}")),
                _ => (rest, String::new()),
            };
            let which = &prefix[..prefix.len() - 1];
            let st = inner_sel(state)?;
            return Some((Some(format!(":where(.{which}{name}){st}{comb}{sel}")), None, 40));
        }
    }
    if let Some(rest) = v.strip_prefix("has-") {
        let x = pseudo(rest).map(|p| format!("*{p}")).or_else(|| arbitrary(rest))?;
        return s(&format!(":has({x})"));
    }
    if let Some(rest) = v.strip_prefix("not-") {
        let x = inner_sel(rest)?;
        return s(&format!(":not({x})"));
    }
    if let Some(rest) = v.strip_prefix("in-") {
        let x = inner_sel(rest)?;
        return Some((Some(format!(":where(*{x}) {sel}")), None, 40));
    }
    None
}

/// Rank of a property in Tailwind's utility order: earlier properties
/// come first; shorthands before their longhands.
fn prop_rank(p: &str) -> u32 {
    const ORDER: &[&str] = &[
        "container-type", "pointer-events", "visibility", "position", "inset", "top", "right", "bottom", "left",
        "isolation", "z-index", "order", "grid-column", "grid-row", "float", "clear", "box-sizing", "margin",
        "display", "aspect-ratio", "width", "min-width", "max-width", "height", "min-height", "max-height",
        "flex", "flex-shrink", "flex-grow", "flex-basis", "table-layout", "caption-side", "border-collapse",
        "border-spacing", "transform-origin", "translate", "scale", "rotate", "transform", "animation", "cursor",
        "touch-action", "resize", "scroll-snap-type", "scroll-snap-align", "scroll-margin", "scroll-padding",
        "list-style-position", "list-style-type", "appearance", "columns", "grid-auto-columns", "grid-auto-flow",
        "grid-auto-rows", "grid-template-columns", "grid-template-rows", "flex-direction", "flex-wrap",
        "place-content", "place-items", "align-content", "align-items", "justify-content", "justify-items", "gap",
        "column-gap", "row-gap", "place-self", "align-self", "justify-self", "overflow", "overscroll-behavior",
        "scroll-behavior", "text-overflow", "white-space", "word-break", "border-radius", "border-width",
        "border-style", "border-color", "background-color", "background-image", "--tw-gradient-from",
        "--tw-gradient-via", "--tw-gradient-to", "background-size", "background-attachment", "background-clip",
        "background-position", "background-repeat", "background-origin", "fill", "stroke", "object-fit",
        "object-position", "padding", "text-align", "text-indent", "vertical-align", "font-family", "font-size",
        "line-height", "font-weight", "letter-spacing", "text-wrap", "overflow-wrap", "hyphens", "color",
        "text-transform", "font-style", "text-decoration-line", "text-decoration-color", "text-decoration-style",
        "text-decoration-thickness", "text-underline-offset", "opacity", "mix-blend-mode", "box-shadow",
        "outline", "filter", "backdrop-filter", "transition-property", "transition-delay", "transition-duration",
        "transition-timing-function", "will-change", "content",
    ];
    let base = p.split('-').take(2).collect::<Vec<_>>().join("-");
    let idx = ORDER.iter().position(|o| *o == p)
        .or_else(|| ORDER.iter().position(|o| *o == base))
        .or_else(|| ORDER.iter().position(|o| p.starts_with(o)))
        .unwrap_or(ORDER.len());
    idx as u32 * 8 + p.matches('-').count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(class: &str) -> String {
        compile(class).unwrap_or_else(|| panic!("{class}")).css
    }

    #[test]
    fn utilities() {
        assert!(body("p-4").contains("padding: calc(0.25rem * 4)"));
        assert!(body("px-2.5").contains("padding-inline: calc(0.25rem * 2.5)"));
        assert!(body("-mt-8").contains("margin-top: calc(calc(0.25rem * 8) * -1)"));
        assert!(body("w-1/2").contains("width: 50%"));
        assert!(body("w-[calc(100%-2rem)]").contains("width: calc(100%-2rem)"));
        assert!(body("bg-sky-500/50").contains("color-mix(in oklab, oklch(68.5% 0.169 237.323) 50%, transparent)"));
        assert!(body("bg-(--brand)").contains("background-color: var(--brand)"));
        assert!(body("text-lg").contains("font-size: 1.125rem"));
        assert!(body("text-sm/6").contains("line-height: calc(0.25rem * 6)"));
        assert!(body("text-red-500").contains("color: oklch(63.7% 0.237 25.331)"));
        assert!(body("rounded-xl").contains("border-radius: 0.75rem"));
        assert!(body("grid-cols-3").contains("repeat(3, minmax(0, 1fr))"));
        assert!(body("shadow-lg").contains("box-shadow: 0 10px 15px -3px"));
        assert!(body("[mask-type:luminance]").contains("mask-type: luminance"));
        assert!(body("bg-linear-to-r").contains("linear-gradient(to right"));
        assert!(body("animate-spin").contains("@keyframes spin"));
        assert!(body("font-bold").contains("font-weight: 700"));
        assert!(body("border").contains("border-width: 1px"));
        assert!(body("p-4!").contains("!important"));
        assert!(compile("not-a-utility").is_none());
        assert!(compile("p-banana").is_none());
    }

    #[test]
    fn variants() {
        assert!(body("hover:bg-red-500").contains(".hover\\:bg-red-500:hover {"));
        assert!(body("md:flex").contains("@media (width >= 48rem)"));
        assert!(body("dark:md:hover:underline").contains("@media (prefers-color-scheme: dark)"));
        assert!(body("group-hover:text-white").contains(":where(.group):hover .group-hover\\:text-white"));
        assert!(body("group-hover/item:p-1").contains(":where(.group\\/item):hover"));
        assert!(body("peer-checked:block").contains(":where(.peer):checked ~ "));
        assert!(body("has-checked:ring-2").contains(":has(*:checked)"));
        assert!(body("data-[state=open]:p-2").contains("[data-state=open]"));
        assert!(body("aria-expanded:p-2").contains("[aria-expanded=\"true\"]"));
        assert!(body("*:p-2").contains(":is(.\\*\\:p-2 > *)"));
        assert!(body("[&>p]:mt-2").contains(r".\[\&\>p\]\:mt-2>p"));
        assert!(body("@md:grid").contains("@container (width >= 28rem)"));
        assert!(body("before:block").contains("::before { content: ''; display: block"));
        assert!(body("max-md:hidden").contains("(width < 48rem)"));
        assert!(compile("md:p-banana").is_none());
        assert!(compile("wat:p-4").is_none());
        // larger breakpoints sort after smaller ones, variants after base utilities
        assert!(compile("lg:p-1").unwrap().sort > compile("md:p-1").unwrap().sort);
        assert!(compile("p-1").unwrap().sort < compile("hover:p-1").unwrap().sort);
        assert!(compile("p-1").unwrap().sort < compile("px-1").unwrap().sort);
    }
}
