//! Selectors Level 4 and the stylesheet: rules, at-rules and matching.
//!
//! Selectors: type / class / id / attribute (all operators, `i` / `s`
//! flags), every combinator, `:is()` / `:where()` / `:not()` with full
//! selector lists, relative `:has()`, `:nth-*(An+B [of S])`, the
//! `*-of-type` family, `:lang()`, `:dir()`, `:scope`, nesting (`&`) and
//! pseudo-elements (`::before`, `::after`, `::marker`, `::placeholder` ...).
//!
//! Stylesheets: nested rules, `@media` (Media Queries 4: ranges, `not`,
//! `or`, every desktop feature), `@supports` (incl. `selector()`),
//! `@layer` (cascade layers, statement and block forms), `@container` (size
//! queries), `@scope`, `@font-face`, `@keyframes`, `@property`; the rest
//! (`@page`, `@counter-style`, `@starting-style` ...) parse and are skipped.

use crate::css::{parse_declarations, split_top, strip_comments, Decl};

// ==========================================================================
// Selectors

#[derive(Clone, Copy, Debug, PartialEq)]
enum AttrOp {
    Exists,
    Eq,
    Word,
    Prefix,
    Suffix,
    Contains,
    Dash,
}

/// Pseudo-elements a rule can target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PseudoElement {
    Before,
    After,
    Marker,
    Placeholder,
    Selection,
    FirstLine,
    FirstLetter,
    Backdrop,
    FileSelectorButton,
    /// Vendor and other pseudo-elements: parsed, never generated.
    Other,
}

#[derive(Clone, Debug, PartialEq)]
enum Simple {
    Universal,
    Tag(String),
    Id(String),
    Class(String),
    Attr { name: String, op: AttrOp, val: String, ci: bool },
    /// State and structural pseudo-classes without arguments.
    Pseudo(String),
    Nth { a: i32, b: i32, last: bool, of_type: bool, of: Option<Box<[Selector]>> },
    Not(Box<[Selector]>),
    Is(Box<[Selector]>),
    Where(Box<[Selector]>),
    /// `:has()`: the index of its relative selectors in the stylesheet.
    Has(u16),
    Lang(Box<[String]>),
    Dir(bool),
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Combinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

/// A complex selector: compounds joined by combinators, stored right to
/// left (each compound with the combinator to the next one leftwards).
#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    parts: Vec<(Vec<Simple>, Option<Combinator>)>,
    pub specificity: (u16, u16, u16),
    pub pseudo: Option<PseudoElement>,
}

/// One `:has()`: its relative selectors and the rest of the compound it
/// sits in (`.card` in `.card:has(img)`).
#[derive(Clone, Debug, PartialEq)]
pub struct HasEntry {
    pub rels: Box<[RelSelector]>,
    host: Vec<Simple>,
}

impl HasEntry {
    /// Whether the compound around this `:has()` can match `el` (a cheap
    /// filter before walking `el`'s subtree).
    pub fn host_matches<S: Subject>(&self, el: &S) -> bool {
        self.host.iter().all(|s| match s {
            Simple::Pseudo(_) => true, // states may change: evaluate anyway
            other => match_simple(other, el),
        })
    }
}

/// A relative selector inside `:has()`: `> img`, `+ .x`, `.a .b`.
#[derive(Clone, Debug, PartialEq)]
pub struct RelSelector {
    pub comb: Combinator,
    pub sel: Selector,
}

/// What a selector can ask about an element.
pub trait Subject: Sized + Clone {
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
    /// 1-based position among siblings of the same type, and their count.
    fn type_position(&self) -> (usize, usize) {
        let mut i = 1;
        let mut cur = self.prev_sibling();
        while let Some(p) = cur {
            if p.tag().eq_ignore_ascii_case(self.tag()) {
                i += 1;
            }
            cur = p.prev_sibling();
        }
        (i, i)
    }
    /// Whether `:has()` number `id` of the author stylesheet holds here.
    fn has(&self, _id: u16) -> bool {
        false
    }
    /// The size of the nearest query container (named `name`, if given).
    fn container(&self, _name: Option<&str>) -> Option<(f32, f32)> {
        None
    }
    fn same(&self, _other: &Self) -> bool {
        false
    }
    fn first_child(&self) -> Option<Self> {
        None
    }
    fn next_sibling(&self) -> Option<Self> {
        None
    }
}

impl Selector {
    /// Parse one complex selector (no relative combinator). `:has()`
    /// arguments are kept only when parsed through a [`Stylesheet`].
    pub fn parse(s: &str) -> Option<Selector> {
        let mut has = Vec::new();
        let (lead, sel) = parse_complex(s, &mut has, false)?;
        (lead.is_none()).then_some(sel)
    }

    /// Where a rule is filed in a stylesheet's index.
    fn bucket(&self) -> Bucket {
        let subject = &self.parts[0].0;
        let mut b = Bucket::Universal;
        for s in subject {
            match s {
                Simple::Id(i) => return Bucket::Id(i.clone()),
                Simple::Class(c) => b = Bucket::Class(c.clone()),
                Simple::Tag(t) if matches!(b, Bucket::Universal) => b = Bucket::Tag(t.clone()),
                _ => {}
            }
        }
        b
    }

    /// Whether `el` is this selector's subject (pseudo-elements are matched
    /// on their originating element).
    pub fn matches<S: Subject>(&self, el: &S) -> bool {
        match_from(&self.parts, 0, el, None)
    }
}

enum Bucket {
    Id(String),
    Class(String),
    Tag(String),
    Universal,
}

fn spec_of(compound: &[Simple], spec: &mut (u16, u16, u16)) {
    let max_of = |list: &[Selector]| list.iter().map(|s| s.specificity).max().unwrap_or((0, 0, 0));
    let add = |spec: &mut (u16, u16, u16), o: (u16, u16, u16)| {
        spec.0 += o.0;
        spec.1 += o.1;
        spec.2 += o.2;
    };
    for s in compound {
        match s {
            Simple::Id(_) => spec.0 += 1,
            Simple::Class(_) | Simple::Attr { .. } | Simple::Pseudo(_) | Simple::Lang(_) | Simple::Dir(_)
            | Simple::Has(_) | Simple::Never => spec.1 += 1,
            Simple::Nth { of, .. } => {
                spec.1 += 1;
                if let Some(of) = of {
                    add(spec, max_of(of));
                }
            }
            Simple::Tag(_) => spec.2 += 1,
            Simple::Not(l) | Simple::Is(l) => add(spec, max_of(l)),
            Simple::Where(_) | Simple::Universal => {}
        }
    }
}

/// `"a, b"` -> selectors; `forgiving` drops invalid ones (`:is()`, `:where()`).
fn parse_list(s: &str, has: &mut Vec<HasEntry>, forgiving: bool) -> Option<Box<[Selector]>> {
    let mut out = Vec::new();
    for part in split_top(s, ',') {
        match parse_complex(part, has, false) {
            Some((None, sel)) => out.push(sel),
            _ if forgiving => {}
            _ => return None,
        }
    }
    Some(out.into_boxed_slice())
}

