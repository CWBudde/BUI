//! Text measuring and wrapping with the kit's font engine (the same one that
//! draws, so layout and paint agree).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::style::Computed;
use blenderui_sys as sys;

/// Font flags for a style (bold / italic; decorations are painted as lines).
pub(crate) fn flags(c: &Computed) -> i32 {
    (if c.bold() { sys::BUI_FONT_BOLD } else { 0 }) | (if c.italic { sys::BUI_FONT_ITALIC } else { 0 })
}

type Widths = crate::style::FnvMap<Box<str>, f32>;

thread_local! {
    /// (size bits, flags) -> text -> width; looked up by `&str`, no allocation.
    static CACHE: RefCell<HashMap<(u32, i32), Widths>> = RefCell::new(HashMap::new());
}

/// Advance width of `s` at `size` px.
pub(crate) fn width(s: &str, size: f32, flags: i32) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    let font = (size.to_bits(), flags);
    if let Some(w) = CACHE.with(|c| c.borrow().get(&font).and_then(|m| m.get(s)).copied()) {
        return w;
    }
    let mut w = 0.0f32;
    // SAFETY: pointers are valid for the call; "" selects the UI font.
    let ok = unsafe {
        sys::BUI_text_measure(c"".as_ptr(), flags, size, s.as_ptr().cast(), s.len() as i32, &mut w)
    };
    if ok == 0 {
        w = s.chars().count() as f32 * size * 0.55; // no native text engine
    }
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let m = c.entry(font).or_default();
        if m.len() > 20_000 {
            m.clear();
        }
        m.insert(s.into(), w);
    });
    w
}

/// Greedy word wrap of `s` into lines no wider than `max` (explicit newlines kept).
pub(crate) fn wrap(s: &str, size: f32, flags: i32, max: f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split(' ').filter(|w| !w.is_empty()) {
            let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
            if !line.is_empty() && width(&candidate, size, flags) > max {
                out.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        out.push(line);
    }
    out
}

/// `s` cut to fit `max` with a trailing ellipsis (`text-overflow: ellipsis`).
pub(crate) fn ellipsize(s: &str, size: f32, flags: i32, max: f32) -> String {
    if width(s, size, flags) <= max {
        return s.to_owned();
    }
    let chars: Vec<char> = s.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let t: String = chars[..mid].iter().collect::<String>() + "\u{2026}";
        if width(&t, size, flags) <= max {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + "\u{2026}"
}
