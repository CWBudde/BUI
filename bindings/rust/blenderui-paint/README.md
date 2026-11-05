# blenderui-paint

The small vocabulary every BUI view core paints with, in one place: `Point`,
`Rect`, `Rgba`, the `PaintSink` trait (an adapter implements it once per UI
framework) and `TextCache`, a bounded generation-tagged text-run cache (the
design of Blender's BLF glyph cache, lifted to runs). `blenderui-table` and
`blenderui-vlist` both depend on it; neither copies it. `std` only.

`testing` holds the seeded `Rng` and the counting allocator the cores' tests
share (allocation-count assertions).
