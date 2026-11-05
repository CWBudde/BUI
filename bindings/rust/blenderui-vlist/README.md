# blenderui-vlist

A variable-height virtualized list core (chat-grade), **one design from 100
items to 10M and beyond**: no small mode and large mode, no state that can
be wrong. `std` only, no dependencies besides [`blenderui-paint`](../blenderui-paint).
Framework adapters hold no list logic: `blenderui`'s `vlist` module
(`variable_list`, `chat_list`) and a gpui adapter.

```text
 ItemSource ---> ListView ---> PaintSink
 (records,       index, anchor,    (rects, text, clips;
  bytes per      one frame         the adapter implements it)
  range)             ^
 Measurer -----------+   how an item lays out; wrapped lines cached per width
```

## The index

`HeightIndex` groups items in blocks of up to 1024. The top level is a
Fenwick tree over blocks (height and count per node) with spare slots on both
sides, so prepending history is a point update, not a shift. Each block is

* `Uniform`: a count and one height (a 10M-row list of equal rows is a few
  hundred KB of block headers),
* `Estimated`: a count and the text bytes it holds, nothing per item; the
  height comes from an estimator that learns from every real measurement,
* `Measured`: `u32` prefix sums (1/64 px) plus an exactness bit per item.

A list of up to one block is one block and the tree is bypassed. Memory
follows what was scrolled through, not the length; `trim` returns far blocks
to estimates, and a new wrap width demotes every measured block at once.

## Scrolling

The position is an anchor, `(item, offset inside it)`, painted relative to it
in small `f32` coordinates, or `Pos::Bottom` (the end at the bottom edge). A
height changing above the viewport cannot move what the user sees; prepending
a page does not jump; `stick_to_bottom` follows new messages while at the
bottom and stops when the user scrolls up.

## Use

```rust
use blenderui_vlist::{ChatList, Options, synth};
use blenderui_paint::Palette;

let mut chat = ChatList::from_messages(
    synth::messages(0, 1_000_000), Palette::blender_dark(), 600.0, 400.0,
    Options { start_at_end: true, ..Options::default() },
);
let mut tw = |s: &str, size: f32| s.len() as f32 * size * 0.5; // the host's text measurement
chat.layout(&mut tw);        // measures what is on screen, within the budget
// chat.paint(&mut my_sink); // my_sink: impl PaintSink
chat.push(synth::message(1_000_000)); // follows, because it was at the bottom
```

For items that are not chat messages, build a `List` from your own
`ItemSource`, `Measurer` and `ItemPainter`.

## Sources

`VecSource` (in memory) and `log::LogSource` (a file-backed append-only log:
fixed 16-byte records plus a text arena, read with positioned reads, so a
10M-message list needs no RAM for the messages and its index is built from
two 8-byte reads per block).

Run `cargo run --release -p blenderui-vlist --example bench_core` for the
index micro-benchmarks and `--example bench_frames` for whole frames.
