//! Wrapping text to a width, once per `(text, width, style)`.
//!
//! Blender wraps a console line on the spot with `textview_wrap_offsets`
//! (`editors/space_info/textview.cc`): one pass over the code points,
//! counting columns, recording the byte offset where each line starts. This
//! is the same pass in pixels, with two additions: the result is cached by
//! the caller ([`crate::measure::TextLayout`]), and ASCII (nearly all chat)
//! is measured with a per-byte advance table filled lazily from the host's
//! text measurement, so a message costs a table lookup per byte instead of
//! a call into the shaper. Other characters are measured one by one by the
//! host (and remembered); the host still shapes the finished lines when it
//! paints.

use std::collections::HashMap;

/// Where each line of a wrapped text starts (the first, at 0, is implicit).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lines {
    starts: Box<[u32]>,
}

impl Lines {
    /// One line (no allocation).
    #[must_use]
    pub fn one() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.starts.len() + 1
    }

    /// Line `i` of `text` without its trailing newline.
    #[must_use]
    pub fn line<'a>(&self, text: &'a str, i: usize) -> &'a str {
        let a = if i == 0 { 0 } else { self.starts[i - 1] as usize };
        let b = self.starts.get(i).map_or(text.len(), |&b| b as usize);
        text[a..b].trim_end_matches(['\n', '\r'])
    }
}

/// Advance widths at one font size: ASCII in a table, the rest in a map.
pub struct Advances {
    size: f32,
    ascii: [f32; 128],
    other: HashMap<char, f32>,
}

impl Default for Advances {
    fn default() -> Self {
        Self { size: 0.0, ascii: [f32::NAN; 128], other: HashMap::new() }
    }
}

impl Advances {
    /// Forget everything when the size (or the font) changes.
    pub fn reset(&mut self, size: f32) {
        self.size = size;
        self.ascii = [f32::NAN; 128];
        self.other.clear();
    }

    #[must_use]
    pub const fn size(&self) -> f32 {
        self.size
    }

    fn of(&mut self, c: char, tw: &mut dyn FnMut(&str, f32) -> f32) -> f32 {
        let mut buf = [0u8; 4];
        let size = self.size;
        let mut measure = |c: char| tw(c.encode_utf8(&mut buf), size).max(0.0);
        if c.is_ascii() {
            let slot = &mut self.ascii[c as usize];
            if slot.is_nan() {
                *slot = measure(c);
            }
            *slot
        } else {
            *self.other.entry(c).or_insert_with(|| measure(c))
        }
    }
}

/// Wrap `text` to `max_w` px: break after spaces, at newlines, and inside a
/// word only when the word alone is wider than the line.
pub fn wrap(text: &str, max_w: f32, adv: &mut Advances, tw: &mut dyn FnMut(&str, f32) -> f32) -> Lines {
    let mut starts: Vec<u32> = Vec::new();
    let (mut line_w, mut line_start) = (0.0f32, 0usize);
    // the best place to break: byte offset after the last space, and the width up to it
    let mut brk: Option<(usize, f32)> = None;
    for (i, c) in text.char_indices() {
        if c == '\n' {
            starts.push((i + 1) as u32);
            (line_w, line_start, brk) = (0.0, i + 1, None);
            continue;
        }
        let w = adv.of(c, tw);
        if c == ' ' || c == '\t' {
            // trailing blanks may hang past the edge
            line_w += w;
            brk = Some((i + 1, line_w));
            continue;
        }
        if line_w + w > max_w && i > line_start {
            match brk {
                Some((at, w_at)) if at > line_start => {
                    starts.push(at as u32);
                    line_start = at;
                    line_w -= w_at;
                }
                _ => {
                    starts.push(i as u32);
                    line_start = i;
                    line_w = 0.0;
                }
            }
            brk = None;
        }
        line_w += w;
    }
    Lines { starts: starts.into_boxed_slice() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tw(s: &str, size: f32) -> f32 {
        s.chars().count() as f32 * size * 0.5
    }

    fn run(text: &str, w: f32) -> Vec<String> {
        let mut adv = Advances::default();
        adv.reset(10.0);
        let l = wrap(text, w, &mut adv, &mut |s, z| tw(s, z));
        (0..l.count()).map(|i| l.line(text, i).to_owned()).collect()
    }

    #[test]
    fn short_text_is_one_line() {
        assert_eq!(run("hello", 100.0), ["hello"]);
        assert_eq!(run("", 100.0), [""]);
    }

    #[test]
    fn breaks_after_spaces() {
        // 5 px per char: 10 chars per 50 px line
        assert_eq!(run("aaaa bbbb cccc dddd", 50.0), ["aaaa bbbb ", "cccc dddd"]);
    }

    #[test]
    fn long_word_breaks_inside() {
        assert_eq!(run("abcdefghijklmno", 50.0), ["abcdefghij", "klmno"]);
    }

    #[test]
    fn newlines_are_hard_breaks() {
        assert_eq!(run("a\nb\n\nc", 50.0), ["a", "b", "", "c"]);
    }

    #[test]
    fn unicode_is_measured_by_the_host() {
        assert_eq!(run("héllo wörld", 30.0), ["héllo ", "wörld"]);
    }

    #[test]
    fn every_line_but_the_unbreakable_fits() {
        let text = "the quick brown fox jumps over the lazy dog ".repeat(20);
        for l in run(&text, 120.0) {
            assert!(tw(l.trim_end(), 10.0) <= 120.0, "{l:?}");
        }
    }
}