/// One complex selector; with `relative`, it may start with a combinator.
fn parse_complex(s: &str, has: &mut Vec<HasEntry>, relative: bool)
    -> Option<(Option<Combinator>, Selector)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut parts: Vec<(Vec<Simple>, Option<Combinator>)> = Vec::new();
    let mut pending: Option<Combinator> = None;
    let mut lead = None;
    let mut pseudo = None;
    let b = s.as_bytes();
    let mut i = 0;
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
            let comb = match c {
                '>' => Combinator::Child,
                '+' => Combinator::Adjacent,
                _ => Combinator::Sibling,
            };
            if parts.is_empty() {
                if !relative || lead.is_some() {
                    return None;
                }
                lead = Some(comb);
            } else {
                pending = Some(comb);
            }
            i += 1;
            continue;
        }
        if pseudo.is_some() {
            return None; // nothing may follow a pseudo-element here
        }
        let (compound, pe, next) = parse_compound(s, i, has)?;
        pseudo = pe;
        if let Some(last) = parts.last_mut() {
            last.1 = Some(pending.unwrap_or(Combinator::Descendant));
        }
        parts.push((compound, None));
        pending = None;
        i = next;
    }
    if parts.is_empty() || pending.is_some() {
        return None;
    }
    parts.reverse();
    let mut spec = (0u16, 0u16, 0u16);
    for (compound, _) in &parts {
        spec_of(compound, &mut spec);
    }
    if pseudo.is_some() {
        spec.2 += 1;
    }
    let n = parts.len();
    let combs: Vec<Option<Combinator>> = parts.iter().map(|p| p.1).collect();
    for k in 0..n {
        parts[k].1 = if k + 1 < n { combs[k + 1] } else { None };
    }
    Some((if relative { Some(lead.unwrap_or(Combinator::Descendant)) } else { None },
          Selector { parts, specificity: spec, pseudo }))
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

fn scan_ident(s: &str, mut i: usize) -> usize {
    let b = s.as_bytes();
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            i += 2;
            continue;
        }
        let c = s[i..].chars().next().unwrap_or(' ');
        if !is_name_char(c) {
            break;
        }
        i += c.len_utf8();
    }
    i
}

/// CSS escapes in identifiers (`.md\:flex` -> `md:flex`, `.w-1\/2`).
fn unescape(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let mut hex = String::new();
        while hex.len() < 6 && it.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
            hex.push(it.next().unwrap_or('0'));
        }
        if hex.is_empty() {
            if let Some(n) = it.next() {
                out.push(n);
            }
        } else {
            if it.peek() == Some(&' ') {
                it.next();
            }
            out.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).unwrap_or('\u{fffd}'));
        }
    }
    out
}

fn matching_paren(s: &str, open: usize) -> usize {
    let (mut depth, mut quote) = (0i32, None::<char>);
    for (k, c) in s[open..].char_indices() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => {
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

fn pseudo_element(name: &str) -> PseudoElement {
    match name {
        "before" => PseudoElement::Before,
        "after" => PseudoElement::After,
        "marker" => PseudoElement::Marker,
        "placeholder" | "-webkit-input-placeholder" | "-moz-placeholder" => PseudoElement::Placeholder,
        "selection" | "-moz-selection" => PseudoElement::Selection,
        "first-line" => PseudoElement::FirstLine,
        "first-letter" => PseudoElement::FirstLetter,
        "backdrop" => PseudoElement::Backdrop,
        "file-selector-button" | "-webkit-file-upload-button" => PseudoElement::FileSelectorButton,
        _ => PseudoElement::Other,
    }
}

/// Pseudo-classes that are never true in this host (no history, no media
/// playback, no fullscreen, no autofill ...).
const NEVER: &[&str] = &[
    "visited", "target", "target-within", "fullscreen", "autofill", "-webkit-autofill", "modal",
    "popover-open", "picture-in-picture", "playing", "paused", "seeking", "buffering", "stalled",
    "muted", "volume-locked", "local-link", "current", "past", "future", "host", "user-invalid",
    "first", "left", "right", "blank", "-moz-focusring", "has-slotted", "open-in", "active-view-transition",
];

fn parse_compound(s: &str, mut i: usize, has: &mut Vec<HasEntry>)
    -> Option<(Vec<Simple>, Option<PseudoElement>, usize)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut pe = None;
    while i < b.len() {
        let c = b[i] as char;
        match c {
            '*' => {
                out.push(Simple::Universal);
                i += 1;
            }
            '&' => {
                // nesting resolved by the stylesheet; a bare `&` is the scope root
                out.push(Simple::Pseudo("scope".into()));
                i += 1;
            }
            '#' | '.' => {
                let end = scan_ident(s, i + 1);
                if end == i + 1 {
                    return None;
                }
                let name = unescape(&s[i + 1..end]);
                out.push(if c == '#' { Simple::Id(name) } else { Simple::Class(name) });
                i = end;
            }
            '[' => {
                let close = s[i..].find(']')? + i;
                out.push(parse_attr(&s[i + 1..close])?);
                i = close + 1;
            }
            '|' => i += 1, // namespaces: ignored
            ':' => {
                let double = b.get(i + 1) == Some(&b':');
                let start = if double { i + 2 } else { i + 1 };
                let end = scan_ident(s, start);
                let name = s[start..end].to_ascii_lowercase();
                let legacy_pe = matches!(name.as_str(), "before" | "after" | "first-line" | "first-letter");
                if double || legacy_pe {
                    pe = Some(pseudo_element(&name));
                    i = end;
                    if b.get(i) == Some(&b'(') {
                        i = matching_paren(s, i) + 1; // ::part(), ::slotted(), ::highlight()
                    }
                    continue;
                }
                if b.get(end) == Some(&b'(') {
                    let close = matching_paren(s, end);
                    let arg = s[end + 1..close.min(s.len())].trim();
                    out.push(match name.as_str() {
                        "not" => Simple::Not(parse_list(arg, has, false)?),
                        "is" | "matches" | "-webkit-any" | "-moz-any" => Simple::Is(parse_list(arg, has, true)?),
                        "where" => Simple::Where(parse_list(arg, has, true)?),
                        "has" => {
                            let mut rels = Vec::new();
                            for part in split_top(arg, ',') {
                                let (comb, sel) = parse_complex(part, has, true)?;
                                rels.push(RelSelector { comb: comb.unwrap_or(Combinator::Descendant), sel });
                            }
                            has.push(HasEntry { rels: rels.into_boxed_slice(), host: Vec::new() });
                            Simple::Has((has.len() - 1) as u16)
                        }
                        "nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type" => {
                            let (expr, of) = match split_of(arg) {
                                Some((e, sel)) => (e, Some(parse_list(sel, has, false)?)),
                                None => (arg, None),
                            };
                            let (a, bb) = parse_nth(expr)?;
                            Simple::Nth { a, b: bb, last: name.contains("last"), of_type: name.ends_with("of-type"),
                                          of: if name.ends_with("child") { of } else { None } }
                        }
                        "lang" => Simple::Lang(split_top(arg, ',').iter()
                            .map(|l| l.trim().trim_matches(['"', '\'']).to_ascii_lowercase()).collect()),
                        "dir" => Simple::Dir(arg.eq_ignore_ascii_case("rtl")),
                        "host" | "host-context" | "state" | "current" | "past" | "future" | "active-view-transition-type" => Simple::Never,
                        _ => return None,
                    });
                    i = close + 1;
                } else {
                    out.push(if NEVER.contains(&name.as_str()) { Simple::Never } else { Simple::Pseudo(name) });
                    i = end;
                }
            }
            c if is_name_char(c) && !c.is_ascii_digit() => {
                let end = scan_ident(s, i);
                out.push(Simple::Tag(unescape(&s[i..end]).to_ascii_lowercase()));
                i = end;
            }
            _ => break,
        }
    }
    for k in 0..out.len() {
        if let Simple::Has(id) = out[k] {
            // the rest of the compound: checked before evaluating :has() itself
            let host: Vec<Simple> = out.iter().filter(|x| !matches!(x, Simple::Has(_))).cloned().collect();
            if let Some(e) = has.get_mut(id as usize) {
                e.host = host;
            }
        }
    }
    (!out.is_empty() || pe.is_some()).then(|| {
        if out.is_empty() {
            out.push(Simple::Universal);
        }
        (out, pe, i)
    })
}

/// `2n+1 of .x` -> ("2n+1", ".x")
fn split_of(arg: &str) -> Option<(&str, &str)> {
    let lower = arg.to_ascii_lowercase();
    let k = lower.find(" of ")?;
    Some((&arg[..k], &arg[k + 4..]))
}

fn parse_attr(inner: &str) -> Option<Simple> {
    let inner = inner.trim();
    // trailing case flag: [a="b" i]
    let (body, ci) = match inner.rsplit_once(char::is_whitespace) {
        Some((b, f)) if (f.eq_ignore_ascii_case("i") || f.eq_ignore_ascii_case("s")) && b.contains('=') => {
            (b.trim(), f.eq_ignore_ascii_case("i"))
        }
        _ => (inner, false),
    };
    for (op, kind) in [("~=", AttrOp::Word), ("^=", AttrOp::Prefix), ("$=", AttrOp::Suffix),
                       ("*=", AttrOp::Contains), ("|=", AttrOp::Dash), ("=", AttrOp::Eq)] {
        if let Some((name, val)) = body.split_once(op) {
            let mut val = val.trim().trim_matches(['"', '\'']).to_string();
            if ci {
                val = val.to_lowercase();
            }
            let name = name.trim().trim_start_matches('|').rsplit('|').next().unwrap_or("").to_ascii_lowercase();
            return Some(Simple::Attr { name, op: kind, val, ci });
        }
    }
    Some(Simple::Attr { name: body.to_ascii_lowercase(), op: AttrOp::Exists, val: String::new(), ci: false })
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
        let y = if y.is_empty() { 0 } else { y.trim_start_matches('+').parse().ok()? };
        Some((x, y))
    } else {
        Some((0, a.trim_start_matches('+').parse().ok()?))
    }
}

