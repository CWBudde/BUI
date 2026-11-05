/* GPU renderer parity: every raster primitive drawn by the software raster and
 * by the GPU backend must give the same pixels (within rounding). Skips (exit
 * 0) where no GPU device can be created. BUI_PARITY_PPM=dir writes both
 * frames and the difference for inspection.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "GPU_raster.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define W 480
#define H 440

/* Textured meshes (BUI_raster_mesh, the ImGui path) in the band y >= 360. */
static void meshes(BuiRaster *r)
{
  static int tex = 0, tex2 = 0;
  if (!tex) {
    /* 16 x 16 RGBA: a hard checker with alpha, plus a white texel at (0, 0)
     * like ImGui's atlas */
    static unsigned char px[16 * 16 * 4];
    for (int y = 0; y < 16; y++) {
      for (int x = 0; x < 16; x++) {
        unsigned char *p = px + (y * 16 + x) * 4;
        const int on = ((x / 4) + (y / 4)) & 1;
        p[0] = (unsigned char)(on ? 250 : 40);
        p[1] = (unsigned char)(x * 16);
        p[2] = (unsigned char)(y * 16);
        p[3] = (unsigned char)(on ? 255 : 120);
      }
    }
    tex = BUI_texture_create(16, 16, px, 0);
    /* an update after creation: GPUs must pick up the logged rectangle */
    tex2 = BUI_texture_create(8, 8, NULL, 0);
    unsigned char blk[4 * 4 * 4];
    for (int i = 0; i < 16; i++) {
      blk[i * 4 + 0] = 255;
      blk[i * 4 + 1] = 200;
      blk[i * 4 + 2] = 40;
      blk[i * 4 + 3] = 255;
    }
    BUI_texture_update(tex2, 2, 2, 4, 4, blk, 0);
  }
  /* a magnified textured quad (bilinear), 16-bit indices */
  const BuiMeshVertex q[4] = {
      {10, 365, 0, 0, 0xFFFFFFFFu}, {110, 365, 1, 0, 0xFFFFFFFFu},
      {110, 435, 1, 1, 0xFFFFFFFFu}, {10, 435, 0, 1, 0xFFFFFFFFu}};
  const uint16_t qi[6] = {0, 1, 2, 0, 2, 3};
  BUI_raster_mesh(r, q, 4, qi, 6, 0, tex, NULL, 0, 0);
  /* the same, nearest sampling, tinted, translated by the mesh offset */
  const BuiMeshVertex q2[4] = {
      {0, 0, 0, 0, 0xC0FFC080u}, {80, 0, 1, 0, 0xC0FFC080u},
      {80, 70.6f, 1, 1, 0xC0FFC080u}, {0, 70.6f, 0, 1, 0xC0FFC080u}};
  BUI_raster_mesh(r, q2, 4, qi, 6, BUI_MESH_NEAREST, tex, NULL, 120, 365);
  /* untextured gradient fan with an alpha fringe, 32-bit indices */
  const BuiMeshVertex fan[6] = {
      {250, 400, 0, 0, 0xFF2020F0u}, {290, 370, 0, 0, 0xFF20F020u},
      {300, 420, 0, 0, 0xFFF02020u}, {240, 432, 0, 0, 0x80FFFFFFu},
      {215, 395, 0, 0, 0x00FFFFFFu}, {251.5f, 366.25f, 0, 0, 0x40000000u}};
  const uint32_t fi[12] = {0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5};
  BUI_raster_mesh(r, fan, 6, fi, 12, BUI_MESH_INDEX32, 0, NULL, 0, 0);
  /* clipped (scissor at fractional coordinates), the updated texture */
  const BuiMeshVertex q3[4] = {
      {320, 365, 0, 0, 0xFFFFFFFFu}, {400, 365, 1, 0, 0xFFFFFFFFu},
      {400, 435, 1, 1, 0xFFFFFFFFu}, {320, 435, 0, 1, 0xFFFFFFFFu}};
  const float clip[4] = {330.6f, 372.2f, 390.9f, 425.5f};
  BUI_raster_mesh(r, q3, 4, qi, 6, 0, tex2, clip, 0, 0);
  /* sub-pixel text-like quads sharing edges (no seams or double blending) */
  BuiMeshVertex strip[8];
  uint16_t si[18];
  for (int i = 0; i < 4; i++) {
    const float x = 410.3f + i * 15.25f;
    strip[2 * i] = (BuiMeshVertex){x, 370.6f, 0.1f * i, 0.2f, 0x9030A0FFu};
    strip[2 * i + 1] = (BuiMeshVertex){x, 430.4f, 0.1f * i, 0.9f, 0x9030A0FFu};
  }
  for (int i = 0; i < 3; i++) {
    const uint16_t a = (uint16_t)(2 * i);
    const uint16_t t6[6] = {a, (uint16_t)(a + 2), (uint16_t)(a + 3), a, (uint16_t)(a + 3),
                            (uint16_t)(a + 1)};
    memcpy(si + 6 * i, t6, sizeof(t6));
  }
  BUI_raster_mesh(r, strip, 8, si, 18, 0, tex, NULL, 0, 0);
}

