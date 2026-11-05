/* Dear ImGui on BUI from C (bindings/imgui/c/cimgui.h, generated from imgui.h):
 * ImGui_ShowDemoWindow() plus a small window of our own, in a native BUI window with
 * Blender's look.
 *
 *   bui_imgui_demo_c                         a window (Win32 / X11 / Cocoa)
 *   bui_imgui_demo_c --screenshot out.png    headless: render a few frames to a PNG
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include "cimgui.h"
#include "cimgui_impl_bui.h"

#include <stdio.h>
#include <string.h>

typedef struct DemoState {
    bool show_demo;
    float value;
    int counter;
    float color[3];
    char text[128];
} DemoState;

static void frame(void* user)
{
    DemoState* s = (DemoState*)user;
    if (s->show_demo)
        ImGui_ShowDemoWindowEx(&s->show_demo);

    const float dpi = ImGui_GetStyle()->FontScaleDpi;
    ImGui_SetNextWindowPosEx((ImVec2){ 20 * dpi, 20 * dpi }, ImGuiCond_FirstUseEver, (ImVec2){ 0, 0 });
    ImGui_SetNextWindowSizeEx((ImVec2){ 340 * dpi, 300 * dpi }, ImGuiCond_FirstUseEver);
    ImGui_Begin("BUI from C");
    ImGui_Text("Dear ImGui %s on BUI, from C", ImGui_GetVersion());
    ImGui_TextDisabled("UI scale %.2f", cImGui_ImplBUI_GetScale());
    ImGui_SeparatorText("Widgets");
    ImGui_Checkbox("Demo window", &s->show_demo);
    ImGui_SliderFloat("Value", &s->value, 0.0f, 1.0f);
    if (ImGui_Button("Count"))
        s->counter++;
    ImGui_SameLine();
    ImGui_Text("%d", s->counter);
    ImGui_InputText("Text", s->text, sizeof(s->text));
    ImGui_ColorEdit3("Color", s->color);
    ImGui_ProgressBarEx(s->value, (ImVec2){ -1.0f, 0.0f }, NULL);

    /* the draw list API */
    ImDrawList* dl = ImGui_GetWindowDrawList();
    const ImVec2 p = ImGui_GetCursorScreenPos();
    ImDrawList_AddRectFilled(dl, p, (ImVec2){ p.x + 60 * dpi, p.y + 20 * dpi }, IM_COL32(71, 114, 179, 255));
    ImDrawList_AddCircle(dl, (ImVec2){ p.x + 90 * dpi, p.y + 10 * dpi }, 9 * dpi, IM_COL32_WHITE);
    ImGui_Dummy((ImVec2){ 100 * dpi, 22 * dpi });
    ImGui_End();
}

int main(int argc, char** argv)
{
    const char* shot = NULL;
    for (int i = 1; i < argc; i++)
        if (!strcmp(argv[i], "--screenshot") && i + 1 < argc)
            shot = argv[++i];

    ImGui_CreateContext();
    ImGuiIO* io = ImGui_GetIO();
    io->ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;

    DemoState state = { true, 0.5f, 0, { 0.278f, 0.447f, 0.702f }, "Edit me" };

    cImGui_ImplBUI_App* app = NULL;
    if (shot == NULL)
        app = cImGui_ImplBUI_CreateApp("Dear ImGui on BUI (C)", 1280, 800, frame, &state, NULL);
    if (app != NULL && !cImGui_ImplBUI_IsHeadless(app))
    {
        cImGui_ImplBUI_RunApp(app);
    }
    else
    {
        if (app)
            cImGui_ImplBUI_DestroyApp(app);
        io->IniFilename = NULL;
        app = cImGui_ImplBUI_CreateHeadlessApp(1280, 800, frame, &state, NULL);
        const int frames = cImGui_ImplBUI_Render(app, 8);
        const char* path = shot ? shot : "imgui_demo_c.png";
        printf("%s %s (%d frames)\n", cImGui_ImplBUI_SavePNG(app, path) ? "wrote" : "could not write", path, frames);
    }
    cImGui_ImplBUI_DestroyApp(app);
    ImGui_DestroyContext();
    return 0;
}
