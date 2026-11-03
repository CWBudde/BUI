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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// A custom property (`--name`): accepted, no effect.
    pub const CUSTOM: Prop = Prop(u16::MAX - 1);

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

    /// The property's name ("" for unknown and custom properties).
    pub fn name(self) -> &'static str {
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

/// A declaration's tokens: one inline token (typed style methods), a slice
/// parsed once for the whole program (fixed style methods), or parsed CSS.
#[derive(Clone, Debug)]
pub enum Value {
    One(Tok),
    Static(&'static [Tok]),
    Owned(Box<[Tok]>),
}

impl std::ops::Deref for Value {
    type Target = [Tok];
    fn deref(&self) -> &[Tok] {
        match self {
            Value::One(t) => std::slice::from_ref(t),
            Value::Static(s) => s,
            Value::Owned(b) => b,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, o: &Value) -> bool {
        match (self, o) {
            (Value::Static(a), Value::Static(b)) if std::ptr::eq(*a, *b) => true,
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

#[derive(Clone, Debug, PartialEq)]
enum AttrOp {
    Exists,
    Eq,
    Word,
    Prefix,
    Suffix,
    Contains,
    Dash,
}

#[derive(Clone, Debug, PartialEq)]
enum Simple {
    Universal,
    Tag(String),
    Id(String),
    Class(String),
    Attr(String, AttrOp, String),
    Pseudo(String),
    NthChild(i32, i32, bool), // an+b, from the end
    Not(Vec<Simple>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Combinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

/// Where a rule is filed in a stylesheet's index.
enum Bucket {
    Id(String),
    Class(String),
    Tag(String),
    Universal,
}

/// A complex selector: compounds joined by combinators (stored right to left).
#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    parts: Vec<(Vec<Simple>, Option<Combinator>)>,
    pub specificity: (u16, u16, u16),
}

/// What a selector can ask about an element.
pub trait Subject: Sized {
    fn tag(&self) -> &str;
    fn id(&self) -> Option<&str>;
    fn has_class(&self, class: &str) -> bool;
    fn attr(&self, name: &str) -> Option<&str>;
    fn state(&self, pseudo: &str) -> bool;
    fn parent(&self) -> Option<Self>;
    fn prev_sibling(&self) -> Option<Self>;
    /// 1-based position among element siblings, and their count.
    fn position(&self) -> (usize, usize);
    fn is_empty(&self) -> bool;
    /// The `class` attribute (used to find candidate rules quickly).
    fn class_attr(&self) -> Option<&str> {
        self.attr("class")
    }
}

impl Selector {
    /// Parse one complex selector; None for unsupported syntax
    /// (pseudo-elements), so the rule is skipped as browsers do.
    pub fn parse(s: &str) -> Option<Selector> {
        let s = s.trim();
        if s.is_empty() || s.contains("::") {
            return None;
        }
        let mut parts: Vec<(Vec<Simple>, Option<Combinator>)> = Vec::new();
        let mut pending: Option<Combinator> = None;
        let mut i = 0;
        let b = s.as_bytes();
        while i < b.len() {
            let c = b[i] as char;
            if c.is_whitespace() {
                i += 1;
                if pending.is_none() && !parts.is_empty() {
                    pending = Some(Combinator::Descendant);
                }
                continue;
            }
            if matches!(c, '>' | '+' | '~') {
                pending = Some(match c {
                    '>' => Combinator::Child,
                    '+' => Combinator::Adjacent,
                    _ => Combinator::Sibling,
                });
                i += 1;
                continue;
            }
            let (compound, next) = parse_compound(s, i)?;
            if let Some(last) = parts.last_mut() {
                last.1 = Some(pending.unwrap_or(Combinator::Descendant));
            }
            parts.push((compound, None));
            pending = None;
            i = next;
        }
        if parts.is_empty() {
            return None;
        }
        parts.reverse(); // match from the subject outwards
        let mut spec = (0u16, 0u16, 0u16);
        for (compound, _) in &parts {
            add_specificity(compound, &mut spec);
        }
        // the combinator stored with a compound links it to the *previous* one (leftwards)
        let n = parts.len();
        let combs: Vec<Option<Combinator>> = parts.iter().map(|p| p.1).collect();
        for k in 0..n {
            parts[k].1 = if k + 1 < n { combs[k + 1] } else { None };
        }
        Some(Selector { parts, specificity: spec })
    }

    /// The bucket a rule is filed under: its subject's id, class, tag, or "*".
    fn bucket(&self) -> Bucket {
        let subject = &self.parts[0].0;
        subject.iter().find_map(|s| match s {
            Simple::Id(i) => Some(Bucket::Id(i.clone())),
            _ => None,
        }).or_else(|| subject.iter().find_map(|s| match s {
            Simple::Class(c) => Some(Bucket::Class(c.clone())),
            _ => None,
        })).or_else(|| subject.iter().find_map(|s| match s {
            Simple::Tag(t) => Some(Bucket::Tag(t.to_ascii_lowercase())),
            _ => None,
        })).unwrap_or(Bucket::Universal)
    }

    pub fn matches<S: Subject + Clone>(&self, el: &S) -> bool {
        match_from(&self.parts, 0, el)
    }
}

fn add_specificity(compound: &[Simple], spec: &mut (u16, u16, u16)) {
    for s in compound {
        match s {
            Simple::Id(_) => spec.0 += 1,
            Simple::Class(_) | Simple::Attr(..) | Simple::Pseudo(_) | Simple::NthChild(..) => {
                spec.1 += 1
            }
            Simple::Tag(_) => spec.2 += 1,
            Simple::Not(inner) => add_specificity(inner, spec),
            Simple::Universal => {}
        }
    }
}

fn match_from<S: Subject + Clone>(parts: &[(Vec<Simple>, Option<Combinator>)], k: usize, el: &S)
    -> bool {
    let (compound, comb) = &parts[k];
    if !compound.iter().all(|s| match_simple(s, el)) {
        return false;
    }
    let Some(comb) = comb else { return true };
    match comb {
        Combinator::Child => el.parent().is_some_and(|p| match_from(parts, k + 1, &p)),
        Combinator::Descendant => {
            let mut cur = el.parent();
            while let Some(p) = cur {
                if match_from(parts, k + 1, &p) {
                    return true;
                }
                cur = p.parent();
            }
            false
        }
        Combinator::Adjacent => el.prev_sibling().is_some_and(|p| match_from(parts, k + 1, &p)),
        Combinator::Sibling => {
            let mut cur = el.prev_sibling();
            while let Some(p) = cur {
                if match_from(parts, k + 1, &p) {
                    return true;
                }
                cur = p.prev_sibling();
            }
            false
        }
    }
}

fn match_simple<S: Subject + Clone>(s: &Simple, el: &S) -> bool {
    match s {
        Simple::Universal => true,
        Simple::Tag(t) => el.tag().eq_ignore_ascii_case(t),
        Simple::Id(id) => el.id() == Some(id.as_str()),
        Simple::Class(c) => el.has_class(c),
        Simple::Attr(name, op, val) => match (el.attr(name), op) {
            (None, _) => false,
            (Some(_), AttrOp::Exists) => true,
            (Some(v), AttrOp::Eq) => v == val,
            (Some(v), AttrOp::Word) => v.split_whitespace().any(|w| w == val),
            (Some(v), AttrOp::Prefix) => v.starts_with(val.as_str()),
            (Some(v), AttrOp::Suffix) => v.ends_with(val.as_str()),
            (Some(v), AttrOp::Contains) => v.contains(val.as_str()),
            (Some(v), AttrOp::Dash) => v == val || v.starts_with(&format!("{val}-")),
        },
        Simple::Pseudo(p) => match p.as_str() {
            "first-child" => el.position().0 == 1,
            "last-child" => {
                let (i, n) = el.position();
                i == n
            }
            "only-child" => el.position().1 == 1,
            "root" => el.parent().is_none(),
            "empty" => el.is_empty(),
            "first-of-type" | "last-of-type" => {
                // approximated with element position
                let (i, n) = el.position();
                if p == "first-of-type" { i == 1 } else { i == n }
            }
            other => el.state(other),
        },
        Simple::NthChild(a, b, from_end) => {
            let (i, n) = el.position();
            let pos = if *from_end { (n + 1 - i) as i32 } else { i as i32 };
            if *a == 0 {
                pos == *b
            } else {
                (pos - b) % a == 0 && (pos - b) / a >= 0
            }
        }
        Simple::Not(inner) => !inner.iter().all(|s| match_simple(s, el)),
    }
}

fn parse_compound(s: &str, mut i: usize) -> Option<(Vec<Simple>, usize)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i] as char;
        match c {
            '*' => {
                out.push(Simple::Universal);
                i += 1;
            }
            '#' | '.' => {
                let end = scan_name(s, i + 1);
                let name = s[i + 1..end].to_string();
                out.push(if c == '#' { Simple::Id(name) } else { Simple::Class(name) });
                i = end;
            }
            '[' => {
                let close = s[i..].find(']')? + i;
                out.push(parse_attr(&s[i + 1..close])?);
                i = close + 1;
            }
            ':' => {
                let end = scan_name(s, i + 1);
                let name = s[i + 1..end].to_ascii_lowercase();
                if b.get(end) == Some(&b'(') {
                    let close = matching_paren(s, end);
                    let arg = s[end + 1..close].trim();
                    out.push(match name.as_str() {
                        "not" => Simple::Not(parse_compound(arg, 0)?.0),
                        "nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type" => {
                            let (a, bb) = parse_nth(arg)?;
                            Simple::NthChild(a, bb, name.contains("last"))
                        }
                        "is" | "where" => {
                            // first alternative only (simple and common)
                            let first = split_top(arg, ',')[0];
                            Simple::Not(vec![Simple::Not(parse_compound(first.trim(), 0)?.0)])
                        }
                        _ => return None,
                    });
                    i = close + 1;
                } else {
                    out.push(Simple::Pseudo(name));
                    i = end;
                }
            }
            c if is_name_start(c) => {
                let end = scan_name(s, i);
                out.push(Simple::Tag(s[i..end].to_ascii_lowercase()));
                i = end;
            }
            _ => break,
        }
    }
    (!out.is_empty()).then_some((out, i))
}

fn parse_attr(inner: &str) -> Option<Simple> {
    for (op, kind) in [("~=", AttrOp::Word), ("^=", AttrOp::Prefix), ("$=", AttrOp::Suffix),
                       ("*=", AttrOp::Contains), ("|=", AttrOp::Dash), ("=", AttrOp::Eq)] {
        if let Some((name, val)) = inner.split_once(op) {
            let val = val.trim().trim_matches(['"', '\'']).to_string();
            return Some(Simple::Attr(name.trim().to_ascii_lowercase(), kind, val));
        }
    }
    Some(Simple::Attr(inner.trim().to_ascii_lowercase(), AttrOp::Exists, String::new()))
}

fn parse_nth(arg: &str) -> Option<(i32, i32)> {
    let a = arg.replace(' ', "").to_ascii_lowercase();
    match a.as_str() {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    if let Some((x, y)) = a.split_once('n') {
        let x = match x {
            "" | "+" => 1,
            "-" => -1,
            v => v.parse().ok()?,
        };
        let y = if y.is_empty() { 0 } else { y.parse().ok()? };
        Some((x, y))
    } else {
        Some((0, a.parse().ok()?))
    }
}

// ==========================================================================
// Stylesheets

/// One rule: a selector with its declarations, in source order.
#[derive(Clone, Debug)]
pub struct Rule {
    pub selector: Selector,
    pub decls: Vec<Decl>,
    pub order: usize,
    media: Option<Media>,
}

#[derive(Clone, Debug, PartialEq)]
struct Media {
    min_width: Option<f32>,
    max_width: Option<f32>,
    min_height: Option<f32>,
    max_height: Option<f32>,
    dark: Option<bool>,
}

impl Media {
    fn parse(q: &str) -> Media {
        let mut m = Media { min_width: None, max_width: None, min_height: None, max_height: None,
                            dark: None };
        for part in q.split("and") {
            let part = part.trim().trim_matches(['(', ')']).trim();
            if let Some((k, v)) = part.split_once(':') {
                let v = v.trim();
                let px = v.trim_end_matches("px").parse::<f32>().ok();
                match k.trim() {
                    "min-width" => m.min_width = px,
                    "max-width" => m.max_width = px,
                    "min-height" => m.min_height = px,
                    "max-height" => m.max_height = px,
                    "prefers-color-scheme" => m.dark = Some(v == "dark"),
                    _ => {}
                }
            }
        }
        m
    }

    fn applies(&self, viewport: (f32, f32)) -> bool {
        self.min_width.is_none_or(|w| viewport.0 >= w)
            && self.max_width.is_none_or(|w| viewport.0 <= w)
            && self.min_height.is_none_or(|h| viewport.1 >= h)
            && self.max_height.is_none_or(|h| viewport.1 <= h)
            && self.dark.is_none_or(|d| d) // Blender's theme is dark
    }
}

/// A parsed stylesheet.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
    /// Rule indices by the subject's id / class / tag, or universal (rebuilt
    /// when `rules` changes length). Looked up by `&str`: no key building.
    by_id: std::collections::HashMap<String, Vec<usize>>,
    by_class: std::collections::HashMap<String, Vec<usize>>,
    by_tag: std::collections::HashMap<String, Vec<usize>>,
    universal: Vec<usize>,
    indexed: usize,
}

/// Rules matching one element: on the stack for the usual handful.
pub(crate) type Matches<'a> = smallvec::SmallVec<[&'a Rule; 8]>;

impl Stylesheet {
    /// Parse CSS text (rules, `@media` blocks; other at-rules are skipped).
    pub fn parse(css: &str) -> Stylesheet {
        let mut sheet = Stylesheet::default();
        sheet.parse_into(&strip_comments(css), None);
        sheet.reindex();
        sheet
    }

