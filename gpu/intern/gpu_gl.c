/* BUI GPU renderer: OpenGL 3.3 core backend (Linux / X11) — see gpu_d3d11.h.
 *
 * The same renderer as gpu_d3d11.c on OpenGL: every primitive is one instance
 * of a single uber-shader (a GLSL 330 translation of gpu/shaders/bui_ui.hlsl),
 * drawn into a persistent framebuffer object so damage-region redraws work
 * like the software raster; present blits that frame to the window. Text
 * masks share one R8 atlas, images one BGRA atlas, and Slug glyph outlines
 * live in RGBA32F / RG32UI textures addressed linearly, exactly as on D3D.
 *
 * Contexts (after Blender's GHOST_ContextGLX / GHOST_ContextEGL):
 *  - a window: GLX on the X11 window given as a BuiGpuX11Window (the kit's
 *    Display, so events and GL share one connection);
 *  - offscreen (NULL handle: tests, BUI_raster_pixels): EGL (surfaceless
 *    platform, else the default display), else a GLX pbuffer.
 * libGL / libEGL / libX11 are loaded with dlopen on first use, never linked:
 * without them bui_gpu_create returns NULL and the software raster draws.
 * The context is created on the calling thread (no worker thread as on D3D);
 * bui_gpu_state is 1 right away, so the raster still switches over at its
 * next poll, the same "pending" handoff the D3D device uses.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#if defined(BUI_GPU_GL) && !defined(_WIN32)

#  include <dlfcn.h>
#  include <stddef.h>
#  include <stdint.h>
#  include <stdio.h>
#  include <stdlib.h>
#  include <string.h>

#  include "GPU_raster.h"
#  include "gpu_d3d11.h"

/* -------------------------------------------------------------------- */
/** \name Vendored GL / GLX / EGL / Xlib declarations
 *
 * Only what this file uses, so the build needs no GL headers and the binary
 * no GL libraries.
 * \{ */

typedef unsigned int GLenum;
typedef unsigned int GLuint;
typedef unsigned int GLbitfield;
typedef int GLint;
typedef int GLsizei;
typedef float GLfloat;
typedef unsigned char GLubyte;
typedef unsigned char GLboolean;
typedef char GLchar;
typedef ptrdiff_t GLsizeiptr;
typedef ptrdiff_t GLintptr;

#  define GL_NO_ERROR 0
#  define GL_FALSE 0
#  define GL_TRIANGLES 0x0004
#  define GL_TRIANGLE_STRIP 0x0005
#  define GL_ONE 1
#  define GL_ONE_MINUS_SRC_ALPHA 0x0303
#  define GL_CULL_FACE 0x0B44
#  define GL_DEPTH_TEST 0x0B71
#  define GL_DITHER 0x0BD0
#  define GL_BLEND 0x0BE2
#  define GL_SCISSOR_TEST 0x0C11
#  define GL_UNPACK_ROW_LENGTH 0x0CF2
#  define GL_UNPACK_ALIGNMENT 0x0CF5
#  define GL_PACK_ROW_LENGTH 0x0D02
#  define GL_PACK_ALIGNMENT 0x0D05
#  define GL_MAX_TEXTURE_SIZE 0x0D33
#  define GL_TEXTURE_2D 0x0DE1
#  define GL_UNSIGNED_BYTE 0x1401
#  define GL_UNSIGNED_INT 0x1405
#  define GL_FLOAT 0x1406
#  define GL_RED 0x1903
#  define GL_RGBA 0x1908
#  define GL_VENDOR 0x1F00
#  define GL_RENDERER 0x1F01
#  define GL_VERSION 0x1F02
#  define GL_NEAREST 0x2600
#  define GL_LINEAR 0x2601
#  define GL_TEXTURE_MAG_FILTER 0x2800
#  define GL_TEXTURE_MIN_FILTER 0x2801
#  define GL_TEXTURE_WRAP_S 0x2802
#  define GL_TEXTURE_WRAP_T 0x2803
#  define GL_COLOR_BUFFER_BIT 0x4000
#  define GL_RGBA8 0x8058
#  define GL_BGRA 0x80E1
#  define GL_CLAMP_TO_EDGE 0x812F
#  define GL_RG_INTEGER 0x8228
#  define GL_RG 0x8227
#  define GL_R8 0x8229
#  define GL_RG8 0x822B
#  define GL_RG32UI 0x823C
#  define GL_UNSIGNED_INT_8_8_8_8_REV 0x8367
#  define GL_TEXTURE0 0x84C0
#  define GL_RGBA32F 0x8814
#  define GL_ARRAY_BUFFER 0x8892
#  define GL_ELEMENT_ARRAY_BUFFER 0x8893
#  define GL_STREAM_DRAW 0x88E0
#  define GL_FRAGMENT_SHADER 0x8B30
#  define GL_VERTEX_SHADER 0x8B31
#  define GL_COMPILE_STATUS 0x8B81
#  define GL_LINK_STATUS 0x8B82
#  define GL_INFO_LOG_LENGTH 0x8B84
#  define GL_READ_FRAMEBUFFER 0x8CA8
#  define GL_DRAW_FRAMEBUFFER 0x8CA9
#  define GL_FRAMEBUFFER_COMPLETE 0x8CD5
#  define GL_COLOR_ATTACHMENT0 0x8CE0
#  define GL_FRAMEBUFFER 0x8D40
#  define GL_FRAMEBUFFER_SRGB 0x8DB9
#  define GL_CONTEXT_LOST 0x0507

/* GL entry points: return type, name, parameters */
#  define GL_FUNCS(F) \
    F(const GLubyte *, glGetString, (GLenum)) \
    F(GLenum, glGetError, (void)) \
    F(void, glGetIntegerv, (GLenum, GLint *)) \
    F(void, glEnable, (GLenum)) \
    F(void, glDisable, (GLenum)) \
    F(void, glBlendFunc, (GLenum, GLenum)) \
    F(void, glViewport, (GLint, GLint, GLsizei, GLsizei)) \
    F(void, glClearColor, (GLfloat, GLfloat, GLfloat, GLfloat)) \
    F(void, glClear, (GLbitfield)) \
    F(void, glFlush, (void)) \
    F(void, glPixelStorei, (GLenum, GLint)) \
    F(void, glReadPixels, (GLint, GLint, GLsizei, GLsizei, GLenum, GLenum, void *)) \
    F(void, glGenTextures, (GLsizei, GLuint *)) \
    F(void, glDeleteTextures, (GLsizei, const GLuint *)) \
    F(void, glBindTexture, (GLenum, GLuint)) \
    F(void, glActiveTexture, (GLenum)) \
    F(void, glTexParameteri, (GLenum, GLenum, GLint)) \
    F(void, glTexImage2D, \
      (GLenum, GLint, GLint, GLsizei, GLsizei, GLint, GLenum, GLenum, const void *)) \
    F(void, glTexSubImage2D, \
      (GLenum, GLint, GLint, GLint, GLsizei, GLsizei, GLenum, GLenum, const void *)) \
    F(void, glGenBuffers, (GLsizei, GLuint *)) \
    F(void, glDeleteBuffers, (GLsizei, const GLuint *)) \
    F(void, glBindBuffer, (GLenum, GLuint)) \
    F(void, glBufferData, (GLenum, GLsizeiptr, const void *, GLenum)) \
    F(void, glGenVertexArrays, (GLsizei, GLuint *)) \
    F(void, glDeleteVertexArrays, (GLsizei, const GLuint *)) \
    F(void, glBindVertexArray, (GLuint)) \
    F(void, glEnableVertexAttribArray, (GLuint)) \
    F(void, glVertexAttribPointer, (GLuint, GLint, GLenum, GLboolean, GLsizei, const void *)) \
    F(void, glVertexAttribDivisor, (GLuint, GLuint)) \
    F(GLuint, glCreateShader, (GLenum)) \
    F(void, glShaderSource, (GLuint, GLsizei, const GLchar *const *, const GLint *)) \
    F(void, glCompileShader, (GLuint)) \
    F(void, glGetShaderiv, (GLuint, GLenum, GLint *)) \
    F(void, glGetShaderInfoLog, (GLuint, GLsizei, GLsizei *, GLchar *)) \
    F(void, glDeleteShader, (GLuint)) \
    F(GLuint, glCreateProgram, (void)) \
    F(void, glAttachShader, (GLuint, GLuint)) \
    F(void, glLinkProgram, (GLuint)) \
    F(void, glGetProgramiv, (GLuint, GLenum, GLint *)) \
    F(void, glGetProgramInfoLog, (GLuint, GLsizei, GLsizei *, GLchar *)) \
    F(void, glDeleteProgram, (GLuint)) \
    F(void, glUseProgram, (GLuint)) \
    F(GLint, glGetUniformLocation, (GLuint, const GLchar *)) \
    F(void, glUniform1i, (GLint, GLint)) \
    F(void, glUniform2f, (GLint, GLfloat, GLfloat)) \
    F(void, glGenFramebuffers, (GLsizei, GLuint *)) \
    F(void, glDeleteFramebuffers, (GLsizei, const GLuint *)) \
    F(void, glBindFramebuffer, (GLenum, GLuint)) \
    F(void, glFramebufferTexture2D, (GLenum, GLenum, GLenum, GLuint, GLint)) \
    F(GLenum, glCheckFramebufferStatus, (GLenum)) \
    F(void, glBlitFramebuffer, \
      (GLint, GLint, GLint, GLint, GLint, GLint, GLint, GLint, GLbitfield, GLenum)) \
    F(void, glDrawArrays, (GLenum, GLint, GLsizei)) \
    F(void, glDrawArraysInstanced, (GLenum, GLint, GLsizei, GLsizei)) \
    F(void, glDrawElements, (GLenum, GLsizei, GLenum, const void *)) \
    F(void, glScissor, (GLint, GLint, GLsizei, GLsizei))

#  define GLF_DECL(ret, name, args) ret(*name) args;
typedef struct GLFuncs {
  GL_FUNCS(GLF_DECL)
} GLFuncs;
#  undef GLF_DECL

/* Xlib (opaque) */
typedef void XDisplay;
typedef unsigned long XID;
typedef int (*XErrHandler)(XDisplay *, void *);

/* GLX */
typedef void *GLXFBConfig;
typedef void *GLXContext;
typedef void (*ProcFn)(void);
#  define GLX_DOUBLEBUFFER 5
#  define GLX_RED_SIZE 8
#  define GLX_GREEN_SIZE 9
#  define GLX_BLUE_SIZE 10
#  define GLX_X_VISUAL_TYPE 0x22
#  define GLX_TRUE_COLOR 0x8002
#  define GLX_VISUAL_ID 0x800B
#  define GLX_DRAWABLE_TYPE 0x8010
#  define GLX_RENDER_TYPE 0x8011
#  define GLX_X_RENDERABLE 0x8012
#  define GLX_WINDOW_BIT 0x1
#  define GLX_PBUFFER_BIT 0x4
#  define GLX_RGBA_BIT 0x1
#  define GLX_WIDTH 0x801D
#  define GLX_HEIGHT 0x801E
#  define GLX_PBUFFER_HEIGHT 0x8040
#  define GLX_PBUFFER_WIDTH 0x8041
#  define GLX_CONTEXT_MAJOR_VERSION_ARB 0x2091
#  define GLX_CONTEXT_MINOR_VERSION_ARB 0x2092
#  define GLX_CONTEXT_PROFILE_MASK_ARB 0x9126
#  define GLX_CONTEXT_CORE_PROFILE_BIT_ARB 0x1

/* EGL */
typedef void *EGLDisplay;
typedef void *EGLConfig;
typedef void *EGLContext;
typedef void *EGLSurface;
typedef int32_t EGLint;
typedef unsigned int EGLBoolean;
typedef unsigned int EGLenum;
#  define EGL_ALPHA_SIZE 0x3021
#  define EGL_BLUE_SIZE 0x3022
#  define EGL_GREEN_SIZE 0x3023
#  define EGL_RED_SIZE 0x3024
#  define EGL_SURFACE_TYPE 0x3033
#  define EGL_NONE 0x3038
#  define EGL_RENDERABLE_TYPE 0x3040
#  define EGL_EXTENSIONS 0x3055
#  define EGL_HEIGHT 0x3056
#  define EGL_WIDTH 0x3057
#  define EGL_OPENGL_API 0x30A2
#  define EGL_CONTEXT_MAJOR_VERSION 0x3098
#  define EGL_CONTEXT_MINOR_VERSION 0x30FB
#  define EGL_CONTEXT_OPENGL_PROFILE_MASK 0x30FD
#  define EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT 0x1
#  define EGL_OPENGL_BIT 0x8
#  define EGL_PBUFFER_BIT 0x1
#  define EGL_PLATFORM_SURFACELESS_MESA 0x31DD

/** \} */

/* -------------------------------------------------------------------- */
/** \name Run-time loading
 * \{ */

/* BUI_GPU_DEBUG=1: why the GPU path wasn't taken, and what it runs on */
static int debug_on(void)
{
  const char *dbg = getenv("BUI_GPU_DEBUG");
  return dbg && dbg[0] == '1';
}

static int fail(const char *what)
{
  if (debug_on()) {
    fprintf(stderr, "bui gpu (gl): %s failed\n", what);
  }
  return 0;
}

static void *open_lib(const char *const *names)
{
  for (; *names; names++) {
    void *h = dlopen(*names, RTLD_NOW | RTLD_LOCAL);
    if (h) {
      return h;
    }
  }
  return NULL;
}

#  define X11_FUNCS(F) \
    F(XDisplay *, XOpenDisplay, (const char *)) \
    F(int, XCloseDisplay, (XDisplay *)) \
    F(int, XDefaultScreen, (XDisplay *)) \
    F(void *, XDefaultVisual, (XDisplay *, int)) \
    F(unsigned long, XVisualIDFromVisual, (void *)) \
    F(int, XFree, (void *)) \
    F(int, XSync, (XDisplay *, int)) \
    F(XErrHandler, XSetErrorHandler, (XErrHandler))

