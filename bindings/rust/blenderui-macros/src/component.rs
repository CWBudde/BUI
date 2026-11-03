//! `#[component]`: a function becomes a component (zopra / Leptos style).
//!
//! ```ignore
//! #[component]
//! fn profile(username: &str, likes: u32) {
//!     let (n, set_n) = use_state(likes);
//!     rsx! { <span onClick={move || set_n(n() + 1)}>{username} " " {n()}</span> }
//! }
//! // used as <profile username="Alice" likes={3} />
//! ```
//!
//! Expands to a props struct named like the function, with an `IntoElement`
//! impl that runs the body in a component scope (hooks, context). `&str`
//! props are stored as `String`; the body still sees `&str`.

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

pub fn component(input: TokenStream) -> TokenStream {
    match expand(input) {
        Ok(ts) => ts,
        Err(e) => format!("::core::compile_error!({e:?});").parse().unwrap(),
    }
}

fn ts(s: &str) -> TokenStream {
    s.parse().expect("#[component]: scaffold")
}

fn expand(input: TokenStream) -> Result<TokenStream, String> {
    let toks: Vec<TokenTree> = input.into_iter().collect();
    let fn_at = toks.iter().position(|t| matches!(t, TokenTree::Ident(i) if i.to_string() == "fn"))
        .ok_or("#[component] goes on a function")?;
    let vis: TokenStream = toks[..fn_at].iter()
        .filter(|t| !matches!(t, TokenTree::Punct(p) if p.as_char() == '#'))
        .filter(|t| !matches!(t, TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket))
        .cloned().collect();
    let name = match toks.get(fn_at + 1) {
        Some(TokenTree::Ident(i)) => i.to_string(),
        _ => return Err("#[component]: expected a function name".into()),
    };
    let params = match toks.get(fn_at + 2) {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => g.stream(),
        _ => return Err("#[component]: generic components are not supported; use concrete prop types".into()),
    };
    let body = match toks.last() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => g.clone(),
        _ => return Err("#[component]: expected a body".into()),
    };
    // props: `name: Type` split at top-level commas (angle brackets count)
    let mut props: Vec<(String, TokenStream)> = Vec::new();
    let mut cur: Vec<TokenTree> = Vec::new();
    let mut depth = 0i32;
    let ptoks: Vec<TokenTree> = params.into_iter().collect();
    for (k, t) in ptoks.iter().enumerate() {
        match t {
            TokenTree::Punct(p) if p.as_char() == '<' => depth += 1,
            TokenTree::Punct(p) if p.as_char() == '>'
                && !matches!(k.checked_sub(1).and_then(|j| ptoks.get(j)), Some(TokenTree::Punct(q)) if q.as_char() == '-') => {
                depth -= 1
            }
            TokenTree::Punct(p) if p.as_char() == ',' && depth == 0 => {
                props.push(prop(&cur)?);
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(t.clone());
    }
    if !cur.is_empty() {
        props.push(prop(&cur)?);
    }
    let mut fields = TokenStream::new();
    let mut names = Vec::new();
    let mut shadows = TokenStream::new();
    let mut typed: Vec<(String, String)> = Vec::new();
    for (n, ty) in &props {
        let s = ty.to_string().replace(' ', "");
        let field_ty = if s == "&str" || s == "&'staticstr" {
            shadows.extend(ts(&format!("let {n}: &str = &{n};")));
            ts("::std::string::String")
        } else if s.starts_with('&') {
            return Err(format!("#[component]: prop `{n}` borrows (`{s}`); use an owned type"));
        } else {
            ty.clone()
        };
        fields.extend(ts(&format!("pub {n}:")));
        typed.push((n.clone(), field_ty.to_string()));
        fields.extend(field_ty);
        fields.extend(ts(","));
        names.push(n.clone());
    }
    let mut out = ts("#[allow(non_camel_case_types)]");
    out.extend(vis);
    out.extend(ts(&format!("struct {name}")));
    out.extend([TokenTree::Group(Group::new(Delimiter::Brace, fields))]);
    // impl IntoElement
    let mut inner = ts(&format!("let {name} {{ {} }} = self;", names.join(", ")));
    inner.extend(shadows);
    inner.extend(ts("::blenderui::hooks::component"));
    let mut closure = ts("|| ::blenderui::element::IntoElement::into_any_element");
    closure.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, body_expr(body)))]);
    inner.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, closure))]);
    let mut method = ts("#[allow(unused_braces)] fn into_any_element(self) -> ::blenderui::element::AnyElement");
    method.extend([TokenTree::Group(Group::new(Delimiter::Brace, inner))]);
    out.extend(ts(&format!("impl ::blenderui::element::IntoElement for {name}")));
    out.extend([TokenTree::Group(Group::new(Delimiter::Brace, method))]);
    out.extend(builder(&name, &vis_string(&toks[..fn_at]), &typed)?);
    Ok(out)
}

