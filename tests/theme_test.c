/* Themes: BK_set_theme switches between Blender's Dark and Light presets,
 * every context redraws, and switching back to dark gives the same pixels.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "BUI_kit.h"
#include "GPU_raster.h"

static int g_fail = 0;
static int g_requests = 0;
#define CHECK(c) \
  do { \
    if (!(c)) { \
      printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #c); \
      g_fail = 1; \
    } \
  } while (0)

static void on_request(void *data)
{
  (void)data;
  g_requests++;
}

static int near(const float c[4], float r, float g, float b, float a)
{
  return fabsf(c[0] - r) < 1e-3f && fabsf(c[1] - g) < 1e-3f && fabsf(c[2] - b) < 1e-3f &&
         fabsf(c[3] - a) < 1e-3f;
}

static uint32_t *render(BKContext *ctx, BuiRaster *r, int w, int h)
{
  BK_render(ctx, r, w, h, 1);
  uint32_t *copy = (uint32_t *)malloc(sizeof(uint32_t) * (size_t)(w * h));
  memcpy(copy, BUI_raster_pixels(r), sizeof(uint32_t) * (size_t)(w * h));
  return copy;
}

int main(void)
{
  const int W = 240, H = 160;
  float c[4];

  /* the default is Blender's dark theme */
  CHECK(strcmp(BK_theme_name(), "dark") == 0);
  CHECK(BK_theme_color("region_bg", c) && near(c, 48 / 255.0f, 48 / 255.0f, 48 / 255.0f, 1));
  CHECK(BK_theme_color("regular.inner", c) && near(c, 84 / 255.0f, 84 / 255.0f, 84 / 255.0f, 1));
  CHECK(BK_theme_color("text", c) && near(c, 230 / 255.0f, 230 / 255.0f, 230 / 255.0f, 1));
  CHECK(!BK_theme_color("nope", c));
  CHECK(!BK_theme_color("regular.nope", c));
  CHECK(!BK_theme_color("nope.inner", c));

  BKNode *col = BK_node_new("column");
  const char *types[] = {"button", "number", "checkbox", "text", "slider", "dropdown"};
  for (int i = 0; i < 6; i++) {
    BKNode *n = BK_node_new(types[i]);
    BK_set_str(n, "text", "Widget");
    BK_node_add(col, n, -1);
  }
  BKContext *ctx = BK_context_new(col);
  BKHost host = {0};
  host.request = on_request;
  BK_context_set_host(ctx, &host);
  BuiRaster *r = BUI_raster_create(W, H);

  uint32_t *dark = render(ctx, r, W, H);
  CHECK((dark[(H - 2) * W + W / 2] & 0xffffffu) == 0x303030u); /* region background */
  CHECK(BK_render(ctx, r, W, H, 0) == 0);                      /* nothing to redraw */

  /* light: Blender_Light.xml */
  CHECK(BK_set_theme("light") == 1);
  CHECK(strcmp(BK_theme_name(), "light") == 0);
  CHECK(g_requests >= 1); /* the host was asked to redraw */
  CHECK(BK_theme_color("region_bg", c) && near(c, 179 / 255.0f, 179 / 255.0f, 179 / 255.0f, 1));
  CHECK(BK_theme_color("regular.inner", c) && near(c, 219 / 255.0f, 219 / 255.0f, 219 / 255.0f, 1));
  CHECK(BK_theme_color("text", c) && near(c, 0, 0, 0, 1));
  CHECK(BK_render(ctx, r, W, H, 0) != 0); /* the theme change damaged the frame */
  uint32_t *light = render(ctx, r, W, H);
  CHECK((light[(H - 2) * W + W / 2] & 0xffffffu) == 0xb3b3b3u);
  int differ = 0;
  for (int i = 0; i < W * H; i++) {
    differ += dark[i] != light[i];
  }
  CHECK(differ > W * H / 2);

  /* unknown names keep the theme; Blender's preset names and any case work */
  CHECK(BK_set_theme("solarized") == 0);
  CHECK(strcmp(BK_theme_name(), "light") == 0);
  CHECK(BK_set_theme("Blender Dark") == 1);
  CHECK(strcmp(BK_theme_name(), "dark") == 0);
  CHECK(BK_set_theme("LIGHT") == 1 && BK_set_theme(NULL) == 1); /* NULL: dark */
  CHECK(strcmp(BK_theme_name(), "dark") == 0);

  /* back to dark: pixel-identical to the first frame */
  uint32_t *dark2 = render(ctx, r, W, H);
  CHECK(memcmp(dark, dark2, sizeof(uint32_t) * (size_t)(W * H)) == 0);

  free(dark);
  free(light);
  free(dark2);
  BUI_raster_free(r);
  BK_context_free(ctx);
  BK_node_free(col);
  if (!g_fail) {
    printf("theme tests passed\n");
  }
  return g_fail;
}
