// cimgui_impl_bui.cpp: C API of the BUI backend (imgui_impl_bui.h) + headless helpers.
// SPDX-License-Identifier: GPL-2.0-or-later

#include "imgui.h"
#include "imgui_impl_bui.h"
#include "BUI_kit.h"
#include "bui_png.h"

#include <string.h>

namespace cimgui {
#include "cimgui_impl_bui.h"
}

using cimgui::cImGui_ImplBUI_FrameFn;

struct cImGui_ImplBUI_App_
{
    ImGui_ImplBUI_App* App = nullptr;
    BuiRaster* Raster = nullptr;
    int Width = 0, Height = 0;
    int Settle = 3;
    ::ImGuiContext* ImCtx = nullptr;
};

static ImGui_ImplBUI_Config ToConfig(const cimgui::cImGui_ImplBUI_Config* c)
{
    ImGui_ImplBUI_Config cfg;
    if (c)
    {
        cfg.BlenderStyle = c->BlenderStyle;
        cfg.BlenderFont = c->BlenderFont;
        cfg.FontSize = c->FontSize;
        cfg.FollowScale = c->FollowScale;
        cfg.SettleFrames = c->SettleFrames;
    }
    return cfg;
}

static cImGui_ImplBUI_App_* A(cimgui::cImGui_ImplBUI_App* app) { return reinterpret_cast<cImGui_ImplBUI_App_*>(app); }
static cimgui::cImGui_ImplBUI_App* C(cImGui_ImplBUI_App_* app) { return reinterpret_cast<cimgui::cImGui_ImplBUI_App*>(app); }

static BKContext* Ctx(cimgui::cImGui_ImplBUI_App* app) { return app ? ImGui_ImplBUI_AppContext(A(app)->App) : nullptr; }

// headless: let ImGui see an event (window hosts render on their own)
static void Settle(cimgui::cImGui_ImplBUI_App* app, int n)
{
    if (app && ImGui_ImplBUI_AppWindow(A(app)->App) == nullptr)
        cimgui::cImGui_ImplBUI_Render(app, n);
}