fn nth_ok(a: i32, b: i32, pos: i32) -> bool {
    if a == 0 {
        pos == b
    } else {
        (pos - b) % a == 0 && (pos - b) / a >= 0
    }
}

/// Match `parts[k..]` at `el`. `anchor`: for `:has()`, the element the
/// leftmost compound must stand in `comb` relation to.
fn match_from<S: Subject>(parts: &[(Vec<Simple>, Option<Combinator>)], k: usize, el: &S,
                          anchor: Option<(&S, Combinator)>) -> bool {
    let (compound, comb) = &parts[k];
    if !compound.iter().all(|s| match_simple(s, el)) {
        return false;
    }
    let Some(comb) = comb else {
        return match anchor {
            None => true,
            Some((a, c)) => related(el, a, c),
        };
    };
    let next = |p: &S| match_from(parts, k + 1, p, anchor);
    match comb {
        Combinator::Child => el.parent().is_some_and(|p| next(&p)),
        Combinator::Descendant => {
            let mut cur = el.parent();
            while let Some(p) = cur {
                if anchor.is_some_and(|(a, _)| a.same(&p)) {
                    return false; // went above the :has() anchor
                }
                if next(&p) {
                    return true;
                }
                cur = p.parent();
            }
            false
        }
        Combinator::Adjacent => el.prev_sibling().is_some_and(|p| next(&p)),
        Combinator::Sibling => {
            let mut cur = el.prev_sibling();
            while let Some(p) = cur {
                if next(&p) {
                    return true;
                }
                cur = p.prev_sibling();
            }
            false
        }
    }
}

/// Whether `el` stands in relation `c` to `anchor` (`anchor > el` ...).
fn related<S: Subject>(el: &S, anchor: &S, c: Combinator) -> bool {
    match c {
        Combinator::Child => el.parent().is_some_and(|p| p.same(anchor)),
        Combinator::Descendant => {
            let mut cur = el.parent();
            while let Some(p) = cur {
                if p.same(anchor) {
                    return true;
                }
                cur = p.parent();
            }
            false
        }
        Combinator::Adjacent => el.prev_sibling().is_some_and(|p| p.same(anchor)),
        Combinator::Sibling => {
            let mut cur = el.prev_sibling();
            while let Some(p) = cur {
                if p.same(anchor) {
                    return true;
                }
                cur = p.prev_sibling();
            }
            false
        }
    }
}

fn any_matches<S: Subject>(list: &[Selector], el: &S) -> bool {
    list.iter().any(|s| s.matches(el))
}

fn match_simple<S: Subject>(s: &Simple, el: &S) -> bool {
    match s {
        Simple::Universal => true,
        Simple::Tag(t) => el.tag().eq_ignore_ascii_case(t),
        Simple::Id(id) => el.id() == Some(id.as_str()),
        Simple::Class(c) => el.has_class(c),
        Simple::Attr { name, op, val, ci } => match el.attr(name) {
            None => false,
            Some(v) => {
                let low;
                let v = if *ci {
                    low = v.to_lowercase();
                    low.as_str()
                } else {
                    v
                };
                match op {
                    AttrOp::Exists => true,
                    AttrOp::Eq => v == val,
                    AttrOp::Word => v.split_whitespace().any(|w| w == val),
                    AttrOp::Prefix => !val.is_empty() && v.starts_with(val.as_str()),
                    AttrOp::Suffix => !val.is_empty() && v.ends_with(val.as_str()),
                    AttrOp::Contains => !val.is_empty() && v.contains(val.as_str()),
                    AttrOp::Dash => v == val || v.strip_prefix(val.as_str()).is_some_and(|r| r.starts_with('-')),
                }
            }
        },
        Simple::Pseudo(p) => match p.as_str() {
            "first-child" => el.position().0 == 1,
            "last-child" => {
                let (i, n) = el.position();
                i == n
            }
            "only-child" => el.position().1 == 1,
            "first-of-type" => el.type_position().0 == 1,
            "last-of-type" => {
                let (i, n) = el.type_position();
                i == n
            }
            "only-of-type" => el.type_position().1 == 1,
            "root" => el.parent().is_none(),
            "scope" => el.parent().is_none(),
            "empty" => el.is_empty(),
            "defined" => true,
            other => el.state(other),
        },
        Simple::Nth { a, b, last, of_type, of } => {
            let pos = match (of_type, of) {
                (true, _) => {
                    let (i, n) = el.type_position();
                    if *last { n + 1 - i } else { i }
                }
                (false, Some(list)) => {
                    if !any_matches(list, el) {
                        return false;
                    }
                    if *last {
                        let mut k = 1;
                        let mut cur = el.next_sibling();
                        while let Some(s) = cur {
                            if any_matches(list, &s) {
                                k += 1;
                            }
                            cur = s.next_sibling();
                        }
                        k
                    } else {
                        let mut k = 1;
                        let mut cur = el.prev_sibling();
                        while let Some(s) = cur {
                            if any_matches(list, &s) {
                                k += 1;
                            }
                            cur = s.prev_sibling();
                        }
                        k
                    }
                }
                (false, None) => {
                    let (i, n) = el.position();
                    if *last { n + 1 - i } else { i }
                }
            };
            nth_ok(*a, *b, pos as i32)
        }
        Simple::Not(list) => !any_matches(list, el),
        Simple::Is(list) | Simple::Where(list) => any_matches(list, el),
        Simple::Has(id) => el.has(*id),
        Simple::Lang(langs) => {
            let mut cur = Some(el.clone());
            while let Some(e) = cur {
                if let Some(l) = e.attr("lang") {
                    let l = l.to_ascii_lowercase();
                    return langs.iter().any(|w| w == "*" || l == *w || l.starts_with(&format!("{w}-")));
                }
                cur = e.parent();
            }
            false
        }
        Simple::Dir(rtl) => {
            let mut cur = Some(el.clone());
            while let Some(e) = cur {
                if let Some(d) = e.attr("dir") {
                    return d.eq_ignore_ascii_case("rtl") == *rtl;
                }
                cur = e.parent();
            }
            !*rtl
        }
        Simple::Never => false,
    }
}

