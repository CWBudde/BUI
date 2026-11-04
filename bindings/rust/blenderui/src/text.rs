//! Text measuring and wrapping with the kit's font engine (the same one that
//! draws, so layout and paint agree).

use crate::style::Computed;
use blenderui_sys as sys;

/// Font flags for a style (bold / italic; decorations are painted as lines),
/// with the font family's id above [`FAMILY_SHIFT`].
pub(crate) fn flags(c: &Computed) -> i32 {
    (if c.bold() { sys::BUI_FONT_BOLD } else { 0 }) | (if c.italic { sys::BUI_FONT_ITALIC } else { 0 })
        | ((c.font_id as i32) << FAMILY_SHIFT)
}

/// Where lines may break inside words: `overflow-wrap: anywhere` /
/// `break-word` ([`BREAK_ANYWHERE`]) or `word-break: break-all` ([`BREAK_ALL`]).
pub(crate) fn break_mode(c: &Computed) -> u8 {
    if c.break_all { BREAK_ALL } else if c.wrap_anywhere { BREAK_ANYWHERE } else { 0 }
}

pub(crate) const BREAK_ANYWHERE: u8 = 1;
pub(crate) const BREAK_ALL: u8 = 2;

/// Font flags live in the low byte; the family id above it.
pub(crate) const FAMILY_SHIFT: i32 = 8;

/// Families by id (0 = the UI font, ""), leaked: canvas items point at them.
static FAMILIES: std::sync::Mutex<Vec<&'static std::ffi::CStr>> = std::sync::Mutex::new(Vec::new());
/// Families registered with `add_font` / `@font-face` (lowercase).
static REGISTERED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn register_family(name: &str) {
    let mut r = REGISTERED.lock().unwrap_or_else(|e| e.into_inner());
    let n = name.to_ascii_lowercase();
    if !r.contains(&n) {
        r.push(n);
    }
}

/// The id of a `font-family` list: its first registered family, else its
/// first generic one, else its first entry (the engine finds installed
/// fonts by name, and falls back to the UI font). 0 = the UI font.
pub(crate) fn font_id(list: &str) -> u16 {
    let names: Vec<String> = list.split(',').map(|n| n.trim().trim_matches(['"', '\'']).trim().to_string())
        .filter(|n| !n.is_empty()).collect();
    let registered = REGISTERED.lock().unwrap_or_else(|e| e.into_inner());
    let generic = |n: &str| match n.to_ascii_lowercase().as_str() {
        "sans-serif" | "system-ui" | "ui-sans-serif" | "-apple-system" | "blinkmacsystemfont" | "inter" => Some(""),
        "monospace" | "ui-monospace" => Some("monospace"),
        "serif" | "ui-serif" => Some("serif"),
        _ => None,
    };
    let pick = names.iter().find(|n| registered.contains(&n.to_ascii_lowercase())).cloned()
        .or_else(|| names.iter().find_map(|n| generic(n)).map(str::to_string))
        .or_else(|| names.first().cloned())
        .unwrap_or_default();
    drop(registered);
    if pick.is_empty() {
        return 0;
    }
    let mut f = FAMILIES.lock().unwrap_or_else(|e| e.into_inner());
    if f.is_empty() {
        f.push(c"");
    }
    if let Some(i) = f.iter().position(|c| c.to_bytes().eq_ignore_ascii_case(pick.as_bytes())) {
        return i as u16;
    }
    if f.len() >= 0x7fff {
        return 0;
    }
    let c = std::ffi::CString::new(pick).unwrap_or_default();
    f.push(Box::leak(c.into_boxed_c_str()));
    (f.len() - 1) as u16
}

/// The family name for `flags` (a static C string) and the engine's flags.
pub(crate) fn family(flags: i32) -> (*const std::os::raw::c_char, i32) {
    let id = (flags >> FAMILY_SHIFT) as usize;
    let low = flags & ((1 << FAMILY_SHIFT) - 1);
    if id == 0 {
        return (c"".as_ptr(), low);
    }
    let f = FAMILIES.lock().unwrap_or_else(|e| e.into_inner());
    (f.get(id).map_or(c"".as_ptr(), |c| c.as_ptr()), low)
}

type Widths = crate::style::FnvMap<Box<str>, f32>;

