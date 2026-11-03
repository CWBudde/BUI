//! The CSS engine: tokens, declarations, stylesheets, selectors, cascade.
//!
//! Every way of styling (gpui/tailwind methods, `.style("...")`, the
//! user-agent stylesheet, `<style>` elements, `:hover` states) ends up as
//! [`Decl`]s applied by one property table (`style.rs`).

use std::borrow::Cow;

// ==========================================================================
// Tokens

/// Text inside tokens: borrowed from the binary for values known at compile
/// time, owned for parsed CSS.
pub type CssStr = Cow<'static, str>;

/// A CSS component value.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(f32),
    Dim(f32, CssStr),
    Pct(f32),
    Ident(CssStr),
    Hash(CssStr),
    Str(CssStr),
    /// `name(args)`, arguments split at top-level commas. Boxed: functions
    /// are rare and this keeps every token at 32 bytes.
    Func(Box<FuncTok>),
    /// A color given as a value (typed style methods: `.bg(rgb(..))`).
    Color(crate::color::Rgba),
    /// A parsed math expression (typed style methods: `.w(calc(..))`).
    Calc(&'static crate::calc::Calc),
    Comma,
    Slash,
    Delim(char),
}

/// A CSS function call: `rgb(1, 2, 3)`, `repeat(3, 1fr)` ...
#[derive(Clone, Debug, PartialEq)]
pub struct FuncTok {
    pub name: CssStr,
    pub args: Vec<Vec<Tok>>,
}

impl Tok {
    pub(crate) fn ident(&self) -> Option<&str> {
        match self {
            Tok::Ident(s) => Some(s),
            _ => None,
        }
    }

    /// A length as the token CSS text would produce (`40px`, `50%`, `auto`).
    pub const fn length(l: crate::geometry::Length) -> Tok {
        use crate::geometry::Length;
        match l {
            Length::Px(v) => Tok::Dim(v, Cow::Borrowed("px")),
            Length::Rem(v) => Tok::Dim(v, Cow::Borrowed("rem")),
            Length::Em(v) => Tok::Dim(v, Cow::Borrowed("em")),
            Length::Vw(v) => Tok::Dim(v, Cow::Borrowed("vw")),
            Length::Vh(v) => Tok::Dim(v, Cow::Borrowed("vh")),
            Length::Vmin(v) => Tok::Dim(v, Cow::Borrowed("vmin")),
            Length::Vmax(v) => Tok::Dim(v, Cow::Borrowed("vmax")),
            Length::Calc(c) => Tok::Calc(c),
            Length::Percent(v) => Tok::Pct(v),
            Length::Auto => Tok::Ident(Cow::Borrowed("auto")),
        }
    }
}

impl std::hash::Hash for Tok {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::mem::discriminant(self).hash(h);
        match self {
            Tok::Num(v) | Tok::Pct(v) => v.to_bits().hash(h),
            Tok::Dim(v, u) => {
                v.to_bits().hash(h);
                u.hash(h);
            }
            Tok::Ident(s) | Tok::Hash(s) | Tok::Str(s) => s.hash(h),
            Tok::Func(fx) => {
                fx.name.hash(h);
                fx.args.hash(h);
            }
            Tok::Color(c) => [c.r, c.g, c.b, c.a].map(f32::to_bits).hash(h),
            Tok::Delim(c) => c.hash(h),
            Tok::Calc(c) => (*c as *const crate::calc::Calc as usize).hash(h),
            Tok::Comma | Tok::Slash => {}
        }
    }
}