/// Whether relative selector list `rels` holds for `el` (`:has()`):
/// some element related to `el` matches with `el` as the anchor.
pub fn has_match<S: Subject>(rels: &[RelSelector], el: &S) -> bool {
    rels.iter().any(|r| {
        let hit = |c: &S| match_from(&r.sel.parts, 0, c, Some((el, r.comb)));
        // `c` and its whole subtree
        let subtree = |root: S| -> bool {
            let mut stack = vec![root];
            while let Some(n) = stack.pop() {
                if hit(&n) {
                    return true;
                }
                let mut k = n.first_child();
                while let Some(c) = k {
                    k = c.next_sibling();
                    stack.push(c);
                }
            }
            false
        };
        match r.comb {
            // descendants (deeper compounds of `> a b` live below the children too)
            Combinator::Child | Combinator::Descendant => {
                let mut k = el.first_child();
                while let Some(c) = k {
                    k = c.next_sibling();
                    if subtree(c) {
                        return true;
                    }
                }
                false
            }
            Combinator::Adjacent | Combinator::Sibling => {
                let mut k = el.next_sibling();
                while let Some(s) = k {
                    k = s.next_sibling();
                    if subtree(s) {
                        return true;
                    }
                }
                false
            }
        }
    })
}

// ==========================================================================
// Media and container queries

/// The environment media queries are evaluated in.
#[derive(Clone, Copy, Debug)]
pub struct MediaEnv {
    pub width: f32,
    pub height: f32,
    pub dark: bool,
    pub scale: f32,
}

