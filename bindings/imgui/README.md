# Dear ImGui on BUI

The real [Dear ImGui](https://github.com/ocornut/imgui) running on BUI: a platform and
renderer backend (`imgui_impl_bui.h/.cpp`), Blender's look (`imgui_bui_style.cpp`) and
Blender's UI font (Inter, embedded in BUI).

- **Vendored version:** Dear ImGui **1.93.1 WIP** (`IMGUI_VERSION_NUM 19302`), `master` at
  commit `8c8b8fc266e8cd7fac09340c0373abc28732adb0` (2026-11-04), MIT license
  (`extern/imgui/LICENSE.txt`). Only the core files, `imconfig.h`, `imstb_*.h` and
  `misc/cpp/imgui_stdlib.*` are vendored; not the upstream `backends/`. Update by copying
  the same files from a newer checkout and bumping this line.
- **CMake:** `BUI_BUILD_IMGUI` (ON by default when C++ builds) gives `bui_imgui`
  (alias `bui::imgui`: ImGui core + demo + this backend + style), `bui_imgui_demo`
  (`examples/imgui_demo.cpp`) and the CTest `imgui_backend`.

- **C, Python, Rust:** generated from `imgui.h` by `gen/gen.py`
  (`py -3 -I bindings/imgui/gen/gen.py`): the C API `c/cimgui.h` (CMake `bui_cimgui`, with
  `c/cimgui_impl_bui.h` for this backend), `blenderui.imgui` for Python (`_buiimgui`) and the
  `blenderui-imgui` crate. See [docs/imgui.md](../../docs/imgui.md).

## Using it

```cpp
#include "imgui.h"
#include "imgui_impl_bui.h"

static void frame(void*) { ImGui::ShowDemoWindow(); }

int main() {
  ImGui::CreateContext();
  ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateApp("Dear ImGui on BUI", 1280, 800, frame, nullptr);
  ImGui_ImplBUI_RunApp(app);      // BUI's message loop
  ImGui_ImplBUI_DestroyApp(app);
  ImGui::DestroyContext();
}
```

Inside an existing BUI UI, ImGui lives in a kit **`custom` node** like any other widget:

```cpp
BKNode* node = BK_node_new("custom");
BK_node_add(column, node, -1);
BK_set_num(node, "fill", 1);
ImGui::SetCurrentContext(my_ctx);
ImGui_ImplBUI_Init(node, frame, user);   // the backend runs NewFrame/frame/Render itself
```

Several nodes may each host their own ImGui context. The backend runs ImGui inside
`BK_render`, so it works in every BUI host: native windows (Win32, X11, Cocoa), the
Tk/Qt hosts, and headless (`BK_render` into a `BuiRaster`, as the test does). `BUI_GPU=1`
draws it with Direct3D 11 / OpenGL 3.3 instead of the software raster.

## How it maps

| ImGui | BUI |
|---|---|
| mouse pos / buttons (left, right, middle, held together) | `BKCustomEvent` PRESS / RELEASE / MOTION (the node grabs the pointer while a button is held) |
| wheel, horizontal wheel (`AddMouseWheelEvent(-x, y)`) | `BK_scroll` precise deltas (Win32 `WM_MOUSEWHEEL` / `WM_MOUSEHWHEEL`, X11 buttons 4-7) |
| keys (`ImGuiKey_*`), modifiers | X11 keysym names in KEY / KEYUP events; Ctrl / Shift / Alt from the event, Super from its key |
| text (`AddInputCharactersUTF8`) | KEY event text |
| focus (`AddFocusEvent`) | the node takes keyboard focus on click; BLUR when another widget takes it |
| `DisplaySize` | the node's size in raster pixels; `DisplayFramebufferScale` = 1 |
| DPI | `BK_scale()`: style metrics scaled, `style.FontScaleDpi`; followed when it changes |
| `GetMouseCursor()` | `BK_custom_set_cursor` (arrow, I-beam, all resizes incl. diagonals, hand, wait, progress, not-allowed, hidden) |
| clipboard | `BK_clipboard_get/set` (the window host's clipboard, else BUI's own) |
| `ImDrawData` | `BUI_raster_mesh` per draw command, with its clip rect, `VtxOffset`, 16/32-bit indices |
| textures (`ImGuiBackendFlags_RendererHasTextures`) | `BUI_texture_create/update/free`; `ImTextureID` is the BUI texture id |
| `DrawCallback_SetSamplerNearest/Linear`, `ResetRenderState` | `BUI_MESH_NEAREST` |

**Idle.** Input queues into `ImGuiIO` and asks for a frame. After each frame the backend
asks for another only when something can still change: settle frames after input,
frames while the output keeps changing (a hash of the draw data: animations, held repeat
buttons) or ImGui still has queued input, a 30 Hz timer while an item is held, timer wakes
for the text cursor blink (two per 1.2 s period) and the hover delays (tooltips). Then no
frames run: an idle window costs no CPU. A frame whose draw data did not change does not
redraw the node.

## Blender style (`ImGui_ImplBUI_StyleBlender`)

Colors from Blender's default theme (`userdef_default_theme.c`), metrics from the widget
unit (U = 20 px at scale 1) and `uiStyle`:

| ImGuiCol | Blender theme value |
|---|---|
| Text | `wcol_regular.text` #e6e6e6 |
| TextDisabled | `wcol_menu_back.text` #999999 |
| WindowBg | region background #303030 |
| ChildBg / BorderShadow / TableRowBg | transparent |
| PopupBg | `wcol_menu_back.inner` #181818 |
| Border | `wcol_*.outline` #3d3d3d |
| FrameBg / Hovered / Active | `wcol_num.inner` #545454 / hover highlight / `wcol_num.inner_sel` #222222 |
| TitleBg / Active / Collapsed | #1d1d1d / `panel_header` #3d3d3d / #1d1d1d cc |
| MenuBarBg | #1d1d1d |
| ScrollbarBg / Grab | `wcol_scroll.inner` (transparent) / `wcol_scroll.item` #545454 (+hover) |
| CheckMark / CheckboxSelectedBg | `wcol_option.item` #ffffff / `wcol_option.inner_sel` #4772b3 |
| SliderGrab / Active | `wcol_numslider.item` #4772b3 / hover |
| Button / Hovered / Active | `wcol_regular.inner` #545454 / hover / `inner_sel` #4772b3 |
| Header / Hovered / Active | `panel_header` #3d3d3d / `wcol_menu.inner_sel` #4772b3 b3 / #4772b3 |
| Separator | text at 15% / #4772b3 b3 / #4772b3 |
| ResizeGrip | #ffffff 1a / #4772b3 b3 / #4772b3 |
| InputTextCursor | `widget_text_cursor` #71a8ff |
| Tab / Hovered / Selected | `wcol_tab.inner` #1d1d1d / #3d3d3d / `wcol_tab.inner_sel` #303030 (overline #4772b3) |
| PlotLines / PlotHistogram | #cccccc / `wcol_progress.item` #4772b3 |
| TableHeaderBg / BorderStrong / BorderLight / RowBgAlt | #3d3d3d / #1d1d1d / #3d3d3d / #ffffff 0a |
| TextLink | theme `link` #6fa9e6 |
| TextSelectedBg | selection blue #4772b3 cc |
| NavCursor, DragDropTarget | #4772b3 |
| ModalWindowDimBg | #000000 59 |

"Hover" is Blender's `widget_active_color`: HSL saturation x1.15, lightness x1.2.

Metrics (scaled by `BK_scale()`): frame height one widget unit (FramePadding.y = (20 -
font) / 2), FrameRounding / ChildRounding / PopupRounding / TabRounding = roundness 0.2 x U
= 4 px, WindowRounding = `panel_roundness` 0.4 x U / 2 = 4 px, scrollbars `V2D_SCROLL_WIDTH`
(0.45 U + 2 = 11 px) with round ends, ItemSpacing = `buttonspacex/y` (8, 2),
ItemInnerSpacing.x = `templatespace` (5), IndentSpacing = U, 1 px window/frame/popup
outlines, DisabledAlpha 0.5.

Font: Inter (BUI's embedded variable font, `BUI_text_default_font`) at Blender's UI text
size, an 11 px em at scale 1 (`UI_DEFAULT_TEXT_POINTS` at 72 dpi, as BUI's `bk_font_size`);
ImGui sizes fonts by ascent - descent, so that is 11 x 2472 / 2048 = 13.3 px
(`ImGui_ImplBUI_BlenderFontSize`).

## Known gaps

- IME candidate window placement (`Platform_SetImeDataFn`) is not forwarded; gamepads and
  multi-viewports are not supported.
- Win32 reports punctuation keys only as text, so `ImGuiKey_Comma` etc. are pressed and
  released at once there (X11 sends real key names and releases).
- Window-level focus loss is not forwarded by the kit (clicking another BUI widget is):
  held modifiers are corrected by the next input event's modifier bits.
- macOS (Cocoa) still feeds wheel steps, not precise deltas.