static void scene(BuiRaster *r, int pass)
{
  BUI_raster_begin_frame(r, W, H);
  const float bg[4] = {0.16f, 0.16f, 0.17f, 1};
  BUI_raster_fill_rect(r, 0, 0, W, H, bg);
  const float red[4] = {0.9f, 0.2f, 0.2f, 1}, blue[4] = {0.2f, 0.4f, 0.9f, 0.6f};
  const float white[4] = {1, 1, 1, 1}, green[4] = {0.3f, 0.85f, 0.4f, 0.8f};
  BUI_raster_fill_rect(r, 10.25f, 10.5f, 60.5f, 20.75f, red);
  BUI_raster_frame_rect(r, 80, 10, 60, 30, 2, white);
  BUI_raster_round_rect(r, 150.5f, 10, 70, 30, 8, blue);
  BUI_raster_circle(r, 250, 25, 14, 0, green);
  BUI_raster_circle(r, 290, 25, 14, 2.5f, white);
  BUI_raster_ellipse(r, 340, 25, 30, 12, 0, red);
  BUI_raster_ellipse(r, 410, 25, 30, 12, 1.5f, white);
  BUI_raster_line(r, 10, 60, 120, 95, 1, white);
  BUI_raster_line(r, 10, 100, 120, 100, 2, green);
  BUI_raster_line(r, 130, 60, 140, 110, 3, red);
  const float tri[3][2] = {{160, 60}, {220, 110}, {150, 110}};
  const float tcol[3][4] = {{1, 0, 0, 1}, {0, 1, 0, 1}, {0, 0, 1, 1}};
  BUI_raster_triangles(r, tri, tcol, 3);
  const float star[5][2] = {{270, 55}, {290, 110}, {240, 75}, {300, 75}, {250, 110}};
  BUI_raster_polygon(r, star, 5, blue);
  const float in1[4] = {0.35f, 0.35f, 0.35f, 1}, in2[4] = {0.25f, 0.25f, 0.25f, 1};
  const float out[4] = {0.1f, 0.1f, 0.1f, 1}, emb[4] = {1, 1, 1, 0.1f};
  BUI_raster_widget(r, 320, 60, 120, 24, 5, BUI_CORNER_ALL, in1, in2, out, 1, emb);
  BUI_raster_widget(r, 320, 90, 120, 24, 5, BUI_CORNER_TOP_LEFT | BUI_CORNER_BOTTOM_RIGHT, in1,
                    NULL, out, 2, NULL);
  BuiWidgetBase wb;
  memset(&wb, 0, sizeof(wb));
  wb.rect[0] = 10;
  wb.rect[1] = 130;
  wb.rect[2] = 140;
  wb.rect[3] = 22;
  wb.rad = 4;
  wb.radi = 3;
  wb.corners = BUI_CORNER_ALL;
  memcpy(wb.inner1, in1, sizeof(in1));
  memcpy(wb.inner2, in2, sizeof(in2));
  memcpy(wb.outline, out, sizeof(out));
  memcpy(wb.emboss, emb, sizeof(emb));
  wb.pixelsize = 1;
  BUI_raster_widget_base(r, &wb);
  wb.rect[0] = 160;
  wb.shade_horizontal = 1;
  BUI_raster_widget_base(r, &wb);
  wb.rect[0] = 310;
  wb.shade_horizontal = 0;
  wb.alpha_checker = 1;
  wb.inner1[3] = wb.inner2[3] = 0.5f;
  BUI_raster_widget_base(r, &wb);
  BUI_raster_shadow(r, 30, 180, 120, 60, 6, 12, 0.5f);
  const float panel[4] = {0.3f, 0.3f, 0.32f, 1};
  BUI_raster_round_rect(r, 30, 180, 120, 60, 6, panel);
  const float c0[4] = {1, 0, 0, 1}, c1[4] = {0, 1, 0, 1}, c2[4] = {0, 0, 1, 1}, c3[4] = {1, 1, 0, 0.5f};
  BUI_raster_gradient(r, 170, 180, 100, 60, c0, c1, c2, c3);
  BUI_raster_hsv_circle(r, 320, 210, 30, 0.9f);
  const float light[3] = {-0.5f, 0.6f, 0.8f}, sph[4] = {0.6f, 0.4f, 0.2f, 1};
  BUI_raster_sphere(r, 400, 210, 30, light, sph);
  uint32_t img[16 * 12];
  for (int i = 0; i < 16 * 12; i++) {
    img[i] = ((i % 16 + i / 16) & 1) ? 0xFFFF8000u : 0x80202080u;
  }
  BUI_raster_image(r, 20, 260, 64, 48, img, 16, 12, 0.9f);
  /* clipped primitives */
  BUI_raster_set_clip(r, 100, 260, 120, 40);
  BUI_raster_round_rect(r, 90, 250, 80, 60, 10, red);
  BUI_raster_circle(r, 200, 280, 25, 0, green);
  BUI_raster_triangles(r, tri, tcol, 3);
  BUI_raster_set_clip(r, 0, 0, 0, 0);
  if (BUI_text_native()) {
    BUI_raster_set_font(r, "", 0);
    BUI_raster_text(r, 240, 300, "GPU parity: Hello", 17, 16, white);
    BUI_raster_set_font(r, "", BUI_FONT_BOLD);
    BUI_raster_text(r, 240, 330, "Bold text 123", 13, 20, green);
  }
  meshes(r);
  (void)pass;
  BUI_raster_end_frame(r);
}