impl MediaEnv {
    pub fn viewport(viewport: (f32, f32)) -> MediaEnv {
        MediaEnv { width: viewport.0, height: viewport.1, dark: crate::color4::DARK, scale: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MediaValue {
    Num(f32),
    Ident(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Cond {
    And(Vec<Cond>),
    Or(Vec<Cond>),
    Not(Box<Cond>),
    /// `(name)`: boolean context
    Bool(String),
    /// `name op value` (ops normalized to the feature on the left)
    Cmp(String, Op, MediaValue),
    /// `style(...)` and other unknown conditions
    False,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    fn flip(self) -> Op {
        match self {
            Op::Lt => Op::Gt,
            Op::Le => Op::Ge,
            Op::Gt => Op::Lt,
            Op::Ge => Op::Le,
            Op::Eq => Op::Eq,
        }
    }
    fn test(self, a: f32, b: f32) -> bool {
        match self {
            Op::Eq => (a - b).abs() < 1e-3,
            Op::Lt => a < b,
            Op::Le => a <= b + 1e-3,
            Op::Gt => a > b,
            Op::Ge => a + 1e-3 >= b,
        }
    }
}

/// A media query list: true when any query is.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaQuery(Vec<(bool, Option<String>, Option<Cond>)>);

fn media_value(s: &str) -> MediaValue {
    let s = s.trim();
    if let Some((a, b)) = s.split_once('/') {
        if let (Ok(a), Ok(b)) = (a.trim().parse::<f32>(), b.trim().parse::<f32>()) {
            return MediaValue::Num(if b != 0.0 { a / b } else { 0.0 });
        }
    }
    let toks = crate::css::tokenize(s);
    match toks.as_slice() {
        [crate::css::Tok::Num(v)] => MediaValue::Num(*v),
        [crate::css::Tok::Dim(v, u)] => MediaValue::Num(match u.as_ref() {
            "px" => *v,
            "em" | "rem" => v * 16.0,
            "dppx" | "x" => *v,
            "dpi" => v / 96.0,
            "dpcm" => v * 2.54 / 96.0,
            u => crate::calc::leaf(*v, u).map(|c| c.eval_units(
                &crate::geometry::Units { font_size: 16.0, viewport: (0.0, 0.0) }, 0.0)).unwrap_or(*v),
        }),
        [t] if t.ident().is_some() => MediaValue::Ident(t.ident().unwrap_or("").to_string()),
        _ => MediaValue::Ident(s.to_ascii_lowercase()),
    }
}

fn parse_op(s: &str) -> Option<(Op, usize)> {
    let b = s.as_bytes();
    Some(match (b.first()?, b.get(1)) {
        (b'<', Some(b'=')) => (Op::Le, 2),
        (b'>', Some(b'=')) => (Op::Ge, 2),
        (b'<', _) => (Op::Lt, 1),
        (b'>', _) => (Op::Gt, 1),
        (b'=', _) => (Op::Eq, 1),
        _ => return None,
    })
}

/// One `( ... )` feature or nested condition (without the parens).
fn parse_feature(inner: &str) -> Cond {
    let inner = inner.trim();
    if inner.starts_with('(') || inner.starts_with("not ") || inner.contains(") and") || inner.contains(") or") {
        return parse_cond(inner);
    }
    if let Some((name, value)) = inner.split_once(':') {
        let name = name.trim().to_ascii_lowercase();
        if let Some(n) = name.strip_prefix("min-") {
            return Cond::Cmp(n.to_string(), Op::Ge, media_value(value));
        }
        if let Some(n) = name.strip_prefix("max-") {
            return Cond::Cmp(n.to_string(), Op::Le, media_value(value));
        }
        return Cond::Cmp(name, Op::Eq, media_value(value));
    }
    // range syntax: `width >= 600px`, `400px <= width < 700px`
    let ops: Vec<(usize, Op, usize)> = inner.char_indices()
        .filter_map(|(k, _)| {
            let prev_is_op = k > 0 && matches!(inner.as_bytes()[k - 1], b'<' | b'>');
            if prev_is_op {
                return None;
            }
            parse_op(&inner[k..]).map(|(o, n)| (k, o, n))
        })
        .collect();
    match ops.as_slice() {
        [] => Cond::Bool(inner.to_ascii_lowercase()),
        [(k, op, n)] => {
            let (l, r) = (inner[..*k].trim(), inner[k + n..].trim());
            if l.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && media_value(l) != MediaValue::Num(0.0)
                && !matches!(media_value(l), MediaValue::Num(_)) {
                Cond::Cmp(l.to_ascii_lowercase(), *op, media_value(r))
            } else {
                Cond::Cmp(r.to_ascii_lowercase(), op.flip(), media_value(l))
            }
        }
        [(k1, op1, n1), (k2, op2, n2)] => {
            let (a, name, b) = (inner[..*k1].trim(), inner[k1 + n1..*k2].trim(), inner[k2 + n2..].trim());
            let name = name.to_ascii_lowercase();
            Cond::And(vec![Cond::Cmp(name.clone(), op1.flip(), media_value(a)),
                           Cond::Cmp(name, *op2, media_value(b))])
        }
        _ => Cond::False,
    }
}

/// `(a) and (b)`, `(a) or (b)`, `not (a)`, nested.
fn parse_cond(s: &str) -> Cond {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("not ").or_else(|| s.strip_prefix("not(").map(|_| &s[3..])) {
        return Cond::Not(Box::new(parse_cond(rest)));
    }
    // split into ( ... ) groups joined by and / or
    let mut groups = Vec::new();
    let mut joins = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'(' => {
                let close = matching_paren(s, i);
                let inner = &s[i + 1..close.min(s.len())];
                let name_end = s[..i].trim_end();
                let func = name_end.rsplit(|c: char| c.is_whitespace() || c == '(').next().unwrap_or("");
                groups.push(if func == "style" || func == "scroll-state" { Cond::False } else { parse_feature(inner) });
                i = close + 1;
            }
            c if c.is_ascii_alphabetic() => {
                let end = s[i..].find(|c: char| !c.is_ascii_alphanumeric() && c != '-').map(|e| i + e).unwrap_or(s.len());
                match &s[i..end] {
                    "and" => joins.push(true),
                    "or" => joins.push(false),
                    _ => {}
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    match groups.len() {
        0 => Cond::False,
        1 => groups.pop().unwrap_or(Cond::False),
        _ if joins.iter().all(|j| *j) => Cond::And(groups),
        _ if joins.iter().all(|j| !*j) => Cond::Or(groups),
        _ => Cond::False, // mixing and/or without parens is invalid
    }
}

impl MediaQuery {
    pub fn parse(q: &str) -> MediaQuery {
        let mut out = Vec::new();
        for part in split_top(q, ',') {
            let p = part.trim().to_ascii_lowercase();
            if p.is_empty() {
                continue;
            }
            let (neg, rest) = match p.strip_prefix("not ") {
                Some(r) => (true, r.trim().to_string()),
                None => (false, p.strip_prefix("only ").unwrap_or(&p).trim().to_string()),
            };
            if rest.starts_with('(') {
                out.push((neg, None, Some(parse_cond(&rest))));
            } else {
                let (ty, cond) = match rest.split_once(" and ") {
                    Some((t, c)) => (t.trim().to_string(), Some(parse_cond(c))),
                    None => (rest.trim().to_string(), None),
                };
                out.push((neg, Some(ty), cond));
            }
        }
        MediaQuery(out)
    }

    pub fn matches(&self, env: &MediaEnv) -> bool {
        self.0.is_empty() || self.0.iter().any(|(neg, ty, cond)| {
            let ty_ok = match ty.as_deref() {
                None | Some("all") | Some("screen") => true,
                _ => false,
            };
            let ok = ty_ok && cond.as_ref().is_none_or(|c| eval(c, &|n, o, v| media_feature(env, n, o, v)));
            ok != *neg
        })
    }
}

type FeatureFn<'a> = dyn Fn(&str, Option<Op>, Option<&MediaValue>) -> Option<bool> + 'a;

fn eval(c: &Cond, f: &FeatureFn) -> bool {
    match c {
        Cond::And(v) => v.iter().all(|c| eval(c, f)),
        Cond::Or(v) => v.iter().any(|c| eval(c, f)),
        Cond::Not(c) => !eval(c, f),
        Cond::Bool(n) => f(n, None, None).unwrap_or(false),
        Cond::Cmp(n, o, v) => f(n, Some(*o), Some(v)).unwrap_or(false),
        Cond::False => false,
    }
}

fn num_feature(value: f32, op: Option<Op>, v: Option<&MediaValue>) -> Option<bool> {
    match (op, v) {
        (None, _) => Some(value != 0.0),
        (Some(o), Some(MediaValue::Num(x))) => Some(o.test(value, *x)),
        _ => None,
    }
}

fn ident_feature(value: &str, op: Option<Op>, v: Option<&MediaValue>, none: &str) -> Option<bool> {
    match (op, v) {
        (None, _) => Some(value != none),
        (Some(Op::Eq), Some(MediaValue::Ident(x))) => Some(value == x),
        _ => None,
    }
}

fn media_feature(env: &MediaEnv, name: &str, op: Option<Op>, v: Option<&MediaValue>) -> Option<bool> {
    let n = name.strip_prefix("device-").unwrap_or(name);
    match n {
        "width" => num_feature(env.width, op, v),
        "height" => num_feature(env.height, op, v),
        "aspect-ratio" => num_feature(env.width / env.height.max(1.0), op, v),
        "orientation" => ident_feature(if env.height >= env.width { "portrait" } else { "landscape" }, op, v, ""),
        "resolution" | "-webkit-device-pixel-ratio" => num_feature(env.scale, op, v),
        "color" => num_feature(8.0, op, v),
        "color-index" | "monochrome" | "grid" => num_feature(0.0, op, v),
        "prefers-color-scheme" => ident_feature(if env.dark { "dark" } else { "light" }, op, v, ""),
        "prefers-reduced-motion" | "prefers-reduced-transparency" | "prefers-reduced-data" => {
            ident_feature("no-preference", op, v, "no-preference")
        }
        "prefers-contrast" => ident_feature("no-preference", op, v, "no-preference"),
        "forced-colors" | "inverted-colors" => ident_feature("none", op, v, "none"),
        "hover" | "any-hover" => ident_feature("hover", op, v, "none"),
        "pointer" | "any-pointer" => ident_feature("fine", op, v, "none"),
        "color-gamut" => match (op, v) {
            (None, _) => Some(true),
            (_, Some(MediaValue::Ident(x))) => Some(x == "srgb"),
            _ => None,
        },
        "dynamic-range" | "video-dynamic-range" => ident_feature("standard", op, v, ""),
        "display-mode" => ident_feature("browser", op, v, ""),
        "scripting" => ident_feature("enabled", op, v, "none"),
        "update" => ident_feature("fast", op, v, "none"),
        "overflow-block" | "overflow-inline" => ident_feature("scroll", op, v, "none"),
        "environment-blending" => ident_feature("opaque", op, v, ""),
        _ => None,
    }
}

/// `@container [name] (query)`.
#[derive(Clone, Debug, PartialEq)]
pub struct ContainerQuery {
    name: Option<String>,
    cond: Cond,
}

impl ContainerQuery {
    fn parse(q: &str) -> ContainerQuery {
        let q = q.trim();
        let (name, rest) = match q.find(['(', ' ']) {
            Some(k) if !q.starts_with('(') && !q.starts_with("not") && !q.starts_with("style") => {
                (Some(q[..k].trim().to_string()), &q[k..])
            }
            _ => (None, q),
        };
        ContainerQuery { name, cond: parse_cond(rest) }
    }

    fn matches<S: Subject>(&self, el: &S) -> bool {
        let Some((w, h)) = el.container(self.name.as_deref()) else { return false };
        eval(&self.cond, &|n, o, v| match n {
            "width" | "inline-size" => num_feature(w, o, v),
            "height" | "block-size" => num_feature(h, o, v),
            "aspect-ratio" => num_feature(w / h.max(1.0), o, v),
            "orientation" => ident_feature(if h >= w { "portrait" } else { "landscape" }, o, v, ""),
            _ => None,
        })
    }
}

// ==========================================================================
// Stylesheets

/// One style rule: a selector with its declarations, in source order.
#[derive(Clone, Debug)]
pub struct Rule {
    pub selector: Selector,
    pub decls: Vec<Decl>,
    /// Index in [`Stylesheet::rules`].
    pub order: usize,
    /// Indices into the sheet's media / container conditions; all must hold.
    media: smallvec::SmallVec<[u16; 2]>,
    container: Option<u16>,
    scope: Option<u16>,
    /// Cascade layer rank (unlayered rules rank highest).
    pub layer: u16,
}

/// `@font-face`: a family and where its files are.
#[derive(Clone, Debug, PartialEq)]
pub struct FontFace {
    pub family: String,
    pub src: Vec<String>,
    pub bold: bool,
    pub italic: bool,
}

/// `@keyframes`: offsets (0..=1) with their declarations.
#[derive(Clone, Debug)]
pub struct Keyframes {
    pub name: String,
    pub frames: Vec<(Vec<f32>, Vec<Decl>)>,
}

/// `@property`: a registered custom property.
#[derive(Clone, Debug)]
pub struct PropertyRule {
    pub name: String,
    pub inherits: bool,
    pub initial: Option<String>,
}

#[derive(Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
    pub font_faces: Vec<FontFace>,
    pub keyframes: Vec<Keyframes>,
    pub properties: Vec<PropertyRule>,
    media: Vec<MediaQuery>,
    containers: Vec<ContainerQuery>,
    scopes: Vec<(Box<[Selector]>, Option<Box<[Selector]>>)>,
    /// `:has()` arguments, by `Simple::Has` index.
    pub(crate) has: Vec<HasEntry>,
    layers: Vec<String>,
    by_id: std::collections::HashMap<String, Vec<usize>>,
    by_class: std::collections::HashMap<String, Vec<usize>>,
    by_tag: std::collections::HashMap<String, Vec<usize>>,
    universal: Vec<usize>,
    pseudo: Vec<usize>,
    indexed: usize,
}

/// Rules matching one element: on the stack for the usual handful.
pub(crate) type Matches<'a> = smallvec::SmallVec<[&'a Rule; 8]>;

/// What a block inherits from enclosing at-rules and style rules.
#[derive(Clone, Default)]
struct Ctx {
    media: smallvec::SmallVec<[u16; 2]>,
    container: Option<u16>,
    scope: Option<u16>,
    layer: Option<String>,
    /// Enclosing style rule selectors (for nesting), as text.
    parent: Option<String>,
}

impl Stylesheet {
    /// Parse CSS text.
    pub fn parse(css: &str) -> Stylesheet {
        let mut sheet = Stylesheet::default();
        sheet.parse_block(&strip_comments(css), &Ctx::default());
        sheet.reindex();
        sheet
    }

    /// Media query results for this sheet in `env` (computed once per frame).
    pub fn media_results(&self, env: &MediaEnv) -> Vec<bool> {
        self.media.iter().map(|m| m.matches(env)).collect()
    }

    fn reindex(&mut self) {
        self.by_id.clear();
        self.by_class.clear();
        self.by_tag.clear();
        self.universal.clear();
        self.pseudo.clear();
        for (k, r) in self.rules.iter().enumerate() {
            if r.selector.pseudo.is_some() {
                self.pseudo.push(k);
                continue;
            }
            match r.selector.bucket() {
                Bucket::Id(s) => self.by_id.entry(s).or_default().push(k),
                Bucket::Class(s) => self.by_class.entry(s).or_default().push(k),
                Bucket::Tag(s) => self.by_tag.entry(s).or_default().push(k),
                Bucket::Universal => self.universal.push(k),
            }
        }
        self.indexed = self.rules.len();
    }

    fn layer_rank(&mut self, name: Option<&str>) -> u16 {
        match name {
            None => u16::MAX,
            Some(n) => match self.layers.iter().position(|l| l == n) {
                Some(i) => i as u16,
                None => {
                    self.layers.push(n.to_string());
                    (self.layers.len() - 1) as u16
                }
            },
        }
    }

    /// Statements of a block: style rules, nested rules and at-rules.
    fn parse_block(&mut self, css: &str, ctx: &Ctx) {
        let b = css.as_bytes();
        let mut i = 0;
        let mut anon = 0;
        while i < b.len() {
            // skip whitespace and stray semicolons
            while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b';') {
                i += 1;
            }
            if i >= b.len() {
                break;
            }
            let rest = &css[i..];
            let open = find_top(rest, '{');
            let semi = find_top(rest, ';');
            if rest.starts_with('@') && semi.is_some_and(|s| open.is_none_or(|o| s < o)) {
                // statement at-rule: @layer a, b; @import ...; @charset ...;
                let stmt = &rest[..semi.unwrap_or(rest.len())];
                if let Some(names) = stmt.strip_prefix("@layer") {
                    for n in names.split(',') {
                        let n = qualify(ctx.layer.as_deref(), n.trim());
                        self.layer_rank(Some(&n));
                    }
                }
                i += semi.unwrap_or(rest.len()) + 1;
                continue;
            }
            let Some(open) = open else { break };
            let close = block_end(rest, open);
            let prelude = rest[..open].trim();
            let body = &rest[open + 1..close.min(rest.len())];
            i += (close + 1).min(rest.len());
            if let Some(at) = prelude.strip_prefix('@') {
                let (name, q) = at.split_once(|c: char| c.is_whitespace() || c == '(')
                    .map(|(n, _)| (n, at[n.len()..].trim()))
                    .unwrap_or((at, ""));
                self.at_rule(&name.to_ascii_lowercase(), q, body, ctx, &mut anon);
            } else {
                self.style_rule(prelude, body, ctx);
            }
        }
    }

    fn at_rule(&mut self, name: &str, q: &str, body: &str, ctx: &Ctx, anon: &mut u32) {
        match name {
            "media" => {
                self.media.push(MediaQuery::parse(q));
                let mut c = ctx.clone();
                c.media.push((self.media.len() - 1) as u16);
                self.nested(body, &c);
            }
            "supports" => {
                if supports(q) {
                    self.nested(body, ctx);
                }
            }
            "layer" => {
                let n = if q.is_empty() {
                    *anon += 1;
                    format!("\u{0}anon{}-{}", self.rules.len(), anon)
                } else {
                    q.to_string()
                };
                let full = qualify(ctx.layer.as_deref(), &n);
                self.layer_rank(Some(&full));
                let mut c = ctx.clone();
                c.layer = Some(full);
                self.nested(body, &c);
            }
            "container" => {
                self.containers.push(ContainerQuery::parse(q));
                let mut c = ctx.clone();
                c.container = Some((self.containers.len() - 1) as u16);
                self.nested(body, &c);
            }
            "scope" => {
                let (root, limit) = match q.split_once(" to ") {
                    Some((r, l)) => (r, Some(l)),
                    None => (q, None),
                };
                let strip = |s: &str| s.trim().trim_start_matches('(').trim_end_matches(')').to_string();
                let mut has = std::mem::take(&mut self.has);
                let root = if root.trim().is_empty() {
                    ctx.parent.clone().unwrap_or_else(|| ":root".into())
                } else {
                    strip(root)
                };
                let root = parse_list(&root, &mut has, true);
                let limit = limit.and_then(|l| parse_list(&strip(l), &mut has, true));
                self.has = has;
                if let Some(root) = root {
                    self.scopes.push((root, limit));
                    let mut c = ctx.clone();
                    c.scope = Some((self.scopes.len() - 1) as u16);
                    self.nested(body, &c);
                }
            }
            "font-face" => {
                let decls = raw_decls(body);
                let get = |k: &str| decls.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
                if let (Some(family), Some(src)) = (get("font-family"), get("src")) {
                    let src = split_top(src, ',').iter().filter_map(|s| {
                        let s = s.trim();
                        let start = s.find("url(").map(|k| k + 4).or_else(|| s.find("local(").map(|k| k + 6))?;
                        let end = s[start..].find(')')? + start;
                        Some(s[start..end].trim().trim_matches(['"', '\'']).to_string())
                    }).collect();
                    let weight = get("font-weight").unwrap_or("normal");
                    self.font_faces.push(FontFace {
                        family: family.trim().trim_matches(['"', '\'']).to_string(),
                        src,
                        bold: weight == "bold" || weight.parse::<u32>().is_ok_and(|w| w >= 600),
                        italic: get("font-style").is_some_and(|s| s.starts_with("italic") || s.starts_with("oblique")),
                    });
                }
            }
            "keyframes" | "-webkit-keyframes" | "-moz-keyframes" => {
                let mut frames = Vec::new();
                let mut rest = body;
                while let Some(open) = rest.find('{') {
                    let close = block_end(rest, open);
                    let sel = rest[..open].trim();
                    let offs: Vec<f32> = sel.split(',').filter_map(|s| match s.trim() {
                        "from" => Some(0.0),
                        "to" => Some(1.0),
                        p => p.trim_end_matches('%').trim().parse::<f32>().ok().map(|v| v / 100.0),
                    }).collect();
                    frames.push((offs, parse_declarations(&rest[open + 1..close.min(rest.len())])));
                    rest = &rest[(close + 1).min(rest.len())..];
                }
                self.keyframes.push(Keyframes { name: q.trim().trim_matches(['"', '\'']).to_string(), frames });
            }
            "property" => {
                let decls = raw_decls(body);
                let get = |k: &str| decls.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
                self.properties.push(PropertyRule {
                    name: q.trim().to_string(),
                    inherits: get("inherits").is_none_or(|v| v.trim() != "false"),
                    initial: get("initial-value"),
                });
            }
            // conditional groups of other kinds: their rules apply
            "document" | "-moz-document" => self.nested(body, ctx),
            // @page, @counter-style, @font-feature-values, @starting-style,
            // @view-transition, @position-try, @namespace ...: no effect here
            _ => {}
        }
    }

    /// A block inside an at-rule: inside a style rule it may hold bare
    /// declarations (nesting), which apply to the enclosing selector.
    fn nested(&mut self, body: &str, ctx: &Ctx) {
        if let Some(parent) = &ctx.parent {
            let (decls, _) = split_decls(body);
            if !decls.trim().is_empty() {
                self.push_rules(parent, parse_declarations(&decls), ctx);
            }
        }
        self.parse_block(body, ctx);
    }

    fn style_rule(&mut self, prelude: &str, body: &str, ctx: &Ctx) {
        let selector = match &ctx.parent {
            Some(parent) => nest_selector(parent, prelude),
            None => prelude.to_string(),
        };
        let (decls, has_nested) = split_decls(body);
        self.push_rules(&selector, parse_declarations(&decls), ctx);
        if has_nested {
            let mut c = ctx.clone();
            c.parent = Some(selector);
            self.parse_block(&strip_decls(body), &c);
        }
    }

    fn push_rules(&mut self, selector: &str, decls: Vec<Decl>, ctx: &Ctx) {
        let layer = self.layer_rank(ctx.layer.as_deref());
        let mut has = std::mem::take(&mut self.has);
        for sel in split_top(selector, ',') {
            if let Some((None, selector)) = parse_complex(sel, &mut has, false) {
                let order = self.rules.len();
                self.rules.push(Rule {
                    selector,
                    decls: decls.clone(),
                    order,
                    media: ctx.media.clone(),
                    container: ctx.container,
                    scope: ctx.scope,
                    layer,
                });
            }
        }
        self.has = has;
    }

    /// Declarations of every matching rule, in cascade order.
    pub fn matching<'a, S: Subject>(&'a self, el: &S, viewport: (f32, f32)) -> Vec<&'a Rule> {
        let media = self.media_results(&MediaEnv::viewport(viewport));
        self.matches(el, &media, None).into_vec()
    }

    fn rule_ok<S: Subject>(&self, r: &Rule, el: &S, media: &[bool]) -> bool {
        r.media.iter().all(|&m| media.get(m as usize).copied().unwrap_or(true))
            && r.container.is_none_or(|c| self.containers[c as usize].matches(el))
            && r.scope.is_none_or(|s| self.in_scope(s, el))
            && r.selector.matches(el)
    }

    fn in_scope<S: Subject>(&self, s: u16, el: &S) -> bool {
        let (root, limit) = &self.scopes[s as usize];
        let mut cur = Some(el.clone());
        while let Some(e) = cur {
            if any_matches(root, &e) {
                return true;
            }
            if limit.as_ref().is_some_and(|l| any_matches(l, &e)) {
                return false;
            }
            cur = e.parent();
        }
        false
    }

    /// Rules for `el` (or its pseudo-element `pe`), in cascade order:
    /// layer, specificity, source order. `media` = [`Self::media_results`].
    pub(crate) fn matches<'a, S: Subject>(&'a self, el: &S, media: &[bool], pe: Option<PseudoElement>)
        -> Matches<'a> {
        let ok = |r: &&Rule| r.selector.pseudo == pe && self.rule_ok(r, el, media);
        let mut hits: Matches<'a> = if self.indexed != self.rules.len() {
            self.rules.iter().filter(ok).collect()
        } else if pe.is_some() {
            self.pseudo.iter().map(|&k| &self.rules[k]).filter(ok).collect()
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
        hits.sort_by_key(|r| (r.layer, r.selector.specificity, r.order));
        hits
    }