/// Tokenize a CSS value (no selectors or blocks).
pub fn tokenize(s: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() {
            i += 1;
        } else if c == ',' {
            out.push(Tok::Comma);
            i += 1;
        } else if c == '/' {
            out.push(Tok::Slash);
            i += 1;
        } else if c == '"' || c == '\'' {
            let end = s[i + 1..].find(c).map(|e| i + 1 + e).unwrap_or(s.len());
            out.push(Tok::Str(s[i + 1..end].to_string().into()));
            i = end + 1;
        } else if c == '#' {
            let end = scan_name(s, i + 1);
            out.push(Tok::Hash(s[i + 1..end].to_ascii_lowercase().into()));
            i = end;
        } else if c.is_ascii_digit()
            || (c == '.' && b.get(i + 1).is_some_and(|d| d.is_ascii_digit()))
            || ((c == '-' || c == '+')
                && b.get(i + 1).is_some_and(|d| d.is_ascii_digit() || *d == b'.'))
        {
            let mut end = i + 1;
            while end < b.len() && (b[end].is_ascii_digit() || b[end] == b'.') {
                end += 1;
            }
            if end < b.len() && (b[end] == b'e' || b[end] == b'E')
                && b.get(end + 1).is_some_and(|d| d.is_ascii_digit() || *d == b'-')
            {
                end += 2;
                while end < b.len() && b[end].is_ascii_digit() {
                    end += 1;
                }
            }
            let v: f32 = s[i..end].parse().unwrap_or(0.0);
            if end < b.len() && b[end] == b'%' {
                out.push(Tok::Pct(v));
                i = end + 1;
            } else {
                let unit_end = scan_name(s, end);
                if unit_end > end {
                    out.push(Tok::Dim(v, s[end..unit_end].to_ascii_lowercase().into()));
                } else {
                    out.push(Tok::Num(v));
                }
                i = unit_end;
            }
        } else if is_name_start(c) {
            let end = scan_name(s, i);
            let name = s[i..end].to_ascii_lowercase();
            if b.get(end) == Some(&b'(') {
                let close = matching_paren(s, end);
                let inner = &s[end + 1..close];
                let args = if name == "url" {
                    vec![vec![Tok::Str(inner.trim().trim_matches(['"', '\'']).to_string().into())]]
                } else {
                    split_top(inner, ',').into_iter().map(tokenize).collect()
                };
                out.push(Tok::Func(Box::new(FuncTok { name: name.into(), args })));
                i = (close + 1).min(s.len());
            } else {
                out.push(Tok::Ident(name.into()));
                i = end;
            }
        } else {
            out.push(Tok::Delim(c));
            i += c.len_utf8();
        }
    }
    out
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '-' || c == '_' || !c.is_ascii()
}

fn scan_name(s: &str, mut i: usize) -> usize {
    let b = s.as_bytes();
    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-' || b[i] == b'_' || b[i] >= 0x80)
    {
        i += 1;
    }
    i
}

fn matching_paren(s: &str, open: usize) -> usize {
    let mut depth = 0;
    for (k, c) in s[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return open + k;
                }
            }
            _ => {}
        }
    }
    s.len()
}

