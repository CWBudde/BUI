# blenderui vs gpui vs gpui-fast

The same UI on three frameworks:
- `bui-bench`: blenderui.
- `gpui-bench`: [gpui](https://crates.io/crates/gpui) 0.2.2, Zed's GPU-rendered framework.
- `gpui-fast-bench`: [gpui-fast](https://github.com/longbridge/gpui-fast) 0.1.13, a gpui fork with rendering and layout optimizations.

Both gpui apps compile the identical source file (`gpui-bench/src/scene.rs`).
The scene is a 1000×800 window filled with N cells (a rounded, coloured box
with a centred number), built with the same element tree in both
(`div().w(..).h(..).bg(..).rounded(..).flex().items_center().child(label)`).

- **anim**: every frame changes every cell's colour and number, and the view
  calls `window.request_animation_frame()`. That's the worst case: full
  re-render, re-layout and re-paint every frame.
- **one**: every frame re-renders the whole scene, but only one cell actually
  changes: closer to a real UI (a hover, a ticking value), where reusing the
  previous frame's work pays off.
- **idle**: the scene renders once and then nothing changes.
- **blenderui (uncapped)**: animation without vsync pacing (`--uncapped`).

## Running

```sh
cd bindings/rust/bench
set GPUI_FXC_PATH=C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\fxc.exe   # gpui needs fxc.exe
cargo build --release
py run.py --reps 3          # ~12 minutes; prints a Markdown table, writes results.json
```

Each app measures itself with the same code (`common/measure.rs`):
- **Warm-up and window:** 3 s warm-up, then a 10 s measured window.
- **CPU:** process CPU **cycles** across all threads (`QueryProcessCycleTime`), converted to time with the TSC frequency calibrated during warm-up.
- **Frames and memory:** frames rendered, the working set sampled every 100 ms, and private bytes at the end.
- **GPU:** `run.py` also samples each process's GPU 3D-engine utilisation and GPU memory from the Windows performance counters, because GPU work isn't process CPU time.

Every configuration ran 3 times, and the table shows the medians.

> An earlier version of this benchmark used `GetProcessTimes`. Windows charges
> that per ~15.6 ms clock tick, which aliases with a 60 Hz frame loop and
> under-reported CPU badly (gpui at 1000 cells showed 6.3 ms per frame; it
> really uses 19 ms). All numbers below use the cycle counter.

## Results

Ryzen 7 5800U (8 cores / 16 threads, laptop, plugged in), AMD Radeon iGPU,
60 Hz display, Windows 11, release builds. All three apps ran back to back in
one session, 3 runs each, medians shown. This is after blenderui's
arena / compile-time rework (no `Rc` / `RefCell`).

| App | Cells | Mode | FPS | CPU % (1 core = 100) | CPU ms / frame | GPU 3D % | RAM working set MB | Private MB | GPU memory MB | First frame ms |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| gpui | 100 | anim | 60.1 | 20.3 | 3.38 | 5.2 | 31.7 | 35.3 | 16.1 | 257 |
| gpui-fast | 100 | anim | 60.0 | 7.8 | 1.30 | 3.3 | 33.6 | 39.9 | 19.1 | 258 |
| blenderui | 100 | anim | 60.0 | 30.3 | 5.05 | 0 | 19.6 | 11.4 | 0 | 6 |
| gpui | 100 | idle | 0.0 | 0.7 | 0 | 0.0 | 31.4 | 34.9 | 16.1 | 274 |
| gpui-fast | 100 | idle | 0.0 | 0.0 | 0 | 0.0 | 32.5 | 39.0 | 18.8 | 254 |
| blenderui | 100 | idle | 0.0 | 0.0 | 0 | - | 17.8 | 9.5 | 0 | 6 |
| blenderui (uncapped) | 100 | anim | 529.3 | 329.9 | 6.24 | 0 | 19.8 | 11.8 | 0 | 7 |
| gpui | 1000 | anim | 60.1 | 88.4 | 14.73 | 6.1 | 46.0 | 51.7 | 17.1 | 275 |
| gpui-fast | 1000 | anim | 60.0 | 40.4 | 6.74 | 7.2 | 43.4 | 50.9 | 19.9 | 262 |
| blenderui | 1000 | anim | 59.8 | 81.8 | 13.67 | 0 | 23.4 | 15.6 | 0 | 7 |
| gpui | 1000 | idle | 0.0 | 1.4 | 0 | 0.0 | 47.1 | 50.9 | 17.1 | 261 |
| gpui-fast | 1000 | idle | 0.0 | 0.0 | 0 | 0.0 | 42.1 | 49.8 | 19.1 | 262 |
| blenderui | 1000 | idle | 0.0 | 0.0 | 0 | - | 21.6 | 13.5 | 0 | 6 |
| blenderui (uncapped) | 1000 | anim | 156.9 | 236.4 | 14.82 | 0 | 23.7 | 15.9 | 0 | 6 |
| gpui | 3000 | anim | 18.9 | 100.5 | 53.07 | 2.5 | 75.9 | 95.4 | 20.6 | 251 |
| gpui-fast | 3000 | anim | 50.5 | 101.2 | 20.05 | 6.6 | 63.7 | 85.3 | 23.5 | 243 |
| blenderui | 3000 | anim | 57.5 | 157.1 | 27.33 | 0 | 31.2 | 27.9 | 0 | 6 |
| gpui | 3000 | idle | 0.0 | 1.0 | 0 | 0.0 | 73.6 | 92.1 | 20.6 | 252 |
| gpui-fast | 3000 | idle | 0.0 | 0.1 | 0 | 0.0 | 59.7 | 80.9 | 20.9 | 254 |
| blenderui | 3000 | idle | 0.0 | 0.0 | 0 | - | 26.9 | 21.7 | 0 | 6 |
| blenderui (uncapped) | 3000 | anim | 58.1 | 168.0 | 28.90 | 0 | 31.3 | 28.3 | 0 | 6 |

blenderui has no GPU counter instances (shown as 0): it draws on the CPU. `CPU %`
counts all threads, so above 100 means more than one core.

## What it says

| Animated cells | blenderui | gpui-fast | gpui |
|---|---:|---:|---:|
| 100 | 60 fps, 5.1 ms CPU/frame (529 uncapped) | 60 fps, **1.3 ms** + GPU | 60 fps, 3.4 ms + GPU |
| 1000 | 60 fps, 13.7 ms (157 uncapped) | 60 fps, **6.7 ms** + GPU | 60 fps, 14.7 ms + GPU |
| 3000 | **57.5 fps**, 27.3 ms | 50.5 fps, **20.1 ms** + GPU | 18.9 fps, 53.1 ms + GPU |
| RAM working set (3000) | **31 MB** | 64 MB + 24 MB GPU | 76 MB + 21 MB GPU |
| First frame | **~6 ms** | ~250 ms | ~260 ms |
| Idle CPU | **0.0 %** | 0.0-0.1 % | 0.7-1.4 % |

- **Frame rate:** blenderui is fastest at 3000 cells in this session (57.5 vs gpui-fast 50.5; per-run ranges 57.4-58.6 vs 49.9-51.6). Smaller scenes hit the 60 Hz cap in all three. gpui-fast's 3000-cell frame rate varies a lot between sessions on this laptop (29.7, 59.8 and 50.5 fps in three sessions), so compare within a session only.
- **CPU efficiency:** gpui-fast still does the least CPU work per frame (plus GPU work). The gap at 3000 cells has narrowed from 2.1× to 1.4×.
- **Memory and startup:** blenderui uses about half the RAM of either gpui and no GPU memory, and starts about 40× faster.

## Frame-to-frame reuse (mode `one`)

Measured in a later session while the laptop was charging from a low battery,
so everything (all three apps) ran slower than in the table above; only
same-session comparisons are meaningful:

| 3000 cells, same session | blenderui | gpui-fast | gpui |
|---|---:|---:|---:|
| all cells change (`anim`) | 36.8 fps, 47.7 ms CPU/frame | 31.3 fps, 27.4 ms + GPU | 11.3 fps, 89.2 ms + GPU |
| one cell changes (`one`) | **54.0 fps, 27.5 ms** | 29.4 fps, 16.3 ms + GPU | 11.3 fps, 89.1 ms + GPU |

blenderui spends 42 % less CPU per frame when little changes (38 % at 1000
cells). gpui-fast was capped near 30 fps throughout that session (its GPU in a
low-power state), so its frame rate there says little.

## Caveats

- One laptop, one display (60 Hz caps both), the iGPU. A discrete GPU would make gpui's GPU share smaller still.
- Laptop clocks vary between sessions. gpui's 1000-cell scene measured 51 fps in this run and 45-60 fps in earlier ones, so the medians are from runs made back to back in one session.
- blenderui's root is wrapped in `body().m_0()` so its grid fills the window exactly like gpui's root `div`.
- **Where gpui can't win:** gpui uses a parallel GPU, so a much larger window or a high-DPI display shifts work its way. Every blenderui pixel is CPU work (spread over up to 8 threads).
- `CPU ms / frame` for gpui excludes its GPU work. Compare it together with the GPU column.

## How blenderui got here

The first version of this benchmark had blenderui at 29 fps (1000 cells) and 12 fps (3000 cells). These changes, all general-purpose, got it to 60 and 34. `BUI_PROFILE=1` prints the per-phase frame times they were found with.

| Change | Where | Effect at 3000 cells |
|---|---|---|
| **Parallel canvas raster:** horizontal bands on the Win32 thread pool. Text is prepared on one thread and drawn read-only from all of them. The pixels are identical to serial drawing (`BUI_RASTER_THREADS=1` forces serial) | `kit_frame.c`, `gpu_raster.c` | raster 8.6 → 2.7 ms |
| **Fill-only rounded rects** use the span-filling `BUI_raster_round_rect` instead of the general widget-base routine | `kit_frame.c` | raster 11.8 → 4.6 ms (headless) |
| **Style sharing:** elements with the same parent style, matched rules and declarations reuse one computed style (as browsers do) | `layout.rs` | cascade 12 → 3.3 ms |
| **Interned and typed CSS values:** `.w(px(40.0))` and `.bg(rgb(..))` reuse tokens cached by the number's bits instead of formatting and re-parsing a string | `css.rs`, `styled.rs` | `render()` 7.4 → 3.6 ms |
| **Rule index:** stylesheet rules are bucketed by their subject's id, class and tag | `css.rs` | selector matching about 5 → 0.4 ms |
| **Fewer allocations:** text runs are moved rather than copied, text-width lookups don't allocate, the key map is built lazily, text is passed to the kit as a C string once | `layout.rs`, `text.rs`, `paint.rs` | about 3 ms |
| **No overdraw:** a background identical to the one under it isn't painted, and the kit skips its clear when the page is opaque | `paint.rs`, `kit_core.c` | about 1 ms |
| **Frame pacing:** animation frames rebuild right before the render (`BK_window_request_frame`). A frame that is already late doesn't wait for another vblank | `kit_window.c`, `window.rs` | removes 10 ms timer and vsync quantisation stalls |

Second round (lower CPU per frame without costing RAM, startup, idle or fps):

| Change | Where | Effect |
|---|---|---|
| **Adaptive raster threads:** the fewest threads that still meet the frame deadline, and all of them when it can't be met | `kit_frame.c` | 1000 cells: 17.4 → 14.3 ms CPU/frame; 100 cells drawn serially |
| **Lone text folded into its box:** one measured layout leaf instead of a text leaf (or one per word) | `layout.rs` | 3000 cells: boxes 5.0 → 2.5 ms, taffy 7.0 → 1.9 ms; pixel-identical |
| **Items bucketed by band** once instead of every band scanning the list | `kit_frame.c` | about 1.5 ms CPU at 3000 cells |
| **Allocation-free style-sharing probe** (hash the inputs, compare in place) | `layout.rs` | cascade 3.5 → 2.5 ms |
| **In-place canvas updates** (`BK_canvas_put_item` / `BK_canvas_truncate` reuse buffers and invalidate once) and inline painter coordinates | `kit_frame.c`, `paint.rs` | hand-off 1.25 → 0.30 ms |

Third round (no `Rc` / `RefCell`, work moved to compile time):

| Change | Effect at 3000 cells |
|---|---|
| Property names resolved at compile time (`const` table), typed values without formatting or parsing, `css!` | element building 3.5 → 3.1 ms |
| Frames as index-linked arenas reused across frames; words as byte ranges; no per-node strings or `Rc`s | build 7.3 → 4.1 ms, boxes 2.8 → 1.4 ms, dropping the old frame 1.4 → 0.03 ms |
| Styles in a per-window store addressed by `StyleId` | the cascade probe compares ids |
| Matched rules and declarations in stack-backed small vectors; rule index without key strings | cascade 2.8 → 2.5 ms |
| Text widths, images, the author sheet and the taffy tree cached in the window | fewer allocations everywhere |

Fourth round (reuse across frames, fewer allocations):

| Change | Effect |
|---|---|
| Matched rules memoized by an element fingerprint (parent, previous sibling, tag, id, attributes, position, state) | selector matching runs once per distinct element |
| A persistent taffy tree reconciled by key; text leaves keyed by a text hash; changed subtrees marked dirty explicitly | taffy's cache skips everything that didn't change: taffy time halves at 3000 cells when one cell changes |
| `Tok` 56 → 32 bytes (function tokens boxed); `Div` ~470 → ~150 bytes (listeners and state styles allocated on use, children inline) | less memory traffic, one allocation less per element |
| Painter, text buffer, layout scratch, box index table reused across frames; small lists on the stack | allocations per frame no longer grow with text items |

Still single-threaded and the largest remaining costs are the cascade, box tree and taffy layout (about 20 ms at 3000 cells): incremental restyling and layout would be the next step.
