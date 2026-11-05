// Dear ImGui on BUI, headless: drives the backend through BUI's own input entry points
// (BK_press / BK_motion / BK_key / BK_scroll, as the window hosts do) and BK_render, and
// checks ImGui's state and the pixels.
//  - the demo window runs with every section opened (test-engine hooks open each
//    openable item it reports), so all of imgui_demo.cpp executes;
//  - a checkbox toggles, a slider drags, a button clicks, text is typed, edited, selected
//    and copied to the clipboard, the wheel scrolls, the cursor shape follows;
//  - an idle UI runs no frames; input runs one;
//  - the font atlas reaches the raster (text pixels), and the GPU renderer (when a device
//    exists) draws the same frame as the software raster.
// BUI_IMGUI_SHOT=dir writes PNG screenshots (CPU and GPU).
//
// SPDX-License-Identifier: GPL-2.0-or-later

#include "imgui.h"
#include "imgui_internal.h"
#include "imgui_impl_bui.h"

#include "BUI_kit.h"
#include "bui_png.h"

#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <set>
#include <string>
#include <thread>
#include <utility>
#include <vector>

static const int W = 1280, H = 800;
static int g_fail = 0;

#define CHECK(cond, ...)                                                         \
    do {                                                                         \
        if (cond) { printf("ok    "); printf(__VA_ARGS__); printf("\n"); }      \
        else { printf("FAIL  "); printf(__VA_ARGS__); printf("  (%s)\n", #cond); g_fail++; } \
    } while (0)

//-----------------------------------------------------------------------------
// Test-engine hooks: collect openable items of the demo window that are closed
//-----------------------------------------------------------------------------

static bool g_collect = false;
static std::vector<std::pair<ImGuiStorage*, ImGuiID>> g_closed;

void ImGuiTestEngineHook_ItemAdd(ImGuiContext*, ImGuiID, const ImRect&, const ImGuiLastItemData*) {}
void ImGuiTestEngineHook_Log(ImGuiContext*, const char*, ...) {}
const char* ImGuiTestEngine_FindItemDebugLabel(ImGuiContext*, ImGuiID) { return nullptr; }
void ImGuiTestEngineHook_ItemInfo(ImGuiContext* ctx, ImGuiID id, const char* label, ImGuiItemStatusFlags flags)
{
    (void)label;
    if (!g_collect || !(flags & ImGuiItemStatusFlags_Openable) || (flags & ImGuiItemStatusFlags_Opened))
        return;
    ImGuiWindow* w = ctx->CurrentWindow;
    if (w == nullptr || w->DC.StateStorage == nullptr || strstr(w->Name, "Dear ImGui Demo") == nullptr)
        return;
    g_closed.emplace_back(w->DC.StateStorage, id);
}

//-----------------------------------------------------------------------------
// The UI under test
//-----------------------------------------------------------------------------

struct TestState
{
    bool checked = false;
    float slider = 0.25f;
    char text[64] = "";
    int clicks = 0;
    bool demo = true;
    float slider_w = 0.0f;
    ImRect check, slider_rc, input, button, empty;
};
static TestState T;

static void Frame(void*)
{
    ImGui::SetNextWindowPos(ImVec2(10, 10), ImGuiCond_Always);
    ImGui::SetNextWindowSize(ImVec2(420, 300), ImGuiCond_Always);
    ImGui::Begin("Test", nullptr, ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoSavedSettings);
    ImGui::Text("Hello from BUI: Blender's Inter in the ImGui atlas");
    ImGui::Checkbox("Check", &T.checked);
    T.check = ImRect(ImGui::GetItemRectMin(), ImGui::GetItemRectMax());
    T.slider_w = ImGui::CalcItemWidth();
    ImGui::SliderFloat("Slider", &T.slider, 0.0f, 1.0f);
    T.slider_rc = ImRect(ImGui::GetItemRectMin(), ImGui::GetItemRectMax());
    ImGui::InputText("Text", T.text, sizeof(T.text));
    T.input = ImRect(ImGui::GetItemRectMin(), ImGui::GetItemRectMax());
    if (ImGui::Button("Button"))
        T.clicks++;
    T.button = ImRect(ImGui::GetItemRectMin(), ImGui::GetItemRectMax());
    ImGui::Dummy(ImVec2(200, 60));
    T.empty = ImRect(ImGui::GetItemRectMin(), ImGui::GetItemRectMax());
    ImGui::End();
    if (T.demo)
        ImGui::ShowDemoWindow(&T.demo);
}

//-----------------------------------------------------------------------------
// Driving BUI
//-----------------------------------------------------------------------------

static BKContext* g_ctx;
static BuiRaster* g_r;

static void Render(int n = 1)
{
    for (int i = 0; i < n; i++)
        BK_render(g_ctx, g_r, W, H, 0);
}

static ImVec2 Center(const ImRect& r) { return ImVec2((r.Min.x + r.Max.x) * 0.5f, (r.Min.y + r.Max.y) * 0.5f); }

// node-relative == window coordinates: the ImGui node fills the context
static void Move(float x, float y) { BK_motion(g_ctx, (int)x, (int)y, 0); Render(2); }
static void Press(float x, float y) { BK_motion(g_ctx, (int)x, (int)y, 0); Render(1); BK_press(g_ctx, (int)x, (int)y, BK_BUTTON1); Render(2); }
static void Release(float x, float y) { BK_release(g_ctx, (int)x, (int)y, 0); Render(3); }
static void Click(float x, float y) { Press(x, y); Release(x, y); }

// typed characters arrive as text with the character as key name (Win32's WM_CHAR path),
// Shift as its own key, as the window hosts send them
static void Type(const char* s)
{
    for (; *s; s++)
    {
        const bool upper = *s >= 'A' && *s <= 'Z';
        char t[2] = { *s, 0 };
        char k[2] = { (char)(upper ? *s - 'A' + 'a' : *s), 0 };
        if (upper)
        {
            BK_key(g_ctx, "", "Shift_L", BK_SHIFT);
            Render(1);
        }
        BK_key(g_ctx, t, t, upper ? BK_SHIFT : 0);
        Render(1);
        BK_key_release(g_ctx, k, upper ? BK_SHIFT : 0);
        Render(1);
        if (upper)
        {
            BK_key_release(g_ctx, "Shift_L", 0);
            Render(1);
        }
        if (getenv("BUI_IMGUI_TRACE"))
            printf("      typed %c: \"%s\"\n", *s, T.text);
    }
    Render(3); // frames keep running while ImGui trickles queued input
}

static void Key(const char* keysym, int mods)
{
    if (mods & BK_CTRL)
        BK_key(g_ctx, "", "Control_L", BK_CTRL);
    BK_key(g_ctx, "", keysym, mods);
    Render(1);
    BK_key_release(g_ctx, keysym, mods);
    if (mods & BK_CTRL)
        BK_key_release(g_ctx, "Control_L", 0);
    Render(2);
}

static int CountTextPixels(const uint32_t* px, int x0, int y0, int x1, int y1)
{
    int n = 0;
    for (int y = y0; y < y1; y++)
        for (int x = x0; x < x1; x++)
        {
            const uint32_t c = px[(size_t)y * W + x];
            if (((c >> 16) & 0xFF) > 0xC0 && ((c >> 8) & 0xFF) > 0xC0 && (c & 0xFF) > 0xC0)
                n++;
        }
    return n;
}

static void Shot(const char* name, const uint32_t* px)
{
    const char* dir = getenv("BUI_IMGUI_SHOT");
    if (dir == nullptr || px == nullptr)
        return;
    std::string path = std::string(dir) + "/" + name;
    if (bui_png_write(path.c_str(), px, W, H))
        printf("wrote %s\n", path.c_str());
}

int main()
{
    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.IniFilename = nullptr;
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    ImGui::GetCurrentContext()->TestEngineHookItems = true;

    BK_set_scale(1.0f);
    ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateHeadlessApp(Frame, nullptr);
    g_ctx = ImGui_ImplBUI_AppContext(app);
    g_r = BUI_raster_create(W, H);
    BK_render(g_ctx, g_r, W, H, 1);
    Render(8);

    // -- the font atlas and text pixels --
    {
        const ImTextureData* tex = io.Fonts->TexData;
        int tw = 0, th = 0;
        const bool live = tex && tex->TexID != ImTextureID_Invalid && BUI_texture_size((int)tex->TexID, &tw, &th);
        CHECK(live, "font atlas uploaded as BUI texture %d (%dx%d)", tex ? (int)tex->TexID : 0, tw, th);
        CHECK(io.Fonts->Fonts.Size > 0 && strstr(io.Fonts->Fonts[0]->GetDebugName(), "Inter") != nullptr,
              "font is Blender's Inter (%s), size %.2f px", io.Fonts->Fonts.Size ? io.Fonts->Fonts[0]->GetDebugName() : "-", ImGui::GetStyle().FontSizeBase);
        const uint32_t* px = BUI_raster_pixels(g_r);
        const int text = CountTextPixels(px, 10, 10, 430, 310);
        CHECK(text > 300, "text drawn in the test window: %d bright pixels", text);
        const uint32_t bg = px[(size_t)(T.empty.Min.y + 20) * W + (int)T.empty.Min.x + 20];
        CHECK(bg == 0x303030u, "window background is Blender's region color #%06x", bg);
        const int demo_text = CountTextPixels(px, 660, 30, 1200, 700);
        CHECK(demo_text > 300, "demo window drawn: %d bright pixels", demo_text);
        Shot("imgui_cpu_start.png", px);
    }

    // -- checkbox --
    {
        const float h = T.check.GetHeight();
        Click(T.check.Min.x + h * 0.5f, T.check.Min.y + h * 0.5f);
        CHECK(T.checked, "checkbox toggled on by a click");
        Click(T.check.Min.x + h * 0.5f, T.check.Min.y + h * 0.5f);
        CHECK(!T.checked, "checkbox toggled off again");
    }

    // -- slider drag --
    {
        const ImVec2 c = Center(T.slider_rc);
        const float w = T.slider_w;
        const float x0 = T.slider_rc.Min.x + 2, x1 = T.slider_rc.Min.x + w - 2;
        Press(x0, c.y);
        const float at_press = T.slider;
        for (int i = 1; i <= 10; i++)
        {
            BK_motion(g_ctx, (int)(x0 + (x1 - x0) * i / 10), (int)c.y + (i % 2), BK_BUTTON1);
            Render(1);
        }
        Release(x1, c.y);
        CHECK(at_press < 0.05f && T.slider > 0.95f, "slider dragged from %.3f to %.3f", at_press, T.slider);
    }

    // -- button --
    {
        const ImVec2 c = Center(T.button);
        Click(c.x, c.y);
        CHECK(T.clicks == 1, "button clicked once (%d)", T.clicks);
    }

    // -- text input: typing, editing, cursor shape, select all + copy --
    {
        const ImVec2 c = Center(T.input);
        Move(c.x - 40, c.y);
        CHECK(BK_cursor(g_ctx) == BK_CURSOR_TEXT, "I-beam cursor over the text field (%d)", BK_cursor(g_ctx));
        Click(c.x - 40, c.y);
        CHECK(io.WantTextInput, "text field active (io.WantTextInput)");
        Type("Hellp");
        Key("BackSpace", 0);
        Type("o");
        CHECK(strcmp(T.text, "Hello") == 0, "typed into InputText: \"%s\"", T.text);
        Key("a", BK_CTRL);
        Key("c", BK_CTRL);
        char* clip = BK_clipboard_get(g_ctx);
        CHECK(clip && strcmp(clip, "Hello") == 0, "Ctrl+A, Ctrl+C copied \"%s\" to BUI's clipboard", clip ? clip : "");
        free(clip);
        Key("End", 0);
        Type(", BUI");
        CHECK(strcmp(T.text, "Hello, BUI") == 0, "End + more typing: \"%s\"", T.text);
        // click elsewhere: the field lets go
        Click(Center(T.empty).x, Center(T.empty).y);
        CHECK(!io.WantTextInput, "text field released by a click elsewhere");
    }

    // -- idle: no frames without input --
    {
        Move(Center(T.empty).x, Center(T.empty).y);
        std::this_thread::sleep_for(std::chrono::milliseconds(700)); // hover-delay wakes pass
        Render(4);
        const int f0 = ImGui_ImplBUI_FrameCount();
        const int animating = BK_animating(g_ctx);
        Render(30);
        const int f1 = ImGui_ImplBUI_FrameCount();
        CHECK(!animating && f1 == f0, "idle: no frames requested (%d -> %d, animating %d)", f0, f1, animating);
        BK_motion(g_ctx, (int)Center(T.empty).x + 3, (int)Center(T.empty).y, 0);
        Render(1);
        CHECK(ImGui_ImplBUI_FrameCount() > f1, "input runs a frame (%d -> %d)", f1, ImGui_ImplBUI_FrameCount());
        Render(8);
    }

    // -- the same frame on the GPU renderer --
    {
        BuiRaster* gpu = BUI_raster_create(W, H);
        const int have_gpu = BUI_raster_gpu_attach(gpu, nullptr, 1);
        std::this_thread::sleep_for(std::chrono::milliseconds(700)); // let the hover wakes pass
        Render(4);
        BK_render(g_ctx, g_r, W, H, 1);
        const uint32_t* cpu = BUI_raster_pixels(g_r);
        Shot("imgui_cpu.png", cpu);
        std::vector<uint32_t> cpu_copy(cpu, cpu + (size_t)W * H);
        if (have_gpu)
        {
            const int f0 = ImGui_ImplBUI_FrameCount();
            BK_render(g_ctx, gpu, W, H, 1);
            const uint32_t* gp = BUI_raster_pixels(gpu);
            int bad = 0, maxd = 0;
            for (size_t i = 0; i < cpu_copy.size(); i++)
            {
                int d = 0;
                for (int s = 0; s < 24; s += 8)
                    d = ImMax(d, abs((int)((cpu_copy[i] >> s) & 0xFF) - (int)((gp[i] >> s) & 0xFF)));
                maxd = ImMax(maxd, d);
                bad += d > 8;
            }
            CHECK(ImGui_ImplBUI_FrameCount() == f0, "GPU render replayed the frame without running ImGui");
            CHECK(bad < W * H / 1000, "GPU frame matches the software raster: %d pixels differ by more than 8 (max %d)", bad, maxd);
            Shot("imgui_gpu.png", gp);
        }
        else
            printf("skip  no GPU device for the GPU comparison\n");
        BUI_raster_free(gpu);
        BK_render(g_ctx, g_r, W, H, 1);
    }

    // -- open every section of the demo window: all of imgui_demo.cpp runs --
    {
        std::set<std::pair<ImGuiStorage*, ImGuiID>> opened;
        g_collect = true;
        int idle_rounds = 0, rounds = 0;
        for (; rounds < 300 && idle_rounds < 3; rounds++)
        {
            g_closed.clear();
            ImGui_ImplBUI_RequestFrames(1);
            Render(1);
            int added = 0;
            for (auto& it : g_closed)
                if (opened.insert(it).second)
                {
                    it.first->SetInt(it.second, 1);
                    added++;
                }
            idle_rounds = added ? 0 : idle_rounds + 1;
            if (rounds == 1)
                Shot("imgui_cpu_sections.png", BUI_raster_pixels(g_r));
        }
        g_collect = false;
        Render(4);
        CHECK(opened.size() > 150, "demo: opened %d sections/tree nodes over %d frames, all ran", (int)opened.size(), rounds);

        // the wheel scrolls the (now long) demo window
        ImGuiWindow* demo = ImGui::FindWindowByName("Dear ImGui Demo");
        const float before = demo ? demo->Scroll.y : 0.0f;
        const ImVec2 at = demo ? ImVec2(demo->Pos.x + 100, demo->Pos.y + 200) : ImVec2(800, 300);
        Move(at.x, at.y);
        BK_scroll(g_ctx, (int)at.x, (int)at.y, 0.0f, -3.0f, 0);
        Render(3);
        CHECK(demo && demo->Scroll.y > before, "wheel scrolled the demo window (%.0f -> %.0f)", before, demo ? demo->Scroll.y : 0.0f);

        // cost of drawing it (software raster, whole window): informational
        int tris = 0;
        for (const ImDrawList* l : ImGui::GetDrawData()->CmdLists)
            tris += l->IdxBuffer.Size / 3;
        const auto t0 = std::chrono::steady_clock::now();
        for (int i = 0; i < 5; i++)
            BK_render(g_ctx, g_r, W, H, 1);
        const double ms = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - t0).count() / 5;
        printf("info  full %dx%d software redraw of the open demo: %.2f ms (%d triangles)\n", W, H, ms, tris);
        // and a horizontal scroll reaches ImGui (tilt wheel): no crash, consumed
        BK_scroll(g_ctx, (int)at.x, (int)at.y, 1.0f, 0.0f, 0);
        Render(2);
    }

    // -- DPI scale changes follow BK_scale() --
    {
        BK_set_scale(1.5f);
        ImGui_ImplBUI_RequestFrames(2);
        Render(3);
        const ImGuiStyle& s = ImGui::GetStyle();
        CHECK(fabsf(s.FontScaleDpi - 1.5f) < 1e-3f && fabsf(s.FrameRounding - 6.0f) < 0.6f,
              "scale 1.5: FontScaleDpi %.2f, FrameRounding %.1f", s.FontScaleDpi, s.FrameRounding);
        Shot("imgui_cpu_scale150.png", BUI_raster_pixels(g_r));
        BK_set_scale(1.0f);
        ImGui_ImplBUI_RequestFrames(2);
        Render(3);
    }

    BUI_raster_free(g_r);
    ImGui_ImplBUI_DestroyApp(app);
    ImGui::DestroyContext();
    printf("%s (%d failure%s)\n", g_fail ? "FAILED" : "passed", g_fail, g_fail == 1 ? "" : "s");
    return g_fail ? 1 : 0;
}
