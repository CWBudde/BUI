// dear imgui: Platform + Renderer Backend for BUI. See imgui_impl_bui.h.
//
// How it fits BUI: the ImGui UI lives in one kit "custom" node (BUI_kit.h). The kit hands
// the node its input (BKCustomEvent), calls `frame` at the start of BK_render when a frame
// is due, and `draw` for every damaged region the node overlaps. The frame runs
// NewFrame / user UI / Render, applies texture requests, updates the cursor, and
// invalidates the node only when the output changed. Drawing replays the last ImDrawData
// through BUI_raster_mesh, so a partial redraw (a tooltip of another widget closing over
// the node) never advances ImGui.
//
// SPDX-License-Identifier: GPL-2.0-or-later

#include "imgui.h"
#ifndef IMGUI_DISABLE
#include "imgui_impl_bui.h"
#include "imgui_internal.h" // ImGuiInputTextState::CursorAnim (text cursor blink timing)

#include "BUI_kit.h"

#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <vector>

// ImDrawVert is BuiMeshVertex: meshes go to the raster without conversion.
static_assert(sizeof(ImDrawVert) == sizeof(BuiMeshVertex), "ImDrawVert layout");
static_assert(offsetof(ImDrawVert, pos) == offsetof(BuiMeshVertex, x), "ImDrawVert layout");
static_assert(offsetof(ImDrawVert, uv) == offsetof(BuiMeshVertex, u), "ImDrawVert layout");
static_assert(offsetof(ImDrawVert, col) == offsetof(BuiMeshVertex, col), "ImDrawVert layout");

struct ImGui_ImplBUI_Data
{
    ImGuiContext*           Ctx = nullptr;
    BKNode*                 Node = nullptr;
    ImGui_ImplBUI_FrameFn   Frame = nullptr;
    void*                   User = nullptr;
    ImGui_ImplBUI_Config    Config;
    double                  Time = 0.0;           // of the last frame
    double                  LastInputTime = 0.0;
    int                     FramesPending = 0;    // settle frames still to run
    int                     FrameCount = 0;
    uint64_t                LastHash = 0;
    bool                    HaveHash = false;
    float                   Scale = 0.0f;         // UI scale the style was built for
    ImGuiMouseCursor        LastCursor = ImGuiMouseCursor_COUNT;
    int                     ButtonsDown = 0;      // bit per ImGui mouse button
    bool                    SuperDown = false;
    char*                   ClipboardText = nullptr;
    bool                    InFrame = false;
};

static double now_seconds()
{
    using clock = std::chrono::steady_clock;
    static const clock::time_point t0 = clock::now();
    return std::chrono::duration<double>(clock::now() - t0).count();
}

// The backend data of the current context.
static ImGui_ImplBUI_Data* GetBackendData()
{
    return ImGui::GetCurrentContext() ? (ImGui_ImplBUI_Data*)ImGui::GetIO().BackendPlatformUserData : nullptr;
}

// Callbacks may come while another ImGui context is current (several ImGui nodes).
struct ContextScope
{
    ImGuiContext* Prev;
    explicit ContextScope(ImGuiContext* ctx) : Prev(ImGui::GetCurrentContext()) { ImGui::SetCurrentContext(ctx); }
    ~ContextScope() { ImGui::SetCurrentContext(Prev); }
};

//-----------------------------------------------------------------------------
// Renderer
//-----------------------------------------------------------------------------

// Markers for the standard draw callbacks: ImGui never calls them, RenderDrawData
// recognizes them.
static void DrawCallback_ResetRenderState(const ImDrawList*, const ImDrawCmd*) {}
static void DrawCallback_SetSamplerLinear(const ImDrawList*, const ImDrawCmd*) {}
static void DrawCallback_SetSamplerNearest(const ImDrawList*, const ImDrawCmd*) {}

static void UploadRect(ImTextureData* tex, int tex_id, int x, int y, int w, int h)
{
    if (w <= 0 || h <= 0)
        return;
    if (tex->Format == ImTextureFormat_RGBA32)
    {
        BUI_texture_update(tex_id, x, y, w, h, tex->GetPixelsAt(x, y), tex->GetPitch());
        return;
    }
    // Alpha8: white with the coverage as alpha
    std::vector<unsigned char> rgba((size_t)w * h * 4);
    for (int row = 0; row < h; row++)
    {
        const unsigned char* src = (const unsigned char*)tex->GetPixelsAt(x, y + row);
        unsigned char* dst = rgba.data() + (size_t)row * w * 4;
        for (int i = 0; i < w; i++)
        {
            dst[i * 4 + 0] = dst[i * 4 + 1] = dst[i * 4 + 2] = 255;
            dst[i * 4 + 3] = src[i];
        }
    }
    BUI_texture_update(tex_id, x, y, w, h, rgba.data(), w * 4);
}