#  define GLX_FUNCS(F) \
    F(int, glXQueryVersion, (XDisplay *, int *, int *)) \
    F(const char *, glXQueryExtensionsString, (XDisplay *, int)) \
    F(GLXFBConfig *, glXChooseFBConfig, (XDisplay *, int, const int *, int *)) \
    F(int, glXGetFBConfigAttrib, (XDisplay *, GLXFBConfig, int, int *)) \
    F(int, glXMakeContextCurrent, (XDisplay *, XID, XID, GLXContext)) \
    F(void, glXDestroyContext, (XDisplay *, GLXContext)) \
    F(void, glXSwapBuffers, (XDisplay *, XID)) \
    F(GLXContext, glXGetCurrentContext, (void)) \
    F(XID, glXCreatePbuffer, (XDisplay *, GLXFBConfig, const int *)) \
    F(void, glXDestroyPbuffer, (XDisplay *, XID)) \
    F(void, glXQueryDrawable, (XDisplay *, XID, int, unsigned int *)) \
    F(ProcFn, glXGetProcAddressARB, (const GLubyte *))

#  define EGL_FUNCS(F) \
    F(ProcFn, eglGetProcAddress, (const char *)) \
    F(EGLDisplay, eglGetDisplay, (void *)) \
    F(EGLBoolean, eglInitialize, (EGLDisplay, EGLint *, EGLint *)) \
    F(EGLBoolean, eglBindAPI, (EGLenum)) \
    F(EGLBoolean, eglChooseConfig, (EGLDisplay, const EGLint *, EGLConfig *, EGLint, EGLint *)) \
    F(EGLContext, eglCreateContext, (EGLDisplay, EGLConfig, EGLContext, const EGLint *)) \
    F(EGLBoolean, eglDestroyContext, (EGLDisplay, EGLContext)) \
    F(EGLBoolean, eglMakeCurrent, (EGLDisplay, EGLSurface, EGLSurface, EGLContext)) \
    F(EGLContext, eglGetCurrentContext, (void)) \
    F(EGLSurface, eglCreatePbufferSurface, (EGLDisplay, EGLConfig, const EGLint *)) \
    F(EGLBoolean, eglDestroySurface, (EGLDisplay, EGLSurface)) \
    F(const char *, eglQueryString, (EGLDisplay, EGLint))

#  define PTR_DECL(ret, name, args) static ret(*p_##name) args;
X11_FUNCS(PTR_DECL)
GLX_FUNCS(PTR_DECL)
EGL_FUNCS(PTR_DECL)
#  undef PTR_DECL

static void *g_libgl = NULL; /* libGL / libOpenGL: fallback for GL entry points */