static void ppm(const char *dir, const char *name, const uint32_t *px)
{
  char path[512];
  snprintf(path, sizeof(path), "%s/%s.ppm", dir, name);
  FILE *f = fopen(path, "wb");
  if (!f) {
    return;
  }
  fprintf(f, "P6\n%d %d\n255\n", W, H);
  for (int i = 0; i < W * H; i++) {
    const unsigned char c[3] = {(unsigned char)(px[i] >> 16), (unsigned char)(px[i] >> 8),
                                (unsigned char)px[i]};
    fwrite(c, 1, 3, f);
  }
  fclose(f);
}

/* Compares one GPU frame against the CPU frame. `text_tol`: Slug computes
 * glyph coverage analytically instead of from stb's bitmaps, so text edges
 * may differ by a few levels of AA; only gross differences count there. */
static int compare(const uint32_t *a, const uint32_t *b, const char *label, int slug)
{
  int bad = 0, gross = 0, maxd = 0, mesh_bad = 0, mesh_max = 0;
  static uint32_t diff[W * H];
  for (int i = 0; i < W * H; i++) {
    int d = 0;
    for (int s = 0; s < 24; s += 8) {
      const int da = abs((int)((a[i] >> s) & 0xFF) - (int)((b[i] >> s) & 0xFF));
      d = da > d ? da : d;
    }
    maxd = d > maxd ? d : maxd;
    bad += d > 3;
    gross += d > 64;
    if (i / W >= 360) { /* the textured meshes */
      mesh_bad += d > 3;
      mesh_max = d > mesh_max ? d : mesh_max;
    }
    const unsigned v = d > 255 / 8 ? 255u : (unsigned)d * 8u;
    diff[i] = (v << 16) | (v << 8) | v;
  }
  const char *dir = getenv("BUI_PARITY_PPM");
  if (dir) {
    char name[64];
    snprintf(name, sizeof(name), "parity_gpu_%s", label);
    ppm(dir, name, b);
    snprintf(name, sizeof(name), "parity_diff_%s", label);
    ppm(dir, name, diff);
  }
  printf("gpu parity (%s): %d of %d pixels differ by more than 3, %d by more than 64 (max %d)\n",
         label, bad, W * H, gross, maxd);
  printf("gpu parity (%s): meshes: %d pixels differ by more than 3 (max %d)\n", label, mesh_bad,
         mesh_max);
  if (mesh_bad > W * 80 / 200) { /* > 0.5% of the mesh band */
    return 0;
  }
  if (slug) {
    return bad < W * H / 100 && gross < W * H / 2000; /* AA differences on text only */
  }
  return bad <= W * H / 500; /* < 0.2% (edge rounding) */
}