void ImGui_ImplBUI_UpdateTexture(ImTextureData* tex)
{
    if (tex->Status == ImTextureStatus_WantCreate)
    {
        IM_ASSERT(tex->TexID == ImTextureID_Invalid && tex->BackendUserData == nullptr);
        const int id = BUI_texture_create(tex->Width, tex->Height, nullptr, 0);
        if (id <= 0)
            return; // stays WantCreate: retried next frame
        UploadRect(tex, id, 0, 0, tex->Width, tex->Height);
        tex->SetTexID((ImTextureID)(intptr_t)id);
        tex->SetStatus(ImTextureStatus_OK);
    }
    else if (tex->Status == ImTextureStatus_WantUpdates)
    {
        const int id = (int)(intptr_t)tex->TexID;
        for (const ImTextureRect& r : tex->Updates)
            UploadRect(tex, id, r.x, r.y, r.w, r.h);
        tex->SetStatus(ImTextureStatus_OK);
    }
    else if (tex->Status == ImTextureStatus_WantDestroy && tex->UnusedFrames > 0)
    {
        BUI_texture_free((int)(intptr_t)tex->TexID);
        tex->SetTexID(ImTextureID_Invalid);
        tex->SetStatus(ImTextureStatus_Destroyed);
    }
}

// Applies pending texture requests; true when any texture changed.
static bool UpdateTextures(ImDrawData* draw_data)
{
    bool changed = false;
    if (draw_data->Textures != nullptr)
        for (ImTextureData* tex : *draw_data->Textures)
            if (tex->Status != ImTextureStatus_OK)
            {
                changed |= tex->Status != ImTextureStatus_WantDestroy || tex->UnusedFrames > 0;
                ImGui_ImplBUI_UpdateTexture(tex);
            }
    return changed;
}

void ImGui_ImplBUI_RenderDrawData(ImDrawData* draw_data, BuiRaster* r, float ox, float oy)
{
    if (draw_data == nullptr || !draw_data->Valid || r == nullptr)
        return;
    UpdateTextures(draw_data); // normally done by the frame already
    const ImGuiPlatformIO& platform_io = ImGui::GetPlatformIO();
    // ImGui positions are in display space: DisplayPos lands on (ox, oy). FramebufferScale is
    // 1: the backend runs ImGui in raster pixels (fonts and style scaled instead).
    const float dx = ox - draw_data->DisplayPos.x, dy = oy - draw_data->DisplayPos.y;
    const int index_flags = sizeof(ImDrawIdx) == 4 ? BUI_MESH_INDEX32 : 0;
    for (const ImDrawList* list : draw_data->CmdLists)
    {
        const ImDrawVert* vtx = list->VtxBuffer.Data;
        const ImDrawIdx* idx = list->IdxBuffer.Data;
        const int n_vtx = list->VtxBuffer.Size;
        int nearest = 0;
        for (const ImDrawCmd& cmd : list->CmdBuffer)
        {
            if (cmd.UserCallback != nullptr)
            {
                if (cmd.UserCallback == DrawCallback_SetSamplerNearest)
                    nearest = BUI_MESH_NEAREST;
                else if (cmd.UserCallback == DrawCallback_SetSamplerLinear || cmd.UserCallback == DrawCallback_ResetRenderState ||
                         cmd.UserCallback == platform_io.DrawCallback_ResetRenderState)
                    nearest = 0;
                else
                    cmd.UserCallback(list, &cmd);
                continue;
            }
            if (cmd.ElemCount == 0 || (int)cmd.VtxOffset >= n_vtx)
                continue;
            const float clip[4] = { cmd.ClipRect.x, cmd.ClipRect.y, cmd.ClipRect.z, cmd.ClipRect.w };
            BUI_raster_mesh(r, (const BuiMeshVertex*)(vtx + cmd.VtxOffset), n_vtx - (int)cmd.VtxOffset,
                            idx + cmd.IdxOffset, (int)cmd.ElemCount, index_flags | nearest,
                            (int)(intptr_t)cmd.GetTexID(), clip, dx, dy);
        }
    }
}

//-----------------------------------------------------------------------------
// Platform: input
//-----------------------------------------------------------------------------

struct KeyName { const char* name; ImGuiKey key; };

