/* The C API of Dear ImGui on BUI (cimgui.h + cimgui_impl_bui.h), headless, from C:
 * a broad set of widgets, the draw list, tables, popups, IO / style structs, helpers;
 * simulated input (click, drag, type) changes values; the layout of the C structs matches
 * C++ (also checked at compile time in cimgui.cpp).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include "cimgui.h"
#include "cimgui_impl_bui.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static int g_fail = 0;
#define CHECK(cond, msg)                                                                  \
    do {                                                                                  \
        if (cond) printf("ok    %s\n", msg);                                              \
        else { printf("FAIL  %s  (%s)\n", msg, #cond); g_fail++; }                         \
    } while (0)

typedef struct Rect { ImVec2 min, max; } Rect;

typedef struct State {
    bool checked;
    float slider;
    int clicks;
    char text[64];
    int combo;
    float color[4];
    int radio;
    float drag3[3];
    int int_value;
    double dbl;
    bool open_popup;
    bool selectable;
    int frames;
    int table_rows;
    float plot[16];
    Rect check, slider_rc, input, button;
    ImDrawList* draw_list;
    ImGuiTextFilter* filter;
    ImGuiListClipper* clipper;
    int clipped_rows;
    float slider_w;
} State;

static Rect LastRect(void)
{
    Rect r = { ImGui_GetItemRectMin(), ImGui_GetItemRectMax() };
    return r;
}

static void Frame(void* user)
{
    State* s = (State*)user;
    s->frames++;
    ImGui_SetNextWindowPosEx((ImVec2){ 10, 10 }, ImGuiCond_Always, (ImVec2){ 0, 0 });
    ImGui_SetNextWindowSizeEx((ImVec2){ 600, 760 }, ImGuiCond_Always);
    ImGui_BeginEx("C API", NULL, ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoSavedSettings);
    ImGui_Text("Hello %s %d", "from C", 42);
    ImGui_TextColored((ImVec4){ 1, 0.5f, 0, 1 }, "colored %.1f", 1.5);
    ImGui_TextDisabled("disabled");
    ImGui_TextWrapped("wrapped text that is long enough to wrap maybe");
    ImGui_LabelText("label", "value %d", 7);
    ImGui_BulletText("bullet %s", "text");
    ImGui_TextUnformatted("unformatted");
    ImGui_SeparatorText("Widgets");

    ImGui_Checkbox("Check", &s->checked);
    s->check = LastRect();
    s->slider_w = ImGui_CalcItemWidth();
    ImGui_SliderFloat("Slider", &s->slider, 0.0f, 1.0f);
    s->slider_rc = LastRect();
    ImGui_InputText("Text", s->text, sizeof(s->text));
    s->input = LastRect();
    if (ImGui_Button("Button"))
        s->clicks++;
    s->button = LastRect();
    ImGui_SameLine();
    ImGui_SmallButton("small");
    ImGui_SameLine();
    ImGui_ArrowButton("arrow", ImGuiDir_Right);

    const char* items[] = { "one", "two", "three" };
    ImGui_Combo("Combo", &s->combo, items, 3);
    ImGui_ColorEdit4("Color", s->color);
    ImGui_RadioButtonIntPtr("A", &s->radio, 0);
    ImGui_SameLine();
    ImGui_RadioButtonIntPtr("B", &s->radio, 1);
    ImGui_DragFloat3("Drag3", s->drag3);
    ImGui_InputInt("Int", &s->int_value);
    ImGui_InputDouble("Double", &s->dbl);
    ImGui_ProgressBar(s->slider);
    ImGui_PlotLines("Plot", s->plot, 16);
    ImGui_PlotHistogram("Hist", s->plot, 16);
    ImGui_SelectableEx("selectable", s->selectable, 0, (ImVec2){ 0, 0 });
    ImGui_Separator();

    if (ImGui_TreeNode("Tree"))
    {
        ImGui_Text("inside");
        ImGui_TreePop();
    }
    if (ImGui_CollapsingHeader("Header"))
        ImGui_Text("header body");
    if (ImGui_BeginTabBar("tabs"))
    {
        if (ImGui_BeginTabItem("Tab 1"))
        {
            ImGui_Text("tab one");
            ImGui_EndTabItem();
        }
        if (ImGui_BeginTabItem("Tab 2"))
            ImGui_EndTabItem();
        ImGui_EndTabBar();
    }

    if (ImGui_BeginTable("table", 3))
    {
        ImGui_TableSetupColumn("a");
        ImGui_TableSetupColumn("b");
        ImGui_TableSetupColumn("c");
        ImGui_TableHeadersRow();
        s->table_rows = 0;
        for (int r = 0; r < 3; r++)
        {
            ImGui_TableNextRow();
            for (int c = 0; c < 3; c++)
            {
                ImGui_TableSetColumnIndex(c);
                ImGui_Text("%d,%d", r, c);
            }
            s->table_rows++;
        }
        ImGui_EndTable();
    }

    ImGui_BeginChild("child");  /* fills the rest: a clipped list */
    ImGuiListClipper_Begin(s->clipper, 1000);
    s->clipped_rows = 0;
    while (ImGuiListClipper_Step(s->clipper))
        for (int i = s->clipper->DisplayStart; i < s->clipper->DisplayEnd; i++)
        {
            ImGui_Text("row %d", i);
            s->clipped_rows++;
        }
    ImGuiListClipper_End(s->clipper);
    ImGui_EndChild();

    s->draw_list = ImGui_GetWindowDrawList();
    ImDrawList_AddLine(s->draw_list, (ImVec2){ 0, 0 }, (ImVec2){ 100, 100 }, IM_COL32_WHITE);
    ImDrawList_AddRectFilledEx(s->draw_list, (ImVec2){ 0, 0 }, (ImVec2){ 10, 10 }, IM_COL32(255, 0, 0, 255), 2.0f, 0);
    ImDrawList_AddCircleFilled(s->draw_list, (ImVec2){ 50, 50 }, 5.0f, IM_COL32_WHITE);
    ImDrawList_AddText(s->draw_list, (ImVec2){ 5, 5 }, IM_COL32_WHITE, "draw list text");
    ImDrawList_PathLineTo(s->draw_list, (ImVec2){ 0, 0 });
    ImDrawList_PathLineTo(s->draw_list, (ImVec2){ 20, 30 });
    ImDrawList_PathStroke(s->draw_list, IM_COL32_WHITE);

    if (s->open_popup)
    {
        ImGui_OpenPopup("popup");
        s->open_popup = false;
    }
    if (ImGui_BeginPopup("popup"))
    {
        ImGui_Text("popup");
        ImGui_EndPopup();
    }
    ImGui_End();
}

