/* BUI GPU renderer: Direct3D 11 backend — see gpu_d3d11.h.
 *
 * Like Blender's widget drawing (interface_widgets.cc batches widget bases
 * into GPU_SHADER_2D_WIDGET_BASE_INST draws), every primitive becomes one
 * instance of a single uber-shader, so a frame of thousands of widgets and
 * text runs is a handful of DrawInstanced calls. Differences from Blender,
 * for a retained UI:
 *  - frames render into a persistent target: only damaged regions are
 *    redrawn (the kit's damage tracking) and the swap chain gets dirty rects,
 *    so the compositor only recomposes what changed;
 *  - text is pre-shaped glyph runs (the same masks the software raster draws)
 *    packed into one R8 atlas, images into one BGRA atlas;
 *  - the device is created on a worker thread so the first frame never waits
 *    for the driver (the software raster draws until it is ready).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#ifdef _WIN32

#  ifndef WIN32_LEAN_AND_MEAN
#    define WIN32_LEAN_AND_MEAN
#  endif
#  define COBJMACROS
#  include <windows.h>
#  include <d3d11.h>
#  include <dxgi1_2.h>

#  include <stdio.h>
#  include <stdlib.h>
#  include <string.h>

#  include "gpu_d3d11.h"
#  include "gpu_d3d11_shaders.h"

static const GUID k_IID_IDXGIDevice = {
    0x54ec77fa, 0x1377, 0x44e6, {0x8c, 0x32, 0x88, 0xfd, 0x5f, 0x44, 0xc8, 0x4c}};
static const GUID k_IID_IDXGIFactory2 = {
    0x50c83a1c, 0xe072, 0x4c48, {0x87, 0xb0, 0x36, 0x30, 0xfa, 0x36, 0xa6, 0xd0}};
static const GUID k_IID_ID3D11Texture2D = {
    0x6f15aaf2, 0xd208, 0x4e89, {0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3, 0x4f, 0x9c}};

#  define REL(p) \
    do { \
      if (p) { \
        IUnknown_Release((IUnknown *)(p)); \
        (p) = NULL; \
      } \
    } while (0)

/* -------------------------------------------------------------------- */
/** \name Atlases (shelf packing)
 * \{ */

#  define SHELVES 128

typedef struct Atlas {
  int size;
  int bpp; /* 1 (R8) or 4 (BGRA8) */
  ID3D11Texture2D *tex;
  ID3D11ShaderResourceView *srv;
  int shelf_y[SHELVES], shelf_h[SHELVES], shelf_x[SHELVES];
  int nshelves, next_y;
} Atlas;

/* cache entry: key -> placement */
typedef struct Slot {
  uint64_t k1, k2; /* 0 = empty */
  int x, y;
} Slot;

#  define MASK_MAP 16384 /* power of two */
#  define MASK_MAP_FILL 12000
#  define IMG_MAP 1024
#  define IMG_MAP_FILL 700

/* Slug glyph data: curves (2 RGBA32F texels each) and bands (RG32_UINT
 * texels), both addressed linearly over 4096-wide textures. */
#  define SLUG_W 4096
#  define SLUG_H 64
#  define SLUG_MAP 8192 /* power of two */
#  define SLUG_MAP_FILL 6000
#  define SLUG_MAX_BANDS 16

typedef struct SlugGlyph {
  uint32_t key; /* (face + 1) << 16 | glyph; 0 = empty */
  int ok;       /* 0: empty glyph */
  float bbox[4], band[4];
  int loc[4];
} SlugGlyph;

typedef struct Batch {
  int tri; /* 0: quad instances, 1: triangles */
  int first, count;
} Batch;

typedef struct TriV {
  float x, y;
  float c[4];
  float clip[4];
} TriV;

struct BuiGpu {
  HWND hwnd;
  volatile LONG state; /* 0 pending, 1 ready, -1 failed */
  HANDLE thread;

  ID3D11Device *dev;
  ID3D11DeviceContext *ctx;
  IDXGIFactory2 *factory;
  IDXGISwapChain1 *swap;
  int sw, sh; /* swap chain size */
  int swap_fresh;

  ID3D11VertexShader *vs_quad, *vs_tri;
  ID3D11PixelShader *ps_quad, *ps_tri;
  ID3D11InputLayout *il_quad, *il_tri;
  ID3D11Buffer *cb, *ib, *tb;
  UINT ib_cap, tb_cap; /* in elements */
  ID3D11BlendState *blend;
  ID3D11RasterizerState *rs;

  ID3D11Texture2D *target;
  ID3D11RenderTargetView *rtv;
  int tw, th;
  int w, h; /* frame size */