// X11 keysym names (BUI's key names on every host) -> ImGuiKey
static const KeyName k_keys[] = {
    { "Tab", ImGuiKey_Tab }, { "ISO_Left_Tab", ImGuiKey_Tab }, { "Left", ImGuiKey_LeftArrow }, { "Right", ImGuiKey_RightArrow },
    { "Up", ImGuiKey_UpArrow }, { "Down", ImGuiKey_DownArrow }, { "Prior", ImGuiKey_PageUp }, { "Page_Up", ImGuiKey_PageUp },
    { "Next", ImGuiKey_PageDown }, { "Page_Down", ImGuiKey_PageDown }, { "Home", ImGuiKey_Home }, { "End", ImGuiKey_End },
    { "Insert", ImGuiKey_Insert }, { "Delete", ImGuiKey_Delete }, { "BackSpace", ImGuiKey_Backspace }, { "space", ImGuiKey_Space },
    { "Return", ImGuiKey_Enter }, { "Escape", ImGuiKey_Escape }, { "KP_Enter", ImGuiKey_KeypadEnter },
    { "apostrophe", ImGuiKey_Apostrophe }, { "comma", ImGuiKey_Comma }, { "minus", ImGuiKey_Minus }, { "period", ImGuiKey_Period },
    { "slash", ImGuiKey_Slash }, { "semicolon", ImGuiKey_Semicolon }, { "equal", ImGuiKey_Equal }, { "bracketleft", ImGuiKey_LeftBracket },
    { "backslash", ImGuiKey_Backslash }, { "bracketright", ImGuiKey_RightBracket }, { "grave", ImGuiKey_GraveAccent },
    { "Caps_Lock", ImGuiKey_CapsLock }, { "Scroll_Lock", ImGuiKey_ScrollLock }, { "Num_Lock", ImGuiKey_NumLock },
    { "Print", ImGuiKey_PrintScreen }, { "Pause", ImGuiKey_Pause },
    { "KP_0", ImGuiKey_Keypad0 }, { "KP_1", ImGuiKey_Keypad1 }, { "KP_2", ImGuiKey_Keypad2 }, { "KP_3", ImGuiKey_Keypad3 },
    { "KP_4", ImGuiKey_Keypad4 }, { "KP_5", ImGuiKey_Keypad5 }, { "KP_6", ImGuiKey_Keypad6 }, { "KP_7", ImGuiKey_Keypad7 },
    { "KP_8", ImGuiKey_Keypad8 }, { "KP_9", ImGuiKey_Keypad9 }, { "KP_Decimal", ImGuiKey_KeypadDecimal },
    { "KP_Divide", ImGuiKey_KeypadDivide }, { "KP_Multiply", ImGuiKey_KeypadMultiply }, { "KP_Subtract", ImGuiKey_KeypadSubtract },
    { "KP_Add", ImGuiKey_KeypadAdd }, { "KP_Equal", ImGuiKey_KeypadEqual },
    { "Control_L", ImGuiKey_LeftCtrl }, { "Control_R", ImGuiKey_RightCtrl }, { "Shift_L", ImGuiKey_LeftShift },
    { "Shift_R", ImGuiKey_RightShift }, { "Alt_L", ImGuiKey_LeftAlt }, { "Alt_R", ImGuiKey_RightAlt }, { "ISO_Level3_Shift", ImGuiKey_RightAlt },
    { "Meta_L", ImGuiKey_LeftAlt }, { "Meta_R", ImGuiKey_RightAlt }, { "Super_L", ImGuiKey_LeftSuper }, { "Super_R", ImGuiKey_RightSuper },
    { "Menu", ImGuiKey_Menu }, { "App", ImGuiKey_AppBack },
};

// Characters typed without a key name (Win32 sends punctuation as text only): no release follows.
static const KeyName k_chars[] = {
    { "'", ImGuiKey_Apostrophe }, { ",", ImGuiKey_Comma }, { "-", ImGuiKey_Minus }, { ".", ImGuiKey_Period },
    { "/", ImGuiKey_Slash }, { ";", ImGuiKey_Semicolon }, { "=", ImGuiKey_Equal }, { "[", ImGuiKey_LeftBracket },
    { "\\", ImGuiKey_Backslash }, { "]", ImGuiKey_RightBracket }, { "`", ImGuiKey_GraveAccent },
};

