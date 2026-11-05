// Dear ImGui's widgets next to BUI's native kit widgets, in one BUI frame: the left column
// is the kit (BK_node_new("checkbox"), "number", ...), the right one the same widgets
// through ImGui. Built with BUI_IMGUI_BLENDER_WIDGETS=ON the two columns should match
// pixel for pixel in style; without it the right column shows ImGui's own widgets in the
// Blender colors. Also renders ShowDemoWindow() with sections open, and an open menu.
//
//   bui_imgui_compare out_dir     writes compare.png, demo.png, demo_menu.png (headless)
//
// SPDX-License-Identifier: GPL-2.0-or-later

#include "imgui.h"
#include "imgui_internal.h"
#include "imgui_impl_bui.h"

#include "BUI_kit.h"
#include "bui_png.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <string>

static const int COL_W = 300;

struct State
{
    bool check = true, check2 = false;
    float number = 0.5f, slider = 0.25f;
    float vec[3] = { 1.0f, 0.0f, 2.5f };
    char text[64] = "Edit me";
    int combo = 0, radio = 0;
    float color[4] = { 0.278f, 0.447f, 0.702f, 1.0f };
    bool demo = false;
    int mode = 0; // 0: the comparison, 1: the demo window
};
static State S;
static ImRect g_items[16]; // the ImGui column's items, for the pixel comparison
static int g_nitems = 0;
static void Item() { if (g_nitems < 16) g_items[g_nitems++] = ImRect(ImGui::GetItemRectMin(), ImGui::GetItemRectMax()); }

static void ImGuiColumn()
{
    ImGui::SetNextWindowPos(ImVec2(0, 0));
    ImGui::SetNextWindowSize(ImVec2((float)COL_W, 400));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(0, 0));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowBorderSize, 0.0f); // (the border would clip a pixel)
    ImGui::Begin("compare", nullptr, ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoBackground | ImGuiWindowFlags_NoSavedSettings);
    g_nitems = 0;
    ImGui::PushItemWidth(-FLT_MIN);
    ImGui::Checkbox("Checkbox", &S.check);
    Item();
    ImGui::Checkbox("Option", &S.check2);
    Item();
    ImGui::DragFloat("##number", &S.number, 0.01f, 0.0f, 0.0f, "Number: %.3f");
    Item();
    ImGui::SliderFloat("##slider", &S.slider, 0.0f, 1.0f, "Slider: %.3f");
    Item();
    ImGui::DragFloat3("##vec", S.vec, 0.01f, 0.0f, 0.0f, "%.3f");
    Item();
    ImGui::InputText("##text", S.text, sizeof(S.text));
    Item();
    ImGui::Button("Button", ImVec2(-FLT_MIN, 0));
    Item();
    ImGui::Combo("##combo", &S.combo, "Alpha\0Beta\0Gamma\0");
    Item();
    ImGui::ProgressBar(0.6f, ImVec2(-FLT_MIN, 0));
    Item();
    ImGui::ColorButton("##swatch", ImVec4(S.color[0], S.color[1], S.color[2], S.color[3]), ImGuiColorEditFlags_NoTooltip, ImVec2(ImGui::GetContentRegionAvail().x, ImGui::GetFrameHeight()));
    Item();
    ImGui::PopItemWidth();
    ImGui::End();
    ImGui::PopStyleVar(2);
}

