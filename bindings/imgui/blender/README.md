# Blender widgets for Dear ImGui (`BUI_IMGUI_BLENDER_WIDGETS`)

A build of Dear ImGui on BUI in which ImGui's widgets **are** Blender's widgets: drawn by
the BUI kit with the same code as its native widgets (the `interface_widgets.cc` port in
`kit/intern/kit_draw.c`, Blender's theme, Inter text), and handled with Blender's
interaction rules (`interface_handlers.cc`) where Blender behaves differently. The whole
Dear ImGui API keeps working, so the C, Python and Rust bindings get Blender widgets with
no change.

```sh
cmake -S . -B build -DBUI_IMGUI_BLENDER_WIDGETS=ON      # default OFF
cargo build -p blenderui-imgui --features blender-widgets
```

At runtime `ImGui_ImplBUI_SetBlenderWidgets(false)` (C `cImGui_ImplBUI_SetBlenderWidgets`,
Python `imgui.set_blender_widgets`, Rust `set_blender_widgets`) goes back to ImGui's own
drawing and interaction; `ImGui_ImplBUI_HasBlenderWidgets()` tells whether the build has them.

## How it works

- `imgui_widgets_blender.cpp` is a copy of `extern/imgui/imgui_widgets.cpp` (1.93.1 WIP)
  that CMake compiles **instead of** it: same functions, same ABI. Every edit is marked
  `BUI:`. ImGui's item / ID / layout / navigation logic and its text-editing engine are
  untouched; the rendering of each widget becomes a draw-list callback and, for number
  fields, sliders and checkboxes, the mouse interaction follows Blender.
- A widget is emitted as `ImDrawList::AddCallback(ImGui_ImplBUI_DrawCallback_Widget, data)`
  where `data` is a `BKWidgetDraw` (kind, rect, state: hover / pressed / selected /
  disabled / editing / arrow-zone hover / mixed / open, value or fill factor, corners for
  aligned groups, color, text rect) followed by its texts. The BUI backend
  (`imgui_impl_bui.cpp`) runs callbacks in draw order while rendering and calls the kit's
  `BK_draw_widget(raster, &w)` (`kit/intern/kit_draw.c`), clipped to the command's clip rect.
- Text in widgets (labels, values, `ImGui::Text`, tree / menu / tab labels, an idle text
  field) is drawn by BUI's text, as the kit's widgets draw theirs, when ImGui uses Blender's
  font (Inter) at Blender's size; other fonts or sizes keep ImGui's own glyphs.
- The draw data hash includes the callback data, so the backend's "redraw only on change"
  still works; the GPU renderers draw the same as the software raster (text antialiasing
  aside, as for every kit widget).

`ctest -R imgui_blender_widgets_match_kit` (`examples/imgui_blender_compare.cpp`) renders
the kit's native checkbox, number field, number slider, aligned vector, text field, button,
menu button, progress bar and color swatch next to the same widgets through ImGui and
requires the two columns to be **the same pixels** (0 differ at scale 1).

## Mapping

| ImGui | Blender widget (kit drawing) | Interaction |
|---|---|---|
| `Button`, `SmallButton` | push button (`wcol_regular`, `button_draw`), label centered | ImGui's (press on release) |
| `ArrowButton` | push button with a triangle | ImGui's |
| `InvisibleButton` | nothing drawn | ImGui's |
| `Checkbox`, `CheckboxFlags` | option box + label (`wcol_option`, `checkbox_draw`), dash when mixed | Blender: toggles on **press**; **drag-toggle**: drag over other checkboxes to give them the same state |
| `RadioButton` | option widget, round, with a dot (Blender has no round radio; its "radio" is a segmented row) | ImGui's |
| `Selectable` | list row (`wcol_list_item`); in popups / menus a menu row (`wcol_menu_item`); in menu bars a pulldown (`wcol_pulldown`) | ImGui's |
| `DragFloat/Int/Scalar(N)`, `DragFloatRange2`, `DragIntRange2` | number field (`wcol_num`, `number_draw`): step arrows on hover, value centered | Blender (`ui_do_but_NUM`): drag past 0.25 U scrubs `v_speed` per pixel, **Shift** 1/10 the speed, **Ctrl** snaps (Ctrl+Shift finer), click on an arrow steps by `v_speed` (ints: 1), click elsewhere / Ctrl+click types, Escape reverts; horizontal-arrows cursor |
| `SliderFloat/Int/Scalar(N)`, `SliderAngle` | number slider (`wcol_numslider`, `slider_draw`): fill bar, value inside | Blender (`ui_do_but_SLI`): drag is **relative** to the press (no jump to the pointer), **Shift** 1/20 the speed, **Ctrl** snaps, click / Ctrl+click types |
| `VSliderFloat/Int/Scalar` | ImGui's vertical slider (Blender has none), Blender colors | ImGui's |
| `InputText`, `InputTextWithHint`, `InputFloat/Int/Double/Scalar(N)` | text field (`wcol_text`, `text_draw`; while typing the `text_sel` look), text 0.4 U from the edge; +/- are Blender buttons | ImGui's text engine |
| `InputTextMultiline` | text field backdrop | ImGui's |
| `Combo`, `BeginCombo` | menu button (`wcol_menu`, the pulldown arrows, preview left) ; popup: Blender menu backdrop with its shadow (`wcol_menu_back`) + menu rows | ImGui's |
| `ColorButton`, `ColorEdit3/4` | color swatch (`swatch_draw`: opaque left half, checker under the alpha half); components as an aligned row of number fields | Blender number fields |
| `ColorPicker3/4` | ImGui's picker (Blender colors) — see gaps | ImGui's |
| `ProgressBar` | progress bar (`wcol_progress`), text centered | — |
| `CollapsingHeader`, framed `TreeNode` | panel header (`panel_draw`: arrow icon, title with the soft shadow) | ImGui's |
| `TreeNode` | tree row (`tree_draw_row`): disclosure triangle icon, list-row hover / selection | ImGui's |
| `BeginTabBar`, `BeginTabItem` | tabs (`wcol_tab`, `tabs_draw`): the current one `inner_sel` with `text_sel`, others `text` | ImGui's |
| `BeginMenuBar`, `BeginMenu` | pulldowns in the bar; menu: Blender menu backdrop with shadow, menu rows, submenu arrow | ImGui's |
| `MenuItem` | menu row (hover highlight, shortcut at 55% text alpha), `selected` shows Blender's menu check box | ImGui's |
| `Separator`, `SeparatorText` | ImGui's line in the Blender separator color (text at 15%) | — |
| `Text`, `LabelText`, `BulletText`, `TextWrapped`, ... | BUI's text (Inter), ImGui's wrapping | — |
| windows, title bars, tooltips, popups from `BeginPopup`, tables, scrollbars, plots | ImGui's, in the Blender style (`ImGui_ImplBUI_StyleBlender`) | ImGui's |

"Label: value" formats: a number field or slider whose format has a `:` (e.g. `"Width: %.0f"`,
ColorEdit's `"R:%3d"`) shows Blender's way: the label on the left, the value on the right.
Other formats are centered. The `label` argument stays outside on the right, where ImGui
lays it out (decided so that layouts, `CalcItemWidth` and label columns keep working).

Aligned groups: the components of `DragFloat2/3/4`, `DragInt*`, `SliderFloat2/3/4`,
`SliderInt*`, `Drag*Range2` and `ColorEdit` are joined into one rounded row as Blender
aligns vectors (no gap, inner corners square), instead of ImGui's `ItemInnerSpacing` gaps.

## Differences from ImGui's widgets (behavior)

- Sliders: a press does not move the value; dragging changes it relative to where the
  press was; a click without a drag types a value. ImGui jumps to the pointer on press.
- Number fields (`Drag*`): a click without a drag types a value (ImGui needs Ctrl+click,
  double-click, or `io.ConfigDragClickToInputText`); the arrow zones step; the drag begins
  after 0.25 widget units (ImGui: half `io.MouseDragThreshold`); **Shift** is 10x *slower*
  (Blender precision; ImGui: Shift faster, Alt slower) and **Ctrl** snaps while dragging.
  A quick second press is a new drag, not a double-click.
- Checkboxes toggle on press and support drag-toggle (ImGui toggles on release).
- Logarithmic sliders / drags, reversed slider ranges, keyboard and gamepad keep ImGui's
  interaction (drawn as Blender widgets).
- Text fields place their text 0.4 widget units from the left edge (Blender's
  `UI_TEXT_MARGIN_X`), not `FramePadding.x`.
- `MenuItem` check marks are Blender's check box in ImGui's mark column (Blender puts the
  box on the left).

## Gaps

- Blender's vertical multi-value drag (drag down over fields, then sideways) and Alt
  (edit all linked fields) are not implemented; the cursor is not hidden / warped while
  scrubbing.
- Code in `imgui.cpp` / `imgui_tables.cpp` is not copied: window title bars and frames,
  tooltips, `BeginPopup` / context-menu backgrounds (only combos and `BeginMenu` menus get
  Blender's menu backdrop), table headers and window titles keep ImGui's rendering in the
  Blender style and ImGui's glyphs.
- `ColorPicker3/4` (Blender's HSV circle is not ported to the kit), `VSlider*`, `PlotLines` /
  `PlotHistogram`, `ImageButton`, `Bullet` and `TextLink` keep ImGui's drawing.
- Text being typed uses ImGui's glyphs (its cursor and selection are measured with them).
- Indeterminate `ProgressBar` (negative fraction) shows the moving end of ImGui's segment.

## Keeping the copy current

`rebase.py` (stdlib Python + git):

```sh
# after copying a newer Dear ImGui into extern/imgui, before committing it:
py -3 bindings/imgui/blender/rebase.py merge   # 3-way merge old upstream -> new, onto the copy
py -3 bindings/imgui/blender/rebase.py check   # every changed hunk carries a BUI: mark
py -3 bindings/imgui/blender/rebase.py diff    # the BUI edits as a unified diff
```

Then build with `-DBUI_IMGUI_BLENDER_WIDGETS=ON` and run `ctest` (`imgui_backend`,
`imgui_c_api`, `imgui_blender_widgets_match_kit`, `buipy_imgui`).