// `r_pulse`: the key has no release event coming (press and release at once).
static ImGuiKey KeysymToImGuiKey(const char* ks, bool* r_pulse)
{
    *r_pulse = false;
    if (ks == nullptr || ks[0] == 0)
        return ImGuiKey_None;
    if (ks[1] == 0)
    {
        const char c = ks[0];
        if (c >= 'a' && c <= 'z') return (ImGuiKey)(ImGuiKey_A + (c - 'a'));
        if (c >= 'A' && c <= 'Z') return (ImGuiKey)(ImGuiKey_A + (c - 'A'));
        if (c >= '0' && c <= '9') return (ImGuiKey)(ImGuiKey_0 + (c - '0'));
        for (const KeyName& k : k_chars)
            if (k.name[0] == c)
            {
                *r_pulse = true;
                return k.key;
            }
        return ImGuiKey_None;
    }
    if ((ks[0] == 'F') && ks[1] >= '1' && ks[1] <= '9')
    {
        const int n = atoi(ks + 1);
        if (n >= 1 && n <= 24 && (ks[2] == 0 || (ks[3] == 0 && ks[2] >= '0' && ks[2] <= '9')))
            return (ImGuiKey)(ImGuiKey_F1 + (n - 1));
    }
    for (const KeyName& k : k_keys)
        if (strcmp(k.name, ks) == 0)
            return k.key;
    return ImGuiKey_None;
}

static int MouseButton(int bui_button)
{
    switch (bui_button)
    {
    case 1: return ImGuiMouseButton_Left;
    case 2: return ImGuiMouseButton_Middle;
    case 3: return ImGuiMouseButton_Right;
    }
    return -1;
}

// Modifier state: from the event's bits, except the modifier key the event itself presses
// or releases (X11 reports the state before the event).
static void UpdateModifiers(ImGui_ImplBUI_Data* bd, ImGuiIO& io, const BKCustomEvent* e, ImGuiKey key)
{
    bool ctrl = (e->mods & BK_CTRL) != 0, shift = (e->mods & BK_SHIFT) != 0, alt = (e->mods & BK_ALT) != 0;
    const bool down = e->type == BK_CUSTOM_KEY, up = e->type == BK_CUSTOM_KEYUP;
    if (down || up)
    {
        if (key == ImGuiKey_LeftCtrl || key == ImGuiKey_RightCtrl) ctrl = down;
        if (key == ImGuiKey_LeftShift || key == ImGuiKey_RightShift) shift = down;
        if (key == ImGuiKey_LeftAlt || key == ImGuiKey_RightAlt) alt = down;
        if (key == ImGuiKey_LeftSuper || key == ImGuiKey_RightSuper) bd->SuperDown = down;
    }
    io.AddKeyEvent(ImGuiMod_Ctrl, ctrl);
    io.AddKeyEvent(ImGuiMod_Shift, shift);
    io.AddKeyEvent(ImGuiMod_Alt, alt);
    io.AddKeyEvent(ImGuiMod_Super, bd->SuperDown);
}

static void InputHappened(ImGui_ImplBUI_Data* bd)
{
    bd->LastInputTime = now_seconds();
    if (bd->FramesPending < bd->Config.SettleFrames)
        bd->FramesPending = bd->Config.SettleFrames;
    if (bd->Node && !bd->InFrame)
        BK_custom_request_frame(bd->Node, 0);
}

static int Bui_Event(void* data, BKNode* node, const BKCustomEvent* e)
{
    ImGui_ImplBUI_Data* bd = (ImGui_ImplBUI_Data*)data;
    IM_UNUSED(node);
    ContextScope scope(bd->Ctx);
    ImGuiIO& io = ImGui::GetIO();
    int used = 1;
    bool pulse = false;
    const ImGuiKey key = (e->type == BK_CUSTOM_KEY || e->type == BK_CUSTOM_KEYUP) ? KeysymToImGuiKey(e->keysym, &pulse) : ImGuiKey_None;
    if (e->type != BK_CUSTOM_FOCUS && e->type != BK_CUSTOM_BLUR)
        UpdateModifiers(bd, io, e, key);
    switch (e->type)
    {
    case BK_CUSTOM_PRESS:
    case BK_CUSTOM_RELEASE:
    {
        const int b = MouseButton(e->button);
        io.AddMouseSourceEvent(ImGuiMouseSource_Mouse);
        io.AddMousePosEvent(e->x, e->y);
        if (b >= 0)
        {
            const bool down = e->type == BK_CUSTOM_PRESS;
            io.AddMouseButtonEvent(b, down);
            bd->ButtonsDown = down ? (bd->ButtonsDown | (1 << b)) : (bd->ButtonsDown & ~(1 << b));
        }
        break;
    }
    case BK_CUSTOM_MOTION:
        io.AddMouseSourceEvent(ImGuiMouseSource_Mouse);
        io.AddMousePosEvent(e->x, e->y);
        break;
    case BK_CUSTOM_LEAVE:
        if (bd->ButtonsDown == 0)
            io.AddMousePosEvent(-FLT_MAX, -FLT_MAX);
        break;
    case BK_CUSTOM_WHEEL:
        io.AddMouseSourceEvent(ImGuiMouseSource_Mouse);
        io.AddMousePosEvent(e->x, e->y);
        io.AddMouseWheelEvent(-e->wheel_x, e->wheel_y); // ImGui: wheel_x > 0 scrolls left
        break;
    case BK_CUSTOM_KEY:
    {
        // decided by the last frame, as ImGui documents for WantCaptureKeyboard
        used = io.WantCaptureKeyboard || io.WantTextInput;
        if (key != ImGuiKey_None)
        {
            io.AddKeyEvent(key, true);
            if (pulse)
                io.AddKeyEvent(key, false);
        }
        const unsigned char c0 = (unsigned char)e->text[0];
        if (c0 >= 32 && c0 != 127)
        {
            io.AddInputCharactersUTF8(e->text);
            used = used || io.WantTextInput;
        }
        break;
    }
    case BK_CUSTOM_KEYUP:
        if (key != ImGuiKey_None)
            io.AddKeyEvent(key, false);
        used = io.WantCaptureKeyboard;
        break;
    case BK_CUSTOM_FOCUS:
        io.AddFocusEvent(true);
        break;
    case BK_CUSTOM_BLUR:
        io.AddFocusEvent(false); // releases held keys and buttons
        bd->ButtonsDown = 0;
        bd->SuperDown = false;
        break;
    }
    InputHappened(bd);
    return used;
}

