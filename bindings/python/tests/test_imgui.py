"""Dear ImGui on BUI from Python (blenderui.imgui / bui.imgui), headless.

Frames call a broad set of widgets; simulated input (clicks, drags, typing, keys, wheel)
changes the values the pyimgui-style functions return; structs (IO, style, draw lists,
helpers) are readable and writable; overloads dispatch; errors in the frame propagate.

Run standalone (PYTHONPATH must contain the built extensions and bindings/python) or via
ctest (buipy_imgui).

SPDX-License-Identifier: GPL-2.0-or-later
"""

import glob
import math
import os
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

import bui  # noqa: E402,F401  (locates the build tree)
import buipy  # noqa: E402
import blenderui.imgui as imgui  # noqa: E402
import bui.imgui as bui_imgui  # noqa: E402

W, H = 1280, 800


def center(r):
    (x0, y0), (x1, y1) = r
    return (x0 + x1) / 2, (y0 + y1) / 2


def item_rect():
    return imgui.get_item_rect_min(), imgui.get_item_rect_max()


class Widgets:
    """The UI under test: one window of widgets, state in attributes."""

    def __init__(self):
        self.checked = False
        self.slider = 0.25
        self.slider_w = 0.0
        self.text = ""
        self.clicks = 0
        self.combo = 0
        self.color = (0.2, 0.4, 0.6, 1.0)
        self.drag = 1.0
        self.ints = (1, 2, 3)
        self.radio = 0
        self.selected = False
        self.opened = True
        self.frames = 0
        self.rects = {}
        self.table_cells = 0
        self.clipped = 0
        self.tree_open = False
        self.extra = None
        self.scroll_y = 0.0
        self.clipper = imgui.ImGuiListClipper()

    def __call__(self):
        self.frames += 1
        imgui.set_next_window_pos((10, 10), imgui.COND_ALWAYS)
        imgui.set_next_window_size((560, 780), imgui.COND_ALWAYS)
        expanded, self.opened = imgui.begin("Python", self.opened, imgui.WINDOW_NO_MOVE | imgui.WINDOW_NO_RESIZE
                                            | imgui.WINDOW_NO_SAVED_SETTINGS)
        imgui.text("Hello from Python %d" % 42)
        imgui.text_colored((1.0, 0.5, 0.0, 1.0), "colored")
        imgui.text_disabled("disabled")
        imgui.text_wrapped("wrapped " * 6)
        imgui.label_text("label", "value")
        imgui.bullet_text("bullet")
        imgui.separator_text("Widgets")

        _, self.checked = imgui.checkbox("Check", self.checked)
        self.rects["check"] = item_rect()
        self.slider_w = imgui.calc_item_width()
        _, self.slider = imgui.slider_float("Slider", self.slider, 0.0, 1.0)
        self.rects["slider"] = item_rect()
        _, self.text = imgui.input_text("Text", self.text)
        self.rects["input"] = item_rect()
        if imgui.button("Button"):
            self.clicks += 1
        self.rects["button"] = item_rect()
        imgui.same_line()
        imgui.small_button("small")
        imgui.same_line()
        imgui.arrow_button("arrow", imgui.DIR_RIGHT)
        _, self.combo = imgui.combo("Combo", self.combo, ["one", "two", "three"])
        _, self.color = imgui.color_edit4("Color", self.color)
        _, self.drag = imgui.drag_float("Drag", self.drag, 0.1)
        _, self.ints = imgui.input_int3("Ints", self.ints)
        _, self.radio = imgui.radio_button_int_ptr("A", self.radio, 0)
        imgui.same_line()
        _, self.radio = imgui.radio_button_int_ptr("B", self.radio, 1)
        clicked, self.selected = imgui.selectable_bool_ptr("selectable", self.selected)
        imgui.progress_bar(self.slider, (-1, 0), "progress")
        imgui.plot_lines("Plot", [math.sin(i * 0.3) for i in range(32)])
        imgui.plot_histogram("Hist", [abs(math.sin(i * 0.3)) for i in range(16)])
        imgui.separator()

        self.tree_open = imgui.tree_node("Tree")
        if self.tree_open:
            imgui.text("inside")
            imgui.tree_pop()
        if imgui.collapsing_header("Header"):
            imgui.text("body")
        if imgui.begin_tab_bar("tabs"):
            if imgui.begin_tab_item("Tab 1")[0]:
                imgui.text("tab one")
                imgui.end_tab_item()
            if imgui.begin_tab_item("Tab 2")[0]:
                imgui.end_tab_item()
            imgui.end_tab_bar()
        if imgui.begin_table("table", 3, imgui.TABLE_BORDERS):
            for name in ("a", "b", "c"):
                imgui.table_setup_column(name)
            imgui.table_headers_row()
            self.table_cells = 0
            for r in range(3):
                imgui.table_next_row()
                for c in range(3):
                    imgui.table_set_column_index(c)
                    imgui.text("%d,%d" % (r, c))
                    self.table_cells += 1
            imgui.end_table()

        # overloads: one name tries str then int; the variants also have their own names
        imgui.push_id("str id")
        imgui.push_id(7)
        imgui.push_id_int(8)
        imgui.pop_id()
        imgui.pop_id()
        imgui.pop_id()

        dl = imgui.get_window_draw_list()
        p = imgui.get_cursor_screen_pos()
        dl.add_rect_filled(p, (p.x + 50, p.y + 20), imgui.im_col32(71, 114, 179))
        dl.add_line(p, (p.x + 50, p.y + 20), imgui.IM_COL32_WHITE if hasattr(imgui, "IM_COL32_WHITE") else 0xFFFFFFFF, 2.0)
        dl.add_circle((p.x + 70, p.y + 10), 8.0, 0xFFFFFFFF)
        dl.add_text((p.x + 90, p.y), 0xFFFFFFFF, "draw list")
        dl.add_polyline([(p.x, p.y), (p.x + 10, p.y + 15), (p.x + 20, p.y)], 0xFF00FF00, 1.5, 0)
        imgui.dummy((120, 24))

        imgui.begin_child("child", (0, 160), imgui.CHILD_BORDERS)
        self.clipper.begin(1000)
        self.clipped = 0
        while self.clipper.step():
            for i in range(self.clipper.display_start, self.clipper.display_end):
                imgui.text("row %d" % i)
                self.clipped += 1
        self.clipper.end()
        self.scroll_y = imgui.get_scroll_y()
        imgui.end_child()
        self.rects["child"] = item_rect()

        if self.extra:
            self.extra()
        imgui.end()


class ImGuiTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        imgui.set_ui_scale(1.0)
        cls.ui = Widgets()
        cls.app = imgui.HeadlessApp(cls.ui, size=(W, H))
        cls.app.render(4)

    @classmethod
    def tearDownClass(cls):
        cls.app.close()

    def click(self, name, fx=0.5):
        (x0, y0), (x1, y1) = self.ui.rects[name]
        self.app.click(x0 + (x1 - x0) * fx, (y0 + y1) / 2)

    # -- module surface -------------------------------------------------------------

    def test_module(self):
        self.assertIs(bui_imgui, imgui)
        self.assertEqual(imgui.get_version(), imgui.IMGUI_VERSION)
        self.assertEqual(imgui.WINDOW_NO_TITLE_BAR, imgui.ImGuiWindowFlags_NoTitleBar)
        self.assertEqual(imgui.KEY_A, imgui.ImGuiKey_A)
        self.assertEqual(imgui.COL_TEXT, 0)
        import _buiimgui  # the module buipy.imgui_module() creates, also registered by name
        self.assertIs(_buiimgui, imgui._buiimgui)
        self.assertGreater(_buiimgui._sizeof("ImGuiIO"), 1000)
        self.assertIn("ImGui::Button", imgui.button.__doc__)
        self.assertEqual(imgui.im_col32(255, 0, 0), 0xFF0000FF)

    def test_frames_ran(self):
        self.assertGreater(self.ui.frames, 0)
        self.assertGreater(self.app.frame_count, 0)
        self.assertTrue(self.app.headless)
        self.assertEqual(self.ui.table_cells, 9)
        self.assertTrue(0 < self.ui.clipped < 1000)

    # -- input changes values -------------------------------------------------------

    def test_checkbox_click(self):
        before = self.ui.checked
        self.click("check")
        self.assertEqual(self.ui.checked, not before)

    def test_button_click(self):
        n = self.ui.clicks
        self.click("button")
        self.assertEqual(self.ui.clicks, n + 1)

    def slider_click(self, fx, typed):
        (x0, y0), (x1, y1) = self.ui.rects["slider"]
        self.app.click(x0 + self.ui.slider_w * fx, (y0 + y1) / 2)
        if imgui.get_blender_widgets():
            # variant (Blender widgets): a click types a value, no jump to the pointer
            self.app.key("a", imgui.BK_CTRL)
            self.app.type_text(typed)
            self.app.key("Return")

    def test_slider_click(self):
        self.slider_click(0.9, "0.9")
        self.assertGreater(self.ui.slider, 0.8)
        self.slider_click(0.1, "0.1")
        self.assertLess(self.ui.slider, 0.2)

    def test_slider_drag(self):
        (x0, y0), (x1, y1) = self.ui.rects["slider"]
        y = (y0 + y1) / 2
        before = self.ui.slider
        self.app.mouse_down(x0 + self.ui.slider_w * 0.2, y)
        self.app.mouse_move(x0 + self.ui.slider_w * 0.5, y)
        self.app.mouse_move(x0 + self.ui.slider_w * 0.7, y)
        self.app.mouse_up(x0 + self.ui.slider_w * 0.7, y)
        # variant (Blender widgets): relative to the value at the press (half the width: +0.5)
        expected = min(1.0, before + 0.5) if imgui.get_blender_widgets() else 0.7
        self.assertAlmostEqual(self.ui.slider, expected, delta=0.06)

    def test_blender_widgets_switch(self):
        self.assertIsInstance(imgui.has_blender_widgets(), bool)
        self.assertEqual(imgui.get_blender_widgets(), imgui.has_blender_widgets())

    def test_typing(self):
        self.click("input")
        self.app.type_text("Hello Py")
        self.assertEqual(self.ui.text, "Hello Py")
        self.app.key("BackSpace")
        self.assertEqual(self.ui.text, "Hello P")
        # longer than the initial buffer: the buffer grows (CallbackResize)
        self.app.type_text("y" * 100)
        self.assertEqual(self.ui.text, "Hello P" + "y" * 100)
        self.app.key("a", imgui.BK_CTRL)
        self.app.key("Delete")
        self.assertEqual(self.ui.text, "")
        self.app.key("Return")

    def test_wheel_scrolls_child(self):
        x, y = center(self.ui.rects["child"])
        self.app.scroll(x, y, 0, -3)
        self.app.render(4)
        self.assertGreater(self.ui.scroll_y, 0.0)

    # -- values and structs ----------------------------------------------------------

    def test_io_and_style(self):
        io = imgui.get_io()
        self.assertEqual(tuple(io.display_size), (float(W), float(H)))
        self.assertEqual(io.DisplaySize.x, float(W))   # C++ names work too
        self.assertIsNone(io.ini_filename)
        io.config_flags |= imgui.CONFIG_NAV_ENABLE_KEYBOARD
        self.assertTrue(io.config_flags & imgui.CONFIG_NAV_ENABLE_KEYBOARD)
        style = imgui.get_style()
        bg = style.colors[imgui.COL_WINDOW_BG]
        self.assertAlmostEqual(bg.x, 0x30 / 255, places=2)   # Blender's region background
        self.assertGreater(style.frame_rounding, 0)
        old = style.colors[imgui.COL_BUTTON]
        style.colors[imgui.COL_BUTTON] = (1, 0, 0, 1)
        self.assertEqual(tuple(style.colors[imgui.COL_BUTTON]), (1.0, 0.0, 0.0, 1.0))
        style.colors[imgui.COL_BUTTON] = old
        self.assertEqual(len(style.colors), imgui.COL_COUNT)
        atlas = io.fonts
        self.assertGreaterEqual(len(atlas.fonts), 1)
        self.assertIsInstance(atlas.fonts[0], imgui.ImFont)
        vp = imgui.get_main_viewport()
        self.assertEqual(tuple(vp.size), (float(W), float(H)))

    def test_helpers(self):
        h, s, v = imgui.color_convert_rgb_to_hsv(1.0, 0.0, 0.0)
        self.assertAlmostEqual(h, 0.0)
        self.assertAlmostEqual(s, 1.0)
        self.assertAlmostEqual(v, 1.0)
        r, g, b = imgui.color_convert_hsv_to_rgb(0.0, 1.0, 1.0)
        self.assertAlmostEqual(r, 1.0)
        self.assertEqual(imgui.color_convert_float4_to_u32((1, 0, 0, 1)), 0xFF0000FF)
        c = imgui.color_convert_u32_to_float4(0xFF00FF00)
        self.assertEqual((c.x, c.y, c.z, c.w), (0.0, 1.0, 0.0, 1.0))
        self.assertEqual(imgui.get_key_name(imgui.KEY_A), "A")
        size = imgui.calc_text_size("Hello")
        self.assertGreater(size.x, 10)
        col = imgui.ImColor.hsv(0.0, 1.0, 1.0)
        self.assertAlmostEqual(col.value.x, 1.0)

    def test_owned_objects(self):
        f = imgui.ImGuiTextFilter("two")
        self.assertTrue(f.pass_filter("one two"))
        self.assertFalse(f.pass_filter("three"))
        f.input_buf = "three"
        f.build()
        self.assertTrue(f.pass_filter("three"))
        st = imgui.ImGuiStorage()
        st.set_int(7, 42)
        self.assertEqual(st.get_int(7), 42)
        self.assertEqual(st.get_int(8, 5), 5)
        tb = imgui.ImGuiTextBuffer()
        tb.append("abc")
        self.assertEqual(tb.c_str(), "abc")
        self.assertEqual(tb.size(), 3)
        self.assertIn("owned", repr(tb))

    def test_widgets_more(self):
        seen = {}

        def extra():
            seen["menu"] = imgui.begin_menu_bar()
            if imgui.begin_popup_context_item("ctx"):
                imgui.end_popup()
            imgui.set_item_tooltip("tooltip %s" % "x")
            _, v = imgui.slider_int("SliderInt", 3, 0, 10)
            seen["slider_int"] = v
            _, seen["vec"] = imgui.drag_float2("Drag2", (1.0, 2.0))
            _, seen["d"] = imgui.input_double("Double", 1.5)
            _, seen["multi"] = imgui.input_text_multiline("Multi", "a\nb")
            _, seen["hint"] = imgui.input_text_with_hint("Hint", "hint", "")
            imgui.color_button("cb", (1, 0, 0, 1))
            seen["listbox"] = imgui.list_box("List", 1, ["x", "y", "z"])
            seen["avail"] = imgui.get_content_region_avail()
            seen["id"] = imgui.get_id("x")
            seen["id_int"] = imgui.get_id_int(5)
            imgui.image(0, (16, 16))
            imgui.begin_disabled()
            imgui.button("disabled")
            imgui.end_disabled()
            imgui.push_style_color(imgui.COL_TEXT, (1, 0, 0, 1))
            imgui.push_style_var(imgui.STYLE_VAR_FRAME_ROUNDING, 0.0)
            imgui.push_style_var(imgui.STYLE_VAR_ITEM_SPACING, (4, 4))  # dispatches to push_style_var_vec2
            imgui.text("styled")
            imgui.pop_style_var(2)
            imgui.pop_style_color()
            imgui.push_item_width(100)
            imgui.v_slider_float("v", (20, 60), 0.5, 0, 1)
            imgui.pop_item_width()
            imgui.new_line()
            imgui.indent()
            imgui.unindent()
            imgui.spacing()
            seen["hovered"] = imgui.is_item_hovered()

        self.ui.extra = extra
        try:
            self.app.render(3)
        finally:
            self.ui.extra = None
        self.assertEqual(seen["slider_int"], 3)
        self.assertEqual(tuple(seen["vec"]), (1.0, 2.0))
        self.assertEqual(seen["multi"], "a\nb")
        self.assertEqual(seen["listbox"], (False, 1))
        self.assertNotEqual(seen["id"], seen["id_int"])
        self.assertGreater(seen["avail"].x, 0)

    def test_popup_and_demo(self):
        state = {"open": False}

        def extra():
            if not state["open"]:
                imgui.open_popup("pop")
                state["open"] = True
            if imgui.begin_popup("pop"):
                state["inside"] = True
                imgui.text("popup")
                imgui.end_popup()
            imgui.show_demo_window()
            imgui.show_metrics_window()
            imgui.show_style_editor()

        self.ui.extra = extra
        try:
            self.app.render(4)
        finally:
            self.ui.extra = None
        self.assertTrue(state.get("inside"))

    def test_frame_error_propagates(self):
        def boom():
            raise ValueError("boom")

        app = None
        ctx = imgui.get_current_context()
        other = imgui.create_context()   # (does not change the current context)
        imgui.set_current_context(other)
        try:
            imgui.get_io().ini_filename = None
            app = imgui.App(boom, "", 200, 100, True)
            with self.assertRaises(ValueError):
                app.render(2)
        finally:
            if app is not None:
                app.destroy()
            imgui.destroy_context(other)
            imgui.set_current_context(ctx)

    def test_pixels(self):
        self.app.render(1)
        w, h, px = self.app.pixels()
        self.assertEqual((w, h), (W, H))
        self.assertEqual(len(px), W * H * 4)
        grey = sum(1 for v in memoryview(px).cast("I") if v & 0xFFFFFF == 0x303030)
        self.assertGreater(grey, 10000)

    def test_type_errors(self):
        def bad():
            imgui.button(123)

        self.ui.extra = bad
        try:
            with self.assertRaises(TypeError):
                self.app.render(2)
        finally:
            self.ui.extra = None
            self.app.render(2)


