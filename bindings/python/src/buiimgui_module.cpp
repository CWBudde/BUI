// buiimgui_module.cpp: the `_buiimgui` CPython extension, Dear ImGui on BUI for Python.
// Imported as blenderui.imgui / bui.imgui (bindings/python/bui/imgui.py adds run() and
// HeadlessApp). The API itself is generated from imgui.h (buiimgui_gen.inc); this file
// adds the BUI app (window or headless) and the module init.
//
// This is not a separate extension: it is compiled into `buipy` so the process holds ONE copy
// of the kit (one set of Cocoa classes, one NSApp, one theme). buipy.imgui_module() creates
// the `_buiimgui` module object on first use.
//
// SPDX-License-Identifier: GPL-2.0-or-later

#include "buiimgui_rt.h"
// the C API of the BUI backend (+ headless input helpers), kept in its own namespace: its
// C structs mirror imgui.h's
namespace cimgui {
#include "cimgui_impl_bui.h"
}

#include "buiimgui_gen.inc"

//-----------------------------------------------------------------------------
// App: a BUI window (or headless raster) running a Python frame function
//-----------------------------------------------------------------------------

struct BiApp
{
    PyObject_HEAD
    cimgui::cImGui_ImplBUI_App* app;
    PyObject* frame;
    PyObject *et, *ev, *etb; // an exception raised by the frame function
    ImGuiContext* ctx;
};

static PyTypeObject* bi_AppType = nullptr;

static void bi_app_frame(void* user)
{
    BiApp* a = (BiApp*)user;
    if (a->et != nullptr || a->frame == nullptr)
        return; // after an error: empty frames until the caller sees it
    PyObject* r = PyObject_CallNoArgs(a->frame);
    if (r == nullptr)
    {
        PyErr_Fetch(&a->et, &a->ev, &a->etb);
        cimgui::cImGui_ImplBUI_CloseApp(a->app);
        return;
    }
    Py_DECREF(r);
}

static PyObject* bi_app_raise(BiApp* a)
{
    if (a->et == nullptr)
        return nullptr;
    PyErr_Restore(a->et, a->ev, a->etb);
    a->et = a->ev = a->etb = nullptr;
    return (PyObject*)1;
}

static PyObject* bi_app_new(PyTypeObject* type, PyObject* args, PyObject* kw)
{
    static const char* kwl[] = { "frame", "title", "width", "height", "headless", "blender_style", "blender_font", "font_size", nullptr };
    PyObject* frame = nullptr;
    const char* title = "Dear ImGui on BUI";
    int width = 1280, height = 800, headless = 0, blender_style = 1, blender_font = 1;
    float font_size = 0.0f;
    if (!PyArg_ParseTupleAndKeywords(args, kw, "O|ziipppf", (char**)kwl, &frame, &title, &width, &height, &headless,
                                     &blender_style, &blender_font, &font_size))
        return nullptr;
    if (!PyCallable_Check(frame))
    {
        PyErr_SetString(PyExc_TypeError, "frame must be callable");
        return nullptr;
    }
    if (ImGui::GetCurrentContext() == nullptr)
    {
        PyErr_SetString(PyExc_RuntimeError, "no current ImGui context: call imgui.create_context() first");
        return nullptr;
    }
    BiApp* a = (BiApp*)type->tp_alloc(type, 0);
    if (a == nullptr)
        return nullptr;
    Py_INCREF(frame);
    a->frame = frame;
    a->ctx = ImGui::GetCurrentContext();
    cimgui::cImGui_ImplBUI_Config cfg = cimgui::cImGui_ImplBUI_DefaultConfig();
    cfg.BlenderStyle = blender_style != 0;
    cfg.BlenderFont = blender_font != 0;
    cfg.FontSize = font_size;
    a->app = headless ? cimgui::cImGui_ImplBUI_CreateHeadlessApp(width, height, bi_app_frame, a, &cfg)
                      : cimgui::cImGui_ImplBUI_CreateApp(title, width, height, bi_app_frame, a, &cfg);
    return (PyObject*)a;
}

static void bi_app_destroy(BiApp* a)
{
    if (a->app)
    {
        ImGuiContext* prev = ImGui::GetCurrentContext();
        ImGui::SetCurrentContext(a->ctx);
        cimgui::cImGui_ImplBUI_DestroyApp(a->app);
        a->app = nullptr;
        if (prev != a->ctx)
            ImGui::SetCurrentContext(prev);
    }
}