//-----------------------------------------------------------------------------
// Platform: frames
//-----------------------------------------------------------------------------

static int BkCursor(ImGuiMouseCursor c)
{
    switch (c)
    {
    case ImGuiMouseCursor_None: return BK_CURSOR_NONE;
    case ImGuiMouseCursor_TextInput: return BK_CURSOR_TEXT;
    case ImGuiMouseCursor_ResizeAll: return BK_CURSOR_MOVE;
    case ImGuiMouseCursor_ResizeNS: return BK_CURSOR_NS;
    case ImGuiMouseCursor_ResizeEW: return BK_CURSOR_EW;
    case ImGuiMouseCursor_ResizeNESW: return BK_CURSOR_NESW;
    case ImGuiMouseCursor_ResizeNWSE: return BK_CURSOR_NWSE;
    case ImGuiMouseCursor_Hand: return BK_CURSOR_HAND;
    case ImGuiMouseCursor_Wait: return BK_CURSOR_WAIT;
    case ImGuiMouseCursor_Progress: return BK_CURSOR_PROGRESS;
    case ImGuiMouseCursor_NotAllowed: return BK_CURSOR_NOT_ALLOWED;
    default: return BK_CURSOR_DEFAULT;
    }
}

static inline uint64_t HashMix(uint64_t h, uint64_t v)
{
    h ^= v + 0x9E3779B97F4A7C15ull + (h << 6) + (h >> 2);
    return h;
}

static uint64_t HashBytes(uint64_t h, const void* data, size_t n)
{
    const unsigned char* p = (const unsigned char*)data;
    while (n >= 8)
    {
        uint64_t v;
        memcpy(&v, p, 8);
        h = HashMix(h, v * 0xff51afd7ed558ccdull);
        p += 8;
        n -= 8;
    }
    uint64_t v = 0;
    memcpy(&v, p, n);
    return HashMix(h, v);
}

// Everything that decides the pixels.
static uint64_t HashDrawData(const ImDrawData* dd)
{
    uint64_t h = 1469598103934665603ull;
    h = HashMix(h, (uint64_t)dd->CmdListsCount);
    for (const ImDrawList* list : dd->CmdLists)
    {
        h = HashBytes(h, list->VtxBuffer.Data, (size_t)list->VtxBuffer.Size * sizeof(ImDrawVert));
        h = HashBytes(h, list->IdxBuffer.Data, (size_t)list->IdxBuffer.Size * sizeof(ImDrawIdx));
        for (const ImDrawCmd& cmd : list->CmdBuffer)
        {
            h = HashBytes(h, &cmd.ClipRect, sizeof(cmd.ClipRect));
            h = HashMix(h, (uint64_t)cmd.GetTexID());
            h = HashMix(h, ((uint64_t)cmd.VtxOffset << 32) ^ cmd.IdxOffset ^ ((uint64_t)cmd.ElemCount << 16));
            h = HashMix(h, (uint64_t)(uintptr_t)cmd.UserCallback);
        }
    }
    return h;
}