  Atlas mask, rgba;
  Slot *mask_map, *img_map;
  int mask_n, img_n;
  unsigned mask_gen;

  int slug; /* -1: off (env or unavailable), 0: not created yet, 1: ready */
  ID3D11Texture2D *curve_tex, *band_tex;
  ID3D11ShaderResourceView *curve_srv, *band_srv;
  SlugGlyph *slug_map;
  int slug_n, curve_next, band_next;
  unsigned slug_gen;

  BuiGpuInst *inst;
  int n_inst, cap_inst;
  TriV *tv;
  int n_tv, cap_tv;
  Batch *b;
  int n_b, cap_b;

  RECT dmg[16];
  int n_dmg, dmg_full;
};

static int atlas_create(BuiGpu *g, Atlas *a, int size, int bpp)
{
  memset(a, 0, sizeof(*a));
  a->size = size;
  a->bpp = bpp;
  D3D11_TEXTURE2D_DESC td = {0};
  td.Width = td.Height = (UINT)size;
  td.MipLevels = td.ArraySize = 1;
  td.Format = bpp == 1 ? DXGI_FORMAT_R8_UNORM : DXGI_FORMAT_B8G8R8A8_UNORM;
  td.SampleDesc.Count = 1;
  td.Usage = D3D11_USAGE_DEFAULT;
  td.BindFlags = D3D11_BIND_SHADER_RESOURCE;
  if (FAILED(ID3D11Device_CreateTexture2D(g->dev, &td, NULL, &a->tex))) {
    return 0;
  }
  if (FAILED(ID3D11Device_CreateShaderResourceView(g->dev, (ID3D11Resource *)a->tex, NULL,
                                                   &a->srv)))
  {
    REL(a->tex);
    return 0;
  }
  return 1;
}

static void atlas_clear(Atlas *a)
{
  a->nshelves = 0;
  a->next_y = 0;
}