/* BUI_gpu_slug() reads BUI_SLUG once and caches it, so setting the variable
 * here would leave every pass on its first value: switch it directly. */
static void set_slug(int on)
{
  BUI_gpu_set_slug(on);
}

/* Meshes drawn through raster views over horizontal bands (what parallel
 * band drawing does) must give exactly the pixels of one serial draw. */
static int mesh_bands(void)
{
  const float bg[4] = {0.16f, 0.16f, 0.17f, 1};
  BuiRaster *serial = BUI_raster_create(W, H), *banded = BUI_raster_create(W, H);
  BUI_raster_begin_frame(serial, W, H);
  BUI_raster_fill_rect(serial, 0, 0, W, H, bg);
  meshes(serial);
  BUI_raster_end_frame(serial);
  BUI_raster_begin_frame(banded, W, H);
  BUI_raster_fill_rect(banded, 0, 0, W, H, bg);
  BuiRaster *view = (BuiRaster *)malloc(BUI_raster_view_size());
  for (int y = 0; y < H; y += 7) { /* odd band height: bands split triangles anywhere */
    BUI_raster_view(banded, view, 0, y, W, y + 7);
    meshes(view);
  }
  free(view);
  BUI_raster_end_frame(banded);
  const int same = memcmp(BUI_raster_pixels(serial), BUI_raster_pixels(banded),
                          sizeof(uint32_t) * W * H) == 0;
  printf("mesh bands: %s\n", same ? "identical to serial drawing" : "DIFFER from serial drawing");
  BUI_raster_free(serial);
  BUI_raster_free(banded);
  return same;
}

int main(void)
{
  BuiRaster *cpu = BUI_raster_create(W, H);
  scene(cpu, 0);
  const uint32_t *a = BUI_raster_pixels(cpu);
  if (getenv("BUI_PARITY_PPM")) {
    ppm(getenv("BUI_PARITY_PPM"), "parity_cpu", a);
  }
  int ok = mesh_bands();
  /* text from glyph bitmaps (exact), then from glyph outlines (Slug) */
  for (int slug = 0; slug < 2; slug++) {
    set_slug(slug);
    BuiRaster *gpu = BUI_raster_create(W, H);
    if (!BUI_raster_gpu_attach(gpu, NULL, 1)) {
      printf("gpu parity: no GPU device, skipped\n");
      BUI_raster_free(gpu);
      break;
    }
    scene(gpu, 1);
    static uint32_t b[W * H];
    memcpy(b, BUI_raster_pixels(gpu), sizeof(b));
    ok &= compare(a, b, slug ? "slug" : "atlas", slug);
    BUI_raster_free(gpu);
  }
  BUI_raster_free(cpu);
  return ok ? 0 : 1;
}
