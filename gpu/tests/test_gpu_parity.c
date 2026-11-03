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
#define H 360

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

int main(void)
{
  BuiRaster *cpu = BUI_raster_create(W, H);
  BuiRaster *gpu = BUI_raster_create(W, H);
  if (!BUI_raster_gpu_attach(gpu, NULL, 1)) {
    printf("gpu parity: no GPU device, skipped\n");
    return 0;
  }
  scene(cpu, 0);
  scene(gpu, 1);
  const uint32_t *a = BUI_raster_pixels(cpu);
  static uint32_t b[W * H];
  memcpy(b, BUI_raster_pixels(gpu), sizeof(b));
  int bad = 0, maxd = 0;
  static uint32_t diff[W * H];
  for (int i = 0; i < W * H; i++) {
    int d = 0;
    for (int s = 0; s < 24; s += 8) {
      const int da = abs((int)((a[i] >> s) & 0xFF) - (int)((b[i] >> s) & 0xFF));
      d = da > d ? da : d;
    }
    maxd = d > maxd ? d : maxd;
    bad += d > 3;
    const unsigned v = d > 255 / 8 ? 255u : (unsigned)d * 8u;
    diff[i] = (v << 16) | (v << 8) | v;
  }
  const char *dir = getenv("BUI_PARITY_PPM");
  if (dir) {
    ppm(dir, "parity_cpu", a);
    ppm(dir, "parity_gpu", b);
    ppm(dir, "parity_diff", diff);
  }
  printf("gpu parity: %d of %d pixels differ by more than 3 (max %d)\n", bad, W * H, maxd);
  BUI_raster_free(cpu);
  BUI_raster_free(gpu);
  return bad > W * H / 500 ? 1 : 0; /* < 0.2% (edge rounding) */
}