#  define PTR_LOAD(ret, name, args) \
    if (!(p_##name = (ret(*) args)dlsym(lib, #name))) { \
      return fail("resolving " #name); \
    }

static int x11_load(void)
{
  static int state = 0; /* 0 untried, 1 loaded, -1 unavailable */
  if (state == 0) {
    static const char *const names[] = {"libX11.so.6", "libX11.so", NULL};
    void *lib = open_lib(names);
    state = -1;
    if (!lib) {
      return fail("loading libX11");
    }
    X11_FUNCS(PTR_LOAD)
    state = 1;
  }
  return state > 0;
}

static int glx_load(void)
{
  static int state = 0;
  if (state == 0) {
    static const char *const names[] = {"libGL.so.1", "libGL.so", "libGLX.so.0", NULL};
    void *lib = open_lib(names);
    state = -1;
    if (!lib) {
      return fail("loading libGL");
    }
    if (!g_libgl) {
      g_libgl = lib;
    }
    GLX_FUNCS(PTR_LOAD)
    state = 1;
  }
  return state > 0;
}

static int egl_load(void)
{
  static int state = 0;
  if (state == 0) {
    static const char *const names[] = {"libEGL.so.1", "libEGL.so", NULL};
    void *lib = open_lib(names);
    state = -1;
    if (!lib) {
      return fail("loading libEGL");
    }
    EGL_FUNCS(PTR_LOAD)
    if (!g_libgl) {
      static const char *const gl_names[] = {"libOpenGL.so.0", "libGL.so.1", NULL};
      g_libgl = open_lib(gl_names);
    }
    state = 1;
  }
  return state > 0;
}

#  undef PTR_LOAD

static int has_ext(const char *list, const char *name)
{
  const size_t n = strlen(name);
  for (const char *p = list; p && (p = strstr(p, name)) != NULL; p += n) {
    if ((p == list || p[-1] == ' ') && (p[n] == ' ' || p[n] == '\0')) {
      return 1;
    }
  }
  return 0;
}

/* Xlib errors (BadMatch from a mismatched config, a destroyed window) end the
 * process with the default handler: trap them around GLX calls, as GHOST
 * does. */
static int g_xerr = 0;
static int xerr_handler(XDisplay *dpy, void *ev)
{
  (void)dpy;
  (void)ev;
  g_xerr = 1;
  return 0;
}

static XErrHandler trap_begin(XDisplay *dpy)
{
  p_XSync(dpy, 0);
  g_xerr = 0;
  return p_XSetErrorHandler(xerr_handler);
}

/* 1 when an X error happened since trap_begin */
static int trap_end(XDisplay *dpy, XErrHandler old)
{
  p_XSync(dpy, 0);
  p_XSetErrorHandler(old);
  return g_xerr;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Shaders (GLSL 330 translation of gpu/shaders/bui_ui.hlsl)
 *
 * The frame is stored top row first: vertices map pixel y straight to NDC
 * (no flip), so gl_FragCoord is the same top-left-origin pixel center as
 * HLSL's SV_Position and every formula below is the HLSL one unchanged.
 * Present flips the frame onto the window.
 * \{ */

static const char *const k_vs_quad =
    "#version 330 core\n"
    "uniform vec2 to_ndc;\n"
    "layout(location = 0) in vec4 a_rect;\n"
    "layout(location = 1) in vec4 a_bounds;\n"
    "layout(location = 2) in vec4 a_c0;\n"
    "layout(location = 3) in vec4 a_c1;\n"
    "layout(location = 4) in vec4 a_c2;\n"
    "layout(location = 5) in vec4 a_c3;\n"
    "layout(location = 6) in vec4 a_p;\n"
    "layout(location = 7) in vec4 a_uv;\n"
    "flat out vec4 v_rect, v_c0, v_c1, v_c2, v_c3, v_p, v_uv;\n"
    "void main()\n"
    "{\n"
    "  vec2 t = vec2(float(gl_VertexID & 1), float(gl_VertexID >> 1));\n"
    "  vec2 xy = mix(a_bounds.xy, a_bounds.zw, t);\n"
    "  gl_Position = vec4(xy.x * to_ndc.x - 1.0, xy.y * to_ndc.y - 1.0, 0.0, 1.0);\n"
    "  v_rect = a_rect;\n"
    "  v_c0 = a_c0;\n"
    "  v_c1 = a_c1;\n"
    "  v_c2 = a_c2;\n"
    "  v_c3 = a_c3;\n"
    "  v_p = a_p;\n"
    "  v_uv = a_uv;\n"
    "}\n";

static const char *const k_fs_quad =
    "#version 330 core\n"
    "#define K_RECT 0\n"
    "#define K_WIDGET 1\n"
    "#define K_ELLIPSE 2\n"
    "#define K_LINE 3\n"
    "#define K_MASK 4\n"
    "#define K_IMAGE 5\n"
    "#define K_SHADOW 6\n"
    "#define K_GRADIENT 7\n"
    "#define K_HSV 8\n"
    "#define K_SPHERE 9\n"
    "#define K_SLUG 10\n"
    "#define W_SHADE_H 1u\n"
    "#define W_CHECKER 2u\n"
    "#define W_LEGACY 4u\n"
    "#define W_INNER 8u\n"
    "uniform sampler2D mask_tex;\n"
    "uniform sampler2D rgba_tex;\n"
    "uniform sampler2D curve_tex;\n"   /* Slug: (p1, p2), (p3, -) per curve */
    "uniform usampler2D band_tex;\n"   /* Slug: band headers (count, at), entries (curve) */
    "uniform sampler2D uv_tex;\n"      /* NV12 video frames: chroma (luma is rgba_tex.r) */
    "flat in vec4 v_rect, v_c0, v_c1, v_c2, v_c3, v_p, v_uv;\n"
    "layout(location = 0) out vec4 frag;\n"
    "float sat(float x) { return clamp(x, 0.0, 1.0); }\n"
    "vec2 sat(vec2 x) { return clamp(x, 0.0, 1.0); }\n"
    "vec3 sat(vec3 x) { return clamp(x, 0.0, 1.0); }\n"
    /* rounded box with a per-corner radius mask (gpu_raster.c sd_widget_box) */
    "float sd_box(vec2 s, vec2 c, vec2 hb, float rad, uint corners)\n"
    "{\n"
    "  vec2 p = s - c;\n"
    "  uint bit = p.y < 0.0 ? (p.x < 0.0 ? 1u : 2u) : (p.x < 0.0 ? 8u : 4u);\n"
    "  float r = (corners & bit) != 0u ? rad : 0.0;\n"
    "  vec2 q = abs(p) - (hb - r);\n"
    "  if (q.x <= 0.0 || q.y <= 0.0) {\n"
    "    return max(q.x, q.y) - r;\n"
    "  }\n"
    "  return length(q) - r;\n"
    "}\n"
    /* source-over of a straight color into a premultiplied accumulator */
    "void over(inout vec4 acc, vec4 c, float cov)\n"
    "{\n"
    "  float a = sat(c.a * cov);\n"
    "  acc.rgb = c.rgb * a + acc.rgb * (1.0 - a);\n"
    "  acc.a = a + acc.a * (1.0 - a);\n"
    "}\n"
    "vec4 premul(vec4 c, float cov)\n"
    "{\n"
    "  float a = sat(c.a * cov);\n"
    "  return vec4(c.rgb * a, a);\n"
    "}\n"
    "vec3 hsv_to_rgb(float h, float s, float v)\n"
    "{\n"
    "  h = h - floor(h);\n"
    "  float h6 = h * 6.0;\n"
    "  vec3 rgb = sat(vec3(abs(h6 - 3.0) - 1.0, 2.0 - abs(h6 - 2.0), 2.0 - abs(h6 - 4.0)));\n"
    "  return ((rgb - 1.0) * s + 1.0) * v;\n"
    "}\n"
    /* Slug glyph coverage, ported from Eric Lengyel's reference shaders
     * (https://github.com/EricLengyel/Slug, MIT license):
     * "Slug shader code Copyright 2017 by Eric Lengyel." */
    "ivec2 slug_loc(uint i)\n"
    "{\n"
    "  return ivec2(int(i & 4095u), int(i >> 12u));\n"
    "}\n"
    "uint slug_root_code(float y1, float y2, float y3)\n"
    "{\n"
    "  uint i1 = floatBitsToUint(y1) >> 31u;\n"
    "  uint i2 = floatBitsToUint(y2) >> 30u;\n"
    "  uint i3 = floatBitsToUint(y3) >> 29u;\n"
    "  uint shift = (i2 & 2u) | (i1 & ~2u);\n"
    "  shift = (i3 & 4u) | (shift & ~4u);\n"
    "  return (0x2E74u >> shift) & 0x0101u;\n"
    "}\n"
    "vec2 slug_solve_h(vec4 p12, vec2 p3)\n"
    "{\n"
    "  vec2 a = p12.xy - p12.zw * 2.0 + p3;\n"
    "  vec2 b = p12.xy - p12.zw;\n"
    "  float ra = 1.0 / a.y, rb = 0.5 / b.y;\n"
    "  float d = sqrt(max(b.y * b.y - a.y * p12.y, 0.0));\n"
    "  float t1 = (b.y - d) * ra, t2 = (b.y + d) * ra;\n"
    "  if (abs(a.y) < 1.0 / 65536.0) {\n"
    "    t1 = p12.y * rb;\n"
    "    t2 = t1;\n"
    "  }\n"
    "  return vec2((a.x * t1 - b.x * 2.0) * t1 + p12.x, (a.x * t2 - b.x * 2.0) * t2 + p12.x);\n"
    "}\n"
    "vec2 slug_solve_v(vec4 p12, vec2 p3)\n"
    "{\n"
    "  vec2 a = p12.xy - p12.zw * 2.0 + p3;\n"
    "  vec2 b = p12.xy - p12.zw;\n"
    "  float ra = 1.0 / a.x, rb = 0.5 / b.x;\n"
    "  float d = sqrt(max(b.x * b.x - a.x * p12.x, 0.0));\n"
    "  float t1 = (b.x - d) * ra, t2 = (b.x + d) * ra;\n"
    "  if (abs(a.x) < 1.0 / 65536.0) {\n"
    "    t1 = p12.x * rb;\n"
    "    t2 = t1;\n"
    "  }\n"
    "  return vec2((a.y * t1 - b.y * 2.0) * t1 + p12.y, (a.y * t2 - b.y * 2.0) * t2 + p12.y);\n"
    "}\n"
    /* rc: sample position (font units); ppu: pixels per font unit;
     * bt: band transform (y scale, y offset, x scale, x offset);
     * at: first band header; nh, nv: band counts. */
    "float slug_coverage(vec2 rc, float ppu, vec4 bt, uint at, int nh, int nv)\n"
    "{\n"
    "  ivec2 bi = clamp(ivec2(int(rc.y * bt.x + bt.y), int(rc.x * bt.z + bt.w)), ivec2(0, 0),\n"
    "                   ivec2(nh - 1, nv - 1));\n"
    "  float xcov = 0.0, xwgt = 0.0;\n"
    "  uvec2 hb = texelFetch(band_tex, slug_loc(at + uint(bi.x)), 0).xy;\n"
    "  for (uint i = 0u; i < hb.x; i++) {\n"
    "    ivec2 cl = slug_loc(texelFetch(band_tex, slug_loc(hb.y + i), 0).x);\n"
    "    vec4 p12 = texelFetch(curve_tex, cl, 0) - rc.xyxy;\n"
    "    vec2 p3 = texelFetch(curve_tex, ivec2(cl.x + 1, cl.y), 0).xy - rc;\n"
    "    if (max(max(p12.x, p12.z), p3.x) * ppu < -0.5) {\n"
    "      break;\n"
    "    }\n"
    "    uint code = slug_root_code(p12.y, p12.w, p3.y);\n"
    "    if (code != 0u) {\n"
    "      vec2 r = slug_solve_h(p12, p3) * ppu;\n"
    "      if ((code & 1u) != 0u) {\n"
    "        xcov += sat(r.x + 0.5);\n"
    "        xwgt = max(xwgt, sat(1.0 - abs(r.x) * 2.0));\n"
    "      }\n"
    "      if (code > 1u) {\n"
    "        xcov -= sat(r.y + 0.5);\n"
    "        xwgt = max(xwgt, sat(1.0 - abs(r.y) * 2.0));\n"
    "      }\n"
    "    }\n"
    "  }\n"
    "  float ycov = 0.0, ywgt = 0.0;\n"
    "  uvec2 vb = texelFetch(band_tex, slug_loc(at + uint(nh) + uint(bi.y)), 0).xy;\n"
    "  for (uint j = 0u; j < vb.x; j++) {\n"
    "    ivec2 cl = slug_loc(texelFetch(band_tex, slug_loc(vb.y + j), 0).x);\n"
    "    vec4 p12 = texelFetch(curve_tex, cl, 0) - rc.xyxy;\n"
    "    vec2 p3 = texelFetch(curve_tex, ivec2(cl.x + 1, cl.y), 0).xy - rc;\n"
    "    if (max(max(p12.y, p12.w), p3.y) * ppu < -0.5) {\n"
    "      break;\n"
    "    }\n"
    "    uint code = slug_root_code(p12.x, p12.z, p3.x);\n"
    "    if (code != 0u) {\n"
    "      vec2 r = slug_solve_v(p12, p3) * ppu;\n"
    "      if ((code & 1u) != 0u) {\n"
    "        ycov -= sat(r.x + 0.5);\n"
    "        ywgt = max(ywgt, sat(1.0 - abs(r.x) * 2.0));\n"
    "      }\n"
    "      if (code > 1u) {\n"
    "        ycov += sat(r.y + 0.5);\n"
    "        ywgt = max(ywgt, sat(1.0 - abs(r.y) * 2.0));\n"
    "      }\n"
    "    }\n"
    "  }\n"
    "  float cov = max(abs(xcov * xwgt + ycov * ywgt) / max(xwgt + ywgt, 1.0 / 65536.0),\n"
    "                  min(abs(xcov), abs(ycov)));\n"
    "  return sat(cov);\n"
    "}\n"
    "vec4 shade(vec2 s)\n"
    "{\n"
    "  int kind = int(v_p.x);\n"
    "  vec4 R = v_rect;\n"
    "  if (kind == K_RECT) {\n"
    "    vec2 px = s - 0.5;\n"
    "    vec2 lo = max(R.xy, px), hi = min(R.xy + R.zw, px + 1.0);\n"
    "    vec2 cv = sat(hi - lo);\n"
    "    return premul(v_c0, cv.x * cv.y);\n"
    "  }\n"
    "  if (kind == K_WIDGET) {\n"
    "    float x = R.x, y = R.y, w = R.z, h = R.w;\n"
    "    vec2 hb = vec2(w, h) * 0.5, c = R.xy + hb;\n"
    "    float rad = v_p.y, radi = v_p.z, px1 = v_p.w;\n"
    "    uint corners = uint(v_uv.x), flags = uint(v_uv.y);\n"
    "    float ow = v_uv.z;\n"
    "    vec4 acc = vec4(0.0);\n"
    "    float d = sd_box(s, c, hb, rad, corners);\n"
    "    float co = sat(0.5 - d);\n"
    "    if (v_c3.a > 0.0 && s.y > c.y) {\n"
    "      float de = sd_box(s - vec2(0.0, px1), c, hb, rad, corners);\n"
    "      float ec = sat(0.5 - de) - co;\n"
    "      if (ec > 0.0) {\n"
    "        over(acc, v_c3, ec);\n"
    "      }\n"
    "    }\n"
    "    if (co > 0.0) {\n"
    "      float ci, band, t;\n"
    "      if ((flags & W_LEGACY) != 0u) {\n"
    "        ci = co;\n"
    "        band = ow > 0.0 ? co - sat(0.5 - (d + ow)) : 0.0;\n"
    "        t = sat((s.y - y) / h);\n"
    "      }\n"
    "      else {\n"
    "        float di = sd_box(s, c, hb - px1, radi, corners);\n"
    "        ci = min(co, sat(0.5 - di));\n"
    "        band = co - ci;\n"
    "        t = (flags & W_SHADE_H) != 0u ? sat((s.x - (x + px1)) / max(1.0, w - 2.0 * px1)) :\n"
    "                                        sat((s.y - (y + px1)) / max(1.0, h - 2.0 * px1));\n"
    "      }\n"
    "      if ((flags & W_INNER) != 0u && ci > 0.0) {\n"
    "        if ((flags & W_CHECKER) != 0u) {\n"
    "          int cell = (int((s.x - x) / 8.0) + int((s.y - y) / 8.0)) & 1;\n"
    "          float g = cell != 0 ? 160.0 / 255.0 : 100.0 / 255.0;\n"
    "          over(acc, vec4(g, g, g, 1.0), ci);\n"
    "        }\n"
    "        over(acc, mix(v_c0, v_c1, t), ci);\n"
    "      }\n"
    "      if (v_c2.a > 0.0 && band > 0.0) {\n"
    "        over(acc, v_c2, band);\n"
    "      }\n"
    "    }\n"
    "    return acc;\n"
    "  }\n"
    "  if (kind == K_ELLIPSE) {\n"
    "    float lw = v_p.y;\n"
    "    vec2 dd = s - R.xy;\n"
    "    float dist;\n"
    "    if (abs(R.z - R.w) < 1e-3) {\n"
    "      dist = length(dd) - R.z;\n"
    "    }\n"
    "    else {\n"
    "      vec2 n = dd / R.zw;\n"
    "      float k = length(n);\n"
    "      float g = length(n / R.zw);\n"
    "      dist = g > 0.0 ? (k - 1.0) * k / g : -min(R.z, R.w);\n"
    "    }\n"
    "    float cov = lw > 0.0 ? sat(lw * 0.5 - abs(dist) + 0.5) : sat(0.5 - dist);\n"
    "    return premul(v_c0, cov);\n"
    "  }\n"
    "  if (kind == K_LINE) {\n"
    "    vec2 a = R.xy, vv = R.zw - R.xy;\n"
    "    vec2 wv = s - a;\n"
    "    float t = sat(dot(wv, vv) / dot(vv, vv));\n"
    "    float d = length(wv - vv * t) - v_p.y;\n"
    "    return premul(v_c0, sat(0.5 - d));\n"
    "  }\n"
    "  if (kind == K_MASK) {\n"
    "    ivec2 texel = ivec2(v_uv.xy) + ivec2(floor(s) - R.xy);\n"
    "    float cov = texelFetch(mask_tex, texel, 0).r;\n"
    "    return premul(v_c0, cov);\n"
    "  }\n"
    "  if (kind == K_SLUG) {\n"
    /* rect: pen x, baseline y, pixels per font unit, italic shear */
    "    float ey = (R.y - s.y) / R.z;\n"
    "    float ex = (s.x - R.x) / R.z - R.w * ey;\n"
    "    float cov = slug_coverage(vec2(ex, ey), R.z, v_c1, uint(v_uv.x), int(v_uv.y), int(v_uv.z));\n"
    /* same lift as the CPU glyph cache (gamma 1/1.45, like GDI's grayscale AA) */
    "    return premul(v_c0, pow(cov, 1.0 / 1.45));\n"
    "  }\n"
    "  if (kind == K_IMAGE) {\n" /* premultiplied, bilinear: as bui_ui.hlsl */
    "    vec2 k = v_uv.zw / R.zw;\n"
    "    vec2 lo = floor(v_uv.xy), hi = max(lo, ceil(v_uv.xy + v_uv.zw) - 1.0);\n"
    "    if (v_p.w > 0.5) {\n" /* NV12 video frame: as bui_ui.hlsl */
    "      vec2 u = v_uv.xy + (s - R.xy) * k;\n"
    "      float y;\n"
    "      if (v_p.z > 0.5) {\n"
    "        y = texelFetch(rgba_tex, ivec2(clamp(floor(u), lo, hi)), 0).r;\n"
    "      } else {\n"
    "        vec2 t = u - 0.5, t0 = floor(t), f = t - t0;\n"
    "        ivec2 a = ivec2(clamp(t0, lo, hi)), b = ivec2(clamp(t0 + 1.0, lo, hi));\n"
    "        y = mix(mix(texelFetch(rgba_tex, a, 0).r, texelFetch(rgba_tex, ivec2(b.x, a.y), 0).r, f.x),\n"
    "                mix(texelFetch(rgba_tex, ivec2(a.x, b.y), 0).r, texelFetch(rgba_tex, b, 0).r, f.x), f.y);\n"
    "      }\n"
    "      vec2 tc = u * 0.5 - 0.5, c0 = floor(tc), fc = tc - c0;\n"
    "      vec2 clo = floor(lo * 0.5), chi = floor(hi * 0.5);\n"
    "      ivec2 ca = ivec2(clamp(c0, clo, chi)), cb = ivec2(clamp(c0 + 1.0, clo, chi));\n"
    "      vec2 cbcr = mix(mix(texelFetch(uv_tex, ca, 0).rg, texelFetch(uv_tex, ivec2(cb.x, ca.y), 0).rg, fc.x),\n"
    "                      mix(texelFetch(uv_tex, ivec2(ca.x, cb.y), 0).rg, texelFetch(uv_tex, cb, 0).rg, fc.x), fc.y) -\n"
    "                  128.0 / 255.0;\n"
    "      float l = 1.164383 * (y - 16.0 / 255.0);\n"
    "      vec4 m = v_p.w > 1.5 ? vec4(1.792741, 0.213249, 0.532909, 2.112402) :\n"
    "                             vec4(1.596027, 0.391762, 0.812968, 2.017232);\n"
    "      vec3 rgb = sat(vec3(l + m.x * cbcr.y, l - m.y * cbcr.x - m.z * cbcr.y, l + m.w * cbcr.x));\n"
    "      return vec4(rgb, 1.0) * v_p.y;\n"
    "    }\n"
    "    if (v_p.z > 0.5) {\n"
    "      ivec2 t = ivec2(clamp(floor(v_uv.xy + (s - R.xy) * k), lo, hi));\n"
    "      return texelFetch(rgba_tex, t, 0) * v_p.y;\n"
    "    }\n"
    "    vec2 t = v_uv.xy + (s - R.xy) * k - 0.5;\n"
    "    vec2 t0 = floor(t), f = t - t0;\n"
    "    ivec2 a = ivec2(clamp(t0, lo, hi)), b = ivec2(clamp(t0 + 1.0, lo, hi));\n"
    "    vec4 top = mix(texelFetch(rgba_tex, a, 0), texelFetch(rgba_tex, ivec2(b.x, a.y), 0), f.x);\n"
    "    vec4 bot = mix(texelFetch(rgba_tex, ivec2(a.x, b.y), 0), texelFetch(rgba_tex, b, 0), f.x);\n"
    "    return mix(top, bot, f.y) * v_p.y;\n"
    "  }\n"
    "  if (kind == K_SHADOW) {\n"
    "    float rad = v_p.y, size = v_p.z, alpha = v_p.w;\n"
    "    vec2 hb = R.zw * 0.5;\n"
    "    vec2 c = R.xy + hb;\n"
    "    float d = sd_box(s, c + vec2(0.0, size * 0.25), hb, rad, 15u);\n"
    "    float dbox = sd_box(s, c, hb, rad, 15u);\n"
    "    if (dbox <= -0.5 || d >= size) {\n"
    "      return vec4(0.0);\n"
    "    }\n"
    "    float t = 1.0 - sat(d / size);\n"
    "    t = t * t * (3.0 - 2.0 * t);\n"
    "    return vec4(0.0, 0.0, 0.0, sat(t * alpha * sat(0.5 + dbox)));\n"
    "  }\n"
    "  if (kind == K_GRADIENT) {\n"
    "    vec2 uv = sat((s - R.xy) / R.zw);\n"
    "    vec4 left = mix(v_c0, v_c3, uv.y), right = mix(v_c1, v_c2, uv.y);\n"
    "    return premul(mix(left, right, uv.x), 1.0);\n"
    "  }\n"
    "  if (kind == K_HSV) {\n"
    "    vec2 dd = s - R.xy;\n"
    "    float dist = length(dd);\n"
    "    float cov = sat(R.z - dist + 0.5);\n"
    "    float hue = atan(dd.x, -dd.y) * 0.15915494309189535 + 0.5;\n"
    "    return premul(vec4(hsv_to_rgb(hue, min(dist / R.z, 1.0), R.w), 1.0), cov);\n"
    "  }\n"
    "  if (kind == K_SPHERE) {\n"
    "    vec2 n2 = vec2(s.x - R.x, -(s.y - R.y)) / R.z;\n"
    "    float d2 = dot(n2, n2);\n"
    "    float cov = sat((1.0 - sqrt(d2)) * R.z + 0.5);\n"
    "    vec3 n = vec3(n2, sqrt(max(0.0, 1.0 - d2)));\n"
    "    float diff = max(0.0, dot(n, v_c1.xyz));\n"
    "    float spec = max(0.0, dot(n, v_c2.xyz));\n"
    "    spec = pow(spec, 16.0);\n" /* the software raster squares 2x then takes ^4 */
    "    return premul(vec4(v_c0.rgb * (0.12 + 0.88 * diff) + 0.6 * spec, v_c0.a), cov);\n"
    "  }\n"
    "  return vec4(0.0);\n"
    "}\n"
    "void main()\n"
    "{\n"
    "  frag = shade(gl_FragCoord.xy);\n" /* pixel center, top-left origin (see above) */
    "}\n";

/* per-vertex colored triangles (the BuiGpuSink seam), clipped per vertex */
static const char *const k_vs_tri =
    "#version 330 core\n"
    "uniform vec2 to_ndc;\n"
    "layout(location = 0) in vec2 a_pos;\n"
    "layout(location = 1) in vec4 a_color;\n"
    "layout(location = 2) in vec4 a_clip;\n"
    "out vec4 v_color;\n"
    "flat out vec4 v_clip;\n"
    "void main()\n"
    "{\n"
    "  gl_Position = vec4(a_pos.x * to_ndc.x - 1.0, a_pos.y * to_ndc.y - 1.0, 0.0, 1.0);\n"
    "  v_color = a_color;\n"
    "  v_clip = a_clip;\n"
    "}\n";

static const char *const k_fs_tri =
    "#version 330 core\n"
    "in vec4 v_color;\n"
    "flat in vec4 v_clip;\n"
    "layout(location = 0) out vec4 frag;\n"
    "void main()\n"
    "{\n"
    "  vec2 p = gl_FragCoord.xy;\n"
    "  if (p.x < v_clip.x || p.y < v_clip.y || p.x >= v_clip.z || p.y >= v_clip.w) {\n"
    "    discard;\n"
    "  }\n"
    "  float a = clamp(v_color.a, 0.0, 1.0);\n"
    "  frag = vec4(v_color.rgb * a, a);\n"
    "}\n";

/* textured meshes (BUI_raster_mesh: Dear ImGui's ImDrawList): position,
 * texture coordinates, RGBA8 color; clipped by the scissor; premultiplied
 * output for the shared ONE, ONE_MINUS_SRC_ALPHA blend (bui_ui.hlsl ps_mesh) */
static const char *const k_vs_mesh =
    "#version 330 core\n"
    "uniform vec2 to_ndc;\n"
    "layout(location = 0) in vec2 a_pos;\n"
    "layout(location = 1) in vec2 a_uv;\n"
    "layout(location = 2) in vec4 a_color;\n"
    "out vec2 v_uv;\n"
    "out vec4 v_color;\n"
    "void main()\n"
    "{\n"
    "  gl_Position = vec4(a_pos.x * to_ndc.x - 1.0, a_pos.y * to_ndc.y - 1.0, 0.0, 1.0);\n"
    "  v_uv = a_uv;\n"
    "  v_color = a_color;\n"
    "}\n";

static const char *const k_fs_mesh =
    "#version 330 core\n"
    "uniform sampler2D mesh_tex;\n"
    "in vec2 v_uv;\n"
    "in vec4 v_color;\n"
    "layout(location = 0) out vec4 frag;\n"
    "void main()\n"
    "{\n"
    "  vec4 c = v_color * texture(mesh_tex, v_uv);\n"
    "  float a = clamp(c.a, 0.0, 1.0);\n"
    "  frag = vec4(c.rgb * a, a);\n"
    "}\n";

/** \} */

/* -------------------------------------------------------------------- */
/** \name State
 * \{ */

#  define SHELVES 128

typedef struct Atlas {
  int size;
  int bpp; /* 1 (R8) or 4 (BGRA8) */
  GLuint tex;
  int shelf_y[SHELVES], shelf_h[SHELVES], shelf_x[SHELVES];
  int nshelves, next_y;
} Atlas;

typedef struct Slot {
  uint64_t k1, k2; /* 0 = empty */
  int x, y;
} Slot;

#  define MASK_MAP 16384 /* power of two */
#  define MASK_MAP_FILL 12000
#  define IMG_MAP 1024
#  define IMG_MAP_FILL 700

#  define SLUG_W 4096
#  define SLUG_H 64
#  define SLUG_MAP 8192 /* power of two */
#  define SLUG_MAP_FILL 6000
#  define SLUG_MAX_BANDS 16

typedef struct SlugGlyph {
  uint32_t key; /* (face + 1) << 16 | glyph; 0 = empty */
  int ok;
  float bbox[4], band[4];
  int loc[4];
} SlugGlyph;

typedef struct Batch {
  int tri; /* 0: quad instances, 1: triangles, 2: textured mesh (indexed) */
  int first, count;
  int img; /* quads: the RGBA texture their images sample (-1 none yet, 0 atlas, n: big[n - 1]) */
  /* meshes: texture (0: white), sampler, scissor x0 y0 x1 y1 */
  int tex, nearest;
  int clip[4];
} Batch;

/* An image too big for the atlas, or a video frame, in a texture of its own
 * (as gpu_d3d11.c). */
#  define BIG_MAX 32
#  define BIG_BUDGET ((size_t)256 << 20)
#  define BIG_IDLE 120
typedef struct BigTex {
  uint64_t key, base; /* 0 = empty */
  int w, h;
  int format; /* BUI_PIXMAP_*: NV12 is luma in tex, chroma in uv_tex */
  int stream; /* `base` is a stream: its frames update the textures in place */
  unsigned used;
  GLuint tex, uv_tex;
} BigTex;

/* A registry texture (BUI_texture_*) in this context, by registry slot. */
typedef struct DevTex {
  int id; /* 0 = empty */
  unsigned gen;
  int w, h;
  int nearest; /* the filter it is set to */
  GLuint tex;
} DevTex;

typedef struct TriV {
  float x, y;
  float c[4];
  float clip[4];
} TriV;

enum { API_GLX = 1, API_EGL = 2 };

struct BuiGpu {
  int state; /* 1 ready (creation is synchronous; failures return NULL) */

  /* context */
  int api;
  XDisplay *xdpy;
  int own_xdpy;
  XID draw;  /* GLX drawable: the window or the pbuffer */
  XID pbuf;  /* GLX pbuffer (offscreen) */
  int onscreen;
  GLXContext glx;
  EGLDisplay edpy;
  EGLContext egl;
  EGLSurface esurf;
  GLFuncs f;

  GLuint prog_quad, prog_tri, prog_mesh;
  GLint ndc_quad, ndc_tri, ndc_mesh;
  GLuint vao_quad, vao_tri, vao_mesh;
  GLuint ib, tb, mvb, mib;
  GLuint white_tex; /* 1x1 white: meshes without a texture */
  DevTex *dtex;
  int n_dtex;
  unsigned dtex_free_gen;
  int max_tex;

  GLuint fbo, target;
  int tw, th;
  int w, h; /* frame size */

  Atlas mask, rgba;
  BigTex big[BIG_MAX];
  size_t big_bytes;
  unsigned frame, forget_seen;
  int big_max; /* the largest big-image texture side (max_tex, or less for tests) */
  Slot *mask_map, *img_map;
  int mask_n, img_n;
  unsigned mask_gen;

  int slug; /* -1: off (unavailable), 0: not created yet, 1: ready */
  GLuint curve_tex, band_tex;
  SlugGlyph *slug_map;
  int slug_n, curve_next, band_next;
  unsigned slug_gen;

  BuiGpuInst *inst;
  int n_inst, cap_inst;
  TriV *tv;
  int n_tv, cap_tv;
  BuiMeshVertex *mv;
  int n_mv, cap_mv;
  uint32_t *mi;
  int n_mi, cap_mi;
  Batch *b;
  int n_b, cap_b;

  int dmg[16][4];
  int n_dmg, dmg_full;
};

#  define GL(fn) (g->f.fn)

/* Makes this renderer's context current on the calling thread. */
static int make_current(BuiGpu *g)
{
  if (g->api == API_GLX) {
    if (p_glXGetCurrentContext() == g->glx) {
      return 1;
    }
    return p_glXMakeContextCurrent(g->xdpy, g->draw, g->draw, g->glx) != 0;
  }
  if (g->api == API_EGL) {
    if (p_eglGetCurrentContext() == g->egl) {
      return 1;
    }
    return p_eglMakeCurrent(g->edpy, g->esurf, g->esurf, g->egl) != 0;
  }
  return 0;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Context creation
 * \{ */

static ProcFn get_proc(BuiGpu *g, const char *name)
{
  ProcFn fn = NULL;
  if (g->api == API_GLX) {
    fn = p_glXGetProcAddressARB((const GLubyte *)name);
  }
  else {
    fn = p_eglGetProcAddress(name);
  }
  if (!fn && g_libgl) {
    fn = (ProcFn)dlsym(g_libgl, name);
  }
  return fn;
}

static int gl_resolve(BuiGpu *g)
{
#  define GLF_LOAD(ret, name, args) \
    if (!(g->f.name = (ret(*) args)get_proc(g, #name))) { \
      return fail("resolving " #name); \
    }
  GL_FUNCS(GLF_LOAD)
#  undef GLF_LOAD
  return 1;
}

/* GLX: a core 3.3 context on `window` (0: on a 1x1 pbuffer). */
static int glx_create(BuiGpu *g, XDisplay *dpy, XID window)
{
  if (!x11_load() || !glx_load()) {
    return 0;
  }
  if (!dpy) {
    dpy = p_XOpenDisplay(NULL);
    if (!dpy) {
      return fail("XOpenDisplay");
    }
    g->own_xdpy = 1;
  }
  g->xdpy = dpy;
  int major = 0, minor = 0;
  if (!p_glXQueryVersion(dpy, &major, &minor) || major < 1 || (major == 1 && minor < 3)) {
    return fail("GLX 1.3");
  }
  const int scr = p_XDefaultScreen(dpy);
  const char *ext = p_glXQueryExtensionsString(dpy, scr);
  if (!has_ext(ext, "GLX_ARB_create_context") ||
      !has_ext(ext, "GLX_ARB_create_context_profile"))
  {
    return fail("GLX_ARB_create_context_profile");
  }
  typedef GLXContext (*CreateCtxFn)(XDisplay *, GLXFBConfig, GLXContext, int, const int *);
  CreateCtxFn create = (CreateCtxFn)p_glXGetProcAddressARB(
      (const GLubyte *)"glXCreateContextAttribsARB");
  if (!create) {
    return fail("glXCreateContextAttribsARB");
  }
  const int cfg_attr[] = {GLX_X_RENDERABLE,
                          1,
                          GLX_DRAWABLE_TYPE,
                          window ? GLX_WINDOW_BIT : GLX_PBUFFER_BIT,
                          GLX_RENDER_TYPE,
                          GLX_RGBA_BIT,
                          GLX_X_VISUAL_TYPE,
                          GLX_TRUE_COLOR,
                          GLX_RED_SIZE,
                          8,
                          GLX_GREEN_SIZE,
                          8,
                          GLX_BLUE_SIZE,
                          8,
                          GLX_DOUBLEBUFFER,
                          window ? 1 : 0,
                          0};
  int n = 0;
  GLXFBConfig *cfgs = p_glXChooseFBConfig(dpy, scr, cfg_attr, &n);
  if (!cfgs || n <= 0) {
    if (cfgs) {
      p_XFree(cfgs);
    }
    return fail("glXChooseFBConfig");
  }
  /* the kit's windows use the default visual: take the config made for it */
  GLXFBConfig cfg = cfgs[0];
  if (window) {
    const unsigned long want = p_XVisualIDFromVisual(p_XDefaultVisual(dpy, scr));
    for (int i = 0; i < n; i++) {
      int vid = 0;
      if (p_glXGetFBConfigAttrib(dpy, cfgs[i], GLX_VISUAL_ID, &vid) == 0 &&
          (unsigned long)vid == want)
      {
        cfg = cfgs[i];
        break;
      }
    }
  }
  const int ctx_attr[] = {GLX_CONTEXT_MAJOR_VERSION_ARB,
                          3,
                          GLX_CONTEXT_MINOR_VERSION_ARB,
                          3,
                          GLX_CONTEXT_PROFILE_MASK_ARB,
                          GLX_CONTEXT_CORE_PROFILE_BIT_ARB,
                          0};
  XErrHandler old = trap_begin(dpy);
  g->glx = create(dpy, cfg, NULL, 1, ctx_attr);
  if (g->glx && !window) {
    const int pb_attr[] = {GLX_PBUFFER_WIDTH, 1, GLX_PBUFFER_HEIGHT, 1, 0};
    g->pbuf = p_glXCreatePbuffer(dpy, cfg, pb_attr);
  }
  g->draw = window ? window : g->pbuf;
  int ok = g->glx && g->draw && p_glXMakeContextCurrent(dpy, g->draw, g->draw, g->glx);
  if (trap_end(dpy, old)) {
    ok = 0;
  }
  p_XFree(cfgs);
  if (!ok) {
    return fail("GLX context");
  }
  g->api = API_GLX;
  g->onscreen = window != 0;
  if (window) { /* present doesn't wait for vblank, like the D3D Present(0) */
    if (has_ext(ext, "GLX_EXT_swap_control")) {
      typedef void (*SwapEXT)(XDisplay *, XID, int);
      SwapEXT fn = (SwapEXT)p_glXGetProcAddressARB((const GLubyte *)"glXSwapIntervalEXT");
      if (fn) {
        old = trap_begin(dpy);
        fn(dpy, window, 0);
        trap_end(dpy, old);
      }
    }
    else if (has_ext(ext, "GLX_MESA_swap_control")) {
      typedef int (*SwapMESA)(unsigned);
      SwapMESA fn = (SwapMESA)p_glXGetProcAddressARB((const GLubyte *)"glXSwapIntervalMESA");
      if (fn) {
        fn(0);
      }
    }
  }
  return 1;
}

/* EGL on one display: a core 3.3 context, surfaceless or on a 1x1 pbuffer. */
static int egl_try(BuiGpu *g, EGLDisplay dpy)
{
  EGLint major = 0, minor = 0;
  if (!dpy || !p_eglInitialize(dpy, &major, &minor)) {
    return 0;
  }
  if (major < 1 || (major == 1 && minor < 4) || !p_eglBindAPI(EGL_OPENGL_API)) {
    return 0;
  }
  const int surfaceless = has_ext(p_eglQueryString(dpy, EGL_EXTENSIONS),
                                  "EGL_KHR_surfaceless_context");
  const EGLint cfg_attr[] = {EGL_SURFACE_TYPE,
                             surfaceless ? 0 : EGL_PBUFFER_BIT,
                             EGL_RENDERABLE_TYPE,
                             EGL_OPENGL_BIT,
                             EGL_RED_SIZE,
                             8,
                             EGL_GREEN_SIZE,
                             8,
                             EGL_BLUE_SIZE,
                             8,
                             EGL_NONE};
  EGLConfig cfg = NULL;
  EGLint n = 0;
  if (!p_eglChooseConfig(dpy, cfg_attr, &cfg, 1, &n) || n < 1) {
    return 0;
  }
  const EGLint ctx_attr[] = {EGL_CONTEXT_MAJOR_VERSION,
                             3,
                             EGL_CONTEXT_MINOR_VERSION,
                             3,
                             EGL_CONTEXT_OPENGL_PROFILE_MASK,
                             EGL_CONTEXT_OPENGL_CORE_PROFILE_BIT,
                             EGL_NONE};
  EGLContext ctx = p_eglCreateContext(dpy, cfg, NULL, ctx_attr);
  if (!ctx) {
    return 0;
  }
  EGLSurface surf = NULL;
  if (!surfaceless) {
    const EGLint pb_attr[] = {EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE};
    surf = p_eglCreatePbufferSurface(dpy, cfg, pb_attr);
    if (!surf) {
      p_eglDestroyContext(dpy, ctx);
      return 0;
    }
  }
  if (!p_eglMakeCurrent(dpy, surf, surf, ctx)) {
    if (surf) {
      p_eglDestroySurface(dpy, surf);
    }
    p_eglDestroyContext(dpy, ctx);
    return 0;
  }
  /* the display stays initialized: EGL displays are per process and
   * terminating one would pull it from under other users */
  g->api = API_EGL;
  g->edpy = dpy;
  g->egl = ctx;
  g->esurf = surf;
  return 1;
}

static int egl_create(BuiGpu *g)
{
  if (!egl_load()) {
    return 0;
  }
  /* BUI_GPU_EGL=default|surfaceless picks one; else headless first */
  const char *pick = getenv("BUI_GPU_EGL");
  const int want_default = pick && !strcmp(pick, "default");
  const int want_surfaceless = pick && !strcmp(pick, "surfaceless");
  const char *cext = p_eglQueryString(NULL, EGL_EXTENSIONS); /* client extensions */
  if (!want_default && has_ext(cext, "EGL_MESA_platform_surfaceless")) {
    typedef EGLDisplay (*GetPlatformFn)(EGLenum, void *, const EGLint *);
    GetPlatformFn get = NULL;
    if (has_ext(cext, "EGL_EXT_platform_base")) {
      get = (GetPlatformFn)p_eglGetProcAddress("eglGetPlatformDisplayEXT");
    }
    if (get && egl_try(g, get(EGL_PLATFORM_SURFACELESS_MESA, NULL, NULL))) {
      return 1;
    }
  }
  if (!want_surfaceless && egl_try(g, p_eglGetDisplay(NULL))) {
    return 1;
  }
  return fail("EGL context");
}

static GLuint compile(BuiGpu *g, GLenum type, const char *src)
{
  const GLuint sh = GL(glCreateShader)(type);
  GL(glShaderSource)(sh, 1, &src, NULL);
  GL(glCompileShader)(sh);
  GLint ok = 0;
  GL(glGetShaderiv)(sh, GL_COMPILE_STATUS, &ok);
  if (!ok) {
    if (debug_on()) {
      char log[2048] = {0};
      GL(glGetShaderInfoLog)(sh, sizeof(log) - 1, NULL, log);
      fprintf(stderr, "bui gpu (gl): shader:\n%s\n", log);
    }
    GL(glDeleteShader)(sh);
    return 0;
  }
  return sh;
}

static GLuint program(BuiGpu *g, const char *vs_src, const char *fs_src)
{
  const GLuint vs = compile(g, GL_VERTEX_SHADER, vs_src);
  const GLuint fs = vs ? compile(g, GL_FRAGMENT_SHADER, fs_src) : 0;
  if (!fs) {
    if (vs) {
      GL(glDeleteShader)(vs);
    }
    return 0;
  }
  const GLuint p = GL(glCreateProgram)();
  GL(glAttachShader)(p, vs);
  GL(glAttachShader)(p, fs);
  GL(glLinkProgram)(p);
  GL(glDeleteShader)(vs);
  GL(glDeleteShader)(fs);
  GLint ok = 0;
  GL(glGetProgramiv)(p, GL_LINK_STATUS, &ok);
  if (!ok) {
    if (debug_on()) {
      char log[2048] = {0};
      GL(glGetProgramInfoLog)(p, sizeof(log) - 1, NULL, log);
      fprintf(stderr, "bui gpu (gl): link:\n%s\n", log);
    }
    GL(glDeleteProgram)(p);
    return 0;
  }
  return p;
}

static GLuint tex_create(BuiGpu *g, GLint ifmt, int w, int h, GLenum fmt, GLenum type)
{
  GLuint t = 0;
  GL(glGenTextures)(1, &t);
  GL(glBindTexture)(GL_TEXTURE_2D, t);
  /* nearest, no mips: texelFetch needs a complete texture */
  GL(glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
  GL(glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
  GL(glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
  GL(glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
  while (GL(glGetError)() != GL_NO_ERROR) {
  }
  GL(glTexImage2D)(GL_TEXTURE_2D, 0, ifmt, w, h, 0, fmt, type, NULL);
  if (GL(glGetError)() != GL_NO_ERROR) {
    GL(glDeleteTextures)(1, &t);
    return 0;
  }
  return t;
}

static int atlas_create(BuiGpu *g, Atlas *a, int size, int bpp)
{
  memset(a, 0, sizeof(*a));
  a->size = size;
  a->bpp = bpp;
  a->tex = bpp == 1 ? tex_create(g, GL_R8, size, size, GL_RED, GL_UNSIGNED_BYTE) :
                      tex_create(g, GL_RGBA8, size, size, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV);
  return a->tex != 0;
}

static int device_init(BuiGpu *g)
{
  if (!gl_resolve(g)) {
    return 0;
  }
  const char *ver = (const char *)GL(glGetString)(GL_VERSION);
  int major = 0, minor = 0;
  if (!ver || sscanf(ver, "%d.%d", &major, &minor) != 2 || major < 3 ||
      (major == 3 && minor < 3))
  {
    return fail("OpenGL 3.3");
  }
  if (debug_on()) {
    fprintf(stderr, "bui gpu (gl): %s, %s | %s | %s\n", g->api == API_GLX ? "GLX" : "EGL",
            (const char *)GL(glGetString)(GL_VENDOR), (const char *)GL(glGetString)(GL_RENDERER),
            ver);
  }
  GL(glGetIntegerv)(GL_MAX_TEXTURE_SIZE, &g->max_tex);
  if (g->max_tex < 2048) {
    return fail("2048 px textures");
  }
  g->big_max = g->max_tex;
  const char *max_env = getenv("BUI_GPU_MAX_TEXTURE"); /* tests: force big-image tiling */
  if (max_env && atoi(max_env) >= 64 && atoi(max_env) < g->big_max) {
    g->big_max = atoi(max_env);
  }
  g->prog_quad = program(g, k_vs_quad, k_fs_quad);
  g->prog_tri = program(g, k_vs_tri, k_fs_tri);
  g->prog_mesh = program(g, k_vs_mesh, k_fs_mesh);
  if (!g->prog_quad || !g->prog_tri || !g->prog_mesh) {
    return fail("shaders");
  }
  g->ndc_quad = GL(glGetUniformLocation)(g->prog_quad, "to_ndc");
  g->ndc_tri = GL(glGetUniformLocation)(g->prog_tri, "to_ndc");
  g->ndc_mesh = GL(glGetUniformLocation)(g->prog_mesh, "to_ndc");
  GL(glUseProgram)(g->prog_mesh);
  GL(glUniform1i)(GL(glGetUniformLocation)(g->prog_mesh, "mesh_tex"), 4); /* unit 4 */
  GL(glUseProgram)(g->prog_quad);
  const char *samplers[4] = {"mask_tex", "rgba_tex", "curve_tex", "band_tex"};
  for (int i = 0; i < 4; i++) {
    GL(glUniform1i)(GL(glGetUniformLocation)(g->prog_quad, samplers[i]), i);
  }
  GL(glUniform1i)(GL(glGetUniformLocation)(g->prog_quad, "uv_tex"), 5); /* unit 4: meshes */
  GL(glUseProgram)(0);

  GL(glGenBuffers)(1, &g->ib);
  GL(glGenBuffers)(1, &g->tb);
  GL(glGenVertexArrays)(1, &g->vao_quad);
  GL(glGenVertexArrays)(1, &g->vao_tri);
  /* instances: 8 float4 attributes, one step per instance (pointers are set
   * per batch: GL 3.3 has no base instance) */
  GL(glBindVertexArray)(g->vao_quad);
  GL(glBindBuffer)(GL_ARRAY_BUFFER, g->ib);
  for (GLuint k = 0; k < 8; k++) {
    GL(glEnableVertexAttribArray)(k);
    GL(glVertexAttribDivisor)(k, 1);
  }
  GL(glBindVertexArray)(g->vao_tri);
  GL(glBindBuffer)(GL_ARRAY_BUFFER, g->tb);
  for (GLuint k = 0; k < 3; k++) {
    GL(glEnableVertexAttribArray)(k);
  }
  GL(glVertexAttribPointer)(0, 2, GL_FLOAT, GL_FALSE, sizeof(TriV), (const void *)offsetof(TriV, x));
  GL(glVertexAttribPointer)(1, 4, GL_FLOAT, GL_FALSE, sizeof(TriV), (const void *)offsetof(TriV, c));
  GL(glVertexAttribPointer)(2, 4, GL_FLOAT, GL_FALSE, sizeof(TriV),
                            (const void *)offsetof(TriV, clip));
  /* meshes: BuiMeshVertex (= ImDrawVert) + a uint32 index buffer */
  GL(glGenBuffers)(1, &g->mvb);
  GL(glGenBuffers)(1, &g->mib);
  GL(glGenVertexArrays)(1, &g->vao_mesh);
  GL(glBindVertexArray)(g->vao_mesh);
  GL(glBindBuffer)(GL_ARRAY_BUFFER, g->mvb);
  GL(glBindBuffer)(GL_ELEMENT_ARRAY_BUFFER, g->mib);
  for (GLuint k = 0; k < 3; k++) {
    GL(glEnableVertexAttribArray)(k);
  }
  GL(glVertexAttribPointer)(0, 2, GL_FLOAT, GL_FALSE, sizeof(BuiMeshVertex),
                            (const void *)offsetof(BuiMeshVertex, x));
  GL(glVertexAttribPointer)(1, 2, GL_FLOAT, GL_FALSE, sizeof(BuiMeshVertex),
                            (const void *)offsetof(BuiMeshVertex, u));
  GL(glVertexAttribPointer)(2, 4, GL_UNSIGNED_BYTE, 1, sizeof(BuiMeshVertex),
                            (const void *)offsetof(BuiMeshVertex, col));
  GL(glBindVertexArray)(0);
  {
    const uint32_t white = 0xFFFFFFFFu;
    g->white_tex = tex_create(g, GL_RGBA8, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE);
    if (!g->white_tex) {
      return fail("white texture");
    }
    GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, 0, 0, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, &white);
  }

  GL(glDisable)(GL_DEPTH_TEST);
  GL(glDisable)(GL_CULL_FACE);
  GL(glDisable)(GL_SCISSOR_TEST);
  GL(glDisable)(GL_DITHER);
  GL(glDisable)(GL_FRAMEBUFFER_SRGB);
  GL(glEnable)(GL_BLEND);
  GL(glBlendFunc)(GL_ONE, GL_ONE_MINUS_SRC_ALPHA); /* premultiplied, color and alpha */
  GL(glGetError)(); /* GL_FRAMEBUFFER_SRGB may be unknown: not an error that matters */

  if (!atlas_create(g, &g->mask, 2048, 1)) {
    return fail("atlas");
  }
  return 1;
}

static void context_free(BuiGpu *g)
{
  /* destroying the context frees its objects; the window may already be gone,
   * so nothing is drawn or made current here */
  if (g->xdpy && (g->glx || g->pbuf)) {
    XErrHandler old = trap_begin(g->xdpy);
    if (g->glx && p_glXGetCurrentContext() == g->glx) {
      p_glXMakeContextCurrent(g->xdpy, 0, 0, NULL);
    }
    if (g->glx) {
      p_glXDestroyContext(g->xdpy, g->glx);
    }
    if (g->pbuf) {
      p_glXDestroyPbuffer(g->xdpy, g->pbuf);
    }
    trap_end(g->xdpy, old);
  }
  if (g->egl) {
    if (p_eglGetCurrentContext() == g->egl) {
      p_eglMakeCurrent(g->edpy, NULL, NULL, NULL);
    }
    if (g->esurf) {
      p_eglDestroySurface(g->edpy, g->esurf);
    }
    p_eglDestroyContext(g->edpy, g->egl);
  }
  if (g->own_xdpy && g->xdpy) {
    p_XCloseDisplay(g->xdpy);
  }
  g->glx = NULL;
  g->egl = NULL;
  g->pbuf = 0;
  g->xdpy = NULL;
  g->api = 0;
}

BuiGpu *bui_gpu_create(void *handle)
{
  BuiGpu *g = (BuiGpu *)calloc(1, sizeof(BuiGpu));
  if (!g) {
    return NULL;
  }
  g->mask_map = (Slot *)calloc(MASK_MAP, sizeof(Slot));
  g->img_map = (Slot *)calloc(IMG_MAP, sizeof(Slot));
  int ok = g->mask_map && g->img_map;
  if (ok) {
    if (handle) { /* a window: GLX on the host's connection */
      const BuiGpuX11Window *xw = (const BuiGpuX11Window *)handle;
      ok = xw->window && glx_create(g, xw->display, (XID)xw->window);
    }
    else { /* offscreen: EGL, else a GLX pbuffer */
      ok = egl_create(g);
      if (!ok) {
        context_free(g);
        ok = glx_create(g, NULL, 0);
      }
    }
  }
  ok = ok && device_init(g);
  if (!ok) {
    bui_gpu_free(g);
    return NULL;
  }
  g->state = 1;
  return g;
}

int bui_gpu_state(BuiGpu *g, int wait)
{
  (void)wait; /* created synchronously: never pending */
  return g->state;
}

void bui_gpu_free(BuiGpu *g)
{
  if (!g) {
    return;
  }
  context_free(g); /* deletes the textures and buffers with it */
  free(g->dtex);
  free(g->inst);
  free(g->tv);
  free(g->mv);
  free(g->mi);
  free(g->b);
  free(g->mask_map);
  free(g->img_map);
  free(g->slug_map);
  free(g);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Atlases (shelf packing, as gpu_d3d11.c)
 * \{ */

static void atlas_clear(Atlas *a)
{
  a->nshelves = 0;
  a->next_y = 0;
}

static int atlas_place(Atlas *a, int w, int h, int *r_x, int *r_y)
{
  const int pw = w + 1, ph = h + 1;
  if (pw > a->size || ph > a->size) {
    return 0;
  }
  int best = -1;
  for (int i = 0; i < a->nshelves; i++) {
    if (a->shelf_h[i] >= ph && a->shelf_h[i] <= ph + ph / 4 + 2 && a->shelf_x[i] + pw <= a->size &&
        (best < 0 || a->shelf_h[i] < a->shelf_h[best]))
    {
      best = i;
    }
  }
  if (best < 0) {
    const int sh = (ph + 3) & ~3;
    if (a->nshelves == SHELVES || a->next_y + sh > a->size) {
      return 0;
    }
    best = a->nshelves++;
    a->shelf_y[best] = a->next_y;
    a->shelf_h[best] = sh;
    a->shelf_x[best] = 0;
    a->next_y += sh;
  }
  *r_x = a->shelf_x[best];
  *r_y = a->shelf_y[best];
  a->shelf_x[best] += pw;
  return 1;
}

static void atlas_upload(BuiGpu *g, Atlas *a, int x, int y, int w, int h, const void *data,
                         int pitch)
{
  if (!make_current(g)) {
    return;
  }
  GL(glBindTexture)(GL_TEXTURE_2D, a->tex);
  GL(glPixelStorei)(GL_UNPACK_ALIGNMENT, 1);
  GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, pitch / a->bpp);
  if (a->bpp == 1) {
    GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, x, y, w, h, GL_RED, GL_UNSIGNED_BYTE, data);
  }
  else { /* 0xAARRGGBB words: BGRA bytes, on any endianness */
    GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, x, y, w, h, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, data);
  }
  GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, 0);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Recording
 * \{ */

static void flush(BuiGpu *g);

static void big_sweep(BuiGpu *g);

void bui_gpu_begin(BuiGpu *g, int w, int h)
{
  g->w = w > 0 ? w : 1;
  g->h = h > 0 ? h : 1;
  g->frame++;
  big_sweep(g);
}

static Batch *batch_new(BuiGpu *g, int tri, int first);

/* The batch to append to: the last one when it is of the same kind. */
static Batch *batch_for(BuiGpu *g, int tri, int first)
{
  if (tri != 2 && g->n_b > 0 && g->b[g->n_b - 1].tri == tri) {
    return &g->b[g->n_b - 1];
  }
  return batch_new(g, tri, first);
}

static Batch *batch_new(BuiGpu *g, int tri, int first)
{
  if (g->n_b == g->cap_b) {
    const int cap = g->cap_b ? g->cap_b * 2 : 64;
    Batch *nb = (Batch *)realloc(g->b, sizeof(Batch) * (size_t)cap);
    if (!nb) {
      return NULL;
    }
    g->b = nb;
    g->cap_b = cap;
  }
  Batch *b = &g->b[g->n_b++];
  memset(b, 0, sizeof(*b));
  b->tri = tri;
  b->first = first;
  b->img = -1;
  return b;
}

/* -- textured meshes (as gpu_d3d11.c) ------------------------------------------------ */

static void dtex_release(BuiGpu *g, DevTex *t)
{
  if (t->tex) {
    GL(glDeleteTextures)(1, &t->tex);
  }
  t->tex = 0;
  t->id = 0;
}

static void dtex_upload(BuiGpu *g, const BuiTexView *v, int x, int y, int w, int h)
{
  GL(glPixelStorei)(GL_UNPACK_ALIGNMENT, 4);
  GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, v->w);
  GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, x, y, w, h, GL_RGBA, GL_UNSIGNED_BYTE,
                      v->rgba + ((size_t)y * v->w + x) * 4);
  GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, 0);
}

/* Creates or refreshes the context's copy of registry texture `id`. */
static void dtex_get(BuiGpu *g, int id)
{
  BuiTexView v;
  if (id <= 0 || !bui_texture_view(id, &v) || !make_current(g)) {
    return;
  }
  const int slot = (id & 0xFFFF) - 1;
  if (slot >= g->n_dtex) {
    const int n = slot + 16;
    DevTex *nd = (DevTex *)realloc(g->dtex, sizeof(DevTex) * (size_t)n);
    if (!nd) {
      return;
    }
    memset(nd + g->n_dtex, 0, sizeof(DevTex) * (size_t)(n - g->n_dtex));
    g->dtex = nd;
    g->n_dtex = n;
  }
  DevTex *t = &g->dtex[slot];
  if (t->id == id && t->gen == v.gen) {
    return;
  }
  if (t->id == id && t->w == v.w && t->h == v.h) {
    flush(g); /* draws recorded so far still need the old pixels */
    int rects[32][4];
    int n = bui_texture_changes(id, t->gen, rects, 32);
    if (n < 0) {
      rects[0][0] = rects[0][1] = 0;
      rects[0][2] = v.w;
      rects[0][3] = v.h;
      n = 1;
    }
    GL(glBindTexture)(GL_TEXTURE_2D, t->tex);
    for (int i = 0; i < n; i++) {
      dtex_upload(g, &v, rects[i][0], rects[i][1], rects[i][2], rects[i][3]);
    }
    t->gen = v.gen;
    return;
  }
  if (t->id) {
    flush(g);
    dtex_release(g, t);
  }
  t->tex = tex_create(g, GL_RGBA8, v.w, v.h, GL_RGBA, GL_UNSIGNED_BYTE);
  if (!t->tex) {
    return;
  }
  dtex_upload(g, &v, 0, 0, v.w, v.h);
  t->id = id;
  t->gen = v.gen;
  t->w = v.w;
  t->h = v.h;
  t->nearest = 1; /* tex_create's filter */
}

/* Binds the texture a recorded batch samples to unit 4. */
static void dtex_bind(BuiGpu *g, int id, int nearest)
{
  const int slot = (id & 0xFFFF) - 1;
  DevTex *t = id > 0 && slot < g->n_dtex && g->dtex[slot].id == id && g->dtex[slot].tex ?
                  &g->dtex[slot] :
                  NULL;
  GL(glActiveTexture)(GL_TEXTURE0 + 4);
  GL(glBindTexture)(GL_TEXTURE_2D, t ? t->tex : g->white_tex);
  if (t && t->nearest != nearest) {
    const GLint f = nearest ? GL_NEAREST : GL_LINEAR;
    GL(glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, f);
    GL(glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, f);
    t->nearest = nearest;
  }
  GL(glActiveTexture)(GL_TEXTURE0);
}

static void dtex_sweep(BuiGpu *g)
{
  const unsigned fg = bui_texture_free_gen();
  if (fg == g->dtex_free_gen || !make_current(g)) {
    return;
  }
  g->dtex_free_gen = fg;
  BuiTexView v;
  for (int i = 0; i < g->n_dtex; i++) {
    if (g->dtex[i].id && !bui_texture_view(g->dtex[i].id, &v)) {
      flush(g);
      dtex_release(g, &g->dtex[i]);
    }
  }
}

static int grow(void **p, int *cap, int need, size_t size)
{
  if (need <= *cap) {
    return 1;
  }
  int c = *cap ? *cap : 4096;
  while (c < need) {
    c *= 2;
  }
  void *np = realloc(*p, size * (size_t)c);
  if (!np) {
    return 0;
  }
  *p = np;
  *cap = c;
  return 1;
}

void bui_gpu_mesh(BuiGpu *g,
                  const BuiMeshVertex *vtx,
                  int n_vtx,
                  const void *idx,
                  int idx32,
                  int n_idx,
                  float ox,
                  float oy,
                  int tex,
                  int nearest,
                  const int clip[4])
{
  n_idx -= n_idx % 3;
  if (n_idx <= 0 || n_vtx <= 0) {
    return;
  }
  dtex_sweep(g);
  dtex_get(g, tex); /* may flush: before anything of this mesh is recorded */
  if (!grow((void **)&g->mv, &g->cap_mv, g->n_mv + n_vtx, sizeof(BuiMeshVertex)) ||
      !grow((void **)&g->mi, &g->cap_mi, g->n_mi + n_idx, sizeof(uint32_t)))
  {
    return;
  }
  Batch *b = NULL;
  if (g->n_b > 0) { /* same texture, sampler and clip: one draw */
    Batch *last = &g->b[g->n_b - 1];
    if (last->tri == 2 && last->tex == tex && last->nearest == nearest &&
        memcmp(last->clip, clip, sizeof(int[4])) == 0)
    {
      b = last;
    }
  }
  if (!b) {
    b = batch_for(g, 2, g->n_mi);
    if (!b) {
      return;
    }
    b->tex = tex;
    b->nearest = nearest;
    memcpy(b->clip, clip, sizeof(int[4]));
  }
  const uint32_t base = (uint32_t)g->n_mv;
  for (int i = 0; i < n_vtx; i++) {
    BuiMeshVertex *v = &g->mv[g->n_mv++];
    *v = vtx[i];
    v->x += ox;
    v->y += oy;
  }
  const uint16_t *i16 = (const uint16_t *)idx;
  const uint32_t *i32 = (const uint32_t *)idx;
  uint32_t *out = g->mi + g->n_mi;
  int n = 0;
  for (int i = 0; i < n_idx; i += 3) {
    const uint32_t a = idx32 ? i32[i] : i16[i], c1 = idx32 ? i32[i + 1] : i16[i + 1],
                   c2 = idx32 ? i32[i + 2] : i16[i + 2];
    if (a >= (uint32_t)n_vtx || c1 >= (uint32_t)n_vtx || c2 >= (uint32_t)n_vtx) {
      continue;
    }
    out[n++] = base + a;
    out[n++] = base + c1;
    out[n++] = base + c2;
  }
  g->n_mi += n;
  b->count += n;
}

static BuiGpuInst *inst_in(BuiGpu *g, Batch *b, int kind, int x0, int y0, int x1, int y1);

BuiGpuInst *bui_gpu_inst_image(BuiGpu *g, int tex, int x0, int y0, int x1, int y1)
{
  Batch *b = g->n_b > 0 ? &g->b[g->n_b - 1] : NULL;
  if (!b || b->tri != 0 || (b->img >= 0 && b->img != tex)) {
    b = batch_new(g, 0, g->n_inst); /* another texture: another draw */
  }
  if (!b) {
    return NULL;
  }
  b->img = tex;
  return inst_in(g, b, BG_IMAGE, x0, y0, x1, y1);
}

BuiGpuInst *bui_gpu_inst(BuiGpu *g, int kind, int x0, int y0, int x1, int y1)
{
  Batch *b = batch_for(g, 0, g->n_inst);
  return b ? inst_in(g, b, kind, x0, y0, x1, y1) : NULL;
}

static BuiGpuInst *inst_in(BuiGpu *g, Batch *b, int kind, int x0, int y0, int x1, int y1)
{
  if (g->n_inst == g->cap_inst) {
    const int cap = g->cap_inst ? g->cap_inst * 2 : 1024;
    BuiGpuInst *ni = (BuiGpuInst *)realloc(g->inst, sizeof(BuiGpuInst) * (size_t)cap);
    if (!ni) {
      return NULL;
    }
    g->inst = ni;
    g->cap_inst = cap;
  }
  b->count++;
  BuiGpuInst *in = &g->inst[g->n_inst++];
  memset(in, 0, sizeof(*in));
  in->bounds[0] = (float)x0;
  in->bounds[1] = (float)y0;
  in->bounds[2] = (float)x1;
  in->bounds[3] = (float)y1;
  in->p[0] = (float)kind;
  return in;
}

void bui_gpu_triangles(BuiGpu *g,
                       const float (*co)[2],
                       const float (*col)[4],
                       int count,
                       const int clip[4])
{
  count -= count % 3;
  if (count <= 0) {
    return;
  }
  if (g->n_tv + count > g->cap_tv) {
    int cap = g->cap_tv ? g->cap_tv : 1024;
    while (cap < g->n_tv + count) {
      cap *= 2;
    }
    TriV *nt = (TriV *)realloc(g->tv, sizeof(TriV) * (size_t)cap);
    if (!nt) {
      return;
    }
    g->tv = nt;
    g->cap_tv = cap;
  }
  Batch *b = batch_for(g, 1, g->n_tv);
  if (!b) {
    return;
  }
  for (int i = 0; i < count; i++) {
    TriV *v = &g->tv[g->n_tv++];
    v->x = co[i][0];
    v->y = co[i][1];
    memcpy(v->c, col[i], sizeof(v->c));
    for (int k = 0; k < 4; k++) {
      v->clip[k] = (float)clip[k];
    }
  }
  b->count += count;
}

static uint64_t mix64(uint64_t h)
{
  h ^= h >> 33;
  h *= 0xff51afd7ed558ccdull;
  h ^= h >> 33;
  return h;
}

static void mask_reset(BuiGpu *g)
{
  flush(g); /* recorded instances still point into the old layout */
  atlas_clear(&g->mask);
  memset(g->mask_map, 0, sizeof(Slot) * MASK_MAP);
  g->mask_n = 0;
}

int bui_gpu_mask(BuiGpu *g, const void *key, unsigned gen, const uint8_t *mask, int w, int h,
                 int *r_x, int *r_y)
{
  if (w + 1 > g->mask.size || h + 1 > g->mask.size) {
    return 0;
  }
  if (gen != g->mask_gen) {
    g->mask_gen = gen;
    mask_reset(g);
  }
  const uint64_t k1 = key ? (uint64_t)(uintptr_t)key : 0, k2 = ((uint64_t)w << 32) | (uint32_t)h;
  uint32_t idx = 0;
  if (key) {
    idx = (uint32_t)mix64(k1) & (MASK_MAP - 1);
    for (;;) {
      Slot *s = &g->mask_map[idx];
      if (s->k1 == 0) {
        break;
      }
      if (s->k1 == k1 && s->k2 == k2) {
        *r_x = s->x;
        *r_y = s->y;
        return 1;
      }
      idx = (idx + 1) & (MASK_MAP - 1);
    }
    if (g->mask_n >= MASK_MAP_FILL) {
      mask_reset(g);
      return bui_gpu_mask(g, key, gen, mask, w, h, r_x, r_y);
    }
  }
  int x, y;
  if (!atlas_place(&g->mask, w, h, &x, &y)) {
    mask_reset(g);
    return bui_gpu_mask(g, key, gen, mask, w, h, r_x, r_y);
  }
  atlas_upload(g, &g->mask, x, y, w, h, mask, w);
  if (key) {
    Slot *s = &g->mask_map[idx];
    s->k1 = k1;
    s->k2 = k2;
    s->x = x;
    s->y = y;
    g->mask_n++;
  }
  *r_x = x;
  *r_y = y;
  return 1;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Slug glyphs (same data layout as gpu_d3d11.c)
 * \{ */

int bui_gpu_slug_on(BuiGpu *g)
{
  if (g->slug == 0) {
    g->slug = -1;
    if (g->max_tex >= SLUG_W && make_current(g) &&
        (g->curve_tex = tex_create(g, GL_RGBA32F, SLUG_W, SLUG_H, GL_RGBA, GL_FLOAT)) != 0 &&
        (g->band_tex = tex_create(g, GL_RG32UI, SLUG_W, SLUG_H, GL_RG_INTEGER, GL_UNSIGNED_INT)) !=
            0 &&
        (g->slug_map = (SlugGlyph *)calloc(SLUG_MAP, sizeof(SlugGlyph))) != NULL)
    {
      g->slug = 1;
    }
  }
  return g->slug > 0;
}

static void slug_reset(BuiGpu *g)
{
  flush(g);
  memset(g->slug_map, 0, sizeof(SlugGlyph) * SLUG_MAP);
  g->slug_n = g->curve_next = g->band_next = 0;
}

/* Uploads `count` texels at linear index `at`, row by row. */
static void slug_upload(BuiGpu *g, GLuint tex, int band, int at, const void *data, int count,
                        int texel)
{
  if (!make_current(g)) {
    return;
  }
  GL(glBindTexture)(GL_TEXTURE_2D, tex);
  GL(glPixelStorei)(GL_UNPACK_ALIGNMENT, 4);
  GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, 0);
  const char *p = (const char *)data;
  while (count > 0) {
    const int x = at % SLUG_W, y = at / SLUG_W;
    const int n = count < SLUG_W - x ? count : SLUG_W - x;
    if (band) {
      GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, x, y, n, 1, GL_RG_INTEGER, GL_UNSIGNED_INT, p);
    }
    else {
      GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, x, y, n, 1, GL_RGBA, GL_FLOAT, p);
    }
    p += (size_t)n * texel;
    at += n;
    count -= n;
  }
}

typedef struct SlugSort {
  float key;
  uint32_t curve;
} SlugSort;

static int slug_cmp(const void *a, const void *b)
{
  const float ka = ((const SlugSort *)a)->key, kb = ((const SlugSort *)b)->key;
  return ka < kb ? 1 : ka > kb ? -1 : 0; /* descending */
}

static float min3(float a, float b, float c)
{
  return a < b ? (a < c ? a : c) : (b < c ? b : c);
}
static float max3(float a, float b, float c)
{
  return a > b ? (a > c ? a : c) : (b > c ? b : c);
}

static int slug_build(BuiGpu *g, SlugGlyph *sg, const float *cv, int n, const float bb[4])
{
  const int nb = n < 8 ? 1 : (n / 4 < SLUG_MAX_BANDS ? n / 4 : SLUG_MAX_BANDS);
  const float w = bb[2] - bb[0] > 1.0f ? bb[2] - bb[0] : 1.0f;
  const float h = bb[3] - bb[1] > 1.0f ? bb[3] - bb[1] : 1.0f;
  const float hs = (float)nb / h, vs = (float)nb / w;
  const float eps = 1.0f / 1024.0f * (w > h ? w : h);

  const int max_entries = 2 * nb * n;
  const int need_band = 2 * nb + max_entries, need_curve = 2 * n;
  if (need_curve > SLUG_W * SLUG_H / 4 || need_band > SLUG_W * SLUG_H / 4) {
    return 0;
  }
  if (g->curve_next + need_curve > SLUG_W * SLUG_H || g->band_next + need_band > SLUG_W * SLUG_H ||
      g->slug_n >= SLUG_MAP_FILL)
  {
    return -1;
  }
  float *ct = (float *)malloc(sizeof(float) * 8 * (size_t)n);
  uint32_t *bt = (uint32_t *)malloc(sizeof(uint32_t) * 2 * (size_t)need_band);
  SlugSort *tmp = (SlugSort *)malloc(sizeof(SlugSort) * (size_t)n);
  if (!ct || !bt || !tmp) {
    free(ct);
    free(bt);
    free(tmp);
    return 0;
  }
  const int cbase = g->curve_next;
  for (int i = 0; i < n; i++) {
    const float *q = cv + 6 * i;
    float *t = ct + 8 * i;
    t[0] = q[0];
    t[1] = q[1];
    t[2] = q[2];
    t[3] = q[3];
    t[4] = q[4];
    t[5] = q[5];
    t[6] = t[7] = 0.0f;
  }
  const int bbase = g->band_next;
  int at = 2 * nb;
  for (int dir = 0; dir < 2; dir++) {
    for (int b = 0; b < nb; b++) {
      const float s = dir == 0 ? hs : vs, o = dir == 0 ? bb[1] : bb[0];
      const float lo = o + (float)b / s - eps, hi = o + (float)(b + 1) / s + eps;
      int m = 0;
      for (int i = 0; i < n; i++) {
        const float *q = cv + 6 * i;
        const float a0 = dir == 0 ? q[1] : q[0], a1 = dir == 0 ? q[3] : q[2],
                    a2 = dir == 0 ? q[5] : q[4];
        const float amin = min3(a0, a1, a2), amax = max3(a0, a1, a2);
        if (amax < lo || amin > hi || (a0 == a1 && a1 == a2)) {
          continue;
        }
        tmp[m].key = dir == 0 ? max3(q[0], q[2], q[4]) : max3(q[1], q[3], q[5]);
        tmp[m].curve = (uint32_t)(cbase + 2 * i);
        m++;
      }
      qsort(tmp, (size_t)m, sizeof(SlugSort), slug_cmp);
      const int hdr = dir * nb + b;
      bt[2 * hdr] = (uint32_t)m;
      bt[2 * hdr + 1] = (uint32_t)(bbase + at);
      for (int k = 0; k < m; k++) {
        bt[2 * (at + k)] = tmp[k].curve;
        bt[2 * (at + k) + 1] = 0;
      }
      at += m;
    }
  }
  slug_upload(g, g->curve_tex, 0, cbase, ct, 2 * n, 16);
  slug_upload(g, g->band_tex, 1, bbase, bt, at, 8);
  g->curve_next += 2 * n;
  g->band_next += at;
  free(ct);
  free(bt);
  free(tmp);
  memcpy(sg->bbox, bb, sizeof(float[4]));
  sg->band[0] = hs;
  sg->band[1] = -bb[1] * hs;
  sg->band[2] = vs;
  sg->band[3] = -bb[0] * vs;
  sg->loc[0] = bbase;
  sg->loc[1] = nb;
  sg->loc[2] = nb;
  sg->loc[3] = 0;
  sg->ok = 1;
  return 1;
}

int bui_gpu_slug_glyph(BuiGpu *g, int face, int glyph, unsigned gen, float r_bbox[4],
                       float r_band[4], int r_loc[4])
{
  if (g->slug <= 0 || face < 0 || face > 0x7ffe || glyph < 0 || glyph > 0xffff) {
    return 0;
  }
  if (gen != g->slug_gen) {
    g->slug_gen = gen;
    slug_reset(g);
  }
  const uint32_t key = ((uint32_t)(face + 1) << 16) | (uint32_t)glyph;
  uint32_t idx = (key * 2654435761u) >> 19 & (SLUG_MAP - 1);
  SlugGlyph *sg;
  for (;;) {
    sg = &g->slug_map[idx];
    if (sg->key == 0 || sg->key == key) {
      break;
    }
    idx = (idx + 1) & (SLUG_MAP - 1);
  }
  if (sg->key == 0) {
    float *cv = NULL;
    int n = 0;
    float bb[4];
    sg->ok = 0;
    if (bui_text_glyph_outline(face, glyph, &cv, &n, bb)) {
      int got = slug_build(g, sg, cv, n, bb);
      if (got < 0) {
        slug_reset(g);
        free(cv);
        return bui_gpu_slug_glyph(g, face, glyph, gen, r_bbox, r_band, r_loc);
      }
    }
    free(cv);
    sg->key = key;
    g->slug_n++;
  }
  if (!sg->ok) {
    return 0;
  }
  memcpy(r_bbox, sg->bbox, sizeof(float[4]));
  memcpy(r_band, sg->band, sizeof(float[4]));
  memcpy(r_loc, sg->loc, sizeof(int[4]));
  return 1;
}

/** \} */

static void img_reset(BuiGpu *g)
{
  flush(g);
  atlas_clear(&g->rgba);
  memset(g->img_map, 0, sizeof(Slot) * IMG_MAP);
  g->img_n = 0;
}

int bui_gpu_image(BuiGpu *g, uint64_t key, const uint32_t *argb, int w, int h, int *r_x, int *r_y)
{
  if (!g->rgba.tex && (!make_current(g) || !atlas_create(g, &g->rgba, 2048, 4))) {
    return 0;
  }
  if (w + 1 > g->rgba.size || h + 1 > g->rgba.size) {
    return 0; /* TODO: images larger than the atlas */
  }
  /* key: BuiPixmap.key or its content hash (gpu_raster.c), with the size */
  const uint64_t k1 = key, k2 = ((uint64_t)w << 32) | (uint32_t)h;
  uint32_t idx = (uint32_t)mix64(k1) & (IMG_MAP - 1);
  for (;;) {
    Slot *s = &g->img_map[idx];
    if (s->k1 == 0) {
      break;
    }
    if (s->k1 == k1 && s->k2 == k2) {
      *r_x = s->x;
      *r_y = s->y;
      return 1;
    }
    idx = (idx + 1) & (IMG_MAP - 1);
  }
  int x, y;
  if (g->img_n >= IMG_MAP_FILL || !atlas_place(&g->rgba, w, h, &x, &y)) {
    img_reset(g);
    return bui_gpu_image(g, key, argb, w, h, r_x, r_y);
  }
  atlas_upload(g, &g->rgba, x, y, w, h, argb, w * 4);
  Slot *s = &g->img_map[idx];
  s->k1 = k1;
  s->k2 = k2;
  s->x = x;
  s->y = y;
  g->img_n++;
  *r_x = x;
  *r_y = y;
  return 1;
}

/* -- big images: textures of their own (as gpu_d3d11.c) ----------------------------- */

static size_t big_size(int format, int w, int h)
{
  return format == BUI_PIXMAP_ARGB ? (size_t)w * h * 4 : (size_t)w * h * 3 / 2;
}

static void big_release(BuiGpu *g, BigTex *t)
{
  if (t->tex) {
    g->big_bytes -= big_size(t->format, t->w, t->h);
    GL(glDeleteTextures)(1, &t->tex);
  }
  if (t->uv_tex) {
    GL(glDeleteTextures)(1, &t->uv_tex);
  }
  memset(t, 0, sizeof(*t));
}

/* `px` into slot `t`'s textures (created first when `create`). */
static int big_upload(BuiGpu *g, BigTex *t, int create, const uint32_t *px, int stride)
{
  const int nv12 = t->format != BUI_PIXMAP_ARGB;
  if (create) {
    t->tex = nv12 ? tex_create(g, GL_R8, t->w, t->h, GL_RED, GL_UNSIGNED_BYTE) :
                    tex_create(g, GL_RGBA8, t->w, t->h, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV);
    t->uv_tex = nv12 ? tex_create(g, GL_RG8, t->w / 2, t->h / 2, GL_RG, GL_UNSIGNED_BYTE) : 0;
    if (!t->tex || (nv12 && !t->uv_tex)) {
      GL(glDeleteTextures)(1, &t->tex);
      GL(glDeleteTextures)(1, &t->uv_tex);
      t->tex = t->uv_tex = 0;
      return 0;
    }
  }
  GL(glBindTexture)(GL_TEXTURE_2D, t->tex);
  if (!nv12) {
    GL(glPixelStorei)(GL_UNPACK_ALIGNMENT, 4);
    GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, stride);
    GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, 0, 0, t->w, t->h, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, px);
  }
  else {
    GL(glPixelStorei)(GL_UNPACK_ALIGNMENT, 1);
    GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, t->w);
    GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, 0, 0, t->w, t->h, GL_RED, GL_UNSIGNED_BYTE, px);
    GL(glBindTexture)(GL_TEXTURE_2D, t->uv_tex);
    GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, t->w / 2);
    GL(glTexSubImage2D)(GL_TEXTURE_2D, 0, 0, 0, t->w / 2, t->h / 2, GL_RG, GL_UNSIGNED_BYTE,
                        (const uint8_t *)px + (size_t)t->w * t->h);
    GL(glPixelStorei)(GL_UNPACK_ALIGNMENT, 4);
  }
  GL(glPixelStorei)(GL_UNPACK_ROW_LENGTH, 0);
  return 1;
}

static void big_sweep(BuiGpu *g)
{
  uint64_t gone[64];
  const int n = bui_image_forgotten(&g->forget_seen, gone, 64); /* -1: missed some */
  int current = -1;
  for (int i = 0; i < BIG_MAX; i++) {
    BigTex *t = &g->big[i];
    if (!t->key) {
      continue;
    }
    int drop = g->frame - t->used > BIG_IDLE || (n < 0 && t->used != g->frame);
    for (int k = 0; k < n && !drop; k++) {
      drop = gone[k] == t->base;
    }
    if (drop) {
      if (current < 0) {
        current = make_current(g);
      }
      if (current) {
        big_release(g, t);
      }
    }
  }
}

int bui_gpu_max_texture(BuiGpu *g)
{
  return g->big_max;
}

int bui_gpu_image_tex(BuiGpu *g, uint64_t key, uint64_t base, int stream, int format,
                      const uint32_t *px, int stride, int w, int h)
{
  if (w <= 0 || h <= 0 || w > g->big_max || h > g->big_max || !make_current(g)) {
    return 0;
  }
  int free_slot = -1, lru = -1;
  for (int i = 0; i < BIG_MAX; i++) {
    BigTex *t = &g->big[i];
    if (t->key && t->w == w && t->h == h && t->format == format &&
        (t->key == key || (stream && t->stream && t->base == base)))
    {
      if (t->key != key) { /* the stream's next frame: in place, no new texture */
        big_upload(g, t, 0, px, stride);
        t->key = key;
      }
      t->used = g->frame;
      return i + 1;
    }
    if (!t->key) {
      free_slot = free_slot < 0 ? i : free_slot;
    }
    else if (t->used != g->frame && (lru < 0 || t->used < g->big[lru].used)) {
      lru = i; /* not drawn this frame: no recorded batch samples it */
    }
  }
  const size_t bytes = big_size(format, w, h);
  while (lru >= 0 && (free_slot < 0 || g->big_bytes + bytes > BIG_BUDGET)) {
    big_release(g, &g->big[lru]);
    free_slot = free_slot < 0 ? lru : free_slot;
    lru = -1;
    for (int i = 0; i < BIG_MAX; i++) {
      const BigTex *t = &g->big[i];
      if (t->key && t->used != g->frame && (lru < 0 || t->used < g->big[lru].used)) {
        lru = i;
      }
    }
  }
  if (free_slot < 0) {
    return 0;
  }
  BigTex *t = &g->big[free_slot];
  t->w = w;
  t->h = h;
  t->format = format;
  t->stream = stream;
  if (!big_upload(g, t, 1, px, stride)) {
    memset(t, 0, sizeof(*t));
    return 0;
  }
  t->key = key;
  t->base = base;
  t->used = g->frame;
  g->big_bytes += bytes;
  return free_slot + 1;
}

void bui_gpu_damage(BuiGpu *g, int x, int y, int w, int h)
{
  if (w <= 0 || h <= 0 || g->n_dmg == (int)(sizeof(g->dmg) / sizeof(g->dmg[0]))) {
    g->dmg_full = 1;
    return;
  }
  int *r = g->dmg[g->n_dmg++];
  r[0] = x;
  r[1] = y;
  r[2] = x + w;
  r[3] = y + h;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Drawing + present
 * \{ */

static int ensure_target(BuiGpu *g)
{
  if (g->target && g->tw == g->w && g->th == g->h) {
    return 1;
  }
  if (g->target) {
    GL(glDeleteTextures)(1, &g->target);
    g->target = 0;
  }
  if (!g->fbo) {
    GL(glGenFramebuffers)(1, &g->fbo);
  }
  if (g->w > g->max_tex || g->h > g->max_tex) {
    return 0;
  }
  g->target = tex_create(g, GL_RGBA8, g->w, g->h, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV);
  if (!g->target) {
    return 0;
  }
  GL(glBindFramebuffer)(GL_FRAMEBUFFER, g->fbo);
  GL(glFramebufferTexture2D)(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, g->target, 0);
  if (GL(glCheckFramebufferStatus)(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
    GL(glDeleteTextures)(1, &g->target);
    g->target = 0;
    return 0;
  }
  g->tw = g->w;
  g->th = g->h;
  GL(glViewport)(0, 0, g->tw, g->th);
  GL(glClearColor)(0, 0, 0, 1);
  GL(glClear)(GL_COLOR_BUFFER_BIT);
  g->dmg_full = 1;
  return 1;
}

static void flush(BuiGpu *g)
{
  if (g->n_b == 0) {
    return;
  }
  if (!make_current(g) || !ensure_target(g)) {
    g->n_b = g->n_inst = g->n_tv = g->n_mv = g->n_mi = 0;
    return;
  }
  GL(glBindFramebuffer)(GL_FRAMEBUFFER, g->fbo);
  GL(glViewport)(0, 0, g->tw, g->th);
  /* orphaned each flush: draws already queued keep their data */
  if (g->n_inst) {
    GL(glBindBuffer)(GL_ARRAY_BUFFER, g->ib);
    GL(glBufferData)(GL_ARRAY_BUFFER, (GLsizeiptr)(sizeof(BuiGpuInst) * (size_t)g->n_inst),
                     g->inst, GL_STREAM_DRAW);
  }
  if (g->n_tv) {
    GL(glBindBuffer)(GL_ARRAY_BUFFER, g->tb);
    GL(glBufferData)(GL_ARRAY_BUFFER, (GLsizeiptr)(sizeof(TriV) * (size_t)g->n_tv), g->tv,
                     GL_STREAM_DRAW);
  }
  if (g->n_mi) {
    GL(glBindVertexArray)(g->vao_mesh); /* the element buffer binding lives in the VAO */
    GL(glBindBuffer)(GL_ARRAY_BUFFER, g->mvb);
    GL(glBufferData)(GL_ARRAY_BUFFER, (GLsizeiptr)(sizeof(BuiMeshVertex) * (size_t)g->n_mv),
                     g->mv, GL_STREAM_DRAW);
    GL(glBufferData)(GL_ELEMENT_ARRAY_BUFFER, (GLsizeiptr)(sizeof(uint32_t) * (size_t)g->n_mi),
                     g->mi, GL_STREAM_DRAW);
    GL(glBindVertexArray)(0);
  }
  const GLuint tex[4] = {g->mask.tex, g->rgba.tex, g->curve_tex, g->band_tex};
  for (int i = 0; i < 4; i++) {
    GL(glActiveTexture)(GL_TEXTURE0 + (GLenum)i);
    GL(glBindTexture)(GL_TEXTURE_2D, tex[i]);
  }
  GL(glActiveTexture)(GL_TEXTURE0);
  const float sx = 2.0f / (float)g->tw, sy = 2.0f / (float)g->th;

  int cur = -1, img = 0;
  for (int i = 0; i < g->n_b; i++) {
    const Batch *b = &g->b[i];
    if (b->count == 0) {
      continue;
    }
    if (b->tri != cur) {
      if (cur == 2) {
        GL(glDisable)(GL_SCISSOR_TEST);
      }
      cur = b->tri;
      if (b->tri == 2) {
        GL(glUseProgram)(g->prog_mesh);
        GL(glUniform2f)(g->ndc_mesh, sx, sy);
        GL(glBindVertexArray)(g->vao_mesh);
        GL(glEnable)(GL_SCISSOR_TEST);
      }
      else if (!b->tri) {
        GL(glUseProgram)(g->prog_quad);
        GL(glUniform2f)(g->ndc_quad, sx, sy);
        GL(glBindVertexArray)(g->vao_quad);
        GL(glBindBuffer)(GL_ARRAY_BUFFER, g->ib);
      }
      else {
        GL(glUseProgram)(g->prog_tri);
        GL(glUniform2f)(g->ndc_tri, sx, sy);
        GL(glBindVertexArray)(g->vao_tri);
      }
    }
    if (b->tri == 2) {
      /* the frame is stored top row first: GL y == BUI y */
      dtex_bind(g, b->tex, b->nearest);
      GL(glScissor)(b->clip[0], b->clip[1], b->clip[2] - b->clip[0], b->clip[3] - b->clip[1]);
      GL(glDrawElements)(GL_TRIANGLES, b->count, GL_UNSIGNED_INT,
                         (const void *)(sizeof(uint32_t) * (size_t)b->first));
    }
    else if (!b->tri) {
      const int want = b->img > 0 && g->big[b->img - 1].tex ? b->img : 0;
      if (want != img) { /* unit 1: the atlas, or a big image's own texture (5: NV12 chroma) */
        img = want;
        GL(glActiveTexture)(GL_TEXTURE0 + 1);
        GL(glBindTexture)(GL_TEXTURE_2D, img ? g->big[img - 1].tex : g->rgba.tex);
        GL(glActiveTexture)(GL_TEXTURE0 + 5);
        GL(glBindTexture)(GL_TEXTURE_2D, img ? g->big[img - 1].uv_tex : 0);
        GL(glActiveTexture)(GL_TEXTURE0);
      }
      const size_t base = sizeof(BuiGpuInst) * (size_t)b->first;
      for (GLuint k = 0; k < 8; k++) {
        GL(glVertexAttribPointer)(k, 4, GL_FLOAT, GL_FALSE, sizeof(BuiGpuInst),
                                  (const void *)(base + 16 * k));
      }
      GL(glDrawArraysInstanced)(GL_TRIANGLE_STRIP, 0, 4, b->count);
    }
    else {
      GL(glDrawArrays)(GL_TRIANGLES, b->first, b->count);
    }
  }
  if (cur == 2) {
    GL(glDisable)(GL_SCISSOR_TEST);
  }
  GL(glBindVertexArray)(0);
  g->n_b = g->n_inst = g->n_tv = g->n_mv = g->n_mi = 0;
}

int bui_gpu_present(BuiGpu *g, int full)
{
  (void)full;
  flush(g);
  if (!g->target || !make_current(g)) {
    g->n_dmg = g->dmg_full = 0;
    return g->target != 0;
  }
  if (g->onscreen) {
    /* the back buffer is undefined after a swap (no GLX dirty-rect present):
     * blit the whole frame, flipped (it is stored top row first), anchored
     * to the window's top edge */
    unsigned int dh = 0;
    p_glXQueryDrawable(g->xdpy, g->draw, GLX_HEIGHT, &dh);
    const int top = dh > 0 ? (int)dh : g->th;
    GL(glBindFramebuffer)(GL_READ_FRAMEBUFFER, g->fbo);
    GL(glBindFramebuffer)(GL_DRAW_FRAMEBUFFER, 0);
    GL(glBlitFramebuffer)(0, 0, g->tw, g->th, 0, top, g->tw, top - g->th, GL_COLOR_BUFFER_BIT,
                          GL_NEAREST);
    GL(glBindFramebuffer)(GL_FRAMEBUFFER, g->fbo);
    p_glXSwapBuffers(g->xdpy, g->draw);
  }
  else {
    GL(glFlush)();
  }
  g->n_dmg = g->dmg_full = 0;
  return GL(glGetError)() != GL_CONTEXT_LOST;
}

int bui_gpu_read(BuiGpu *g, uint32_t *dst, int w, int h)
{
  flush(g);
  if (!g->target || g->tw != w || g->th != h || !make_current(g)) {
    return 0;
  }
  GL(glBindFramebuffer)(GL_FRAMEBUFFER, g->fbo);
  GL(glPixelStorei)(GL_PACK_ALIGNMENT, 4);
  GL(glPixelStorei)(GL_PACK_ROW_LENGTH, 0);
  /* rows come back top first (that is how the frame is stored), as
   * 0xAARRGGBB words */
  GL(glReadPixels)(0, 0, w, h, GL_BGRA, GL_UNSIGNED_INT_8_8_8_8_REV, dst);
  const size_t n = (size_t)w * h;
  for (size_t i = 0; i < n; i++) {
    dst[i] &= 0xFFFFFFu; /* 0x00RRGGBB like the raster */
  }
  return 1;
}

/** \} */

#endif /* BUI_GPU_GL */
