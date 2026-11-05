//! Deterministic synthetic chat data: the same messages for the tests, the
//! benchmarks, the file-backed log and the competitor harnesses, so every
//! comparison is on identical data. Message `i` is a pure function of `i`.

use crate::chat::classify;
use crate::source::Message;

const WORDS: [&str; 24] = [
    "the", "build", "is", "green", "again", "can", "someone", "review", "my", "patch", "before", "lunch", "ok", "that", "works", "on",
    "windows", "but", "not", "linux", "thanks", "see", "you", "tomorrow",
];

const EPOCH: u32 = 1_700_000_000;

const fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// Message `i` (its id is `i + 1`), flags unset (see [`messages`]).
#[must_use]
pub fn message(i: u64) -> Message {
    let h = mix(i);
    let sender = (h % 9) as u16;
    let time = EPOCH + (i * 37) as u32 + (mix(i ^ 7) % 30) as u32;
    // mostly one-liners, some paragraphs, a few walls of text
    let words = match (h >> 8) & 15 {
        0..=9 => 3 + (h >> 16) % 10,
        10..=14 => 14 + (h >> 16) % 30,
        _ => 60 + (h >> 16) % 140,
    };
    let mut text = String::with_capacity(words as usize * 6);
    let mut x = h;
    for k in 0..words {
        x = mix(x.wrapping_add(k));
        if k > 0 {
            text.push(' ');
        }
        text.push_str(WORDS[(x % WORDS.len() as u64) as usize]);
    }
    let media_h = if (h >> 40).is_multiple_of(40) { 120 + ((h >> 48) % 100) as u16 } else { 0 };
    Message { id: i + 1, time, sender, flags: 0, media_h, text }
}

/// The classified messages `from..to`.
#[must_use]
pub fn messages(from: u64, to: u64) -> Vec<Message> {
    let mut prev = from.checked_sub(1).map(|p| {
        let m = message(p);
        (m.sender, m.time)
    });
    (from..to)
        .map(|i| {
            let mut m = message(i);
            m.flags = classify(prev, m.sender, m.time);
            prev = Some((m.sender, m.time));
            m
        })
        .collect()
}
