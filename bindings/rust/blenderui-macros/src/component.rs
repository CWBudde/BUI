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
    let mut method = ts("fn into_any_element(self) -> ::blenderui::element::AnyElement");
    method.extend([TokenTree::Group(Group::new(Delimiter::Brace, inner))]);
    out.extend(ts(&format!("impl ::blenderui::element::IntoElement for {name}")));
    out.extend([TokenTree::Group(Group::new(Delimiter::Brace, method))]);
    Ok(out)
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
