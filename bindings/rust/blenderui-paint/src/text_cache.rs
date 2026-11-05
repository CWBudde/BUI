//! A text-run cache: the design of Blender's font glyph cache
//! (`blf_glyph.cc`: a bounded cache keyed by font, size and glyph, filled on
//! first use, dropped whole when the font changes) lifted to runs. A table
//! shows mostly the same strings frame after frame (scrolling only moves
//! them), so a shaped run, or its measured width, is computed once and found
//! again by `(text, font, size)`.
//!
//! * `V` is whatever the framework wants to keep: a width (`f32`), a shaped
//!   line, a glyph run.
//! * Lookups of cached text do not allocate; a miss allocates the key once.
//! * Bounded: entries carry the generation of their last use, and when the
//!   cache reaches its capacity the entries unused for a whole generation
//!   go (all of them when that frees too little). A generation is one frame
//!   ([`TextCache::next_generation`]).
//! * [`TextCache::clear`] is the invalidation for a font, scale or theme change.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A hasher for keys that are already hashes.
#[derive(Default)]
pub struct KeyHasher(u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
        }
    }

    fn write_u64(&mut self, v: u64) {
        self.0 = v;
    }
}

struct Entry<V> {
    text: Box<str>,
    font: u32,
    size: u32,
    generation: u32,
    value: V,
}

/// See the module docs.
pub struct TextCache<V> {
    map: HashMap<u64, Entry<V>, BuildHasherDefault<KeyHasher>>,
    generation: u32,
    capacity: usize,
    hits: u64,
    misses: u64,
}

/// FNV-1a over the text, mixed with the font and size, then finalized.
fn hash_key(text: &str, font: u32, size: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (u64::from(font) << 32 | u64::from(size));
    for &b in text.as_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    h ^= h >> 32;
    h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^ (h >> 29)
}

impl<V> TextCache<V> {
    /// A cache holding up to `capacity` runs.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self { map: HashMap::default(), generation: 0, capacity: capacity.max(1), hits: 0, misses: 0 }
    }

    /// Start a new generation (call once per frame).
    pub const fn next_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Make room for at least `n` runs (a table that shows more than the
    /// current capacity needs a cache that holds a screenful and the next).
    pub fn ensure_capacity(&mut self, n: usize) {
        self.capacity = self.capacity.max(n);
    }

    /// Drop everything (the font, scale or theme changed).
    pub fn clear(&mut self) {
        self.map.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// `(hits, misses)` since creation.
    #[must_use]
    pub const fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    fn make_room(&mut self) {
        if self.map.len() < self.capacity {
            return;
        }
        let current = self.generation;
        // keep what the current or the previous generation used
        self.map.retain(|_, e| current.wrapping_sub(e.generation) <= 1);
        if self.map.len() >= self.capacity {
            self.map.clear();
        }
    }

    /// The value for `(text, font, size)` if it is cached (a pure lookup: no
    /// generation bump, no statistics, so it works through a shared reference).
    #[must_use]
    pub fn peek(&self, text: &str, font: u32, size: f32) -> Option<&V> {
        let size_bits = size.to_bits();
        self.map.get(&hash_key(text, font, size_bits)).filter(|e| e.font == font && e.size == size_bits && &*e.text == text).map(|e| &e.value)
    }

    /// The value for `(text, font, size)`, made with `make` on a miss.
    pub fn get_or_insert_with(&mut self, text: &str, font: u32, size: f32, make: impl FnOnce() -> V) -> &V {
        let size_bits = size.to_bits();
        let key = hash_key(text, font, size_bits);
        let generation = self.generation;
        let found = self.map.get_mut(&key).is_some_and(|e| {
            let same = e.font == font && e.size == size_bits && &*e.text == text;
            if same {
                e.generation = generation;
            }
            same
        });
        if found {
            self.hits += 1;
        } else {
            self.misses += 1;
            self.make_room();
            self.map.insert(key, Entry { text: text.into(), font, size: size_bits, generation, value: make() });
        }
        // present: found or just inserted
        &self.map[&key].value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_do_not_remake() {
        let mut c = TextCache::new(8);
        let mut made = 0;
        for _ in 0..5 {
            let v = *c.get_or_insert_with("hello", 0, 13.0, || {
                made += 1;
                5.0f32
            });
            assert_eq!(v, 5.0);
        }
        assert_eq!(made, 1);
        assert_eq!(c.stats(), (4, 1));
    }

    #[test]
    fn key_is_text_font_and_size() {
        let mut c = TextCache::new(16);
        c.get_or_insert_with("a", 0, 13.0, || 1);
        c.get_or_insert_with("a", 1, 13.0, || 2);
        c.get_or_insert_with("a", 0, 14.0, || 3);
        c.get_or_insert_with("b", 0, 13.0, || 4);
        assert_eq!(c.len(), 4);
        assert_eq!(*c.get_or_insert_with("a", 1, 13.0, || 99), 2);
        assert_eq!(*c.get_or_insert_with("a", 0, 14.0, || 99), 3);
    }

    #[test]
    fn stays_bounded_and_keeps_recent_entries() {
        let mut c = TextCache::new(100);
        for frame in 0..50u32 {
            c.next_generation();
            // a few runs reused every frame, many new ones
            for i in 0..30u32 {
                c.get_or_insert_with(&format!("hot{i}"), 0, 12.0, || 0u32);
                c.get_or_insert_with(&format!("cold{frame}-{i}"), 0, 12.0, || 0u32);
            }
            assert!(c.len() <= 100, "frame {frame}: {}", c.len());
        }
        // the reused runs survive every sweep
        let (_, misses_before) = c.stats();
        c.next_generation();
        for i in 0..30u32 {
            c.get_or_insert_with(&format!("hot{i}"), 0, 12.0, || 0u32);
        }
        assert_eq!(c.stats().1, misses_before, "hot runs were evicted");
    }

    #[test]
    fn clear_invalidates() {
        let mut c = TextCache::new(4);
        c.get_or_insert_with("x", 0, 1.0, || 1);
        c.clear();
        assert!(c.is_empty());
        assert_eq!(*c.get_or_insert_with("x", 0, 1.0, || 2), 2);
    }

    #[test]
    fn lookups_of_cached_text_do_not_allocate() {
        use crate::testing::count_allocs;
        let mut c = TextCache::new(64);
        let words = ["alpha", "beta", "gamma"];
        for w in words {
            c.get_or_insert_with(w, 0, 12.0, || 1u8);
        }
        let n = count_allocs(|| {
            for _ in 0..100 {
                for w in words {
                    c.get_or_insert_with(w, 0, 12.0, || 1u8);
                }
            }
        });
        assert_eq!(n, 0);
    }
}