static void ApplyScale(ImGui_ImplBUI_Data* bd, float scale)
{
    ImGuiStyle& style = ImGui::GetStyle();
    if (bd->Config.BlenderStyle)
        ImGui_ImplBUI_StyleBlender(&style, scale);
    else if (bd->Scale > 0.0f)
        style.ScaleAllSizes(scale / bd->Scale);
    else
        style.ScaleAllSizes(scale);
    if (bd->Config.FontSize > 0.0f)
        style.FontSizeBase = bd->Config.FontSize;
    else if (bd->Config.BlenderFont && !bd->Config.BlenderStyle)
        style.FontSizeBase = ImGui_ImplBUI_BlenderFontSize(1.0f); // (StyleBlender sets it)
    style.FontScaleDpi = scale;
    bd->Scale = scale;
}

static void Bui_Frame(void* data, BKNode* node)
{
    ImGui_ImplBUI_Data* bd = (ImGui_ImplBUI_Data*)data;
    ContextScope scope(bd->Ctx);
    ImGuiIO& io = ImGui::GetIO();
    if (bd->Config.FollowScale && BK_scale() != bd->Scale)
        ApplyScale(bd, BK_scale());

    int rect[4];
    BK_node_rect(node, rect);
    io.DisplaySize = ImVec2((float)ImMax(rect[2], 1), (float)ImMax(rect[3], 1));
    io.DisplayFramebufferScale = ImVec2(1.0f, 1.0f);
    const double now = now_seconds();
    io.DeltaTime = bd->Time > 0.0 ? (float)ImMax(now - bd->Time, 1e-5) : 1.0f / 60.0f;
    bd->Time = now;

    bd->InFrame = true;
    ImGui::NewFrame();
    if (bd->Frame)
        bd->Frame(bd->User);
    ImGui::Render();
    bd->InFrame = false;

    ImDrawData* dd = ImGui::GetDrawData();
    const bool textures_changed = UpdateTextures(dd);
    bd->FrameCount++;

    // cursor
    if (!(io.ConfigFlags & ImGuiConfigFlags_NoMouseCursorChange))
    {
        const ImGuiMouseCursor c = io.MouseDrawCursor ? ImGuiMouseCursor_None : ImGui::GetMouseCursor();
        if (c != bd->LastCursor)
        {
            bd->LastCursor = c;
            BK_custom_set_cursor(node, BkCursor(c));
        }
    }

    // redraw only when the pixels can differ
    const uint64_t hash = HashDrawData(dd);
    const bool changed = !bd->HaveHash || hash != bd->LastHash || textures_changed;
    bd->LastHash = hash;
    bd->HaveHash = true;
    if (changed)
        BK_node_invalidate(node);

    // what may change without input
    bool next = false;
    if (bd->FramesPending > 0)
    {
        bd->FramesPending--;
        next = true;
    }
    if (changed)
        next = true; // animating (or still settling): until two frames match
    ImGuiContext& g = *bd->Ctx;
    if (g.InputEventsQueue.Size > 0)
        next = true; // ImGui trickles fast input over several frames
    double wake = -1.0; // seconds until a frame is due
    auto wake_at = [&](double s) { if (s > 0.0 && (wake < 0.0 || s < wake)) wake = s; };
    if (ImGui::IsAnyItemActive() && ImGui::IsAnyMouseDown())
        wake_at(1.0 / 30.0); // held repeat buttons, drag-scrolling: ImGui acts on time
    if (g.InputTextState.ID != 0 && g.ActiveId == g.InputTextState.ID && io.ConfigInputTextCursorBlink)
    {
        // the caret is visible while CursorAnim <= 0 or fmod(CursorAnim, 1.2) <= 0.8
        const float a = g.InputTextState.CursorAnim;
        const float m = a <= 0.0f ? a : fmodf(a, 1.20f);
        wake_at((m <= 0.80f ? 0.80f - m : 1.20f - m) + 0.005);
    }
    {
        // hover delays (tooltips, stationary checks) count from the last pointer move
        const ImGuiStyle& style = ImGui::GetStyle();
        const double since = now - bd->LastInputTime;
        const double delays[3] = { style.HoverStationaryDelay, style.HoverDelayShort, style.HoverDelayNormal };
        for (double d : delays)
            if (since < d + 0.02)
                wake_at(d + 0.02 - since);
    }
    if (next)
        BK_custom_request_frame(node, 0);
    else if (wake > 0.0)
        BK_custom_request_frame(node, (int)ceil(wake * 1000.0));
}

static void Bui_Draw(void* data, BKNode* node, BuiRaster* r, const int rect[4])
{
    ImGui_ImplBUI_Data* bd = (ImGui_ImplBUI_Data*)data;
    IM_UNUSED(node);
    ContextScope scope(bd->Ctx);
    ImGui_ImplBUI_RenderDrawData(ImGui::GetDrawData(), r, (float)rect[0], (float)rect[1]);
}

