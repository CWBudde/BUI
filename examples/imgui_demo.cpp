// Dear ImGui on BUI: ImGui::ShowDemoWindow() plus a small window of our own, in a native
// BUI window, drawn by BUI's renderers (software raster, or the GPU with BUI_GPU=1) in
// Blender's look.
//
//   bui_imgui_demo                         a window (Win32 / X11 / Cocoa)
//   bui_imgui_demo --screenshot out.png    headless: render a few frames to a PNG
//                  [--gpu] [--size 1280x800] [--scale 1.5]
//
// SPDX-License-Identifier: GPL-2.0-or-later

#define IMGUI_DEFINE_MATH_OPERATORS
#include "imgui.h"
#include "imgui_impl_bui.h"

#include "BUI_kit.h"
#include "bui_png.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>

struct DemoState
{
    bool show_demo = true;
    float value = 0.5f;
    int counter = 0;
    float color[3] = { 0.278f, 0.447f, 0.702f }; // Blender's selection blue
    char text[128] = "Edit me";
    int texture = 0; // a BUI texture shown with ImGui::Image
    BuiRaster* raster = nullptr; // where the frames go (window or headless)
};

static int MakeTexture()
{
    // a 64x64 checker with a gradient: user textures are BUI textures (ImTextureID)
    static unsigned char px[64 * 64 * 4];
    for (int y = 0; y < 64; y++)
        for (int x = 0; x < 64; x++)
        {
            unsigned char* p = px + (y * 64 + x) * 4;
            const bool on = ((x / 8) + (y / 8)) & 1;
            p[0] = (unsigned char)(on ? 230 : 71);
            p[1] = (unsigned char)(on ? 230 : 114);
            p[2] = (unsigned char)(on ? 230 : 179);
            p[3] = (unsigned char)(128 + x * 2);
        }
    return BUI_texture_create(64, 64, px, 0);
}

static void Frame(void* user)
{
    DemoState* s = (DemoState*)user;
    if (s->show_demo)
        ImGui::ShowDemoWindow(&s->show_demo);

    ImGui::SetNextWindowPos(ImVec2(20, 20) * ImGui::GetStyle().FontScaleDpi, ImGuiCond_FirstUseEver);
    ImGui::SetNextWindowSize(ImVec2(340, 330) * ImGui::GetStyle().FontScaleDpi, ImGuiCond_FirstUseEver);
    ImGui::Begin("BUI");
    ImGui::Text("Dear ImGui %s on BUI", IMGUI_VERSION);
    // (nothing here changes by itself: an idle window runs no frames)
    ImGui::TextDisabled("UI scale %.2f, %s renderer", BK_scale(),
                        s->raster && BUI_raster_is_gpu(s->raster) ? "GPU" : "software");
    ImGui::SeparatorText("Widgets");
    ImGui::Checkbox("Demo window", &s->show_demo);
    ImGui::SliderFloat("Value", &s->value, 0.0f, 1.0f);
    if (ImGui::Button("Count"))
        s->counter++;
    ImGui::SameLine();
    ImGui::Text("%d", s->counter);
    ImGui::InputText("Text", s->text, sizeof(s->text));
    ImGui::ColorEdit3("Color", s->color);
    ImGui::ProgressBar(s->value, ImVec2(-FLT_MIN, 0));
    ImGui::SeparatorText("BUI texture");
    if (s->texture)
        ImGui::Image((ImTextureID)(intptr_t)s->texture, ImVec2(64, 64) * ImGui::GetStyle().FontScaleDpi);
    ImGui::End();
}

int main(int argc, char** argv)
{
    const char* shot = nullptr;
    int width = 1280, height = 800;
    float scale = 0.0f;
    bool gpu = false;
    for (int i = 1; i < argc; i++)
    {
        if (!strcmp(argv[i], "--screenshot") && i + 1 < argc)
            shot = argv[++i];
        else if (!strcmp(argv[i], "--size") && i + 1 < argc)
            sscanf(argv[++i], "%dx%d", &width, &height);
        else if (!strcmp(argv[i], "--scale") && i + 1 < argc)
            scale = (float)atof(argv[++i]);
        else if (!strcmp(argv[i], "--gpu"))
            gpu = true;
    }

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;

    DemoState state;
    state.texture = MakeTexture();

    if (shot == nullptr && BK_window_supported())
    {
        ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateApp("Dear ImGui on BUI", width, height, Frame, &state);
        if (ImGui_ImplBUI_AppWindow(app))
            state.raster = BK_window_raster(ImGui_ImplBUI_AppWindow(app));
        ImGui_ImplBUI_RunApp(app);
        ImGui_ImplBUI_DestroyApp(app);
    }
    else
    {
        // headless: BK_render into a raster (CPU, or the GPU offscreen)
        io.IniFilename = nullptr;
        if (scale > 0.0f)
            BK_set_scale(scale);
        ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateHeadlessApp(Frame, &state);
        BuiRaster* r = BUI_raster_create(width, height);
        state.raster = r;
        if (gpu && !BUI_raster_gpu_attach(r, nullptr, 1))
            fprintf(stderr, "no GPU device: software raster\n");
        BKContext* ctx = ImGui_ImplBUI_AppContext(app);
        for (int i = 0; i < 8; i++)
        {
            BK_render(ctx, r, width, height, i == 0);
            if (getenv("BUI_IMGUI_TRACE"))
                printf("render %d: %d frames, animating %d\n", i, ImGui_ImplBUI_FrameCount(), BK_animating(ctx));
        }
        const char* path = shot ? shot : "imgui_demo.png";
        const int ok = bui_png_write(path, BUI_raster_pixels(r), width, height);
        printf("%s %s (%s, %d frames)\n", ok ? "wrote" : "could not write", path,
               BUI_raster_is_gpu(r) ? "GPU" : "CPU", ImGui_ImplBUI_FrameCount());
        BUI_raster_free(r);
        ImGui_ImplBUI_DestroyApp(app);
    }
    BUI_texture_free(state.texture);
    ImGui::DestroyContext();
    return 0;
}