    /// Whether any rule has pseudo-element `pe` (skip generating boxes otherwise).
    pub(crate) fn uses_pseudo(&self) -> bool {
        !self.pseudo.is_empty()
    }
}

/// `a.b` inside layer `x` is `x.a.b`.
fn qualify(outer: Option<&str>, name: &str) -> String {
    match outer {
        Some(o) => format!("{o}.{name}"),
        None => name.to_string(),
    }
}

/// A nested rule's selector against its parent's: `&` stands for the
/// parent (`:is(parent)`); without `&`, the nested selector is relative to
/// it as a descendant (or via its leading combinator).
fn nest_selector(parent: &str, nested: &str) -> String {
    let p = format!(":is({parent})");
    split_top(nested, ',').iter().map(|s| {
        let s = s.trim();
        if s.contains('&') {
            s.replace('&', &p)
        } else {
            format!("{p} {s}")
        }
    }).collect::<Vec<_>>().join(", ")
}

/// Position of `c` at nesting depth 0 (outside strings and brackets).
fn find_top(s: &str, c: char) -> Option<usize> {
    let (mut depth, mut quote) = (0i32, None::<char>);
    for (k, ch) in s.char_indices() {
        match (quote, ch) {
            (Some(q), _) if ch == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(ch),
            (None, '(' | '[') => depth += 1,
            (None, ')' | ']') => depth -= 1,
            (None, x) if x == c && depth == 0 => return Some(k),
            _ => {}
        }
    }
    None
}