fn vis_string(toks: &[TokenTree]) -> String {
    let s: String = toks.iter().map(|t| t.to_string() + " ").collect();
    match s.find("pub") {
        Some(i) => {
            let rest = &s[i..];
            // `pub` or `pub (crate)`
            let end = if rest[3..].trim_start().starts_with('(') { rest.find(')').map_or(3, |j| j + 1) } else { 3 };
            rest[..end].to_string()
        }
        None => String::new(),
    }
}

/// `#[derive(Props)]` on a struct component: the same props builder.
pub fn derive_props(input: TokenStream) -> TokenStream {
    let r = (|| -> Result<TokenStream, String> {
        let toks: Vec<TokenTree> = input.into_iter().collect();
        let at = toks.iter().position(|t| matches!(t, TokenTree::Ident(i) if i.to_string() == "struct"))
            .ok_or("#[derive(Props)] goes on a struct")?;
        let name = match toks.get(at + 1) {
            Some(TokenTree::Ident(i)) => i.to_string(),
            _ => return Err("#[derive(Props)]: expected a struct name".into()),
        };
        let body = match toks.get(at + 2) {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => g.stream(),
            _ => return Err("#[derive(Props)]: needs named fields (and no generics)".into()),
        };
        let vis = vis_string(&toks[..at]);
        let mut fields = Vec::new();
        let mut cur: Vec<TokenTree> = Vec::new();
        let mut depth = 0i32;
        let btoks: Vec<TokenTree> = body.into_iter().collect();
        for (k, t) in btoks.iter().enumerate() {
            match t {
                TokenTree::Punct(p) if p.as_char() == '<' => depth += 1,
                TokenTree::Punct(p) if p.as_char() == '>'
                    && !matches!(k.checked_sub(1).and_then(|j| btoks.get(j)), Some(TokenTree::Punct(q)) if q.as_char() == '-') => {
                    depth -= 1
                }
                TokenTree::Punct(p) if p.as_char() == ',' && depth == 0 => {
                    field(&mut cur, &mut fields)?;
                    continue;
                }
                _ => {}
            }
            cur.push(t.clone());
        }
        field(&mut cur, &mut fields)?;
        builder(&name, &vis, &fields)
    })();
    r.unwrap_or_else(|e| format!("::core::compile_error!({e:?});").parse().unwrap())
}

/// One struct field (`#[attr] pub name: Type`) as (name, type).
fn field(cur: &mut Vec<TokenTree>, fields: &mut Vec<(String, String)>) -> Result<(), String> {
    let mut t: Vec<TokenTree> = Vec::new();
    let mut k = 0;
    while k < cur.len() {
        match &cur[k] {
            TokenTree::Punct(p) if p.as_char() == '#' => k += 2,
            TokenTree::Ident(i) if i.to_string() == "pub" => {
                k += 1;
                if matches!(cur.get(k), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis) {
                    k += 1;
                }
            }
            other => {
                t.push(other.clone());
                k += 1;
            }
        }
    }
    if !t.is_empty() {
        let (n, ty) = prop(&t)?;
        fields.push((n, ty.to_string()));
    }
    cur.clear();
    Ok(())
}

/// The inner type of `Option<T>` (any path to Option), else None.
fn option_inner(ty: &str) -> Option<String> {
    let squashed: String = ty.chars().filter(|c| !c.is_whitespace()).collect();
    let tail = squashed.strip_prefix("::").unwrap_or(&squashed);
    let tail = tail.strip_prefix("std::option::").or_else(|| tail.strip_prefix("core::option::")).unwrap_or(tail);
    if !tail.starts_with("Option<") || !tail.ends_with('>') {
        return None;
    }
    let t = ty.trim_end();
    let open = t.find('<')?;
    Some(t[open + 1..t.len() - 1].to_string())
}

