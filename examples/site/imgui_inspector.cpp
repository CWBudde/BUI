// Dear ImGui on BUI from C++: a scene inspector (the same app as
// examples/site/imgui_inspector.py).
//
//   bui_site_imgui                           native window, Blender widgets + style
//   bui_site_imgui --stock                   Dear ImGui's own widgets and colors
//   bui_site_imgui --screenshot out.png      off-screen (BUI_THEME, BUI_UI_SCALE)
//
// SPDX-License-Identifier: GPL-2.0-or-later

#include <cmath>
#include <cstdio>
#include <cstring>

#include "BUI_kit.h"
#include "GPU_raster.h"
#include "imgui.h"
#include "imgui_impl_bui.h"
extern "C" {
#include "site_png.h"
}

struct Object
{
    const char* name;
    const char* type;
    int faces;
};

static const Object kObjects[] = {{"Camera", "CAMERA", 1}, {"Key Light", "LIGHT", 1}, {"Robot Arm", "MESH", 48210},
                                  {"Floor", "MESH", 2}, {"Backdrop", "MESH", 512}};

struct State
{
    float loc[3] = {0.0f, 1.25f, 0.4f};
    float rot[3] = {0.0f, 0.0f, 90.0f};
    float color[3] = {0.80f, 0.33f, 0.18f};
    float metal = 0.9f, rough = 0.35f;
    int engine = 0, samples = 128, selected = 2;
    bool denoise = true, shadows = true;
    float frame_ms[90];
};

static void Frame(void* user)
{
    State& s = *(State*)user;
    const ImVec2 size = ImGui::GetIO().DisplaySize;
    ImGui::SetNextWindowPos(ImVec2(0, 0));
    ImGui::SetNextWindowSize(size);
    ImGui::Begin("Scene Inspector", nullptr, ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoResize);
    if (ImGui::BeginTable("objects", 3, ImGuiTableFlags_BordersInnerV | ImGuiTableFlags_RowBg | ImGuiTableFlags_BordersOuter))
    {
        ImGui::TableSetupColumn("Object");
        ImGui::TableSetupColumn("Type");
        ImGui::TableSetupColumn("Faces");
        ImGui::TableHeadersRow();
        for (int i = 0; i < IM_COUNTOF(kObjects); i++)
        {
            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            if (ImGui::Selectable(kObjects[i].name, s.selected == i, ImGuiSelectableFlags_SpanAllColumns))
                s.selected = i;
            ImGui::TableNextColumn();
            ImGui::TextUnformatted(kObjects[i].type);
            ImGui::TableNextColumn();
            ImGui::Text(kObjects[i].faces >= 1000 ? "%d,%03d" : "%d", kObjects[i].faces >= 1000 ? kObjects[i].faces / 1000 : kObjects[i].faces,
                        kObjects[i].faces % 1000);
        }
        ImGui::EndTable();
    }
    ImGui::Spacing();
    if (ImGui::CollapsingHeader("Transform", ImGuiTreeNodeFlags_DefaultOpen))
    {
        ImGui::DragFloat3("Location", s.loc, 0.01f);
        ImGui::DragFloat3("Rotation", s.rot, 1.0f);
    }
    if (ImGui::CollapsingHeader("Material", ImGuiTreeNodeFlags_DefaultOpen))
    {
        ImGui::ColorEdit3("Base Color", s.color);
        ImGui::SliderFloat("Metallic", &s.metal, 0.0f, 1.0f);
        ImGui::SliderFloat("Roughness", &s.rough, 0.0f, 1.0f);
    }
    if (ImGui::CollapsingHeader("Render", ImGuiTreeNodeFlags_DefaultOpen))
    {
        const char* engines[] = {"Cycles", "EEVEE", "Workbench"};
        ImGui::Combo("Engine", &s.engine, engines, IM_COUNTOF(engines));
        ImGui::SliderInt("Samples", &s.samples, 1, 4096);
        ImGui::Checkbox("Denoise", &s.denoise);
        ImGui::SameLine();
        ImGui::Checkbox("Soft Shadows", &s.shadows);
        ImGui::ProgressBar(0.62f, ImVec2(-1, 0), "Rendering 62%");
        ImGui::PlotLines("##ms", s.frame_ms, IM_COUNTOF(s.frame_ms), 0, "Frame time (ms)", 0.0f, 30.0f, ImVec2(-1, 60));
        ImGui::Button("Render Image");
        ImGui::SameLine();
        ImGui::Button("Cancel");
    }
    ImGui::End();
}

int main(int argc, char** argv)
{
    const char* shot = nullptr;
    bool stock = false;
    for (int i = 1; i < argc; i++)
    {
        if (!strcmp(argv[i], "--screenshot") && i + 1 < argc)
            shot = argv[++i];
        else if (!strcmp(argv[i], "--stock"))
            stock = true;
    }
    State state;
    for (int i = 0; i < IM_COUNTOF(state.frame_ms); i++)
        state.frame_ms[i] = 16.6f + 2.5f * sinf(i * 0.45f) + (i % 17 == 0 ? 6.0f : 0.0f);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGui::GetIO().IniFilename = nullptr;
    ImGui_ImplBUI_Config config;
    config.BlenderStyle = !stock;
    if (stock)
    {
        // Dear ImGui's own look: its widgets and its dark / light colors, following BUI_THEME
        if (strcmp(BK_theme_name(), "light") == 0)
            ImGui::StyleColorsLight();
        else
            ImGui::StyleColorsDark();
    }
    const int W = 520, H = 490;
    if (shot == nullptr && BK_window_supported())
    {
        ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateApp("Scene Inspector", W, H, Frame, &state, &config);
        ImGui_ImplBUI_SetBlenderWidgets(!stock && ImGui_ImplBUI_HasBlenderWidgets());
        ImGui_ImplBUI_RunApp(app);
        ImGui_ImplBUI_DestroyApp(app);
    }
    else
    {
        BK_set_scale(site_scale());
        const int w = (int)(W * BK_scale()), h = (int)(H * BK_scale());
        ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateHeadlessApp(Frame, &state, &config);
        ImGui_ImplBUI_SetBlenderWidgets(!stock && ImGui_ImplBUI_HasBlenderWidgets());
        BuiRaster* r = BUI_raster_create(w, h);
        BKContext* ctx = ImGui_ImplBUI_AppContext(app);
        for (int i = 0; i < 8; i++)
            BK_render(ctx, r, w, h, i == 0);
        if (shot)
            site_write_png(shot, BUI_raster_pixels(r), w, h);
        BUI_raster_free(r);
        ImGui_ImplBUI_DestroyApp(app);
    }
    ImGui::DestroyContext();
    return 0;
}