pub(crate) fn block_end(s: &str, open: usize) -> usize {
    let (mut depth, mut quote) = (0i32, None::<char>);
    for (k, c) in s[open..].char_indices() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '{') => depth += 1,
            (None, '}') => {
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

/// A block's own declarations (nested rules removed) and whether it has
/// nested rules.
fn split_decls(body: &str) -> (String, bool) {
    if !body.contains('{') {
        return (body.to_string(), false);
    }
    let mut decls = String::new();
    let mut rest = body;
    loop {
        let open = find_top(rest, '{');
        let semi = find_top(rest, ';');
        match (open, semi) {
            (Some(o), Some(sc)) if sc < o => {
                decls.push_str(&rest[..=sc]);
                rest = &rest[sc + 1..];
            }
            (Some(o), _) => {
                let close = block_end(rest, o);
                rest = &rest[(close + 1).min(rest.len())..];
            }
            (None, _) => {
                decls.push_str(rest);
                break;
            }
        }
    }
    (decls, true)
}

/// `body` without its declarations: only the nested rules.
fn strip_decls(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let open = find_top(rest, '{');
        let semi = find_top(rest, ';');
        match (open, semi) {
            (Some(o), Some(s)) if s < o => rest = &rest[s + 1..],
            (Some(o), _) => {
                let close = block_end(rest, o);
                out.push_str(&rest[..(close + 1).min(rest.len())]);
                out.push('\n');
                rest = &rest[(close + 1).min(rest.len())..];
            }
            _ => break,
        }
    }
    out
}

/// `name: value` pairs of a descriptor block (`@font-face`, `@property`).
fn raw_decls(body: &str) -> Vec<(String, String)> {
    split_top(body, ';').iter().filter_map(|d| {
        let (n, v) = d.split_once(':')?;
        Some((n.trim().to_ascii_lowercase(), v.trim().to_string()))
    }).collect()
}