static void bi_app_dealloc(PyObject* self)
{
    BiApp* a = (BiApp*)self;
    PyTypeObject* tp = Py_TYPE(self);
    bi_app_destroy(a);
    Py_CLEAR(a->frame);
    Py_CLEAR(a->et);
    Py_CLEAR(a->ev);
    Py_CLEAR(a->etb);
    tp->tp_free(self);
    Py_DECREF(tp);
}

#define BI_APP(self) BiApp* a = (BiApp*)self; if (a->app == nullptr) { PyErr_SetString(PyExc_RuntimeError, "app destroyed"); return nullptr; }
#define BI_APP_DONE() if (bi_app_raise(a)) return nullptr; Py_RETURN_NONE

static PyObject* bi_app_run(PyObject* self, PyObject*)
{
    BI_APP(self);
    cimgui::cImGui_ImplBUI_RunApp(a->app);
    BI_APP_DONE();
}

static PyObject* bi_app_close(PyObject* self, PyObject*)
{
    BI_APP(self);
    cimgui::cImGui_ImplBUI_CloseApp(a->app);
    Py_RETURN_NONE;
}

static PyObject* bi_app_destroy_m(PyObject* self, PyObject*)
{
    bi_app_destroy((BiApp*)self);
    Py_RETURN_NONE;
}

static PyObject* bi_app_render(PyObject* self, PyObject* args)
{
    int n = 1;
    if (!PyArg_ParseTuple(args, "|i", &n))
        return nullptr;
    BI_APP(self);
    const int frames = cimgui::cImGui_ImplBUI_Render(a->app, n);
    if (bi_app_raise(a))
        return nullptr;
    return PyLong_FromLong(frames);
}

static PyObject* bi_app_pixels(PyObject* self, PyObject*)
{
    BI_APP(self);
    int w = 0, h = 0;
    const uint32_t* px = cimgui::cImGui_ImplBUI_Pixels(a->app, &w, &h);
    if (px == nullptr)
        Py_RETURN_NONE;
    PyObject* b = PyBytes_FromStringAndSize((const char*)px, (Py_ssize_t)w * h * 4);
    return Py_BuildValue("(iiN)", w, h, b);
}

static PyObject* bi_app_save_png(PyObject* self, PyObject* args)
{
    const char* path;
    if (!PyArg_ParseTuple(args, "s", &path))
        return nullptr;
    BI_APP(self);
    return PyBool_FromLong(cimgui::cImGui_ImplBUI_SavePNG(a->app, path));
}

static PyObject* bi_app_xy(PyObject* self, PyObject* args, int what)
{
    float x, y;
    int button = 0;
    if (!PyArg_ParseTuple(args, "ff|i", &x, &y, &button))
        return nullptr;
    BI_APP(self);
    switch (what)
    {
    case 0: cimgui::cImGui_ImplBUI_MouseMove(a->app, x, y); break;
    case 1: cimgui::cImGui_ImplBUI_MouseDown(a->app, x, y, button); break;
    case 2: cimgui::cImGui_ImplBUI_MouseUp(a->app, x, y, button); break;
    case 3:
        cimgui::cImGui_ImplBUI_MouseDown(a->app, x, y, button);
        cimgui::cImGui_ImplBUI_MouseUp(a->app, x, y, button);
        break;
    }
    BI_APP_DONE();
}

static PyObject* bi_app_mouse_move(PyObject* s, PyObject* a) { return bi_app_xy(s, a, 0); }
static PyObject* bi_app_mouse_down(PyObject* s, PyObject* a) { return bi_app_xy(s, a, 1); }
static PyObject* bi_app_mouse_up(PyObject* s, PyObject* a) { return bi_app_xy(s, a, 2); }
static PyObject* bi_app_click(PyObject* s, PyObject* a) { return bi_app_xy(s, a, 3); }

static PyObject* bi_app_scroll(PyObject* self, PyObject* args)
{
    float x, y, dx, dy;
    if (!PyArg_ParseTuple(args, "ffff", &x, &y, &dx, &dy))
        return nullptr;
    BI_APP(self);
    cimgui::cImGui_ImplBUI_Scroll(a->app, x, y, dx, dy);
    BI_APP_DONE();
}