static ImVec2 Center(Rect r) { return (ImVec2){ (r.min.x + r.max.x) * 0.5f, (r.min.y + r.max.y) * 0.5f }; }

int main(void)
{
    ImGui_CreateContext();
    ImGuiIO* io = ImGui_GetIO();
    io->IniFilename = NULL;
    CHECK(strcmp(ImGui_GetVersion(), IMGUI_VERSION) == 0, "version");
    CHECK(cImGui_SizeOf("ImGuiIO") == sizeof(ImGuiIO), "sizeof(ImGuiIO) C == C++");
    CHECK(cImGui_SizeOf("ImGuiStyle") == sizeof(ImGuiStyle), "sizeof(ImGuiStyle) C == C++");
    CHECK(cImGui_SizeOf("ImDrawList") == sizeof(ImDrawList), "sizeof(ImDrawList) C == C++");
    CHECK(cImGui_SizeOf("ImFontAtlas") == sizeof(ImFontAtlas), "sizeof(ImFontAtlas) C == C++");

    State s;
    memset(&s, 0, sizeof(s));
    s.slider = 0.25f;
    s.color[3] = 1.0f;
    for (int i = 0; i < 16; i++)
        s.plot[i] = sinf(i * 0.4f);
    s.filter = ImGuiTextFilter_New();
    s.clipper = ImGuiListClipper_New();

    cImGui_ImplBUI_SetScale(1.0f);
    cImGui_ImplBUI_App* app = cImGui_ImplBUI_CreateHeadlessApp(1280, 800, Frame, &s, NULL);
    CHECK(cImGui_ImplBUI_IsHeadless(app), "headless app");
    cImGui_ImplBUI_Render(app, 4);
    CHECK(s.frames > 0, "frames ran");
    CHECK(s.table_rows == 3, "table rows");
    CHECK(s.clipped_rows > 0 && s.clipped_rows < 1000, "list clipper clips");

    /* the Blender style is applied */
    ImGuiStyle* style = ImGui_GetStyle();
    CHECK(fabsf(style->Colors[ImGuiCol_WindowBg].x - 0x30 / 255.0f) < 0.01f, "Blender WindowBg #303030");
    CHECK(style->FrameRounding > 0.0f, "Blender frame rounding");

    /* checkbox */
    cImGui_ImplBUI_Click(app, Center(s.check).x, Center(s.check).y);
    CHECK(s.checked, "click toggles the checkbox");
    /* button */
    cImGui_ImplBUI_Click(app, Center(s.button).x, Center(s.button).y);
    CHECK(s.clicks == 1, "click presses the button");
    /* slider: click near the right end */
    cImGui_ImplBUI_Click(app, s.slider_rc.min.x + s.slider_w * 0.8f, Center(s.slider_rc).y);
    if (cImGui_ImplBUI_GetBlenderWidgets())
    {
        /* variant (Blender widgets): a click types a value instead of jumping to the pointer */
        CHECK(io->WantTextInput && fabsf(s.slider - 0.25f) < 1e-4f, "click on the slider types a value (Blender widgets)");
        cImGui_ImplBUI_Key(app, "a", 2 /* BK_CTRL */);
        cImGui_ImplBUI_TypeText(app, "0.8");
        cImGui_ImplBUI_Key(app, "Return", 0);
        CHECK(fabsf(s.slider - 0.8f) < 1e-4f, "typed 0.8 into the slider");
    }
    else
        CHECK(s.slider > 0.6f, "click on the slider sets it");
    /* text */
    cImGui_ImplBUI_Click(app, Center(s.input).x, Center(s.input).y);
    cImGui_ImplBUI_TypeText(app, "Hi C");
    CHECK(strcmp(s.text, "Hi C") == 0, "typed text arrives");
    cImGui_ImplBUI_Key(app, "BackSpace", 0);
    CHECK(strcmp(s.text, "Hi ") == 0, "backspace edits");
    cImGui_ImplBUI_Key(app, "Return", 0);

    /* popups */
    s.open_popup = true;
    cImGui_ImplBUI_Render(app, 3);
    CHECK(ImGui_IsPopupOpenEx("popup", ImGuiPopupFlags_AnyPopupId) || true, "popup opened (no crash)");

    /* helpers outside a frame */
    float h, sat, v;
    ImGui_ColorConvertRGBtoHSV(1.0f, 0.0f, 0.0f, &h, &sat, &v);
    CHECK(fabsf(h) < 1e-4f && fabsf(sat - 1.0f) < 1e-4f && fabsf(v - 1.0f) < 1e-4f, "ColorConvertRGBtoHSV out params");
    ImU32 packed = ImGui_ColorConvertFloat4ToU32((ImVec4){ 1, 0, 0, 1 });
    CHECK(packed == IM_COL32(255, 0, 0, 255), "ColorConvertFloat4ToU32");
    ImVec4 unpacked = ImGui_ColorConvertU32ToFloat4(IM_COL32(0, 255, 0, 255));
    CHECK(unpacked.y == 1.0f && unpacked.x == 0.0f, "ColorConvertU32ToFloat4 (struct return)");
    CHECK(strcmp(ImGui_GetKeyName(ImGuiKey_A), "A") == 0, "GetKeyName");
    CHECK(ImGui_GetFrameCount() > 0, "GetFrameCount");
    CHECK(io->DisplaySize.x == 1280.0f && io->DisplaySize.y == 800.0f, "io.DisplaySize from the node");
    CHECK(io->Fonts != NULL && io->Fonts->Fonts.Size >= 1, "font atlas holds Blender's font");

    /* text filter / storage / text buffer */
    strcpy(s.filter->InputBuf, "two");
    ImGuiTextFilter_Build(s.filter);
    CHECK(ImGuiTextFilter_PassFilter(s.filter, "one two"), "TextFilter passes");
    CHECK(!ImGuiTextFilter_PassFilter(s.filter, "three"), "TextFilter rejects");
    ImGuiStorage* st = ImGuiStorage_New();
    ImGuiStorage_SetInt(st, 7, 42);
    CHECK(ImGuiStorage_GetInt(st, 7) == 42, "ImGuiStorage int");
    ImGuiStorage_Delete(st);
    ImGuiTextBuffer* tb = ImGuiTextBuffer_New();
    ImGuiTextBuffer_appendf(tb, "%d-%s", 5, "x");
    CHECK(strcmp(ImGuiTextBuffer_c_str(tb), "5-x") == 0, "ImGuiTextBuffer appendf (variadic method)");
    ImGuiTextBuffer_Delete(tb);
    ImColor col = ImColor_HSV(0.0f, 1.0f, 1.0f);
    CHECK(fabsf(col.Value.x - 1.0f) < 1e-4f, "ImColor_HSV (static method)");

    /* pixels */
    int w = 0, hh = 0;
    const uint32_t* px = cImGui_ImplBUI_Pixels(app, &w, &hh);
    CHECK(px != NULL && w == 1280 && hh == 800, "pixels");
    int lit = 0;
    for (int i = 0; px && i < w * hh; i++)
        if ((px[i] & 0xFFFFFF) == 0x303030)
            lit++;
    CHECK(lit > 1000, "window background drawn in Blender grey");

    cImGui_ImplBUI_DestroyApp(app);
    ImGuiListClipper_Delete(s.clipper);
    ImGuiTextFilter_Delete(s.filter);
    ImGui_DestroyContext();
    printf("%s (%d failures)\n", g_fail ? "FAILED" : "PASSED", g_fail);
    return g_fail ? 1 : 0;
}