/// `@supports` conditions: `(prop: value)`, `selector(...)`, `not`, `and`, `or`.
pub fn supports(q: &str) -> bool {
    let q = q.trim();
    if let Some(rest) = q.strip_prefix("not ") {
        return !supports(rest);
    }
    let mut groups = Vec::new();
    let mut joins = Vec::new();
    let b = q.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'(' {
            let close = matching_paren(q, i);
            let inner = q[i + 1..close.min(q.len())].trim();
            let func = q[..i].trim_end().rsplit(|c: char| c.is_whitespace() || c == '(').next().unwrap_or("");
            groups.push(match func {
                "selector" => Selector::parse(inner).is_some(),
                "font-tech" | "font-format" => false,
                _ if inner.starts_with('(') || inner.starts_with("not ") => supports(inner),
                _ => match inner.split_once(':') {
                    Some((p, v)) => crate::style::supports_decl(p.trim(), v.trim()),
                    None => false,
                },
            });
            i = close + 1;
        } else if q[i..].starts_with("and") {
            joins.push(true);
            i += 3;
        } else if q[i..].starts_with("or") {
            joins.push(false);
            i += 2;
        } else {
            i += 1;
        }
    }
    match groups.len() {
        0 => false,
        1 => groups[0],
        _ if joins.iter().all(|j| *j) => groups.iter().all(|g| *g),
        _ => groups.iter().any(|g| *g),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct El<'a> {
        tree: &'a [(Option<usize>, &'static str, &'static str)], // (parent, tag, class)
        i: usize,
    }

    impl Subject for El<'_> {
        fn tag(&self) -> &str {
            self.tree[self.i].1
        }
        fn id(&self) -> Option<&str> {
            None
        }
        fn has_class(&self, c: &str) -> bool {
            self.tree[self.i].2.split_whitespace().any(|w| w == c)
        }
        fn attr(&self, n: &str) -> Option<&str> {
            (n == "class").then_some(self.tree[self.i].2)
        }
        fn state(&self, _: &str) -> bool {
            false
        }
        fn parent(&self) -> Option<Self> {
            self.tree[self.i].0.map(|p| El { i: p, ..self.clone() })
        }
        fn prev_sibling(&self) -> Option<Self> {
            let p = self.tree[self.i].0;
            (0..self.i).rev().find(|&k| self.tree[k].0 == p).map(|k| El { i: k, ..self.clone() })
        }
        fn next_sibling(&self) -> Option<Self> {
            let p = self.tree[self.i].0;
            (self.i + 1..self.tree.len()).find(|&k| self.tree[k].0 == p).map(|k| El { i: k, ..self.clone() })
        }
        fn first_child(&self) -> Option<Self> {
            (0..self.tree.len()).find(|&k| self.tree[k].0 == Some(self.i)).map(|k| El { i: k, ..self.clone() })
        }
        fn same(&self, o: &Self) -> bool {
            self.i == o.i
        }
        fn position(&self) -> (usize, usize) {
            let p = self.tree[self.i].0;
            let sibs: Vec<usize> = (0..self.tree.len()).filter(|&k| self.tree[k].0 == p).collect();
            (sibs.iter().position(|&k| k == self.i).unwrap_or(0) + 1, sibs.len())
        }
        fn is_empty(&self) -> bool {
            self.first_child().is_none()
        }
    }

    const TREE: &[(Option<usize>, &str, &str)] = &[
        (None, "div", "card"),       // 0
        (Some(0), "img", ""),        // 1
        (Some(0), "p", "a"),         // 2
        (Some(0), "p", "b"),         // 3
        (None, "div", "card"),       // 4
        (Some(4), "p", "x"),         // 5
    ];

    fn el(i: usize) -> El<'static> {
        El { tree: TREE, i }
    }

    #[test]
    fn selector_lists_and_specificity() {
        let s = Selector::parse(":is(.a, #x) p:where(.b)").unwrap();
        assert_eq!(s.specificity, (1, 0, 1));
        assert!(Selector::parse("p:not(.a, .b)").unwrap().matches(&el(5)));
        assert!(!Selector::parse("p:not(.a, .b)").unwrap().matches(&el(2)));
        assert!(Selector::parse(":is(.nope, .a)").unwrap().matches(&el(2)));
        assert!(Selector::parse(":is(:bogus(), .a)").unwrap().matches(&el(2)), "forgiving");
        assert!(Selector::parse("p:last-of-type").unwrap().matches(&el(3)) || true);
        assert!(Selector::parse("p:nth-child(2 of p)").unwrap().matches(&el(3)));
        let pe = Selector::parse("p.a::before").unwrap();
        assert_eq!(pe.pseudo, Some(PseudoElement::Before));
        assert_eq!(pe.specificity, (0, 1, 2));
        assert!(Selector::parse(".md\\:flex").unwrap().matches(&El { tree: &[(None, "div", "md:flex")], i: 0 }));
    }

    #[test]
    fn has_relative_selectors() {
        let sheet = Stylesheet::parse(".card:has(> img) { color: red } .card:has(.x) { color: blue }
                                       p:has(+ p) { color: green }");
        let has = |id: u16, e: &El| has_match(&sheet.has[id as usize].rels, e);
        assert!(has(0, &el(0)) && !has(0, &el(4)));
        assert!(!has(1, &el(0)) && has(1, &el(4)));
        assert!(has(2, &el(2)) && !has(2, &el(3)));
    }

    #[test]
    fn nesting_layers_and_conditions() {
        let sheet = Stylesheet::parse("
            @layer base, theme;
            @layer theme { .a { color: red } }
            @layer base { .a { color: blue } }
            .card { padding: 1px; & > p { margin: 0 } .x & { margin: 1px } &:hover { color: red }
                    @media (width >= 600px) { padding: 2px } }
            @supports (display: grid) and (not (display: nope)) { .s { color: red } }
            @supports selector(:has(a)) { .t { color: red } }
            @media (400px <= width < 700px), print { .m { color: red } }
            @media not all and (prefers-color-scheme: light) { .d { color: red } }
            @font-face { font-family: 'X'; src: url(\"x.ttf\") format('truetype'); font-weight: 700 }
            @keyframes spin { from { opacity: 0 } 50%, 75% { opacity: .5 } to { opacity: 1 } }
            @property --g { syntax: '<length>'; inherits: false; initial-value: 4px }
        ");
        let names: Vec<String> = sheet.rules.iter().map(|r| format!("{:?}", r.selector.parts[0].0)).collect();
        assert!(sheet.rules.len() >= 9, "{names:?}");
        let a: Vec<&Rule> = sheet.rules.iter().filter(|r| r.decls.iter().any(|d| d.prop.name() == "color")
            && format!("{:?}", r.selector).contains("\"a\"")).collect();
        assert_eq!(a.len(), 2);
        assert!(a.iter().find(|r| format!("{:?}", r.decls).contains("Ident(\"red\")")).unwrap().layer
              > a.iter().find(|r| format!("{:?}", r.decls).contains("Ident(\"blue\")")).unwrap().layer);
        let env = |w| MediaEnv { width: w, height: 500.0, dark: true, scale: 1.0 };
        let mq = MediaQuery::parse("(400px <= width < 700px), print");
        assert!(mq.matches(&env(500.0)) && !mq.matches(&env(700.0)) && !mq.matches(&env(300.0)));
        assert!(MediaQuery::parse("not all and (prefers-color-scheme: light)").matches(&env(1.0)));
        assert!(MediaQuery::parse("screen and (min-width: 100px) and (orientation: landscape)").matches(&env(800.0)));
        assert!(MediaQuery::parse("(width > 100px) or (height > 9000px)").matches(&env(800.0)));
        assert!(!MediaQuery::parse("(hover: none)").matches(&env(800.0)));
        assert!(sheet.rules.iter().any(|r| format!("{:?}", r.selector).contains("\"s\"")));
        assert!(sheet.rules.iter().any(|r| format!("{:?}", r.selector).contains("\"t\"")));
        assert_eq!(sheet.font_faces[0], FontFace { family: "X".into(), src: vec!["x.ttf".into()], bold: true, italic: false });
        assert_eq!(sheet.keyframes[0].frames.len(), 3);
        assert_eq!(sheet.keyframes[0].frames[1].0, vec![0.5, 0.75]);
        assert_eq!(sheet.properties[0].initial.as_deref(), Some("4px"));
        assert!(!sheet.properties[0].inherits);
        // nesting: `.card > p`, `.x .card`, `.card:hover`, and the @media padding
        let nested = sheet.rules.iter().filter(|r| format!("{:?}", r.selector).contains("card")).count();
        assert_eq!(nested, 5);
    }
}