    fn reindex(&mut self) {
        self.by_id.clear();
        self.by_class.clear();
        self.by_tag.clear();
        self.universal.clear();
        for (k, r) in self.rules.iter().enumerate() {
            match r.selector.bucket() {
                Bucket::Id(s) => self.by_id.entry(s).or_default().push(k),
                Bucket::Class(s) => self.by_class.entry(s).or_default().push(k),
                Bucket::Tag(s) => self.by_tag.entry(s).or_default().push(k),
                Bucket::Universal => self.universal.push(k),
            }
        }
        self.indexed = self.rules.len();
    }

    fn parse_into(&mut self, css: &str, media: Option<Media>) {
        let mut rest = css;
        while let Some(open) = rest.find('{') {
            let prelude = rest[..open].trim();
            let close = block_end(rest, open);
            let body = &rest[open + 1..close.min(rest.len())];
            if let Some(q) = prelude.strip_prefix("@media") {
                self.parse_into(body, Some(Media::parse(q)));
            } else if !prelude.starts_with('@') {
                let decls = parse_declarations(body);
                for sel in split_top(prelude, ',') {
                    if let Some(selector) = Selector::parse(sel) {
                        let order = self.rules.len();
                        self.rules.push(Rule { selector, decls: decls.clone(), order,
                                               media: media.clone() });
                    }
                }
            }
            rest = &rest[(close + 1).min(rest.len())..];
        }
    }