/// A typestate props builder (`<Name>::__builder()`): `Option<T>` props and
/// `children` may be left out; leaving out any other prop is a compile
/// error naming it (`Name_missing_<prop>` in the builder's type).
fn builder(name: &str, vis: &str, fields: &[(String, String)]) -> Result<TokenStream, String> {
    enum Kind {
        Required(usize),
        Optional(String),
        Children,
    }
    let mut req = 0;
    let kinds: Vec<Kind> = fields.iter().map(|(n, ty)| {
        if let Some(inner) = option_inner(ty) {
            Kind::Optional(inner)
        } else if n == "children" {
            Kind::Children
        } else {
            req += 1;
            Kind::Required(req - 1)
        }
    }).collect();
    let b = format!("__{name}Props");
    let gens: Vec<String> = (0..req).map(|i| format!("__R{i}")).collect();
    let g = gens.join(", ");
    let missing: Vec<String> = fields.iter().zip(&kinds)
        .filter(|(_, k)| matches!(k, Kind::Required(_)))
        .map(|((n, _), _)| format!("{name}_missing_{n}")).collect();
    let pd = "::core::marker::PhantomData";
    // setters take `impl Into<T>` ("x" for a String prop), except numbers,
    // where a generic would stop integer literals from inferring
    let param = |ty: &str| {
        const NUM: &[&str] = &["u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64",
                               "i128", "isize", "f32", "f64", "bool", "char"];
        if NUM.contains(&ty.trim()) { ty.to_string() } else { format!("impl ::core::convert::Into<{ty}>") }
    };
    let mut out = String::new();
    for m in &missing {
        out += &format!("#[doc(hidden)] #[allow(non_camel_case_types)] {vis} struct {m};\n");
    }
    out += &format!("#[doc(hidden)] #[allow(non_camel_case_types, non_snake_case)] {vis} struct {b}<{g}> {{");
    for ((n, ty), k) in fields.iter().zip(&kinds) {
        match k {
            Kind::Required(_) => out += &format!("{n}: ::core::option::Option<{ty}>,"),
            _ => out += &format!("{n}: {ty},"),
        }
    }
    out += &format!("__s: {pd}<({g}{})>, }}\n", if req == 1 { "," } else { "" });
    let init: String = fields.iter().zip(&kinds).map(|((n, _), k)| match k {
        Kind::Required(_) => format!("{n}: ::core::option::Option::None,"),
        _ => format!("{n}: ::core::default::Default::default(),"),
    }).collect();
    out += &format!("#[allow(non_camel_case_types)] impl {name} {{ #[doc(hidden)] {vis} fn __builder() -> {b}<{}> {{ {b} {{ {init} __s: {pd} }} }} }}\n",
                    missing.join(", "));
    out += &format!("#[allow(non_camel_case_types, non_snake_case)] impl<{g}> {b}<{g}> {{");
    for ((n, ty), k) in fields.iter().zip(&kinds) {
        match k {
            Kind::Required(i) => {
                let to: Vec<String> = gens.iter().enumerate()
                    .map(|(j, x)| if j == *i { "::blenderui::hooks::Set".to_string() } else { x.clone() }).collect();
                let rest: String = fields.iter().filter(|(m, _)| m != n).map(|(m, _)| format!("{m}: self.{m},")).collect();
                out += &format!("pub fn {n}(self, v: {}) -> {b}<{}> {{ {b} {{ {n}: ::core::option::Option::Some(v.into()), {rest} __s: {pd} }} }}",
                                param(ty), to.join(", "));
            }
            Kind::Optional(inner) => {
                out += &format!("pub fn {n}(mut self, v: {}) -> Self {{ self.{n} = ::core::option::Option::Some(v.into()); self }}", param(inner));
            }
            Kind::Children => out += &format!("pub fn {n}(mut self, v: {ty}) -> Self {{ self.{n} = v; self }}"),
        }
    }
    out += "}\n";
    let set = vec!["::blenderui::hooks::Set"; req].join(", ");
    let fin: String = fields.iter().zip(&kinds).map(|((n, _), k)| match k {
        Kind::Required(_) => format!("{n}: match self.{n} {{ ::core::option::Option::Some(v) => v, ::core::option::Option::None => ::core::unreachable!() }},"),
        _ => format!("{n}: self.{n},"),
    }).collect();
    out += &format!("impl ::blenderui::hooks::Complete for {b}<{set}> {{ type Props = {name}; fn finish(self) -> {name} {{ {name} {{ {fin} }} }} }}\n");
    out.parse().map_err(|e| format!("#[component]: builder for {name}: {e:?}"))
}


/// The body as an expression: a block, whose last expression may also be
/// bare markup (zopra lets a component end in `<div>..</div>`).
fn body_expr(body: Group) -> TokenStream {
    TokenStream::from(TokenTree::Group(body))
}

fn prop(toks: &[TokenTree]) -> Result<(String, TokenStream), String> {
    let colon = toks.iter().position(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ':'))
        .ok_or("#[component]: props are `name: Type`")?;
    let name = toks[..colon].iter()
        .filter(|t| !matches!(t, TokenTree::Ident(i) if i.to_string() == "mut"))
        .map(|t| t.to_string()).collect::<String>();
    Ok((name, toks[colon + 1..].iter().cloned().collect()))
}