static PyObject* bi_app_key_impl(PyObject* self, PyObject* args, int what)
{
    const char* keysym;
    int mods = 0;
    if (!PyArg_ParseTuple(args, "s|i", &keysym, &mods))
        return nullptr;
    BI_APP(self);
    if (what == 0)
        cimgui::cImGui_ImplBUI_Key(a->app, keysym, mods);
    else if (what == 1)
        cimgui::cImGui_ImplBUI_KeyDown(a->app, keysym, mods);
    else
        cimgui::cImGui_ImplBUI_KeyUp(a->app, keysym, mods);
    BI_APP_DONE();
}

static PyObject* bi_app_key(PyObject* s, PyObject* a) { return bi_app_key_impl(s, a, 0); }
static PyObject* bi_app_key_down(PyObject* s, PyObject* a) { return bi_app_key_impl(s, a, 1); }
static PyObject* bi_app_key_up(PyObject* s, PyObject* a) { return bi_app_key_impl(s, a, 2); }

static PyObject* bi_app_type_text(PyObject* self, PyObject* args)
{
    const char* text;
    if (!PyArg_ParseTuple(args, "s", &text))
        return nullptr;
    BI_APP(self);
    cimgui::cImGui_ImplBUI_TypeText(a->app, text);
    BI_APP_DONE();
}

static PyObject* bi_app_headless(PyObject* self, void*)
{
    BI_APP(self);
    return PyBool_FromLong(cimgui::cImGui_ImplBUI_IsHeadless(a->app));
}

static PyObject* bi_app_frame_count(PyObject* self, void*)
{
    BI_APP(self);
    (void)a;
    return PyLong_FromLong(cimgui::cImGui_ImplBUI_FrameCount());
}

static PyMethodDef bi_app_methods[] = {
    { "run", bi_app_run, METH_NOARGS, "run() -- the BUI message loop until the window closes" },
    { "close", bi_app_close, METH_NOARGS, "close() -- close the window (ends run())" },
    { "destroy", bi_app_destroy_m, METH_NOARGS, "destroy() -- shut the backend down" },
    { "render", bi_app_render, METH_VARARGS, "render(n=1) -> frames: n BK_render passes into the offscreen raster" },
    { "pixels", bi_app_pixels, METH_NOARGS, "pixels() -> (width, height, bytes BGRA): the last rendered frame" },
    { "save_png", bi_app_save_png, METH_VARARGS, "save_png(path) -> bool" },
    { "mouse_move", bi_app_mouse_move, METH_VARARGS, "mouse_move(x, y)" },
    { "mouse_down", bi_app_mouse_down, METH_VARARGS, "mouse_down(x, y, button=0)" },
    { "mouse_up", bi_app_mouse_up, METH_VARARGS, "mouse_up(x, y, button=0)" },
    { "click", bi_app_click, METH_VARARGS, "click(x, y, button=0)" },
    { "scroll", bi_app_scroll, METH_VARARGS, "scroll(x, y, dx, dy) -- wheel notches, dy > 0 up" },
    { "key", bi_app_key, METH_VARARGS, "key(keysym, mods=0) -- press and release (X11 names: 'Return', 'BackSpace', 'a')" },
    { "key_down", bi_app_key_down, METH_VARARGS, "key_down(keysym, mods=0)" },
    { "key_up", bi_app_key_up, METH_VARARGS, "key_up(keysym, mods=0)" },
    { "type_text", bi_app_type_text, METH_VARARGS, "type_text(str) -- typed characters" },
    { nullptr, nullptr, 0, nullptr },
};

static PyGetSetDef bi_app_getset[] = {
    { "headless", bi_app_headless, nullptr, nullptr, nullptr },
    { "frame_count", bi_app_frame_count, nullptr, nullptr, nullptr },
    { nullptr, nullptr, nullptr, nullptr, nullptr },
};

static PyType_Slot bi_app_slots[] = {
    { Py_tp_new, (void*)bi_app_new },
    { Py_tp_dealloc, (void*)bi_app_dealloc },
    { Py_tp_methods, (void*)bi_app_methods },
    { Py_tp_getset, (void*)bi_app_getset },
    { Py_tp_doc, (void*)"App(frame, title='Dear ImGui on BUI', width=1280, height=800, headless=False, blender_style=True, blender_font=True, font_size=0.0)\n\n"
                        "Dear ImGui in a BUI window (or headless): frame() is called inside each ImGui frame." },
    { 0, nullptr },
};
static PyType_Spec bi_app_spec = { "imgui.App", sizeof(BiApp), 0, Py_TPFLAGS_DEFAULT, bi_app_slots };