    /// Declarations of every matching rule, in cascade order.
    pub fn matching<'a, S: Subject + Clone>(&'a self, el: &S, viewport: (f32, f32))
        -> Vec<&'a Rule> {
        self.matches(el, viewport).into_vec()
    }

    /// [`Self::matching`] without heap allocation for the usual few rules.
    pub(crate) fn matches<'a, S: Subject + Clone>(&'a self, el: &S, viewport: (f32, f32)) -> Matches<'a> {
        let ok = |r: &&Rule| r.media.as_ref().is_none_or(|m| m.applies(viewport)) && r.selector.matches(el);
        let mut hits: Matches<'a> = if self.indexed != self.rules.len() {
            self.rules.iter().filter(ok).collect() // `rules` edited by hand: scan all
        } else {
            let mut cand: smallvec::SmallVec<[usize; 16]> = smallvec::SmallVec::new();
            cand.extend_from_slice(&self.universal);
            let tag = el.tag();
            let found = if tag.bytes().any(|b| b.is_ascii_uppercase()) {
                self.by_tag.get(&tag.to_ascii_lowercase())
            } else {
                self.by_tag.get(tag)
            };
            if let Some(v) = found {
                cand.extend_from_slice(v);
            }
            if let Some(v) = el.id().and_then(|id| self.by_id.get(id)) {
                cand.extend_from_slice(v);
            }
            if let Some(classes) = el.class_attr() {
                for c in classes.split_whitespace() {
                    if let Some(v) = self.by_class.get(c) {
                        cand.extend_from_slice(v);
                    }
                }
            }
            cand.sort_unstable();
            cand.dedup();
            cand.into_iter().map(|k| &self.rules[k]).filter(ok).collect()
        };
        hits.sort_by_key(|r| (r.selector.specificity, r.order));
        hits
    }
}

fn block_end(s: &str, open: usize) -> usize {
    let mut depth = 0;
    for (k, c) in s[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
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
        assert!(Selector::parse("p::before").is_none());
        let sheet = Stylesheet::parse("@media (max-width: 100px) { p { color: red } } a { b: c }");
        assert_eq!(sheet.rules.len(), 2);
    }
}