/// Split at `sep` outside parentheses and quotes.
pub(crate) fn split_top(s: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut quote, mut start) = (0i32, None::<char>, 0);
    for (k, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(' | '[') => depth += 1,
            (None, ')' | ']') => depth -= 1,
            (None, _) if c == sep && depth == 0 => {
                parts.push(&s[start..k]);
                start = k + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

// ==========================================================================
// Declarations

/// A CSS property, as an index into the property table (`style::HANDLERS`,
/// then `style::ACCEPTED`). Style methods resolve it at compile time with
/// [`Prop::named`]; parsed CSS with [`Prop::lookup`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Prop(pub(crate) u16);

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

impl Prop {
    /// Not a property this engine knows.
    pub const UNKNOWN: Prop = Prop(u16::MAX);
    /// A custom property whose name isn't known yet (`const` contexts).
    pub const CUSTOM: Prop = Prop(u16::MAX - 1);
    /// Custom properties (`--name`) are interned at run time from here up.
    pub(crate) const CUSTOM_BASE: u16 = 0x4000;

    /// Whether this is a custom property (`--name`).
    pub fn is_custom(self) -> bool {
        self.0 >= Prop::CUSTOM_BASE && self != Prop::UNKNOWN
    }

    /// The property for custom property `name` (`--x`), interned.
    pub fn custom(name: &str) -> Prop {
        use std::sync::{Mutex, OnceLock};
        type Names = (crate::style::FnvMap<&'static str, u16>, Vec<&'static str>);
        static NAMES: OnceLock<Mutex<Names>> = OnceLock::new();
        let mut t = NAMES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&i) = t.0.get(name) {
            return Prop(Prop::CUSTOM_BASE + i);
        }
        let n = t.1.len() as u16;
        if Prop::CUSTOM_BASE as u32 + n as u32 >= (u16::MAX - 1) as u32 {
            return Prop::CUSTOM;
        }
        let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
        t.0.insert(leaked, n);
        t.1.push(leaked);
        CUSTOM_NAMES.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap_or_else(|e| e.into_inner()).push(leaked);
        Prop(Prop::CUSTOM_BASE + n)
    }

    /// The property called `name`, evaluated at compile time in `const`
    /// contexts (an unknown name there is a compile error).
    pub const fn named(name: &str) -> Prop {
        let h = crate::style::HANDLERS;
        let mut i = 0;
        while i < h.len() {
            if str_eq(h[i].0, name) {
                return Prop(i as u16);
            }
            i += 1;
        }
        let a = crate::style::ACCEPTED;
        let mut j = 0;
        while j < a.len() {
            if str_eq(a[j], name) {
                return Prop((h.len() + j) as u16);
            }
            j += 1;
        }
        let b = name.as_bytes();
        if b.len() > 2 && b[0] == b'-' && b[1] == b'-' {
            return Prop::CUSTOM;
        }
        panic!("unknown CSS property")
    }

    /// The property called `name` (lowercase), at run time.
    pub fn lookup(name: &str) -> Prop {
        crate::style::prop_index(name)
    }

    /// The property's name ("" for unknown properties).
    pub fn name(self) -> &'static str {
        if self.is_custom() && self != Prop::CUSTOM {
            let names = CUSTOM_NAMES.get_or_init(|| std::sync::Mutex::new(Vec::new()));
            let names = names.lock().unwrap_or_else(|e| e.into_inner());
            return names.get((self.0 - Prop::CUSTOM_BASE) as usize).copied().unwrap_or("");
        }
        let (h, a) = (crate::style::HANDLERS, crate::style::ACCEPTED);
        let i = self.0 as usize;
        if i < h.len() {
            h[i].0
        } else if i - h.len() < a.len() && self != Prop::CUSTOM && self != Prop::UNKNOWN {
            a[i - h.len()]
        } else {
            ""
        }
    }
}

impl std::fmt::Display for Prop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Custom property names by index (`Prop::custom`).
static CUSTOM_NAMES: std::sync::OnceLock<std::sync::Mutex<Vec<&'static str>>> = std::sync::OnceLock::new();

/// A declaration's tokens: one inline token (typed style methods), a slice
/// parsed once for the whole program (fixed style methods), or parsed CSS.
#[derive(Clone, Debug)]
pub enum Value {
    One(Tok),
    Static(&'static [Tok]),
    Owned(Box<[Tok]>),
    /// Shared between elements: cloning bumps a count (styles parsed once
    /// and reused every render, e.g. by the Node binding).
    Shared(std::sync::Arc<[Tok]>),
}

impl std::ops::Deref for Value {
    type Target = [Tok];
    fn deref(&self) -> &[Tok] {
        match self {
            Value::One(t) => std::slice::from_ref(t),
            Value::Static(s) => s,
            Value::Owned(b) => b,
            Value::Shared(a) => a,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, o: &Value) -> bool {
        match (self, o) {
            (Value::Static(a), Value::Static(b)) if std::ptr::eq(*a, *b) => true,
            (Value::Shared(a), Value::Shared(b)) if std::sync::Arc::ptr_eq(a, b) => true,
            _ => **self == **o,
        }
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        (**self).hash(h)
    }
}

/// One `property: value` declaration.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Decl {
    pub prop: Prop,
    pub value: Value,
    pub important: bool,
}

impl Decl {
    /// A cheap hash for style sharing: fixed values (parsed once, unique per
    /// program) by address, others by content. Equal declarations may hash
    /// differently across variants; that only costs a cache miss.
    pub(crate) fn probe_hash<H: std::hash::Hasher>(&self, h: &mut H) {
        use std::hash::Hash;
        self.prop.hash(h);
        self.important.hash(h);
        match &self.value {
            Value::Static(s) => (s.as_ptr() as usize).hash(h),
            v => (**v).hash(h),
        }
    }

    /// Parse `value` for `prop` (the run-time path: `.css()`, stylesheets).
    pub fn new(prop: impl AsRef<str>, value: &str) -> Decl {
        let mut value = value.trim();
        let mut important = false;
        if let Some(v) = value.strip_suffix("!important") {
            value = v.trim_end();
            important = true;
        }
        let prop = prop.as_ref();
        let prop = if prop.bytes().any(|b| b.is_ascii_uppercase()) {
            Prop::lookup(&prop.to_ascii_lowercase())
        } else {
            Prop::lookup(prop)
        };
        Decl { prop, value: Value::Owned(tokenize(value).into_boxed_slice()), important }
    }

    /// A declaration with one already-typed token (no parsing).
    pub const fn one(prop: Prop, value: Tok) -> Decl {
        Decl { prop, value: Value::One(value), important: false }
    }
}

/// Parse a declaration block: `color: red; padding: 4px 8px`.
pub fn parse_declarations(block: &str) -> Vec<Decl> {
    split_top(&strip_comments(block), ';')
        .into_iter()
        .filter_map(|d| {
            let (name, value) = d.split_once(':')?;
            let name = name.trim();
            (!name.is_empty()).then(|| Decl::new(name.to_ascii_lowercase(), value))
        })
        .collect()
}

pub(crate) fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start + 2..].find("*/").map(|e| &rest[start + 2 + e + 2..]).unwrap_or("");
    }
    out.push_str(rest);
    out
}

