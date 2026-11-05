// dear imgui: Platform + Renderer Backend for BUI (Blender-derived UI toolkit)
// This needs to be used along with the BUI kit (kit/include/BUI_kit.h).
//
// Implemented features:
//  [X] Platform: hosted in a BUI "custom" node: any BKContext, any BUI window host
//      (Win32, X11, Cocoa) or headless (BK_render into a BuiRaster).
//  [X] Platform: mouse (3 buttons, held together), precise vertical + horizontal wheel,
//      keyboard (ImGuiKey from X11 keysym names), typed text (UTF-8), modifiers, focus.
//  [X] Platform: mouse cursor shapes (ImGuiBackendFlags_HasMouseCursors), clipboard (the
//      host's clipboard through BUI), DPI scale (BK_scale()), time.
//  [X] Platform: damage driven: frames run on input, timers and animation only; an idle
//      UI costs no CPU (see "Idle" below).
//  [X] Renderer: BUI_raster_mesh on every BUI renderer (software raster, Direct3D 11,
//      OpenGL 3.3), per-command clip rectangles, ImDrawCmd::VtxOffset (large meshes).
//  [X] Renderer: dynamic textures (ImGuiBackendFlags_RendererHasTextures): the font atlas
//      grows and updates incrementally; user textures are BUI textures (ImTextureID =
//      BUI_texture_create() id).
//  [X] Renderer: DrawCallback_SetSamplerNearest / SetSamplerLinear / ResetRenderState.
//  [X] Blender's look by default: ImGui_ImplBUI_StyleBlender() colors and metrics from
//      Blender's default theme, Blender's UI font (Inter) at Blender's UI text size.
//  [ ] Platform: IME candidate window placement, gamepads, multi-viewports.
//
// Usage (one window):
//
//     static void frame(void*) { ImGui::ShowDemoWindow(); }
//     int main() {
//       ImGui::CreateContext();
//       ImGui_ImplBUI_App* app = ImGui_ImplBUI_CreateApp("Dear ImGui on BUI", 1280, 800, frame, nullptr);
//       ImGui_ImplBUI_RunApp(app);       // BUI's message loop until the window closes
//       ImGui_ImplBUI_DestroyApp(app);   // shuts the backend down
//       ImGui::DestroyContext();
//     }
//
// Embedding in an existing BUI UI: create a "custom" node, add it to a layout, make the
// ImGui context current and call ImGui_ImplBUI_Init(node, frame, user). The backend runs
// ImGui::NewFrame() / frame() / ImGui::Render() itself whenever BUI renders and a frame is
// due; do not call them yourself.
//
// Idle: every input event queues into ImGuiIO and asks the node for a frame. After a frame
// the backend asks for more only while something can still change: a few settle frames
// after input (hover states), frames while the output keeps changing (animations, a held
// repeat button), the text cursor blink while typing (two timer wakes per blink period)
// and the hover-delay tooltips (timer wakes after the pointer stops). Then nothing runs.
//
// SPDX-License-Identifier: GPL-2.0-or-later

#pragma once
#include "imgui.h" // IMGUI_IMPL_API
#ifndef IMGUI_DISABLE

struct BKNode;
struct BKContext;
struct BKWindow;
struct BuiRaster;

// Called inside each ImGui frame (between ImGui::NewFrame() and ImGui::Render()).
typedef void (*ImGui_ImplBUI_FrameFn)(void* user);

struct ImGui_ImplBUI_Config
{
    // ImGui_ImplBUI_StyleBlender() at the UI scale (else the current style is kept, scaled).
    bool    BlenderStyle = true;
    // Load Blender's UI font (Inter, embedded in BUI) into io.Fonts.
    bool    BlenderFont = true;
    // Font size in pixels at scale 1 (0: Blender's UI text, 11 px em). Scaled by BK_scale().
    float   FontSize = 0.0f;
    // Follow BK_scale() changes (DPI): re-apply the style and font size when it changes.
    bool    FollowScale = true;
    // Extra frames after an input event (ImGui settles hover state over a few frames).
    int     SettleFrames = 3;
};

// Attaches the current ImGui context to `node`, a BUI "custom" node (BK_node_new("custom")).
// The node may be added to a context before or after. `frame` builds the UI each frame.
IMGUI_IMPL_API bool     ImGui_ImplBUI_Init(BKNode* node, ImGui_ImplBUI_FrameFn frame, void* user, const ImGui_ImplBUI_Config* config = nullptr);
IMGUI_IMPL_API void     ImGui_ImplBUI_Shutdown();
IMGUI_IMPL_API BKNode*  ImGui_ImplBUI_GetNode();

