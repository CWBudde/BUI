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

    /// Greedy word wrap of `s` into lines no wider than `max` (explicit newlines kept).
    pub(crate) fn wrap(&mut self, s: &str, size: f32, flags: i32, max: f32) -> Vec<String> {
        let mut out = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split(' ').filter(|w| !w.is_empty()) {
                let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
                if !line.is_empty() && self.width(&candidate, size, flags) > max {
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