/* Finds room for w x h (+1 px gutter). 0 when full. */
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
    const int sh = (ph + 3) & ~3; /* round up: more strings share a shelf */
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
  D3D11_BOX box = {(UINT)x, (UINT)y, 0, (UINT)(x + w), (UINT)(y + h), 1};
  ID3D11DeviceContext_UpdateSubresource(g->ctx, (ID3D11Resource *)a->tex, 0, &box, data,
                                        (UINT)pitch, 0);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Device (worker thread)
 * \{ */

typedef HRESULT(WINAPI *CreateDeviceFn)(IDXGIAdapter *, D3D_DRIVER_TYPE, HMODULE, UINT,
                                        const D3D_FEATURE_LEVEL *, UINT, UINT, ID3D11Device **,
                                        D3D_FEATURE_LEVEL *, ID3D11DeviceContext **);

/* BUI_GPU_DEBUG=1: why the GPU path wasn't taken */
static int fail(const char *what, HRESULT hr)
{
  const char *dbg = getenv("BUI_GPU_DEBUG");
  if (dbg && dbg[0] == '1') {
    fprintf(stderr, "bui gpu: %s failed (0x%08lx)\n", what, (unsigned long)hr);
  }
  return 0;
}

static int device_init(BuiGpu *g)
{
  HMODULE dll = LoadLibraryW(L"d3d11.dll");
  if (!dll) {
    return fail("loading d3d11.dll", 0);
  }
  CreateDeviceFn create = (CreateDeviceFn)(void *)GetProcAddress(dll, "D3D11CreateDevice");
  if (!create) {
    return 0;
  }
  const D3D_FEATURE_LEVEL levels[] = {D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0,
                                      D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_10_0};
  const char *warp = getenv("BUI_GPU_WARP");
  const D3D_DRIVER_TYPE type = (warp && warp[0] == '1') ? D3D_DRIVER_TYPE_WARP :
                                                          D3D_DRIVER_TYPE_HARDWARE;
  D3D_FEATURE_LEVEL got;
  HRESULT hr = create(NULL, type, NULL, D3D11_CREATE_DEVICE_BGRA_SUPPORT, levels, 4,
                      D3D11_SDK_VERSION, &g->dev, &got, &g->ctx);
  if (FAILED(hr)) { /* 11_1 unknown to the runtime: retry without it */
    hr = create(NULL, type, NULL, D3D11_CREATE_DEVICE_BGRA_SUPPORT, levels + 1, 3,
                D3D11_SDK_VERSION, &g->dev, &got, &g->ctx);
  }
  if (FAILED(hr)) {
    return fail("D3D11CreateDevice", hr);
  }
  IDXGIDevice *xd = NULL;
  IDXGIAdapter *ad = NULL;
  if (FAILED(ID3D11Device_QueryInterface(g->dev, &k_IID_IDXGIDevice, (void **)&xd)) ||
      FAILED(IDXGIDevice_GetAdapter(xd, &ad)) ||
      FAILED(IDXGIAdapter_GetParent(ad, &k_IID_IDXGIFactory2, (void **)&g->factory)))
  {
    REL(ad);
    REL(xd);
    return fail("DXGI factory", 0);
  }
  REL(ad);
  REL(xd);

  ID3D11Device *d = g->dev;
  if (FAILED(ID3D11Device_CreateVertexShader(d, g_vs_quad, sizeof(g_vs_quad), NULL, &g->vs_quad)) ||
      FAILED(ID3D11Device_CreatePixelShader(d, g_ps_quad, sizeof(g_ps_quad), NULL, &g->ps_quad)) ||
      FAILED(ID3D11Device_CreateVertexShader(d, g_vs_tri, sizeof(g_vs_tri), NULL, &g->vs_tri)) ||
      FAILED(ID3D11Device_CreatePixelShader(d, g_ps_tri, sizeof(g_ps_tri), NULL, &g->ps_tri)))
  {
    return fail("shaders", 0);
  }
#  define INST(name, idx, off) \
    {name, idx, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, off, D3D11_INPUT_PER_INSTANCE_DATA, 1}
  const D3D11_INPUT_ELEMENT_DESC il_quad[] = {
      INST("RECT", 0, 0),     INST("BOUNDS", 0, 16), INST("COLOR", 0, 32), INST("COLOR", 1, 48),
      INST("COLOR", 2, 64),   INST("COLOR", 3, 80),  INST("PARAMS", 0, 96), INST("EXTRA", 0, 112),
  };
#  undef INST
  const D3D11_INPUT_ELEMENT_DESC il_tri[] = {
      {"POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0},
      {"COLOR", 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 8, D3D11_INPUT_PER_VERTEX_DATA, 0},
      {"CLIP", 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 24, D3D11_INPUT_PER_VERTEX_DATA, 0},
  };
  if (FAILED(ID3D11Device_CreateInputLayout(d, il_quad, 8, g_vs_quad, sizeof(g_vs_quad),
                                            &g->il_quad)) ||
      FAILED(ID3D11Device_CreateInputLayout(d, il_tri, 3, g_vs_tri, sizeof(g_vs_tri), &g->il_tri)))
  {
    return fail("input layouts", 0);
  }
  D3D11_BUFFER_DESC bd = {0};
  bd.ByteWidth = 16;
  bd.Usage = D3D11_USAGE_DYNAMIC;
  bd.BindFlags = D3D11_BIND_CONSTANT_BUFFER;
  bd.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
  if (FAILED(ID3D11Device_CreateBuffer(d, &bd, NULL, &g->cb))) {
    return fail("constant buffer", 0);
  }
  D3D11_BLEND_DESC bl = {0};
  bl.RenderTarget[0].BlendEnable = TRUE;
  bl.RenderTarget[0].SrcBlend = D3D11_BLEND_ONE;
  bl.RenderTarget[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
  bl.RenderTarget[0].BlendOp = D3D11_BLEND_OP_ADD;
  bl.RenderTarget[0].SrcBlendAlpha = D3D11_BLEND_ONE;
  bl.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
  bl.RenderTarget[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
  bl.RenderTarget[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL;
  if (FAILED(ID3D11Device_CreateBlendState(d, &bl, &g->blend))) {
    return fail("blend state", 0);
  }
  D3D11_RASTERIZER_DESC rd = {0};
  rd.FillMode = D3D11_FILL_SOLID;
  rd.CullMode = D3D11_CULL_NONE;
  rd.DepthClipEnable = TRUE;
  if (FAILED(ID3D11Device_CreateRasterizerState(d, &rd, &g->rs))) {
    return fail("rasterizer state", 0);
  }
  if (!atlas_create(g, &g->mask, 2048, 1)) {
    return fail("atlas", 0);
  }
  return 1;
}

static DWORD WINAPI device_thread(void *arg)
{
  BuiGpu *g = (BuiGpu *)arg;
  const int ok = device_init(g);
  InterlockedExchange(&g->state, ok ? 1 : -1);
  return 0;
}

BuiGpu *bui_gpu_create(void *hwnd)
{
  BuiGpu *g = (BuiGpu *)calloc(1, sizeof(BuiGpu));
  if (!g) {
    return NULL;
  }
  g->hwnd = (HWND)hwnd;
  g->mask_map = (Slot *)calloc(MASK_MAP, sizeof(Slot));
  g->img_map = (Slot *)calloc(IMG_MAP, sizeof(Slot));
  g->thread = CreateThread(NULL, 0, device_thread, g, 0, NULL);
  if (!g->thread || !g->mask_map || !g->img_map) {
    if (g->thread) {
      WaitForSingleObject(g->thread, INFINITE);
      CloseHandle(g->thread);
      g->thread = NULL;
    }
    bui_gpu_free(g);
    return NULL;
  }
  return g;
}

int bui_gpu_state(BuiGpu *g, int wait)
{
  if (wait && g->thread) {
    WaitForSingleObject(g->thread, INFINITE);
  }
  const LONG s = InterlockedCompareExchange(&g->state, 0, 0);
  if (s != 0 && g->thread) {
    WaitForSingleObject(g->thread, INFINITE);
    CloseHandle(g->thread);
    g->thread = NULL;
  }
  return (int)s;
}

void bui_gpu_free(BuiGpu *g)
{
  if (!g) {
    return;
  }
  if (g->thread) {
    WaitForSingleObject(g->thread, INFINITE);
    CloseHandle(g->thread);
  }
  if (g->ctx) {
    ID3D11DeviceContext_ClearState(g->ctx);
  }
  REL(g->mask.srv);
  REL(g->mask.tex);
  REL(g->rgba.srv);
  REL(g->rgba.tex);
  REL(g->curve_srv);
  REL(g->curve_tex);
  REL(g->band_srv);
  REL(g->band_tex);
  REL(g->rtv);
  REL(g->target);
  REL(g->rs);
  REL(g->blend);
  REL(g->cb);
  REL(g->ib);
  REL(g->tb);
  REL(g->il_quad);
  REL(g->il_tri);
  REL(g->vs_quad);
  REL(g->vs_tri);
  REL(g->ps_quad);
  REL(g->ps_tri);
  REL(g->swap);
  REL(g->factory);
  REL(g->ctx);
  REL(g->dev);
  free(g->inst);
  free(g->tv);
  free(g->b);
  free(g->mask_map);
  free(g->img_map);
  free(g->slug_map);
  free(g);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Recording
 * \{ */

static void flush(BuiGpu *g);

void bui_gpu_begin(BuiGpu *g, int w, int h)
{
  g->w = w > 0 ? w : 1;
  g->h = h > 0 ? h : 1;
}

static Batch *batch_for(BuiGpu *g, int tri, int first)
{
  if (g->n_b > 0 && g->b[g->n_b - 1].tri == tri) {
    return &g->b[g->n_b - 1];
  }
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
  b->tri = tri;
  b->first = first;
  b->count = 0;
  return b;
}

BuiGpuInst *bui_gpu_inst(BuiGpu *g, int kind, int x0, int y0, int x1, int y1)
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
  Batch *b = batch_for(g, 0, g->n_inst);
  if (!b) {
    return NULL;
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
  if (gen != g->mask_gen) { /* text runs were freed: their keys may be reused */
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
    mask_reset(g); /* empty now, and the map slot must be found again */
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

/* -------------------------------------------------------------------- */
/** \name Slug glyphs
 *
 * Eric Lengyel's Slug algorithm (https://github.com/EricLengyel/Slug):
 * glyph outlines live on the GPU as quadratic curves, split into horizontal
 * and vertical bands; the pixel shader casts a ray each way through the
 * pixel's bands and integrates exact coverage. Resolution independent, one
 * quad per glyph, no rasterizing or atlas packing on the CPU.
 * \{ */

static int slug_tex(BuiGpu *g, DXGI_FORMAT fmt, ID3D11Texture2D **r_tex,
                    ID3D11ShaderResourceView **r_srv)
{
  D3D11_TEXTURE2D_DESC td = {0};
  td.Width = SLUG_W;
  td.Height = SLUG_H;
  td.MipLevels = td.ArraySize = 1;
  td.Format = fmt;
  td.SampleDesc.Count = 1;
  td.Usage = D3D11_USAGE_DEFAULT;
  td.BindFlags = D3D11_BIND_SHADER_RESOURCE;
  return SUCCEEDED(ID3D11Device_CreateTexture2D(g->dev, &td, NULL, r_tex)) &&
         SUCCEEDED(ID3D11Device_CreateShaderResourceView(g->dev, (ID3D11Resource *)*r_tex, NULL,
                                                         r_srv));
}

int bui_gpu_slug_on(BuiGpu *g)
{
  if (g->slug == 0) {
    g->slug = -1;
    if (slug_tex(g, DXGI_FORMAT_R32G32B32A32_FLOAT, &g->curve_tex, &g->curve_srv) &&
        slug_tex(g, DXGI_FORMAT_R32G32_UINT, &g->band_tex, &g->band_srv) &&
        (g->slug_map = (SlugGlyph *)calloc(SLUG_MAP, sizeof(SlugGlyph))) != NULL)
    {
      g->slug = 1;
    }
  }
  return g->slug > 0;
}

static void slug_reset(BuiGpu *g)
{
  flush(g); /* recorded instances still point at the old data */
  memset(g->slug_map, 0, sizeof(SlugGlyph) * SLUG_MAP);
  g->slug_n = g->curve_next = g->band_next = 0;
}

/* Uploads `count` texels at linear index `at`, row by row. */
static void slug_upload(BuiGpu *g, ID3D11Texture2D *tex, int at, const void *data, int count,
                        int texel)
{
  const char *p = (const char *)data;
  while (count > 0) {
    const int x = at % SLUG_W, y = at / SLUG_W;
    const int n = count < SLUG_W - x ? count : SLUG_W - x;
    D3D11_BOX box = {(UINT)x, (UINT)y, 0, (UINT)(x + n), (UINT)(y + 1), 1};
    ID3D11DeviceContext_UpdateSubresource(g->ctx, (ID3D11Resource *)tex, 0, &box, p,
                                          (UINT)(n * texel), 0);
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

/* Builds and uploads a glyph's curve and band data. */
static int slug_build(BuiGpu *g, SlugGlyph *sg, const float *cv, int n, const float bb[4])
{
  const int nb = n < 8 ? 1 : (n / 4 < SLUG_MAX_BANDS ? n / 4 : SLUG_MAX_BANDS);
  const float w = bb[2] - bb[0] > 1.0f ? bb[2] - bb[0] : 1.0f;
  const float h = bb[3] - bb[1] > 1.0f ? bb[3] - bb[1] : 1.0f;
  /* horizontal bands split y, vertical bands split x */
  const float hs = (float)nb / h, vs = (float)nb / w;
  const float eps = 1.0f / 1024.0f * (w > h ? w : h);

  /* worst case every curve lands in every band */
  const int max_entries = 2 * nb * n;
  const int need_band = 2 * nb + max_entries, need_curve = 2 * n;
  if (need_curve > SLUG_W * SLUG_H / 4 || need_band > SLUG_W * SLUG_H / 4) {
    return 0; /* absurd glyph */
  }
  if (g->curve_next + need_curve > SLUG_W * SLUG_H || g->band_next + need_band > SLUG_W * SLUG_H ||
      g->slug_n >= SLUG_MAP_FILL)
  {
    return -1; /* full: caller resets and retries */
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
  const int cbase = g->curve_next; /* even: a curve never straddles a row */
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
  int at = 2 * nb; /* entries follow the 2 * nb headers */
  for (int dir = 0; dir < 2; dir++) {
    for (int b = 0; b < nb; b++) {
      /* band range in font units */
      const float s = dir == 0 ? hs : vs, o = dir == 0 ? bb[1] : bb[0];
      const float lo = o + (float)b / s - eps, hi = o + (float)(b + 1) / s + eps;
      int m = 0;
      for (int i = 0; i < n; i++) {
        const float *q = cv + 6 * i;
        /* dir 0 (horizontal rays) uses y extents, sorted by max x; dir 1 the reverse */
        const float a0 = dir == 0 ? q[1] : q[0], a1 = dir == 0 ? q[3] : q[2],
                    a2 = dir == 0 ? q[5] : q[4];
        const float amin = min3(a0, a1, a2), amax = max3(a0, a1, a2);
        if (amax < lo || amin > hi || (a0 == a1 && a1 == a2)) {
          continue; /* outside, or parallel to the ray: never crosses it */
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
  slug_upload(g, g->curve_tex, cbase, ct, 2 * n, 16);
  slug_upload(g, g->band_tex, bbase, bt, at, 8);
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
  if (gen != g->slug_gen) { /* faces were freed: ids may be reused */
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

int bui_gpu_image(BuiGpu *g, const uint32_t *argb, int w, int h, int *r_x, int *r_y)
{
  if (!g->rgba.tex && !atlas_create(g, &g->rgba, 2048, 4)) {
    return 0;
  }
  if (w + 1 > g->rgba.size || h + 1 > g->rgba.size) {
    return 0; /* TODO: images larger than the atlas */
  }
  /* content hash: an image updated in place gets a new entry */
  uint64_t hsh = 1469598103934665603ull;
  const size_t n = (size_t)w * h;
  for (size_t i = 0; i < n; i++) {
    hsh = (hsh ^ argb[i]) * 1099511628211ull;
  }
  const uint64_t k1 = hsh | 1, k2 = ((uint64_t)w << 32) | (uint32_t)h;
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
    return bui_gpu_image(g, argb, w, h, r_x, r_y);
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

void bui_gpu_damage(BuiGpu *g, int x, int y, int w, int h)
{
  if (w <= 0 || h <= 0 || g->n_dmg == (int)(sizeof(g->dmg) / sizeof(g->dmg[0]))) {
    g->dmg_full = 1;
    return;
  }
  RECT *r = &g->dmg[g->n_dmg++];
  r->left = x;
  r->top = y;
  r->right = x + w;
  r->bottom = y + h;
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
  REL(g->rtv);
  REL(g->target);
  D3D11_TEXTURE2D_DESC td = {0};
  td.Width = (UINT)g->w;
  td.Height = (UINT)g->h;
  td.MipLevels = td.ArraySize = 1;
  td.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
  td.SampleDesc.Count = 1;
  td.Usage = D3D11_USAGE_DEFAULT;
  td.BindFlags = D3D11_BIND_RENDER_TARGET;
  if (FAILED(ID3D11Device_CreateTexture2D(g->dev, &td, NULL, &g->target)) ||
      FAILED(ID3D11Device_CreateRenderTargetView(g->dev, (ID3D11Resource *)g->target, NULL,
                                                 &g->rtv)))
  {
    REL(g->target);
    return 0;
  }
  g->tw = g->w;
  g->th = g->h;
  const float black[4] = {0, 0, 0, 1};
  ID3D11DeviceContext_ClearRenderTargetView(g->ctx, g->rtv, black);
  g->dmg_full = 1;
  return 1;
}

static int ensure_buffer(BuiGpu *g, ID3D11Buffer **buf, UINT *cap, UINT need, UINT stride)
{
  if (*buf && *cap >= need) {
    return 1;
  }
  REL(*buf);
  UINT c = *cap ? *cap : 4096;
  while (c < need) {
    c *= 2;
  }
  D3D11_BUFFER_DESC bd = {0};
  bd.ByteWidth = c * stride;
  bd.Usage = D3D11_USAGE_DYNAMIC;
  bd.BindFlags = D3D11_BIND_VERTEX_BUFFER;
  bd.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
  if (FAILED(ID3D11Device_CreateBuffer(g->dev, &bd, NULL, buf))) {
    *cap = 0;
    return 0;
  }
  *cap = c;
  return 1;
}

static void upload(BuiGpu *g, ID3D11Buffer *buf, const void *data, size_t bytes)
{
  D3D11_MAPPED_SUBRESOURCE m;
  if (SUCCEEDED(ID3D11DeviceContext_Map(g->ctx, (ID3D11Resource *)buf, 0, D3D11_MAP_WRITE_DISCARD,
                                        0, &m)))
  {
    memcpy(m.pData, data, bytes);
    ID3D11DeviceContext_Unmap(g->ctx, (ID3D11Resource *)buf, 0);
  }
}

static void flush(BuiGpu *g)
{
  if (g->n_b == 0) {
    return;
  }
  if (!ensure_target(g)) {
    g->n_b = g->n_inst = g->n_tv = 0;
    return;
  }
  ID3D11DeviceContext *c = g->ctx;
  if (g->n_inst && ensure_buffer(g, &g->ib, &g->ib_cap, (UINT)g->n_inst, sizeof(BuiGpuInst))) {
    upload(g, g->ib, g->inst, sizeof(BuiGpuInst) * (size_t)g->n_inst);
  }
  if (g->n_tv && ensure_buffer(g, &g->tb, &g->tb_cap, (UINT)g->n_tv, sizeof(TriV))) {
    upload(g, g->tb, g->tv, sizeof(TriV) * (size_t)g->n_tv);
  }
  const float cbd[4] = {2.0f / (float)g->tw, 2.0f / (float)g->th, 0, 0};
  upload(g, g->cb, cbd, sizeof(cbd));

  ID3D11DeviceContext_OMSetRenderTargets(c, 1, &g->rtv, NULL);
  const float bf[4] = {0, 0, 0, 0};
  ID3D11DeviceContext_OMSetBlendState(c, g->blend, bf, 0xffffffffu);
  D3D11_VIEWPORT vp = {0, 0, (float)g->tw, (float)g->th, 0, 1};
  ID3D11DeviceContext_RSSetViewports(c, 1, &vp);
  ID3D11DeviceContext_RSSetState(c, g->rs);
  ID3D11DeviceContext_VSSetConstantBuffers(c, 0, 1, &g->cb);
  ID3D11ShaderResourceView *srv[4] = {g->mask.srv, g->rgba.srv, g->curve_srv, g->band_srv};
  ID3D11DeviceContext_PSSetShaderResources(c, 0, 4, srv);

  int cur = -1;
  for (int i = 0; i < g->n_b; i++) {
    const Batch *b = &g->b[i];
    if (b->count == 0) {
      continue;
    }
    if (b->tri != cur) {
      cur = b->tri;
      UINT off = 0;
      if (!b->tri) {
        UINT stride = sizeof(BuiGpuInst);
        ID3D11DeviceContext_IASetInputLayout(c, g->il_quad);
        ID3D11DeviceContext_IASetVertexBuffers(c, 0, 1, &g->ib, &stride, &off);
        ID3D11DeviceContext_IASetPrimitiveTopology(c, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
        ID3D11DeviceContext_VSSetShader(c, g->vs_quad, NULL, 0);
        ID3D11DeviceContext_PSSetShader(c, g->ps_quad, NULL, 0);
      }
      else {
        UINT stride = sizeof(TriV);
        ID3D11DeviceContext_IASetInputLayout(c, g->il_tri);
        ID3D11DeviceContext_IASetVertexBuffers(c, 0, 1, &g->tb, &stride, &off);
        ID3D11DeviceContext_IASetPrimitiveTopology(c, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        ID3D11DeviceContext_VSSetShader(c, g->vs_tri, NULL, 0);
        ID3D11DeviceContext_PSSetShader(c, g->ps_tri, NULL, 0);
      }
    }
    if (!b->tri) {
      ID3D11DeviceContext_DrawInstanced(c, 4, (UINT)b->count, 0, (UINT)b->first);
    }
    else {
      ID3D11DeviceContext_Draw(c, (UINT)b->count, (UINT)b->first);
    }
  }
  g->n_b = g->n_inst = g->n_tv = 0;
}

static int ensure_swap(BuiGpu *g)
{
  if (g->swap && g->sw == g->w && g->sh == g->h) {
    return 1;
  }
  if (g->swap) {
    ID3D11DeviceContext_OMSetRenderTargets(g->ctx, 0, NULL, NULL);
    if (FAILED(IDXGISwapChain1_ResizeBuffers(g->swap, 0, (UINT)g->w, (UINT)g->h,
                                             DXGI_FORMAT_UNKNOWN, 0)))
    {
      return 0;
    }
  }
  else {
    DXGI_SWAP_CHAIN_DESC1 sd = {0};
    sd.Width = (UINT)g->w;
    sd.Height = (UINT)g->h;
    sd.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    sd.SampleDesc.Count = 1;
    sd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    sd.BufferCount = 2;
    sd.Scaling = DXGI_SCALING_NONE;
    sd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL;
    sd.AlphaMode = DXGI_ALPHA_MODE_IGNORE;
    if (FAILED(IDXGIFactory2_CreateSwapChainForHwnd(g->factory, (IUnknown *)g->dev, g->hwnd, &sd,
                                                    NULL, NULL, &g->swap)))
    {
      return 0;
    }
    IDXGIFactory2_MakeWindowAssociation(g->factory, g->hwnd, DXGI_MWA_NO_ALT_ENTER);
  }
  g->sw = g->w;
  g->sh = g->h;
  g->swap_fresh = 1;
  return 1;
}

int bui_gpu_present(BuiGpu *g, int full)
{
  flush(g);
  if (!g->target || !ensure_swap(g)) {
    g->n_dmg = g->dmg_full = 0;
    return g->target != NULL;
  }
  ID3D11Texture2D *back = NULL;
  if (FAILED(IDXGISwapChain1_GetBuffer(g->swap, 0, &k_IID_ID3D11Texture2D, (void **)&back))) {
    return 0;
  }
  /* the flip model hands back an older buffer: copy the whole frame */
  ID3D11DeviceContext_CopyResource(g->ctx, (ID3D11Resource *)back, (ID3D11Resource *)g->target);
  REL(back);
  DXGI_PRESENT_PARAMETERS pp = {0};
  if (!(full || g->dmg_full || g->swap_fresh) && g->n_dmg > 0) {
    for (int i = 0; i < g->n_dmg; i++) { /* clamp: DXGI rejects rects outside the buffer */
      RECT *r = &g->dmg[i];
      r->left = r->left < 0 ? 0 : r->left;
      r->top = r->top < 0 ? 0 : r->top;
      r->right = r->right > g->w ? g->w : r->right;
      r->bottom = r->bottom > g->h ? g->h : r->bottom;
      if (r->right <= r->left || r->bottom <= r->top) {
        *r = g->dmg[--g->n_dmg];
        i--;
      }
    }
    pp.DirtyRectsCount = (UINT)g->n_dmg;
    pp.pDirtyRects = g->n_dmg ? g->dmg : NULL;
  }
  const HRESULT hr = IDXGISwapChain1_Present1(g->swap, 0, 0, &pp);
  g->swap_fresh = 0;
  g->n_dmg = g->dmg_full = 0;
  return !(hr == DXGI_ERROR_DEVICE_REMOVED || hr == DXGI_ERROR_DEVICE_RESET);
}

int bui_gpu_read(BuiGpu *g, uint32_t *dst, int w, int h)
{
  flush(g);
  if (!g->target || g->tw != w || g->th != h) {
    return 0;
  }
  D3D11_TEXTURE2D_DESC td = {0};
  td.Width = (UINT)w;
  td.Height = (UINT)h;
  td.MipLevels = td.ArraySize = 1;
  td.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
  td.SampleDesc.Count = 1;
  td.Usage = D3D11_USAGE_STAGING;
  td.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
  ID3D11Texture2D *st = NULL;
  if (FAILED(ID3D11Device_CreateTexture2D(g->dev, &td, NULL, &st))) {
    return 0;
  }
  ID3D11DeviceContext_CopyResource(g->ctx, (ID3D11Resource *)st, (ID3D11Resource *)g->target);
  D3D11_MAPPED_SUBRESOURCE m;
  int ok = 0;
  if (SUCCEEDED(ID3D11DeviceContext_Map(g->ctx, (ID3D11Resource *)st, 0, D3D11_MAP_READ, 0, &m))) {
    for (int y = 0; y < h; y++) {
      const uint32_t *src = (const uint32_t *)((const uint8_t *)m.pData + (size_t)y * m.RowPitch);
      for (int x = 0; x < w; x++) {
        dst[(size_t)y * w + x] = src[x] & 0xFFFFFFu; /* 0x00RRGGBB like the raster */
      }
    }
    ID3D11DeviceContext_Unmap(g->ctx, (ID3D11Resource *)st, 0);
    ok = 1;
  }
  REL(st);
  return ok;
}

/** \} */

#else /* !_WIN32: no GPU backend yet; every raster stays in software */

#  include "gpu_d3d11.h"

int bui_gpu_slug_on(BuiGpu *g)
{
  (void)g;
  return 0;
}
int bui_gpu_slug_glyph(BuiGpu *g, int face, int glyph, unsigned gen, float r_bbox[4],
                       float r_band[4], int r_loc[4])
{
  (void)g;
  (void)face;
  (void)glyph;
  (void)gen;
  (void)r_bbox;
  (void)r_band;
  (void)r_loc;
  return 0;
}

#  include <stddef.h>

#  include "gpu_d3d11.h"

BuiGpu *bui_gpu_create(void *hwnd)
{
  (void)hwnd;
  return NULL;
}
void bui_gpu_free(BuiGpu *g)
{
  (void)g;
}
int bui_gpu_state(BuiGpu *g, int wait)
{
  (void)g;
  (void)wait;
  return -1;
}
void bui_gpu_begin(BuiGpu *g, int w, int h)
{
  (void)g;
  (void)w;
  (void)h;
}
BuiGpuInst *bui_gpu_inst(BuiGpu *g, int kind, int x0, int y0, int x1, int y1)
{
  (void)g;
  (void)kind;
  (void)x0;
  (void)y0;
  (void)x1;
  (void)y1;
  return NULL;
}
int bui_gpu_mask(BuiGpu *g, const void *key, unsigned gen, const uint8_t *mask, int w, int h,
                 int *r_x, int *r_y)
{
  (void)g;
  (void)key;
  (void)gen;
  (void)mask;
  (void)w;
  (void)h;
  (void)r_x;
  (void)r_y;
  return 0;
}
int bui_gpu_image(BuiGpu *g, const uint32_t *argb, int w, int h, int *r_x, int *r_y)
{
  (void)g;
  (void)argb;
  (void)w;
  (void)h;
  (void)r_x;
  (void)r_y;
  return 0;
}
void bui_gpu_triangles(
    BuiGpu *g, const float (*co)[2], const float (*col)[4], int count, const int clip[4])
{
  (void)g;
  (void)co;
  (void)col;
  (void)count;
  (void)clip;
}
void bui_gpu_damage(BuiGpu *g, int x, int y, int w, int h)
{
  (void)g;
  (void)x;
  (void)y;
  (void)w;
  (void)h;
}
int bui_gpu_present(BuiGpu *g, int full)
{
  (void)g;
  (void)full;
  return 0;
}
int bui_gpu_read(BuiGpu *g, uint32_t *dst, int w, int h)
{
  (void)g;
  (void)dst;
  (void)w;
  (void)h;
  return 0;
}

#endif /* _WIN32 */
