/* Input methods: the kit's IME model (Blender's wmIMEData handling for text buttons) driven by
 * synthetic composition events, the way a window host feeds it. No real IME is needed.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "BUI_kit.h"
#include "GPU_raster.h"

static int g_fail = 0;
#define CHECK(c) \
  do { \
    if (!(c)) { \
      printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #c); \
      g_fail = 1; \
    } \
  } while (0)

enum { W = 320, H = 260 };

static BKContext *g_ctx;
static BuiRaster *g_raster;

static void draw(void)
{
  BK_render(g_ctx, g_raster, W, H, 1);
}

static void click(BKNode *n, int fx)
{
  int r[4];
  BK_node_rect(n, r);
  const int x = r[0] + r[2] * fx / 100, y = r[1] + r[3] / 2;
  BK_press(g_ctx, x, y, 0);
  BK_release(g_ctx, x, y, 0);
  draw();
}

static void ime(int type, const char *result, const char *composite, int cursor, int s0, int s1)
{
  BKImeEvent e = {result, composite, cursor, s0, s1};
  BK_ime_event(g_ctx, type, &e);
  draw();
}

static uint32_t checksum(void)
{
  const uint32_t *p = BUI_raster_pixels(g_raster);
  uint32_t h = 2166136261u;
  for (int i = 0; i < W * H; i++) {
    h = (h ^ p[i]) * 16777619u;
  }
  return h;
}

int main(void)
{
  BKNode *col = BK_node_new("column");
  BKNode *text = BK_node_new("text");
  BK_set_str(text, "value", "ab");
  BKNode *num = BK_node_new("number");
  BKNode *pass = BK_node_new("text");
  BK_set_str(pass, "value", "secret");
  BK_set_str(pass, "mask", "*");
  BKNode *area = BK_node_new("textarea");
  BK_set_str(area, "value", "hello");
  BK_node_add(col, text, -1);
  BK_node_add(col, num, -1);
  BK_node_add(col, pass, -1);
  BK_node_add(col, area, -1);
  g_ctx = BK_context_new(col);
  g_raster = BUI_raster_create(W, H);
  draw();

  BKImeState s;
  /* nothing is being edited: the IME stays off and the host has nothing to tell the platform */
  CHECK(BK_ime_state(g_ctx, &s) == 0 && !s.enabled);

  /* -- a text field -------------------------------------------------------------------- */
  click(text, 95);
  CHECK(BK_ime_state(g_ctx, &s) == 1);
  CHECK(s.enabled && s.restart);
  int r[4];
  BK_node_rect(text, r);
  CHECK(s.y == r[1] + r[3] + 1);                    /* just below the field (Blender: y - 4) */
  CHECK(s.x > r[0] && s.x < r[0] + r[2]);           /* at the caret, 5px to its right */
  CHECK(BK_ime_state(g_ctx, &s) == 0);              /* unchanged until the caret moves */
  const int x0 = s.x;
  const uint32_t before = checksum();

  ime(BK_IME_COMPOSITION_START, "", "", -1, -1, -1);
  CHECK(BK_ime_composing(g_ctx) == 1);
  ime(BK_IME_COMPOSITION, "", "\xE3\x81\xAB\xE3\x81\xBB", 6, 0, 6); /* "にほ", all converting */
  CHECK(checksum() != before);                      /* the composite is drawn, underlined */
  CHECK(BK_ime_state(g_ctx, &s) == 1 && s.enabled && !s.restart);
  CHECK(s.x > x0);                                  /* the candidate window follows the caret */

  /* typed text and Escape belong to the IME while it composes */
  BK_key(g_ctx, "z", "z", 0);
  BK_key(g_ctx, "", "Escape", 0);
  CHECK(BK_ime_composing(g_ctx) == 1);
  CHECK(BK_ime_state(g_ctx, &s) == 0 && s.enabled);

  /* the result is typed in, the composite is gone */
  ime(BK_IME_COMPOSITION, "\xE6\x97\xA5\xE6\x9C\xAC", "", -1, -1, -1); /* "日本" */
  ime(BK_IME_COMPOSITION_END, "", "", -1, -1, -1);
  CHECK(BK_ime_composing(g_ctx) == 0);
  BK_key(g_ctx, "", "Return", 0);
  CHECK(strcmp(BK_get_str(text, "value"), "ab\xE6\x97\xA5\xE6\x9C\xAC") == 0);
  CHECK(BK_ime_state(g_ctx, &s) == 1 && !s.enabled); /* editing ended: the IME is off again */

  /* -- a composition that is still going when the edit ends is kept ----------------------- */
  click(text, 95);
  BK_ime_state(g_ctx, &s);
  ime(BK_IME_COMPOSITION_START, "", "", -1, -1, -1);
  ime(BK_IME_COMPOSITION, "", "\xE3\x81\x82", 3, -1, -1); /* "あ" */
  click(area, 50);                                        /* a click outside */
  CHECK(strcmp(BK_get_str(text, "value"), "ab\xE6\x97\xA5\xE6\x9C\xAC\xE3\x81\x82") == 0);

  /* -- a text area ------------------------------------------------------------------------ */
  CHECK(BK_ime_state(g_ctx, &s) == 1 && s.enabled && s.restart); /* another field took over */
  BK_textarea_set_selection(area, 0, 2); /* a selection is replaced when composing starts */
  ime(BK_IME_COMPOSITION_START, "", "", -1, -1, -1);
  CHECK(strcmp(BK_get_str(area, "value"), "llo") == 0);
  ime(BK_IME_COMPOSITION, "\xE7\xAA\x93", "", -1, -1, -1); /* "窓" */
  ime(BK_IME_COMPOSITION_END, "", "", -1, -1, -1);
  CHECK(strstr(BK_get_str(area, "value"), "\xE7\xAA\x93") != NULL);

  /* -- fields that take no IME ------------------------------------------------------------- */
  click(num, 50); /* Blender's textedit_ime_begin skips number buttons */
  BK_ime_state(g_ctx, &s);
  CHECK(!s.enabled);
  BK_key(g_ctx, "", "Escape", 0);
  click(pass, 50); /* a password never shows its composition */
  BK_ime_state(g_ctx, &s);
  CHECK(!s.enabled);
  ime(BK_IME_COMPOSITION_START, "", "", -1, -1, -1); /* ignored */
  CHECK(BK_ime_composing(g_ctx) == 0);

  BUI_raster_free(g_raster);
  BK_context_free(g_ctx);
  if (!g_fail) {
    printf("ime ok\n");
  }
  return g_fail;
}