static void Bui_Free(void* data, BKNode* node)
{
    ImGui_ImplBUI_Data* bd = (ImGui_ImplBUI_Data*)data;
    if (bd->Node == node)
        bd->Node = nullptr; // the node went first: Shutdown has nothing to detach
}

//-----------------------------------------------------------------------------
// Platform: clipboard
//-----------------------------------------------------------------------------

static const char* GetClipboardText(ImGuiContext* ctx)
{
    ImGui_ImplBUI_Data* bd = (ImGui_ImplBUI_Data*)ctx->IO.BackendPlatformUserData;
    free(bd->ClipboardText);
    BKContext* bk = bd->Node ? BK_node_context(bd->Node) : nullptr;
    bd->ClipboardText = bk ? BK_clipboard_get(bk) : nullptr;
    return bd->ClipboardText;
}

static void SetClipboardText(ImGuiContext* ctx, const char* text)
{
    ImGui_ImplBUI_Data* bd = (ImGui_ImplBUI_Data*)ctx->IO.BackendPlatformUserData;
    BKContext* bk = bd->Node ? BK_node_context(bd->Node) : nullptr;
    if (bk)
        BK_clipboard_set(bk, text ? text : "");
}

//-----------------------------------------------------------------------------
// Init / shutdown
//-----------------------------------------------------------------------------

bool ImGui_ImplBUI_Init(BKNode* node, ImGui_ImplBUI_FrameFn frame, void* user, const ImGui_ImplBUI_Config* config)
{
    ImGuiIO& io = ImGui::GetIO();
    IMGUI_CHECKVERSION();
    IM_ASSERT(io.BackendPlatformUserData == nullptr && "Already initialized a platform backend!");
    if (node == nullptr || strcmp(BK_node_type(node), "custom") != 0)
        return false;

    ImGui_ImplBUI_Data* bd = IM_NEW(ImGui_ImplBUI_Data)();
    bd->Ctx = ImGui::GetCurrentContext();
    bd->Node = node;
    bd->Frame = frame;
    bd->User = user;
    if (config)
        bd->Config = *config;

    io.BackendPlatformUserData = (void*)bd;
    io.BackendRendererUserData = (void*)bd;
    io.BackendPlatformName = "imgui_impl_bui";
    io.BackendRendererName = "imgui_impl_bui";
    io.BackendFlags |= ImGuiBackendFlags_HasMouseCursors;
    io.BackendFlags |= ImGuiBackendFlags_RendererHasVtxOffset;
    io.BackendFlags |= ImGuiBackendFlags_RendererHasTextures;

    ImGuiPlatformIO& platform_io = ImGui::GetPlatformIO();
    platform_io.Platform_GetClipboardTextFn = GetClipboardText;
    platform_io.Platform_SetClipboardTextFn = SetClipboardText;
    platform_io.Renderer_TextureMaxWidth = platform_io.Renderer_TextureMaxHeight = 8192;
    platform_io.DrawCallback_ResetRenderState = DrawCallback_ResetRenderState;
    platform_io.DrawCallback_SetSamplerLinear = DrawCallback_SetSamplerLinear;
    platform_io.DrawCallback_SetSamplerNearest = DrawCallback_SetSamplerNearest;

    // Blender's font and look
    if (bd->Config.BlenderFont && io.Fonts->Fonts.Size == 0)
        ImGui_ImplBUI_AddBlenderFont(bd->Config.FontSize > 0.0f ? bd->Config.FontSize : 0.0f);
    ApplyScale(bd, BK_scale());

    BKCustomClass cls = {};
    cls.event = Bui_Event;
    cls.frame = Bui_Frame;
    cls.draw = Bui_Draw;
    cls.free = Bui_Free;
    cls.natural_w = cls.natural_h = 1; // takes the room its layout gives it ("fill")
    BK_custom_set(node, &cls, bd);
    bd->FramesPending = bd->Config.SettleFrames; // new windows size themselves over a few frames
    BK_custom_request_frame(node, 0);
    return true;
}