// ==========================================================================
// Selectors

pub use crate::selector::{
    has_match, Combinator, ContainerQuery, HasEntry, FontFace, Keyframes, MediaEnv, MediaQuery, PropertyRule,
    PseudoElement, RelSelector, Rule, Selector, Stylesheet, Subject,
};
pub(crate) use crate::selector::Matches;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_values() {
        let t = tokenize("1px solid rgb(255, 0, 0) 50% -2.5rem #abc 'x'");
        assert_eq!(t[0], Tok::Dim(1.0, "px".into()));
        assert_eq!(t[1], Tok::Ident("solid".into()));
        assert!(matches!(&t[2], Tok::Func(fx) if fx.name == "rgb" && fx.args.len() == 3));
        assert_eq!(std::mem::size_of::<Tok>(), 32);
        assert_eq!(t[3], Tok::Pct(50.0));
        assert_eq!(t[4], Tok::Dim(-2.5, "rem".into()));
        assert_eq!(t[5], Tok::Hash("abc".into()));
        assert_eq!(t[6], Tok::Str("x".into()));
    }

    #[test]
    fn parses_blocks_and_important() {
        let d = parse_declarations("color: red; padding: 4px 8px !important; /* c */ ;");
        assert_eq!(d.len(), 2);
        assert!(d[1].important && d[1].value.len() == 2);
        assert_eq!(d[0].prop, Prop::named("color"));
    }

    #[test]
    fn selector_specificity() {
        let s = Selector::parse("ul > li.item:hover a#x").unwrap();
        assert_eq!(s.specificity, (1, 2, 3));
        assert_eq!(Selector::parse("p::before").unwrap().pseudo, Some(PseudoElement::Before));
        let sheet = Stylesheet::parse("@media (max-width: 100px) { p { color: red } } a { b: c }");
        assert_eq!(sheet.rules.len(), 2);
    }
}