class OneKitTest(unittest.TestCase):
    """Dear ImGui is compiled into buipy: a process holds ONE copy of the kit.

    Two extensions linking the kit statically registered the Cocoa classes of
    kit_window_cocoa.m twice on macOS ("Class BUIKitView is implemented in both ...") and
    kept two copies of the kit's state elsewhere."""

    def test_no_second_extension(self):
        here = os.path.dirname(os.path.abspath(buipy.__file__))
        self.assertEqual(glob.glob(os.path.join(here, "_buiimgui*")), [])

    def test_kit_state_is_shared(self):
        # a second kit would keep its own theme: setting it through one is seen through the other
        was = buipy.kit_theme_name()
        other = "light" if was == "dark" else "dark"
        try:
            imgui.set_theme(other)
            self.assertEqual(buipy.kit_theme_name(), other)
            bui.set_theme(was)
            self.assertEqual(imgui.theme_name(), was)
        finally:
            buipy.kit_set_theme(was)

    def test_loading_both_registers_nothing_twice(self):
        code = "import buipy, blenderui.imgui, sys; sys.exit(0 if 'imgui_module' in dir(buipy) else 1)"
        env = dict(os.environ, PYTHONPATH=os.pathsep.join(sys.path))
        r = subprocess.run([sys.executable, "-c", code], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("is implemented in both", r.stderr)  # the Objective-C runtime's warning

    @unittest.skipUnless(sys.platform == "darwin", "Cocoa host classes")
    def test_cocoa_classes_come_from_buipy(self):
        import ctypes
        objc = ctypes.CDLL("/usr/lib/libobjc.A.dylib")
        objc.objc_getClass.restype = ctypes.c_void_p
        objc.objc_getClass.argtypes = [ctypes.c_char_p]
        objc.class_getImageName.restype = ctypes.c_char_p
        objc.class_getImageName.argtypes = [ctypes.c_void_p]
        for name in (b"BUIKitNSWindow", b"BUIKitView", b"BUIKitWindowDelegate", b"BUIKitAppDelegate"):
            cls = objc.objc_getClass(name)
            self.assertTrue(cls, name)
            self.assertEqual(os.path.realpath(objc.class_getImageName(cls).decode()),
                             os.path.realpath(buipy.__file__), name)


if __name__ == "__main__":
    unittest.main(verbosity=2)
