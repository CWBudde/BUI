# blenderui-vlist-gpui

A variable-height virtualized list (chat-grade) for [gpui](https://gpui.rs),
on the shared [`blenderui-vlist`](../blenderui-vlist) core: a height index
that is one block for 100 items and a Fenwick tree over blocks for 10M,
anchor-based scrolling, wrapped-text measuring with a per-frame budget, and
item sources (in memory, file-backed). It paints only the visible items,
straight into the window's scene, with one custom `Element`.

```rust
use blenderui_vlist_gpui::*;

let chat = cx.new(|cx| {
    ListState::new(
        ChatList::from_messages(messages, Palette::blender_dark(), 600.0, 400.0,
                                Options { start_at_end: true, ..Options::default() }),
        cx,
    )
});
// in render
div().size_full().child(chat_list(&chat))
// from anywhere
chat.update(cx, |s, _| s.push(message));   // follows while at the bottom
```

Kept out of the main workspace (gpui is a huge build). To build it on
Windows, point `GPUI_FXC_PATH` at the SDK's `fxc.exe`:
`set GPUI_FXC_PATH=C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\fxc.exe`.

`cargo build --release --features bench --example bench_list_gpui -j 1` builds
the head-to-head against gpui's `list` (a `SumTree`) and gpui-component's
`virtual_list`; `node bench/run.mjs` runs it.