/// Text widths by (size, flags) then text: looked up by `&str`, no
/// allocation on a hit. Owned by the window, kept across frames.
#[derive(Default)]
pub(crate) struct TextCache {
    fonts: crate::style::FnvMap<(u32, i32), Widths>,
}

impl TextCache {
    /// Advance width of `s` at `size` px.
    pub(crate) fn width(&mut self, s: &str, size: f32, flags: i32) -> f32 {
        if s.is_empty() {
            return 0.0;
        }
        let m = self.fonts.entry((size.to_bits(), flags)).or_default();
        if let Some(&w) = m.get(s) {
            return w;
        }
        let mut w = 0.0f32;
        let (fam, low) = family(flags);
        // SAFETY: pointers are valid for the call (families are 'static).
        let ok = unsafe {
            sys::BUI_text_measure(fam, low, size, s.as_ptr().cast(), s.len() as i32, &mut w)
        };
        if ok == 0 {
            w = s.chars().count() as f32 * size * 0.55; // no native text engine
        }
        if m.len() > 20_000 {
            m.clear();
        }
        m.insert(s.into(), w);
        w
    }

    /// Greedy word wrap of `s` into lines no wider than `max` (explicit
    /// newlines kept). `brk` ([`break_mode`]) allows breaks inside words:
    /// anywhere only for a word too wide for a line of its own, break-all
    /// between any two characters.
    pub(crate) fn wrap(&mut self, s: &str, size: f32, flags: i32, max: f32, brk: u8) -> Vec<String> {
        let mut out = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split(' ').filter(|w| !w.is_empty()) {
                let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
                if self.width(&candidate, size, flags) <= max || (line.is_empty() && brk == 0) {
                    line = candidate;
                    continue;
                }
                if brk == BREAK_ALL {
                    // fill the current line with as much of the word as fits
                    let mut cur = if line.is_empty() { String::new() } else { format!("{line} ") };
                    for ch in word.chars() {
                        let mut t = cur.clone();
                        t.push(ch);
                        if self.width(&t, size, flags) > max && !cur.trim().is_empty() {
                            out.push(cur.trim_end().to_owned());
                            cur = ch.to_string();
                        } else {
                            cur = t;
                        }
                    }
                    line = cur;
                    continue;
                }
                if !line.is_empty() {
                    out.push(std::mem::take(&mut line));
                }
                if brk == BREAK_ANYWHERE && self.width(word, size, flags) > max {
                    for ch in word.chars() {
                        let mut t = line.clone();
                        t.push(ch);
                        if self.width(&t, size, flags) > max && !line.is_empty() {
                            out.push(std::mem::replace(&mut line, ch.to_string()));
                        } else {
                            line = t;
                        }
                    }
                } else {
                    line = word.to_owned();
                }
            }
            out.push(line);
        }
        out
    }

    /// `s` cut to fit `max` with a trailing ellipsis (`text-overflow: ellipsis`).
    pub(crate) fn ellipsize(&mut self, s: &str, size: f32, flags: i32, max: f32) -> String {
        if self.width(s, size, flags) <= max {
            return s.to_owned();
        }
        let chars: Vec<char> = s.chars().collect();
        let (mut lo, mut hi) = (0, chars.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            let t: String = chars[..mid].iter().collect::<String>() + "\u{2026}";
            if self.width(&t, size, flags) <= max {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        chars[..lo].iter().collect::<String>() + "\u{2026}"
    }
}

// ==========================================================================
// Bidirectional text: a compact Unicode Bidirectional Algorithm (UAX #9)

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    L,
    R,
    /// European / Arabic digits
    N,
    /// whitespace and punctuation
    Neutral,
}

fn class(c: char) -> Class {
    let u = c as u32;
    if matches!(u, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF) {
        Class::R
    } else if c.is_ascii_digit() || matches!(u, 0x0660..=0x0669 | 0x06F0..=0x06F9) {
        Class::N
    } else if c.is_alphabetic() {
        Class::L
    } else {
        Class::Neutral
    }
}