namespace cimgui {
extern "C" {

CIMGUI_API cImGui_ImplBUI_Config cImGui_ImplBUI_DefaultConfig(void)
{
    ImGui_ImplBUI_Config d;
    cImGui_ImplBUI_Config c;
    c.BlenderStyle = d.BlenderStyle;
    c.BlenderFont = d.BlenderFont;
    c.FontSize = d.FontSize;
    c.FollowScale = d.FollowScale;
    c.SettleFrames = d.SettleFrames;
    return c;
}

CIMGUI_API bool cImGui_ImplBUI_Init(BKNode* node, cImGui_ImplBUI_FrameFn frame, void* user, const cImGui_ImplBUI_Config* config)
{
    ImGui_ImplBUI_Config cfg = ToConfig(config);
    return ImGui_ImplBUI_Init(reinterpret_cast<::BKNode*>(node), frame, user, &cfg);
}
CIMGUI_API void cImGui_ImplBUI_Shutdown(void) { ImGui_ImplBUI_Shutdown(); }
CIMGUI_API BKNode* cImGui_ImplBUI_GetNode(void) { return reinterpret_cast<BKNode*>(ImGui_ImplBUI_GetNode()); }
CIMGUI_API void cImGui_ImplBUI_RequestFrames(int n) { ImGui_ImplBUI_RequestFrames(n); }
CIMGUI_API void cImGui_ImplBUI_RequestFrameIn(int ms) { ImGui_ImplBUI_RequestFrameIn(ms); }
CIMGUI_API int cImGui_ImplBUI_FrameCount(void) { return ImGui_ImplBUI_FrameCount(); }

CIMGUI_API void cImGui_ImplBUI_StyleBlender(ImGuiStyle* dst, float scale) { ImGui_ImplBUI_StyleBlender(reinterpret_cast<::ImGuiStyle*>(dst), scale); }
CIMGUI_API float cImGui_ImplBUI_BlenderFontSize(float scale) { return ImGui_ImplBUI_BlenderFontSize(scale); }
CIMGUI_API ImFont* cImGui_ImplBUI_AddBlenderFont(float size_pixels) { return reinterpret_cast<ImFont*>(ImGui_ImplBUI_AddBlenderFont(size_pixels)); }
CIMGUI_API float cImGui_ImplBUI_GetScale(void) { return BK_scale(); }
CIMGUI_API void cImGui_ImplBUI_SetScale(float scale) { BK_set_scale(scale); }

CIMGUI_API cImGui_ImplBUI_App* cImGui_ImplBUI_CreateApp(const char* title, int width, int height, cImGui_ImplBUI_FrameFn frame, void* user, const cImGui_ImplBUI_Config* config)
{
    ImGui_ImplBUI_Config cfg = ToConfig(config);
    cImGui_ImplBUI_App_* app = IM_NEW(cImGui_ImplBUI_App_)();
    app->ImCtx = ImGui::GetCurrentContext();
    app->App = ImGui_ImplBUI_CreateApp(title ? title : "Dear ImGui", width, height, frame, user, config ? &cfg : nullptr);
    app->Width = width;
    app->Height = height;
    app->Settle = config ? config->SettleFrames : 3;
    return C(app);
}

CIMGUI_API cImGui_ImplBUI_App* cImGui_ImplBUI_CreateHeadlessApp(int width, int height, cImGui_ImplBUI_FrameFn frame, void* user, const cImGui_ImplBUI_Config* config)
{
    ImGui_ImplBUI_Config cfg = ToConfig(config);
    cImGui_ImplBUI_App_* app = IM_NEW(cImGui_ImplBUI_App_)();
    app->ImCtx = ImGui::GetCurrentContext();
    app->App = ImGui_ImplBUI_CreateHeadlessApp(frame, user, config ? &cfg : nullptr);
    app->Width = width > 0 ? width : 1280;
    app->Height = height > 0 ? height : 800;
    app->Settle = config ? config->SettleFrames : 3;
    return C(app);
}

CIMGUI_API void cImGui_ImplBUI_RunApp(cImGui_ImplBUI_App* app)
{
    if (app)
        ImGui_ImplBUI_RunApp(A(app)->App);
}

CIMGUI_API void cImGui_ImplBUI_CloseApp(cImGui_ImplBUI_App* app)
{
    if (app && ImGui_ImplBUI_AppWindow(A(app)->App))
        BK_window_close(ImGui_ImplBUI_AppWindow(A(app)->App));
}

CIMGUI_API void cImGui_ImplBUI_DestroyApp(cImGui_ImplBUI_App* app)
{
    if (app == nullptr)
        return;
    ImGui_ImplBUI_DestroyApp(A(app)->App);
    if (A(app)->Raster)
        BUI_raster_free(A(app)->Raster);
    IM_DELETE(A(app));
}

CIMGUI_API bool cImGui_ImplBUI_IsHeadless(cImGui_ImplBUI_App* app) { return app && ImGui_ImplBUI_AppWindow(A(app)->App) == nullptr; }
CIMGUI_API BKWindow* cImGui_ImplBUI_AppWindow(cImGui_ImplBUI_App* app) { return app ? reinterpret_cast<BKWindow*>(ImGui_ImplBUI_AppWindow(A(app)->App)) : nullptr; }
CIMGUI_API BKContext* cImGui_ImplBUI_AppContext(cImGui_ImplBUI_App* app) { return reinterpret_cast<BKContext*>(Ctx(app)); }
CIMGUI_API BKNode* cImGui_ImplBUI_AppNode(cImGui_ImplBUI_App* app) { return app ? reinterpret_cast<BKNode*>(ImGui_ImplBUI_AppNode(A(app)->App)) : nullptr; }
CIMGUI_API ImGuiContext* cImGui_ImplBUI_AppImGuiContext(cImGui_ImplBUI_App* app)
{
    return app ? reinterpret_cast<ImGuiContext*>(A(app)->ImCtx) : nullptr;
}

CIMGUI_API int cImGui_ImplBUI_Render(cImGui_ImplBUI_App* app, int n)
{
    if (app == nullptr)
        return 0;
    cImGui_ImplBUI_App_* a = A(app);
    bool full = false;
    if (a->Raster == nullptr)
    {
        a->Raster = BUI_raster_create(a->Width, a->Height);
        full = true;
    }
    // ask the app's backend for the frames (an idle UI otherwise runs none)
    ::ImGuiContext* prev = ImGui::GetCurrentContext();
    ImGui::SetCurrentContext(a->ImCtx);
    ImGui_ImplBUI_RequestFrames(n);
    ImGui::SetCurrentContext(prev);
    for (int i = 0; i < n; i++)
        BK_render(ImGui_ImplBUI_AppContext(a->App), a->Raster, a->Width, a->Height, full && i == 0);
    ImGui::SetCurrentContext(a->ImCtx);
    const int frames = ImGui_ImplBUI_FrameCount();
    ImGui::SetCurrentContext(prev);
    return frames;
}

CIMGUI_API const uint32_t* cImGui_ImplBUI_Pixels(cImGui_ImplBUI_App* app, int* width, int* height)
{
    if (app == nullptr || A(app)->Raster == nullptr)
        return nullptr;
    if (width)
        *width = A(app)->Width;
    if (height)
        *height = A(app)->Height;
    return BUI_raster_pixels(A(app)->Raster);
}

CIMGUI_API bool cImGui_ImplBUI_SavePNG(cImGui_ImplBUI_App* app, const char* path)
{
    int w = 0, h = 0;
    const uint32_t* px = cImGui_ImplBUI_Pixels(app, &w, &h);
    return px && bui_png_write(path, px, w, h) != 0;
}

CIMGUI_API void cImGui_ImplBUI_MouseMove(cImGui_ImplBUI_App* app, float x, float y)
{
    BK_motion(Ctx(app), (int)x, (int)y, 0);
    Settle(app, 2);
}

CIMGUI_API void cImGui_ImplBUI_MouseDown(cImGui_ImplBUI_App* app, float x, float y, int button)
{
    BK_motion(Ctx(app), (int)x, (int)y, 0);
    Settle(app, 1);
    if (button == 0)
        BK_press(Ctx(app), (int)x, (int)y, BK_BUTTON1);
    else
        BK_button(Ctx(app), (int)x, (int)y, button == 1 ? 3 : 2, 0, 0);
    Settle(app, 2);
}

CIMGUI_API void cImGui_ImplBUI_MouseUp(cImGui_ImplBUI_App* app, float x, float y, int button)
{
    if (button == 0)
        BK_release(Ctx(app), (int)x, (int)y, 0);
    else
        BK_button(Ctx(app), (int)x, (int)y, button == 1 ? 3 : 2, 1, 0);
    Settle(app, A(app) ? A(app)->Settle : 3);
}

CIMGUI_API void cImGui_ImplBUI_Click(cImGui_ImplBUI_App* app, float x, float y)
{
    cImGui_ImplBUI_MouseDown(app, x, y, 0);
    cImGui_ImplBUI_MouseUp(app, x, y, 0);
}

CIMGUI_API void cImGui_ImplBUI_Scroll(cImGui_ImplBUI_App* app, float x, float y, float dx, float dy)
{
    BK_motion(Ctx(app), (int)x, (int)y, 0);
    BK_scroll(Ctx(app), (int)x, (int)y, dx, dy, 0);
    Settle(app, 3);
}

CIMGUI_API void cImGui_ImplBUI_KeyDown(cImGui_ImplBUI_App* app, const char* keysym, int mods)
{
    BK_key(Ctx(app), "", keysym, mods);
    Settle(app, 1);
}

CIMGUI_API void cImGui_ImplBUI_KeyUp(cImGui_ImplBUI_App* app, const char* keysym, int mods)
{
    BK_key_release(Ctx(app), keysym, mods);
    Settle(app, 2);
}

CIMGUI_API void cImGui_ImplBUI_Key(cImGui_ImplBUI_App* app, const char* keysym, int mods)
{
    if (mods & BK_CTRL)
        cImGui_ImplBUI_KeyDown(app, "Control_L", BK_CTRL);
    if (mods & BK_SHIFT)
        cImGui_ImplBUI_KeyDown(app, "Shift_L", mods & (BK_SHIFT | BK_CTRL));
    cImGui_ImplBUI_KeyDown(app, keysym, mods);
    cImGui_ImplBUI_KeyUp(app, keysym, mods);
    if (mods & BK_SHIFT)
        cImGui_ImplBUI_KeyUp(app, "Shift_L", mods & BK_CTRL);
    if (mods & BK_CTRL)
        cImGui_ImplBUI_KeyUp(app, "Control_L", 0);
}

// typed text arrives as KEY events carrying the character (the Win32 WM_CHAR path)
CIMGUI_API void cImGui_ImplBUI_TypeText(cImGui_ImplBUI_App* app, const char* s)
{
    while (s && *s)
    {
        int len = 1;
        const unsigned char c = (unsigned char)*s;
        if (c >= 0xF0) len = 4; else if (c >= 0xE0) len = 3; else if (c >= 0xC0) len = 2;
        char t[8] = {};
        for (int i = 0; i < len && s[i]; i++)
            t[i] = s[i];
        const bool upper = len == 1 && c >= 'A' && c <= 'Z';
        char k[8];
        memcpy(k, t, sizeof(k));
        if (upper)
        {
            k[0] = (char)(c - 'A' + 'a');
            BK_key(Ctx(app), "", "Shift_L", BK_SHIFT);
            Settle(app, 1);
        }
        BK_key(Ctx(app), t, t, upper ? BK_SHIFT : 0);
        Settle(app, 1);
        BK_key_release(Ctx(app), k, upper ? BK_SHIFT : 0);
        Settle(app, 1);
        if (upper)
        {
            BK_key_release(Ctx(app), "Shift_L", 0);
            Settle(app, 1);
        }
        s += (int)strlen(t);
    }
    Settle(app, 3);
}

} // extern "C"
} // namespace cimgui