static void Frame(void*)
{
    if (S.mode == 0)
    {
        ImGuiColumn();
        return;
    }
    // the demo window, large, plus a window of our own with a menu bar
    ImGui::ShowDemoWindow(&S.demo);
    ImGui::SetNextWindowPos(ImVec2(20, 20), ImGuiCond_Always);
    ImGui::SetNextWindowSize(ImVec2(360, 420), ImGuiCond_Always);
    ImGui::Begin("Custom window", nullptr, ImGuiWindowFlags_MenuBar | ImGuiWindowFlags_NoSavedSettings);
    if (ImGui::BeginMenuBar())
    {
        if (ImGui::BeginMenu("File"))
        {
            ImGui::MenuItem("New", "Ctrl N");
            ImGui::MenuItem("Open...", "Ctrl O");
            static bool autosave = true;
            ImGui::MenuItem("Auto Save", nullptr, &autosave);
            ImGui::Separator();
            if (ImGui::BeginMenu("Export"))
            {
                ImGui::MenuItem("glTF");
                ImGui::EndMenu();
            }
            ImGui::MenuItem("Quit", "Ctrl Q");
            ImGui::EndMenu();
        }
        if (ImGui::BeginMenu("Edit"))
            ImGui::EndMenu();
        ImGui::EndMenuBar();
    }
    ImGui::Text("Blender widgets: %s", ImGui_ImplBUI_GetBlenderWidgets() ? "on" : "off");
    ImGui::Checkbox("Checkbox", &S.check);
    ImGui::RadioButton("One", &S.radio, 0); ImGui::SameLine();
    ImGui::RadioButton("Two", &S.radio, 1);
    ImGui::DragFloat("Drag", &S.number, 0.01f);
    ImGui::SliderFloat("Slider", &S.slider, 0.0f, 1.0f);
    ImGui::DragFloat3("Vector", S.vec, 0.01f);
    ImGui::InputText("Text", S.text, sizeof(S.text));
    ImGui::Combo("Combo", &S.combo, "Alpha\0Beta\0Gamma\0");
    ImGui::ColorEdit4("Color", S.color);
    ImGui::ProgressBar(0.6f);
    if (ImGui::Button("Button")) {}
    ImGui::SameLine();
    ImGui::SmallButton("Small");
    ImGui::SameLine();
    ImGui::ArrowButton("##left", ImGuiDir_Left);
    if (ImGui::CollapsingHeader("Collapsing header", ImGuiTreeNodeFlags_DefaultOpen))
    {
        if (ImGui::TreeNode("Tree node"))
        {
            ImGui::Selectable("Selectable", true);
            ImGui::Selectable("Another");
            ImGui::TreePop();
        }
    }
    if (ImGui::BeginTabBar("tabs"))
    {
        if (ImGui::BeginTabItem("Tab A")) { ImGui::Text("Contents A"); ImGui::EndTabItem(); }
        if (ImGui::BeginTabItem("Tab B")) { ImGui::EndTabItem(); }
        ImGui::EndTabBar();
    }
    ImGui::End();
}

static BKNode* Add(BKNode* parent, const char* type)
{
    BKNode* n = BK_node_new(type);
    BK_node_add(parent, n, -1);
    return n;
}

static void Shot(BKContext* ctx, BuiRaster* r, int w, int h, const std::string& path)
{
    BK_render(ctx, r, w, h, 1);
    if (bui_png_write(path.c_str(), BUI_raster_pixels(r), w, h))
        printf("wrote %s\n", path.c_str());
}

