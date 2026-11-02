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
one session, 3 runs each, medians shown.

| App | Cells | Mode | FPS | CPU % (1 core = 100) | CPU ms / frame | GPU 3D % | RAM working set MB | Private MB | GPU memory MB | First frame ms |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| gpui | 100 | anim | 60.1 | 18.8 | 3.12 | 5.4 | 33.5 | 35.8 | 16.1 | 225 |
| gpui-fast | 100 | anim | 60.0 | 8.1 | 1.35 | 2.2 | 35.6 | 40.3 | 19.1 | 206 |
| blenderui | 100 | anim | 60.1 | 38.1 | 6.33 | 0 | 22.3 | 12.9 | 0 | 7 |
| gpui | 100 | idle | 0.0 | 1.1 | 0 | 0.0 | 33.3 | 35.4 | 16.1 | 219 |
| gpui-fast | 100 | idle | 0.0 | 0.1 | 0 | 0.0 | 34.4 | 39.4 | 18.8 | 201 |
| blenderui | 100 | idle | 0.1 | 0.2 | 0 | - | 19.9 | 9.9 | 0 | 6 |
| blenderui (uncapped) | 100 | anim | 494.1 | 259.2 | 5.25 | 0 | 22.9 | 13.4 | 0 | 7 |
| gpui | 1000 | anim | 60.1 | 94.1 | 15.66 | 6.8 | 47.9 | 52.0 | 17.1 | 223 |
| gpui-fast | 1000 | anim | 60.0 | 49.7 | 8.27 | 7.9 | 44.8 | 51.1 | 19.9 | 203 |
| blenderui | 1000 | anim | 60.1 | 99.7 | 16.59 | 0 | 26.8 | 17.8 | 0 | 8 |
| gpui | 1000 | idle | 0.3 | 3.7 | 0 | 0.8 | 47.0 | 51.1 | 17.1 | 219 |
| gpui-fast | 1000 | idle | 0.2 | 0.5 | 0 | 0.0 | 44.1 | 50.3 | 19.9 | 200 |
| blenderui | 1000 | idle | 0.0 | 0.1 | 0 | - | 22.8 | 12.8 | 0 | 5 |
| blenderui (uncapped) | 1000 | anim | 96.7 | 165.4 | 16.85 | 0 | 27.3 | 17.6 | 0 | 7 |
| gpui | 3000 | anim | 22.8 | 100.6 | 44.09 | 4.2 | 77.5 | 94.9 | 20.6 | 215 |
| gpui-fast | 3000 | anim | 29.7 | 54.7 | 18.42 | 4.5 | 64.7 | 85.3 | 23.5 | 213 |
| blenderui | 3000 | anim | 32.6 | 129.7 | 39.67 | 0 | 36.3 | 31.6 | 0 | 5 |
| gpui | 3000 | idle | 0.0 | 0.9 | 0 | 0.0 | 75.2 | 92.3 | 20.6 | 233 |
| gpui-fast | 3000 | idle | 0.0 | 0.1 | 0 | 0.0 | 61.6 | 81.4 | 20.9 | 201 |
| blenderui | 3000 | idle | 0.2 | 1.0 | 0 | - | 27.8 | 19.3 | 0 | 6 |
| blenderui (uncapped) | 3000 | anim | 31.4 | 128.6 | 40.97 | 0 | 35.4 | 37.8 | 0 | 6 |

blenderui has no GPU counter instances (shown as 0): it draws on the CPU. `CPU %`
counts all threads, so above 100 means more than one core.

## What it says

| Animated cells | blenderui | gpui-fast | gpui |
|---|---:|---:|---:|
| 100 | 60 (500 uncapped) | 60 | 60 |
| 1000 | 60 (97 uncapped) | 60 | 60 |
| 3000 | **32.6** | 29.7 | 22.8 |
| CPU ms / frame at 3000 | 39.7 | **18.4** (+GPU) | 44.1 (+GPU) |
| RAM working set at 3000 | **36 MB** | 65 MB (+24 MB GPU) | 78 MB (+21 MB GPU) |
| First frame | **~6 ms** | ~205 ms | ~220 ms |

- **FPS:** at the display cap (100 and 1000 cells) all three tie at 60. At 3000 cells blenderui is fastest: 10 % ahead of gpui-fast (per-run ranges 30.3-32.9 vs 29.7-29.9) and 43 % ahead of gpui. Uncapped, blenderui reaches 97 fps at 1000 cells; the gpui apps can't run uncapped.
- **CPU efficiency: gpui-fast is best,** about 2× less CPU per frame than blenderui on the large scene, plus a little GPU. Its 3000-cell frame rate is held back by something other than CPU (it uses half a core), while blenderui uses 1.3 cores to get there.
- **Memory and startup: blenderui wins clearly.** It uses 1.8-2.2× less working set and 2.7-3.2× less private memory, holds no GPU memory, and shows its first frame about 35× sooner.
- **Idle:** blenderui and gpui-fast both use about 0.1 % CPU; gpui uses about 1 %.
- **gpui across sessions:** at 1000 cells it measured 51 fps in an earlier session and 60 in this one, which is why these numbers come from one session.

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

Still single-threaded and the largest remaining costs are the cascade, box tree and taffy layout (about 20 ms at 3000 cells): incremental restyling and layout would be the next step.