// Ask for frames: e.g. after the application changed state shown by the UI. n frames
// back to back (paced to the display in a window), or one frame after `ms` milliseconds.
IMGUI_IMPL_API void     ImGui_ImplBUI_RequestFrames(int n = 1);
IMGUI_IMPL_API void     ImGui_ImplBUI_RequestFrameIn(int ms);
// Frames run since Init (tests, profiling) and whether the last frame changed the output.
IMGUI_IMPL_API int      ImGui_ImplBUI_FrameCount();

// Renderer pieces, for custom integrations (the backend calls them itself):
// draws ImDrawData into `r` with the DisplayPos at (ox, oy) in raster pixels, clipped to
// the raster's current clip.
IMGUI_IMPL_API void     ImGui_ImplBUI_RenderDrawData(ImDrawData* draw_data, BuiRaster* r, float ox, float oy);
// Handles one texture request (create / update / destroy) with BUI textures.
IMGUI_IMPL_API void     ImGui_ImplBUI_UpdateTexture(ImTextureData* tex);

// Blender's look: theme colors (userdef_default_theme.c) and metrics (widget unit 20 px,
// roundness, spacing) at `scale` (BK_scale()). Sets dst->FontSizeBase to Blender's UI text.
IMGUI_IMPL_API void     ImGui_ImplBUI_StyleBlender(ImGuiStyle* dst = nullptr, float scale = 1.0f);
// ImGui font size (ascent - descent, ImGui's size_pixels) giving Inter the em size of
// Blender's UI text (11 px) at `scale`.
IMGUI_IMPL_API float    ImGui_ImplBUI_BlenderFontSize(float scale = 1.0f);
// Adds Blender's UI font (Inter) to io.Fonts (data stays owned by BUI). nullptr when the
// BUI build has no embedded font.
IMGUI_IMPL_API ImFont*  ImGui_ImplBUI_AddBlenderFont(float size_pixels = 0.0f);

// Blender widgets (CMake BUI_IMGUI_BLENDER_WIDGETS=ON: blender/imgui_widgets_blender.cpp
// replaces imgui_widgets.cpp): ImGui's widgets drawn by the kit as Blender's widgets and
// handled with Blender's interaction rules. HasBlenderWidgets: built with them;
// SetBlenderWidgets: runtime switch (on by default when built; off draws ImGui's own).
IMGUI_IMPL_API bool     ImGui_ImplBUI_HasBlenderWidgets();
IMGUI_IMPL_API void     ImGui_ImplBUI_SetBlenderWidgets(bool on);
IMGUI_IMPL_API bool     ImGui_ImplBUI_GetBlenderWidgets();
// The draw callback marker those widgets emit (data: BKWidgetDraw + its texts).
IMGUI_IMPL_API void     ImGui_ImplBUI_DrawCallback_Widget(const ImDrawList* list, const ImDrawCmd* cmd);

// A complete app: a native BUI window (or, without a window system, nothing) whose whole
// client area is one ImGui node. CreateApp needs a current ImGui context.
struct ImGui_ImplBUI_App;
IMGUI_IMPL_API ImGui_ImplBUI_App* ImGui_ImplBUI_CreateApp(const char* title, int width, int height, ImGui_ImplBUI_FrameFn frame, void* user, const ImGui_ImplBUI_Config* config = nullptr);
IMGUI_IMPL_API void       ImGui_ImplBUI_RunApp(ImGui_ImplBUI_App* app);
IMGUI_IMPL_API void       ImGui_ImplBUI_DestroyApp(ImGui_ImplBUI_App* app);
IMGUI_IMPL_API BKWindow*  ImGui_ImplBUI_AppWindow(ImGui_ImplBUI_App* app);   // nullptr when headless
IMGUI_IMPL_API BKContext* ImGui_ImplBUI_AppContext(ImGui_ImplBUI_App* app);  // render it with BK_render
IMGUI_IMPL_API BKNode*    ImGui_ImplBUI_AppNode(ImGui_ImplBUI_App* app);
// Headless apps (no window system, or `headless`): CreateApp still builds the BUI context.
IMGUI_IMPL_API ImGui_ImplBUI_App* ImGui_ImplBUI_CreateHeadlessApp(ImGui_ImplBUI_FrameFn frame, void* user, const ImGui_ImplBUI_Config* config = nullptr);

#endif // #ifndef IMGUI_DISABLE