/// `s` in visual (left-to-right display) order. `rtl`: the paragraph's
/// direction; `override_`: `unicode-bidi: bidi-override` (every character
/// takes the paragraph's direction). Returns None when nothing moves (pure
/// left-to-right text, the common case).
pub(crate) fn bidi_visual(s: &str, rtl: bool, override_: bool) -> Option<String> {
    if override_ {
        return rtl.then(|| s.chars().rev().collect());
    }
    let chars: Vec<char> = s.chars().collect();
    let classes: Vec<Class> = chars.iter().map(|&c| class(c)).collect();
    if !rtl && !classes.contains(&Class::R) {
        return None;
    }
    let base: u8 = rtl as u8;
    // resolved levels: R odd, L even (W/I rules simplified: digits take the
    // level above the base, so they stay left-to-right inside RTL text)
    let mut levels: Vec<u8> = classes.iter().map(|c| match (c, base) {
        (Class::R, _) => 1,
        (Class::L, 0) => 0,
        (Class::L, _) => 2,
        (Class::N, 0) => 0,
        (Class::N, _) => 2,
        (Class::Neutral, _) => u8::MAX,
    }).collect();
    // digits right after RTL text in an LTR paragraph sit inside it
    if base == 0 {
        let mut prev_r = false;
        for k in 0..levels.len() {
            match classes[k] {
                Class::R => prev_r = true,
                Class::L => prev_r = false,
                Class::N if prev_r => levels[k] = 2,
                _ => {}
            }
        }
    }
    // neutrals (N1/N2): between two strong runs of the same direction they
    // take it; otherwise the paragraph's level
    let dir = |l: u8| l % 2;
    let n = levels.len();
    let mut k = 0;
    while k < n {
        if levels[k] != u8::MAX {
            k += 1;
            continue;
        }
        let start = k;
        while k < n && levels[k] == u8::MAX {
            k += 1;
        }
        let before = if start == 0 { base } else { levels[start - 1] };
        let after = if k == n { base } else { levels[k] };
        let l = match (dir(before), dir(after)) {
            (1, 1) => 1,
            (0, 0) => if base == 0 { 0 } else { 2 },
            _ => base,
        };
        for lv in &mut levels[start..k] {
            *lv = l;
        }
    }
    // trailing whitespace goes back to the paragraph level (L1)
    for k in (0..n).rev() {
        if chars[k].is_whitespace() {
            levels[k] = base;
        } else {
            break;
        }
    }
    // L2: from the highest level down to the lowest odd one, reverse every
    // run at that level or above
    let max = levels.iter().copied().max().unwrap_or(0);
    let min_odd = levels.iter().copied().filter(|l| l % 2 == 1).min().unwrap_or(1);
    let mut order: Vec<usize> = (0..n).collect();
    let mut lvl = max;
    while lvl >= min_odd && lvl > 0 {
        let mut k = 0;
        while k < n {
            if levels[order[k]] >= lvl {
                let start = k;
                while k < n && levels[order[k]] >= lvl {
                    k += 1;
                }
                order[start..k].reverse();
            } else {
                k += 1;
            }
        }
        lvl -= 1;
    }
    // mirrored brackets in RTL runs
    let mirror = |c: char| match c {
        '(' => ')', ')' => '(', '[' => ']', ']' => '[', '{' => '}', '}' => '{', '<' => '>', '>' => '<',
        c => c,
    };
    let out: String = order.iter().map(|&k| if levels[k] % 2 == 1 { mirror(chars[k]) } else { chars[k] }).collect();
    (out != s).then_some(out)
}

#[cfg(test)]
mod bidi_tests {
    use super::bidi_visual;

    #[test]
    fn reorders_rtl_runs() {
        // Hebrew inside English: the Hebrew run is reversed, the rest stays
        assert_eq!(bidi_visual("abc \u{05d0}\u{05d1}\u{05d2} def", false, false).as_deref(),
                   Some("abc \u{05d2}\u{05d1}\u{05d0} def"));
        // pure LTR: untouched
        assert_eq!(bidi_visual("hello world", false, false), None);
        // RTL paragraph: English words keep their order inside, runs swap
        assert_eq!(bidi_visual("abc \u{05d0}\u{05d1}", true, false).as_deref(), Some("\u{05d1}\u{05d0} abc"));
        // numbers stay left-to-right inside RTL text
        assert_eq!(bidi_visual("\u{05d0} 123", true, false).as_deref(), Some("123 \u{05d0}"));
        // override
        assert_eq!(bidi_visual("abc", true, true).as_deref(), Some("cba"));
    }
}