void ImGui_ImplBUI_Shutdown()
{
    ImGui_ImplBUI_Data* bd = GetBackendData();
    IM_ASSERT(bd != nullptr && "No platform backend to shutdown, or already shutdown?");
    ImGuiIO& io = ImGui::GetIO();
    ImGuiPlatformIO& platform_io = ImGui::GetPlatformIO();

    // textures only this context uses
    for (ImTextureData* tex : platform_io.Textures)
        if (tex->RefCount == 1 && tex->TexID != ImTextureID_Invalid)
        {
            BUI_texture_free((int)(intptr_t)tex->TexID);
            tex->SetTexID(ImTextureID_Invalid);
            tex->SetStatus(ImTextureStatus_Destroyed);
        }
    if (bd->Node)
        BK_custom_set(bd->Node, nullptr, nullptr);
    free(bd->ClipboardText);

    io.BackendPlatformName = io.BackendRendererName = nullptr;
    io.BackendPlatformUserData = io.BackendRendererUserData = nullptr;
    io.BackendFlags &= ~(ImGuiBackendFlags_HasMouseCursors | ImGuiBackendFlags_RendererHasVtxOffset | ImGuiBackendFlags_RendererHasTextures);
    platform_io.ClearPlatformHandlers();
    platform_io.ClearRendererHandlers();
    IM_DELETE(bd);
}

BKNode* ImGui_ImplBUI_GetNode()
{
    ImGui_ImplBUI_Data* bd = GetBackendData();
    return bd ? bd->Node : nullptr;
}

void ImGui_ImplBUI_RequestFrames(int n)
{
    ImGui_ImplBUI_Data* bd = GetBackendData();
    if (bd == nullptr || n <= 0)
        return;
    bd->FramesPending = ImMax(bd->FramesPending, n - 1);
    if (bd->Node && !bd->InFrame)
        BK_custom_request_frame(bd->Node, 0);
}

void ImGui_ImplBUI_RequestFrameIn(int ms)
{
    ImGui_ImplBUI_Data* bd = GetBackendData();
    if (bd && bd->Node)
        BK_custom_request_frame(bd->Node, ms);
}

int ImGui_ImplBUI_FrameCount()
{
    ImGui_ImplBUI_Data* bd = GetBackendData();
    return bd ? bd->FrameCount : 0;
}

//-----------------------------------------------------------------------------
// App helper
//-----------------------------------------------------------------------------

struct ImGui_ImplBUI_App
{
    BKNode*     Root = nullptr;
    BKNode*     Node = nullptr;
    BKContext*  Ctx = nullptr;
    BKWindow*   Win = nullptr;
    ImGuiContext* ImCtx = nullptr;
};

static ImGui_ImplBUI_App* CreateApp(const char* title, int width, int height, bool window, ImGui_ImplBUI_FrameFn frame, void* user, const ImGui_ImplBUI_Config* config)
{
    ImGui_ImplBUI_App* app = IM_NEW(ImGui_ImplBUI_App)();
    app->ImCtx = ImGui::GetCurrentContext();
    app->Root = BK_node_new("column");
    BK_set_num(app->Root, "padding", 0);
    BK_set_num(app->Root, "spacing", 0);
    app->Node = BK_node_new("custom");
    BK_node_add(app->Root, app->Node, -1);
    BK_set_num(app->Node, "fill", 1);
    app->Ctx = BK_context_new(app->Root);
    if (window && BK_window_supported())
        app->Win = BK_window_new(app->Ctx, title, width, height); // sets the UI scale (DPI)
    ImGui_ImplBUI_Init(app->Node, frame, user, config);
    if (app->Win)
        BK_window_show(app->Win);
    return app;
}

ImGui_ImplBUI_App* ImGui_ImplBUI_CreateApp(const char* title, int width, int height, ImGui_ImplBUI_FrameFn frame, void* user, const ImGui_ImplBUI_Config* config)
{
    return CreateApp(title, width, height, true, frame, user, config);
}

ImGui_ImplBUI_App* ImGui_ImplBUI_CreateHeadlessApp(ImGui_ImplBUI_FrameFn frame, void* user, const ImGui_ImplBUI_Config* config)
{
    return CreateApp(nullptr, 0, 0, false, frame, user, config);
}

void ImGui_ImplBUI_RunApp(ImGui_ImplBUI_App* app)
{
    if (app && app->Win)
        BK_window_run();
}

void ImGui_ImplBUI_DestroyApp(ImGui_ImplBUI_App* app)
{
    if (app == nullptr)
        return;
    {
        ContextScope scope(app->ImCtx);
        if (GetBackendData())
            ImGui_ImplBUI_Shutdown();
    }
    BK_window_free(app->Win);
    BK_context_free(app->Ctx);
    BK_node_free(app->Root);
    IM_DELETE(app);
}

BKWindow* ImGui_ImplBUI_AppWindow(ImGui_ImplBUI_App* app) { return app ? app->Win : nullptr; }
BKContext* ImGui_ImplBUI_AppContext(ImGui_ImplBUI_App* app) { return app ? app->Ctx : nullptr; }
BKNode* ImGui_ImplBUI_AppNode(ImGui_ImplBUI_App* app) { return app ? app->Node : nullptr; }

#endif // #ifndef IMGUI_DISABLE