int main(int argc, char** argv)
{
    const std::string dir = argc > 1 ? argv[1] : ".";
    int compare_bad = -1;
    BK_set_scale(1.0f);
    ImGui::CreateContext();
    ImGui::GetIO().IniFilename = nullptr;

    // -- the comparison: kit column | ImGui column --
    {
        const int W = 2 * COL_W + 3 * 16, H = 280;
        BKNode* root = BK_node_new("row");
        BK_set_num(root, "padding", 16);
        BK_set_num(root, "spacing", 16);
        BKNode* kit = Add(root, "column");
        BK_set_num(kit, "padding", 0);
        BK_set_num(kit, "fill", 1); // the row splits evenly: COL_W each
        BKNode* n;
        n = Add(kit, "checkbox"); BK_set_str(n, "label", "Checkbox"); BK_set_num(n, "value", 1);
        n = Add(kit, "checkbox"); BK_set_str(n, "label", "Option");
        n = Add(kit, "number"); BK_set_str(n, "label", "Number"); BK_set_num(n, "value", 0.5); BK_set_num(n, "precision", 3);
        n = Add(kit, "slider"); BK_set_str(n, "label", "Slider"); BK_set_num(n, "value", 0.25); BK_set_num(n, "precision", 3);
        BKNode* row = Add(kit, "row");
        BK_set_num(row, "align", 1);
        const double vec[3] = { 1.0, 0.0, 2.5 };
        for (double v : vec) { n = Add(row, "number"); BK_set_num(n, "value", v); BK_set_num(n, "precision", 3); BK_set_num(n, "fill", 1); }
        n = Add(kit, "text"); BK_set_str(n, "text", "Edit me");
        n = Add(kit, "button"); BK_set_str(n, "text", "Button");
        n = Add(kit, "dropdown");
        const char* items[] = { "Alpha", "Beta", "Gamma" };
        BK_set_strs(n, "items", items, 3);
        n = Add(kit, "progress"); BK_set_num(n, "factor", 0.6); BK_set_str(n, "text", "60%");
        n = Add(kit, "swatch"); BK_set_color(n, "color", S.color);
        BKNode* im = Add(root, "custom");

        BK_set_num(im, "fill", 1);
        BKContext* ctx = BK_context_new(root);
        ImGui_ImplBUI_Init(im, Frame, nullptr);
        BuiRaster* r = BUI_raster_create(W, H);
        for (int i = 0; i < 8; i++)
            BK_render(ctx, r, W, H, 1);
        Shot(ctx, r, W, H, dir + "/compare.png");
        // per widget: the kit node's pixels against the ImGui item's (same size, top-left aligned)
        {
            const uint32_t* px = BUI_raster_pixels(r);
            int nr[4];
            BK_node_rect(im, nr);
            int total_bad = 0;
            for (int i = 0; i < BK_node_child_count(kit) && i < g_nitems; i++)
            {
                int kr[4];
                BK_node_rect(BK_node_child(kit, i), kr);
                const int ix = nr[0] + (int)lroundf(g_items[i].Min.x), iy = nr[1] + (int)lroundf(g_items[i].Min.y);
                const int w = ImMin(kr[2], (int)lroundf(g_items[i].GetWidth())), h = ImMin(kr[3], (int)lroundf(g_items[i].GetHeight()));
                int bad = 0, maxd = 0;
                for (int y = 0; y < h; y++)
                    for (int x = 0; x < w; x++)
                    {
                        const uint32_t a = px[(size_t)(kr[1] + y) * W + kr[0] + x], b = px[(size_t)(iy + y) * W + ix + x];
                        int d = 0;
                        for (int sh = 0; sh < 24; sh += 8)
                            d = ImMax(d, abs((int)((a >> sh) & 0xFF) - (int)((b >> sh) & 0xFF)));
                        maxd = ImMax(maxd, d);
                        bad += d > 8;
                    }
                total_bad += bad;
                printf("  %-10s kit %3dx%-3d imgui %3.0fx%-3.0f: %4d of %5d px differ by more than 8 (max %d)\n", BK_node_type(BK_node_child(kit, i)),
                       kr[2], kr[3], g_items[i].GetWidth(), g_items[i].GetHeight(), bad, w * h, maxd);
            }
            printf("compare: %d px differ in total\n", total_bad);
            compare_bad = total_bad;
        }
        ImGui_ImplBUI_Shutdown();
        BUI_raster_free(r);
        BK_context_free(ctx);
        BK_node_free(root);
    }

    // -- the demo window with sections open, and a custom window with an open menu --
    {
        S.mode = 1;
        S.demo = true;
        const int W = 1280, H = 1000;
        ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateHeadlessApp(Frame, nullptr);
        BKContext* ctx = ImGui_ImplBUI_AppContext(app);
        BuiRaster* r = BUI_raster_create(W, H);
        BK_render(ctx, r, W, H, 1);
        ImGui::SetWindowPos("Dear ImGui Demo", ImVec2(400, 10));
        ImGui::SetWindowSize("Dear ImGui Demo", ImVec2(860, 980));
        if (ImGuiWindow* demo = ImGui::FindWindowByName("Dear ImGui Demo"))
        {
            const char* open[] = { "Widgets", "Basic", "Combo", "Trees", "Collapsing Headers" };
            for (const char* id : open)
                demo->DC.StateStorage->SetInt(demo->GetID(id), 1);
        }
        for (int i = 0; i < 10; i++)
        {
            ImGui_ImplBUI_RequestFrames(1);
            BK_render(ctx, r, W, H, 1);
        }
        Shot(ctx, r, W, H, dir + "/demo.png");
        // open the custom window's File menu (a click on it), hover "Open..."
        ImGuiWindow* win = ImGui::FindWindowByName("Custom window");
        if (win)
        {
            const ImRect bar = win->MenuBarRect();
            const int x = (int)bar.Min.x + 20, y = (int)bar.GetCenter().y;
            BK_motion(ctx, x, y, 0);
            BK_render(ctx, r, W, H, 0);
            BK_press(ctx, x, y, BK_BUTTON1);
            BK_render(ctx, r, W, H, 0);
            BK_release(ctx, x, y, 0);
            for (int i = 0; i < 4; i++)
                BK_render(ctx, r, W, H, 0);
            BK_motion(ctx, x + 10, y + 46, 0);
            for (int i = 0; i < 6; i++)
                BK_render(ctx, r, W, H, 0);
        }
        Shot(ctx, r, W, H, dir + "/demo_menu.png");
        BUI_raster_free(r);
        ImGui_ImplBUI_DestroyApp(app);
    }
    ImGui::DestroyContext();
    // with Blender widgets the two columns must be the same pixels (a CTest in that build)
    if (ImGui_ImplBUI_HasBlenderWidgets() && compare_bad != 0)
    {
        printf("FAIL  ImGui's Blender widgets differ from the kit's native widgets\n");
        return 1;
    }
    return 0;
}