//-----------------------------------------------------------------------------
// Module-level helpers outside imgui.h
//-----------------------------------------------------------------------------

static PyObject* bi_style_blender(PyObject*, PyObject* args)
{
    float scale = 1.0f;
    if (!PyArg_ParseTuple(args, "|f", &scale))
        return nullptr;
    cimgui::cImGui_ImplBUI_StyleBlender(nullptr, scale);
    Py_RETURN_NONE;
}

static PyObject* bi_get_scale(PyObject*, PyObject*) { return PyFloat_FromDouble(cimgui::cImGui_ImplBUI_GetScale()); }

static PyObject* bi_has_blender_widgets(PyObject*, PyObject*) { return PyBool_FromLong(cimgui::cImGui_ImplBUI_HasBlenderWidgets()); }
static PyObject* bi_get_blender_widgets(PyObject*, PyObject*) { return PyBool_FromLong(cimgui::cImGui_ImplBUI_GetBlenderWidgets()); }
static PyObject* bi_set_blender_widgets(PyObject*, PyObject* arg)
{
    const int on = PyObject_IsTrue(arg);
    if (on < 0)
        return nullptr;
    cimgui::cImGui_ImplBUI_SetBlenderWidgets(on != 0);
    Py_RETURN_NONE;
}

static PyObject* bi_set_scale(PyObject*, PyObject* args)
{
    float s;
    if (!PyArg_ParseTuple(args, "f", &s))
        return nullptr;
    cimgui::cImGui_ImplBUI_SetScale(s);
    Py_RETURN_NONE;
}

// The kit is shared with buipy: this is the same theme bui.set_theme() sets.
extern "C" int BK_set_theme(const char* name);
extern "C" const char* BK_theme_name(void);

static PyObject* bi_set_theme(PyObject*, PyObject* args)
{
    const char* name;
    if (!PyArg_ParseTuple(args, "s", &name))
        return nullptr;
    if (!BK_set_theme(name))
    {
        PyErr_Format(PyExc_ValueError, "unknown theme '%s' (expected 'dark' or 'light')", name);
        return nullptr;
    }
    Py_RETURN_NONE;
}

static PyObject* bi_theme_name(PyObject*, PyObject*)
{
    return PyUnicode_FromString(BK_theme_name());
}

static PyObject* bi_sizeof(PyObject*, PyObject* args)
{
    const char* n;
    if (!PyArg_ParseTuple(args, "s", &n))
        return nullptr;
    return PyLong_FromSize_t(cimgui::cImGui_SizeOf(n));
}

static PyMethodDef bi_module_methods_extra[] = {
    { "style_blender", bi_style_blender, METH_VARARGS, "style_blender(scale=1.0) -- Blender's theme on the current style" },
    { "has_blender_widgets", bi_has_blender_widgets, METH_NOARGS, "has_blender_widgets() -- built with Blender's widgets (BUI_IMGUI_BLENDER_WIDGETS)" },
    { "get_blender_widgets", bi_get_blender_widgets, METH_NOARGS, "get_blender_widgets() -- ImGui's widgets are drawn and handled as Blender's" },
    { "set_blender_widgets", bi_set_blender_widgets, METH_O, "set_blender_widgets(on) -- switch Blender's widgets on or off (when built with them)" },
    { "get_ui_scale", bi_get_scale, METH_NOARGS, "get_ui_scale() -> BUI's UI scale (DPI)" },
    { "set_ui_scale", bi_set_scale, METH_VARARGS, "set_ui_scale(scale)" },
    { "set_theme", bi_set_theme, METH_VARARGS, "set_theme(name) -- Blender's 'dark' or 'light' theme (also bui.set_theme)" },
    { "theme_name", bi_theme_name, METH_NOARGS, "theme_name() -> 'dark' or 'light'" },
    { "_sizeof", bi_sizeof, METH_VARARGS, "_sizeof(type_name) -> sizeof() in C++ (tests)" },
    { nullptr, nullptr, 0, nullptr },
};

