//! Custom properties and `var()` / `env()` substitution.
//!
//! A computed style holds a [`VarId`] into the style store's arena of
//! variable maps: children share their parent's map until they declare a
//! custom property of their own. Substitution works on tokens (no
//! re-parsing), including inside nested functions such as
//! `calc(var(--gap) * 2)` and `rgb(var(--r) 0 0)`.

use crate::css::{Prop, Tok};

/// Custom property values, sorted by property.
pub(crate) type VarMap = Box<[(Prop, Box<[Tok]>)]>;

/// Arena of variable maps (index 0 = no variables).
pub(crate) struct Vars {
    maps: Vec<VarMap>,
}

impl Default for Vars {
    fn default() -> Self {
        Vars { maps: vec![Box::new([])] }
    }
}

impl Vars {
    pub fn get(&self, id: u32) -> &[(Prop, Box<[Tok]>)] {
        &self.maps[id as usize]
    }

    pub fn add(&mut self, map: VarMap) -> u32 {
        self.maps.push(map);
        (self.maps.len() - 1) as u32
    }

    pub fn clear(&mut self) {
        self.maps.truncate(1);
    }
}

pub(crate) fn lookup<'a>(map: &'a [(Prop, Box<[Tok]>)], p: Prop) -> Option<&'a [Tok]> {
    map.binary_search_by_key(&p, |e| e.0).ok().map(|i| &*map[i].1)
}

/// Whether the tokens use `var()` or `env()` anywhere.
pub(crate) fn uses_vars(toks: &[Tok]) -> bool {
    toks.iter().any(|t| match t {
        Tok::Func(fx) => fx.name == "var" || fx.name == "env" || fx.args.iter().any(|a| uses_vars(a)),
        _ => false,
    })
}

/// `toks` with every `var()` / `env()` replaced. None when a reference
/// can't be resolved and has no fallback (the declaration is then invalid
/// at computed-value time, which acts like `unset`).
pub(crate) fn substitute(toks: &[Tok], map: &[(Prop, Box<[Tok]>)]) -> Option<Vec<Tok>> {
    let mut out = Vec::with_capacity(toks.len());
    subst_into(toks, map, &mut out, 0)?;
    Some(out)
}

fn subst_into(toks: &[Tok], map: &[(Prop, Box<[Tok]>)], out: &mut Vec<Tok>, depth: u32) -> Option<()> {
    if depth > 32 {
        return None; // a reference cycle
    }
    for t in toks {
        match t {
            Tok::Func(fx) if fx.name == "var" => {
                let first = fx.args.first()?;
                let name = first.first().and_then(Tok::ident)?;
                let value = lookup(map, Prop::custom(name));
                match value {
                    Some(v) if !v.is_empty() => subst_into(v, map, out, depth + 1)?,
                    _ => {
                        // fallback: everything after the first comma (commas included)
                        if fx.args.len() < 2 {
                            return None;
                        }
                        for (k, a) in fx.args[1..].iter().enumerate() {
                            if k > 0 {
                                out.push(Tok::Comma);
                            }
                            subst_into(a, map, out, depth + 1)?;
                        }
                    }
                }
            }
            Tok::Func(fx) if fx.name == "env" => {
                let name = fx.args.first().and_then(|a| a.first()).and_then(Tok::ident).unwrap_or("");
                match env_value(name) {
                    Some(v) => out.push(v),
                    None => {
                        let fb = fx.args.get(1)?;
                        subst_into(fb, map, out, depth + 1)?;
                    }
                }
            }
            Tok::Func(fx) if fx.args.iter().any(|a| uses_vars(a)) => {
                let mut args = Vec::with_capacity(fx.args.len());
                for a in &fx.args {
                    let mut v = Vec::with_capacity(a.len());
                    subst_into(a, map, &mut v, depth)?;
                    // a substituted value may carry commas: split them back into arguments
                    let mut cur = Vec::new();
                    for t in v {
                        if t == Tok::Comma {
                            args.push(std::mem::take(&mut cur));
                        } else {
                            cur.push(t);
                        }
                    }
                    args.push(cur);
                }
                out.push(Tok::Func(Box::new(crate::css::FuncTok { name: fx.name.clone(), args })));
            }
            other => out.push(other.clone()),
        }
    }
    Some(())
}

/// Environment variables of a desktop window: no insets, no keyboard.
fn env_value(name: &str) -> Option<Tok> {
    match name {
        "safe-area-inset-top" | "safe-area-inset-right" | "safe-area-inset-bottom"
        | "safe-area-inset-left" | "safe-area-max-inset-top" | "safe-area-max-inset-right"
        | "safe-area-max-inset-bottom" | "safe-area-max-inset-left" | "keyboard-inset-top"
        | "keyboard-inset-right" | "keyboard-inset-bottom" | "keyboard-inset-left"
        | "keyboard-inset-width" | "keyboard-inset-height" | "titlebar-area-x"
        | "titlebar-area-y" | "titlebar-area-height" => Some(Tok::Num(0.0)),
        "preferred-text-scale" => Some(Tok::Num(1.0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::tokenize;

    fn map(entries: &[(&str, &str)]) -> Vec<(Prop, Box<[Tok]>)> {
        let mut m: Vec<(Prop, Box<[Tok]>)> =
            entries.iter().map(|(k, v)| (Prop::custom(k), tokenize(v).into_boxed_slice())).collect();
        m.sort_by_key(|e| e.0);
        m
    }

    #[test]
    fn substitutes_nested_and_fallbacks() {
        let m = map(&[("--gap", "8px"), ("--c", "1, 2, 3"), ("--alias", "var(--gap)")]);
        assert_eq!(substitute(&tokenize("var(--gap) 4px"), &m).unwrap(), tokenize("8px 4px"));
        assert_eq!(substitute(&tokenize("calc(var(--alias) * 2)"), &m).unwrap(), tokenize("calc(8px * 2)"));
        assert_eq!(substitute(&tokenize("rgb(var(--c))"), &m).unwrap(), tokenize("rgb(1, 2, 3)"));
        assert_eq!(substitute(&tokenize("var(--nope, 1px 2px)"), &m).unwrap(), tokenize("1px 2px"));
        assert_eq!(substitute(&tokenize("var(--nope, var(--gap))"), &m).unwrap(), tokenize("8px"));
        assert!(substitute(&tokenize("var(--nope)"), &m).is_none());
        assert_eq!(substitute(&tokenize("env(safe-area-inset-top, 9px)"), &m).unwrap(), vec![Tok::Num(0.0)]);
    }

    #[test]
    fn cycles_fail() {
        let m = map(&[("--a", "var(--b)"), ("--b", "var(--a)")]);
        assert!(substitute(&tokenize("var(--a)"), &m).is_none());
    }
}
