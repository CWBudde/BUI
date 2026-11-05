/* cimgui_impl_bui.h: the BUI backend of Dear ImGui (imgui_impl_bui.h) for C, plus headless
 * helpers (render into an offscreen raster, inject input) used by the C, Python and Rust
 * bindings and their tests.
 *
 *     static void frame(void* user) { ImGui_ShowDemoWindow(NULL); }
 *     int main(void) {
 *         ImGui_CreateContext();
 *         cImGui_ImplBUI_App* app = cImGui_ImplBUI_CreateApp("Dear ImGui on BUI", 1280, 800, frame, NULL, NULL);
 *         cImGui_ImplBUI_RunApp(app);       // BUI's message loop until the window closes
 *         cImGui_ImplBUI_DestroyApp(app);
 *         ImGui_DestroyContext();
 *     }
 *
 * The backend runs ImGui_NewFrame() / frame(user) / ImGui_Render() itself; Blender's style
 * and font (Inter) are applied by default (see cImGui_ImplBUI_Config).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#pragma once
#include "cimgui.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct BKNode BKNode;
typedef struct BKContext BKContext;
typedef struct BKWindow BKWindow;
typedef struct BuiRaster BuiRaster;
typedef struct cImGui_ImplBUI_App cImGui_ImplBUI_App;

/* Called inside each ImGui frame (between ImGui_NewFrame() and ImGui_Render()). */
typedef void (*cImGui_ImplBUI_FrameFn)(void* user);

typedef struct cImGui_ImplBUI_Config {
    bool  BlenderStyle;  /* Blender's theme colors and metrics (true) */
    bool  BlenderFont;   /* Blender's UI font, Inter (true) */
    float FontSize;      /* pixels at scale 1; 0: Blender's UI text size */
    bool  FollowScale;   /* follow DPI changes (true) */
    int   SettleFrames;  /* frames after an input event (3) */
} cImGui_ImplBUI_Config;

/* The defaults above. */
CIMGUI_API cImGui_ImplBUI_Config cImGui_ImplBUI_DefaultConfig(void);

/* Hosting in a BUI "custom" node (any BUI UI). config may be NULL. */
CIMGUI_API bool    cImGui_ImplBUI_Init(BKNode* node, cImGui_ImplBUI_FrameFn frame, void* user, const cImGui_ImplBUI_Config* config);
CIMGUI_API void    cImGui_ImplBUI_Shutdown(void);
CIMGUI_API BKNode* cImGui_ImplBUI_GetNode(void);
CIMGUI_API void    cImGui_ImplBUI_RequestFrames(int n);
CIMGUI_API void    cImGui_ImplBUI_RequestFrameIn(int ms);
CIMGUI_API int     cImGui_ImplBUI_FrameCount(void);

/* Blender's look. */
CIMGUI_API void    cImGui_ImplBUI_StyleBlender(ImGuiStyle* dst, float scale);
CIMGUI_API float   cImGui_ImplBUI_BlenderFontSize(float scale);
CIMGUI_API ImFont* cImGui_ImplBUI_AddBlenderFont(float size_pixels);
/* BUI's UI scale (DPI): BK_scale() / BK_set_scale(). */
CIMGUI_API float   cImGui_ImplBUI_GetScale(void);
CIMGUI_API void    cImGui_ImplBUI_SetScale(float scale);

/* A complete app: a native BUI window whose client area is one ImGui node. Needs a current
 * ImGui context (ImGui_CreateContext). NULL title / no window system: see IsHeadless. */
CIMGUI_API cImGui_ImplBUI_App* cImGui_ImplBUI_CreateApp(const char* title, int width, int height, cImGui_ImplBUI_FrameFn frame, void* user, const cImGui_ImplBUI_Config* config);
/* No window: render with cImGui_ImplBUI_Render(). */
CIMGUI_API cImGui_ImplBUI_App* cImGui_ImplBUI_CreateHeadlessApp(int width, int height, cImGui_ImplBUI_FrameFn frame, void* user, const cImGui_ImplBUI_Config* config);
CIMGUI_API void       cImGui_ImplBUI_RunApp(cImGui_ImplBUI_App* app);   /* until the window closes */
CIMGUI_API void       cImGui_ImplBUI_CloseApp(cImGui_ImplBUI_App* app); /* close the window (ends RunApp) */
CIMGUI_API void       cImGui_ImplBUI_DestroyApp(cImGui_ImplBUI_App* app);
CIMGUI_API bool       cImGui_ImplBUI_IsHeadless(cImGui_ImplBUI_App* app);
CIMGUI_API BKWindow*  cImGui_ImplBUI_AppWindow(cImGui_ImplBUI_App* app);
CIMGUI_API BKContext* cImGui_ImplBUI_AppContext(cImGui_ImplBUI_App* app);
CIMGUI_API BKNode*    cImGui_ImplBUI_AppNode(cImGui_ImplBUI_App* app);
CIMGUI_API ImGuiContext* cImGui_ImplBUI_AppImGuiContext(cImGui_ImplBUI_App* app);

/* Headless rendering: requests n ImGui frames and runs n BK_render passes into the app's
 * offscreen raster (width x height from CreateHeadlessApp, or the window size). Returns the
 * frames ImGui ran in total. */
CIMGUI_API int        cImGui_ImplBUI_Render(cImGui_ImplBUI_App* app, int n);
/* The last rendered pixels: 0xAARRGGBB, width * height, row-major. */
CIMGUI_API const uint32_t* cImGui_ImplBUI_Pixels(cImGui_ImplBUI_App* app, int* width, int* height);
/* Writes the last rendered frame as a PNG; false on error. */
CIMGUI_API bool       cImGui_ImplBUI_SavePNG(cImGui_ImplBUI_App* app, const char* path);

/* Input as the window hosts send it (pixel coordinates in the app; each call renders
 * `settle` frames afterwards when headless, so ImGui sees the event). button: 0 left,
 * 1 right, 2 middle. mods: BK_SHIFT 1 | BK_CTRL 2 | BK_ALT 4. keysym: X11 names
 * ("Return", "BackSpace", "Left", "a", "Control_L"...). */
CIMGUI_API void cImGui_ImplBUI_MouseMove(cImGui_ImplBUI_App* app, float x, float y);
CIMGUI_API void cImGui_ImplBUI_MouseDown(cImGui_ImplBUI_App* app, float x, float y, int button);
CIMGUI_API void cImGui_ImplBUI_MouseUp(cImGui_ImplBUI_App* app, float x, float y, int button);
CIMGUI_API void cImGui_ImplBUI_Click(cImGui_ImplBUI_App* app, float x, float y);
CIMGUI_API void cImGui_ImplBUI_Scroll(cImGui_ImplBUI_App* app, float x, float y, float dx, float dy);
CIMGUI_API void cImGui_ImplBUI_KeyDown(cImGui_ImplBUI_App* app, const char* keysym, int mods);
CIMGUI_API void cImGui_ImplBUI_KeyUp(cImGui_ImplBUI_App* app, const char* keysym, int mods);
CIMGUI_API void cImGui_ImplBUI_Key(cImGui_ImplBUI_App* app, const char* keysym, int mods); /* down + up */
CIMGUI_API void cImGui_ImplBUI_TypeText(cImGui_ImplBUI_App* app, const char* utf8);

#ifdef __cplusplus
} /* extern "C" */
#endif
