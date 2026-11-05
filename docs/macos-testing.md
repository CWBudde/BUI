# Testing BUI on macOS

Thanks for testing BUI on a Mac. The macOS window host is new. CI checks it, and
[Krish Kalaria](https://github.com/krishkalaria12) has tried it by hand on an Apple Silicon MacBook Air. His
report (issue #3) and fixes became pull request #4: Quit, window sizing and tkinter. More Macs find
more, especially Intel Macs and older macOS versions, because a person clicking
around finds what CI can't.
This guide takes about 30–60 minutes: build from source, run the tests, try a
few apps, and report back.

What runs on macOS:

- **Native windows** through AppKit (`kit/intern/kit_window_cocoa.m`, ported
  from Blender's GHOST Cocoa backend): Retina scaling, trackpad scrolling,
  cursor warping for drag-to-scrub, the clipboard, cursors, full screen.
- **Drawing on the CPU.** The GPU renderer is Direct3D 11 on Windows and
  OpenGL on Linux. There is no Metal renderer yet, so `BUI_GPU=1` is ignored
  and frames are drawn by the software rasterizer and shown with
  CoreGraphics.
- The C kit, Python (`blenderui` / `bui`, the tkinter layer, Dear ImGui), Rust
  (`blenderui`, `blenderui-imgui`) and C/C++ (Dear ImGui on BUI).

## 1. Prerequisites

You need macOS 11 (Big Sur) or newer, on Apple Silicon or Intel.

| Tool | How to get it | Check |
| --- | --- | --- |
| Xcode Command Line Tools (clang, git) | `xcode-select --install` | `clang --version` |
| CMake 3.20+ | `brew install cmake` (or cmake.org) | `cmake --version` |
| Python 3.11+ **with tkinter** | python.org installer, or `brew install python@3.12 python-tk@3.12` | `python3.12 -c "import tkinter; print(tkinter.TkVersion)"` |
| Rust (stable) | `curl https://sh.rustup.rs -sSf \| sh` | `cargo --version` |

The `python3` that comes with the Command Line Tools is 3.9, which is too old.
Use the full name of a newer one (`python3.12` below) everywhere.

## 2. Get the code

```sh
git clone https://github.com/ppmpreetham/BUI.git
cd BUI
git rev-parse --short HEAD   # note this for your report
```

## 3. Build the C/C++ project and run the tests

```sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DBUI_BUILD_EXAMPLES=ON \
      -DPython3_EXECUTABLE="$(which python3.12)"
cmake --build build --parallel
ctest --test-dir build --output-on-failure
```

Configuring should print `BUI: building buipy Python bindings for Python 3.12…`.
If it says the Python development files were not found, the
`-DPython3_EXECUTABLE` path is wrong.

All tests should pass. What they cover:

| Test | What it checks |
| --- | --- |
| `bui_window_smoke` | Opens a real Cocoa window with a node editor, renders, runs timers, sends input and quits on its own. A window appears for about a second. Exit 77 (shown as skipped) means no GUI session, for example over plain `ssh`. |
| `bui_theme_test` | Blender's Dark and Light themes |
| `core`, `gpu_parity` | the C core, and the rasterizer (the GPU half is skipped on macOS) |
| `imgui_backend`, `imgui_c_api` | Dear ImGui on BUI, headless, from C++ and C |
| `buipy*` | the Python bindings: kit, Qt and tkinter syntax, components, nodes, themes, the file browser, ImGui |

Run the window smoke test on its own to see what it printed:

```sh
./build/bui_window_smoke
# frames 7 ticks 5 size 960x640 gpu 0   (size is in pixels: twice the points on Retina)
```

## 4. Try the apps

Run these from the repository root, in Terminal, one at a time. Close each
window with its close button or Cmd+W. Cmd+Q asks every window to close.

**C: a slider and a button**

```sh
./build/examples/bui_kit_hello
```

Drag the slider and click the button: the terminal prints the values.

**Python: every Blender component**

```sh
python3.12 examples/components_demo.py
```

This is the main test app. It finds the Python extension in `build/` by itself.

**Python: an unchanged tkinter app**

```sh
PYTHONPATH=bindings/python python3.12 -m bui.tksyntax examples/tk_unchanged_app.py
```

Compare with the real Tk: `python3.12 examples/tk_unchanged_app.py`. Also open
the app's file dialog: it is Blender's File Browser in a BUI window. Check that
the Volumes list on the left shows your disks (from `/Volumes`) and that the
Home, Desktop, Documents, Downloads, Movies, Pictures and Music folders are listed.

**Rust**

```sh
cd bindings/rust
cargo test -p blenderui
cargo run --release -p blenderui --example hello
cargo run --release -p blenderui --example showcase
cargo run --release -p blenderui-imgui --example demo
cd ../..
```

The first build compiles the C kit through CMake, so it takes a few minutes.

**Dear ImGui (C++)**

```sh
./build/bindings/imgui/bui_imgui_demo
```

**Light theme.** Run any of the above with `BUI_THEME=light`:

```sh
BUI_THEME=light python3.12 examples/components_demo.py
```

## 5. What to check

Use `components_demo.py` for most of these. On macOS, **Command works as
Blender's Ctrl** (Cmd+C, Cmd+V, Cmd+Z, Cmd+wheel) and **Option is Alt**.

- [ ] **Window opens** in front, with a Dock icon and a menu bar (the app menu
      and a Window menu). Text and icons are sharp, not blurry.
- [ ] **Retina:** widgets are the same size in points as on a 1x screen, with
      crisp edges. `BUI_UI_SCALE=1.5 python3.12 examples/components_demo.py`
      should make the whole UI 1.5 times larger in pixels.
- [ ] **Resize:** drag a window edge. The contents redraw *while* you drag, with
      no stretched or black frames. Try the green button (full screen), then
      leave full screen. Try Window > Zoom and Cmd+M (minimize), then restore it.
- [ ] **Multiple monitors:** drag the window between a Retina and a non-Retina
      display. The UI should re-render sharp at the new scale when it lands.
- [ ] **Trackpad scrolling:** two-finger scroll in lists and scroll views. It
      should feel smooth and not too fast. Horizontal swipes scroll horizontally
      (BUI treats them as Shift+wheel). After a flick, press a key: the leftover
      momentum must not keep scrolling or turn into Cmd+scroll.
- [ ] **Mouse wheel** (if you have a mouse): one step per notch.
- [ ] **Drag to scrub:** press and drag a number field or slider sideways. The
      pointer hides and the value changes smoothly. Keep dragging far past the
      edge of the window: BUI moves the hidden pointer back inside (a warp), so
      the value keeps changing and never gets stuck. Release: the pointer
      reappears where the drag started (on a slider, at the new value). Watch
      for jumps in the value right after each warp.
- [ ] **Typing:** click a number field and type `2*pi`, Return. Type in text
      fields, including Shift, Option characters (Option+e then e should give
      `é` if the layout allows it; report what happens) and emoji from the
      Character Viewer (Ctrl+Cmd+Space). Arrow keys, Home/End (Fn+arrows),
      Backspace and Forward Delete (Fn+Backspace).
- [ ] **Clipboard:** Cmd+C on a value, Cmd+V into another; copy text from BUI
      and paste it into TextEdit, and the other way round.
- [ ] **Tooltips:** hover a button for about half a second; the tooltip appears
      and goes away when the pointer moves.
- [ ] **Menus:** the File and Edit menus in the demo: hover to switch, type to
      search, submenus. Right-click (two-finger click on a trackpad) a value for
      its context menu. Press **Q** for the pie menu.
- [ ] **Cursors:** the pointer turns into an I-beam over text fields and into
      resize arrows over splitters and panel edges.
- [ ] **Color picker:** click a color swatch; drag in the wheel; try the eyedropper.
- [ ] **Focus:** click another app and back; keys go to the BUI window again.
      If an app has two windows (the tkinter app's dialogs), closing one makes
      the other one take the keyboard.
- [ ] **Quit:** Cmd+Q closes the app cleanly (no crash report).

## 6. Collect logs and screenshots

- **Terminal output:** run the app with its output saved:
  `python3.12 examples/components_demo.py 2>&1 | tee bui-log.txt`.
- **Screenshots:** Cmd+Shift+4, then Space, then click the window. For a
  problem with motion (scrolling, scrubbing, resizing), a screen recording
  helps the most: Cmd+Shift+5.
- **Off-screen renders** to compare with Windows and Linux pixel for pixel:
  `./build/bindings/imgui/bui_imgui_demo --screenshot demo.png` (also with
  `BUI_THEME=light` or `BUI_UI_SCALE=2` in front).
- **Frame timings (Rust apps):** `BUI_PROFILE=1 cargo run --release -p blenderui --example showcase`
  prints averaged per-phase frame times to the terminal. (In the C window host
  `BUI_PROFILE` is only implemented on Windows; `BUI_GPU_DEBUG` reports on the GPU
  renderers of Windows and Linux and prints nothing on macOS, which has none.)
- **Crashes:** macOS writes a report to `~/Library/Logs/DiagnosticReports/`
  (a `.ips` file named after the program, for example `python3.12-…ips` or
  `bui_kit_hello-…ips`). Attach it.
- **Failing tests:** attach the whole output of
  `ctest --test-dir build --output-on-failure`.

## 7. Known limitations

- **No GPU renderer on macOS** (no Metal backend). `BUI_GPU=1`,
  `BK_window_set_gpu` and `buipy.set_gpu` are ignored; everything draws on the CPU.
  Big windows on 5K displays may feel slower than on Windows.
- **Node.js and React** (`@blender-ui/native`, `@blender-ui/react`) run headless on
  macOS: BUI runs on its own thread under Node, and AppKit only allows windows on
  the main thread. The QuickJS runtime (`bui-runtime`, apps made with `bui build`)
  has the same limitation.
- **Windows only from the main thread**, and only in a GUI session: over `ssh`,
  BUI runs headless (`bui_window_smoke` is skipped).
- **No input method (IME) support yet:** composed input such as Japanese or
  Chinese, and possibly dead-key accents, is not expected to work.
- Fonts are Blender's (Inter, drawn with FreeType like Blender), not the system font.
- Python: when tkinter is importable, BUI starts a hidden Tk first (Tk insists on
  creating the application object). `BUI_MAC_TK=0` skips that; `BUI_HOST=tk`
  draws BUI inside a Tk window instead of a native one, for comparison.

## 8. Report your results

Open an issue at <https://github.com/ppmpreetham/BUI/issues> titled
**"macOS test report: <your Mac>"** (or one issue per bug) with:

```text
Commit:            (git rev-parse --short HEAD)
macOS version:     (Apple menu > About This Mac, e.g. 15.1)
Mac:               (e.g. MacBook Air M2 / iMac 2019 Intel)
Displays:          (built-in Retina; external monitors and their scaling)
Input:             (trackpad / Magic Mouse / wheel mouse)
Keyboard layout:   (e.g. U.S., German)
Tools:             clang --version | cmake --version | python3.12 --version | cargo --version

Build:             OK / failed (attach the error)
ctest:             N/N passed (attach the output if not all passed)

Checklist (section 5): what worked, what didn't
For each problem:  what you did, what you expected, what happened,
                   screenshot or recording, terminal log, crash report
```

Even "everything worked" is a useful report. Thank you!