static PyStructSequence_Field bi_vec2_fields[] = { { "x", nullptr }, { "y", nullptr }, { nullptr, nullptr } };
static PyStructSequence_Field bi_vec4_fields[] = { { "x", nullptr }, { "y", nullptr }, { "z", nullptr }, { "w", nullptr }, { nullptr, nullptr } };
static PyStructSequence_Desc bi_vec2_desc = { "imgui.Vec2", "ImVec2 (x, y)", bi_vec2_fields, 2 };
static PyStructSequence_Desc bi_vec4_desc = { "imgui.Vec4", "ImVec4 (x, y, z, w)", bi_vec4_fields, 4 };

static int bi_add_functions(PyObject* m, PyMethodDef* defs)
{
    for (PyMethodDef* d = defs; d->ml_name; d++)
    {
        PyObject* f = PyCFunction_NewEx(d, nullptr, PyModule_GetNameObject(m));
        if (f == nullptr || PyModule_AddObject(m, d->ml_name, f) < 0)
            return -1;
    }
    return 0;
}

static struct PyModuleDef bi_moduledef = {
    PyModuleDef_HEAD_INIT, "_buiimgui",
    "Dear ImGui " IMGUI_VERSION " on BUI (generated from imgui.h). Use it as blenderui.imgui / bui.imgui.",
    -1, nullptr,
};

// The `_buiimgui` module object, created once (its types are process-wide).
static PyObject* bi_module = nullptr;

static PyObject* bi_create_module(void)
{
    PyObject* m = PyModule_Create(&bi_moduledef);
    if (m == nullptr)
        return nullptr;
    bi_Vec2Type = PyStructSequence_NewType(&bi_vec2_desc);
    bi_Vec4Type = PyStructSequence_NewType(&bi_vec4_desc);
    bi_ArrayType = (PyTypeObject*)PyType_FromSpec(&bi_array_spec);
    bi_AppType = (PyTypeObject*)PyType_FromSpec(&bi_app_spec);
    if (!bi_Vec2Type || !bi_Vec4Type || !bi_ArrayType || !bi_AppType)
        return nullptr;
    Py_INCREF(bi_Vec2Type);
    Py_INCREF(bi_Vec4Type);
    Py_INCREF(bi_AppType);
    PyModule_AddObject(m, "Vec2", (PyObject*)bi_Vec2Type);
    PyModule_AddObject(m, "Vec4", (PyObject*)bi_Vec4Type);
    PyModule_AddObject(m, "App", (PyObject*)bi_AppType);
    if (bi_add_functions(m, bi_module_methods_gen) < 0 || bi_add_functions(m, bi_module_methods_extra) < 0)
        return nullptr;
    for (BiStructInfo& s : bi_structs)
        if (!bi_make_type(m, s))
            return nullptr;
    for (const BiConst* c = bi_consts; c->name; c++)
        PyModule_AddObject(m, c->name, PyLong_FromLongLong(c->value));
    for (int i = 0; bi_str_consts[i][0]; i++)
        PyModule_AddStringConstant(m, bi_str_consts[i][0], bi_str_consts[i][1]);
    return m;
}

// buipy.imgui_module(): the `_buiimgui` module, created on first call (a program that never
// touches Dear ImGui does not pay for its 400 functions and types) and registered in
// sys.modules so `import _buiimgui` keeps working.
static PyObject* bi_imgui_module(PyObject*, PyObject*)
{
    if (bi_module == nullptr)
    {
        PyObject* m = bi_create_module();
        if (m == nullptr)
            return nullptr;
        if (PyDict_SetItemString(PyImport_GetModuleDict(), "_buiimgui", m) < 0)
        {
            Py_DECREF(m);
            return nullptr;
        }
        bi_module = m;
    }
    Py_INCREF(bi_module);
    return bi_module;
}

static PyMethodDef bi_buipy_methods[] = {
    { "imgui_module", bi_imgui_module, METH_NOARGS, "imgui_module() -> the Dear ImGui module (bui.imgui)" },
    { nullptr, nullptr, 0, nullptr },
};

// Called by PyInit_buipy (BUI_PY_IMGUI: the build has Dear ImGui).
extern "C" int buipy_imgui_init(PyObject* buipy)
{
    return PyModule_AddFunctions(buipy, bi_buipy_methods);
}
