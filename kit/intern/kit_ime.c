/* Input methods (IME), the platform independent half.
 *
 * Blender: the text button's IME handling (interface_handlers.cc textedit_ime_*, WM_IME_COMPOSITE_*),
 * its drawing (interface_widgets.cc widget_draw_text_ime_underline) and the window manager's
 * IME owner (wm_window.cc WM_window_IME_*). A window host (kit_ime_win32.c) turns the platform's
 * composition messages into BK_ime_event calls and asks BK_ime_state what to tell the platform.
 *
 * Nothing here runs unless a host calls BK_ime_event / BK_ime_state, so hosts without an IME
 * (X11, Cocoa for now, headless) are unaffected.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "kit_intern.h"

/* -------------------------------------------------------------------- */
/** \name Composition state shared by every editable node
 * \{ */

void bk_ime_comp_free(BKImeComp *c)
{
  free(c->text);
  memset(c, 0, sizeof(*c));
}

void bk_ime_comp_set(BKImeComp *c, const BKImeEvent *e)
{
  const char *s = (e && e->composite) ? e->composite : "";
  const int n = (int)strlen(s);
  if (n + 1 > c->cap) { /* grown, never shrunk: a composition does not allocate per event */
    c->cap = n + 64;
    c->text = (char *)realloc(c->text, (size_t)c->cap);
  }
  if (c->cap) {
    memcpy(c->text, s, (size_t)n + 1);
  }
  c->len = n;
  c->cursor = (n && e) ? e->cursor_pos : -1;
  c->sel_start = (n && e) ? e->sel_start : -1;
  c->sel_end = (n && e) ? e->sel_end : -1;
}

float bk_ime_caret_offset(const BKImeComp *c, bool bold)
{
  /* Blender: the cursor moves by `cursor_pos` into the inserted composite */
  if (c->len == 0 || c->cursor < 0) {
    return 0.0f;
  }
  return bk_text_width(c->text, BK_MIN(c->cursor, c->len), bold);
}

void bk_ime_draw_underline(const BKImeComp *c, float x, float bottom, float clip_w,
                           const uchar color[4], bool bold)
{
  if (c->len == 0) {
    return;
  }
  const float y = bottom - 6.0f; /* rect->ymin + 6 * U.pixelsize, measured from the top here */
  const float w = BK_MIN(bk_text_width(c->text, c->len, bold), clip_w - 2.0f);
  bk_fill(floorf(x), y - 1.0f, w, 1.0f, color);
  if (c->sel_start >= 0 && c->sel_end > c->sel_start && c->sel_end <= c->len) {
    /* the thick line: the clause being converted */
    const float sx = x + bk_text_width(c->text, c->sel_start, bold);
    const float sw = bk_text_width(c->text + c->sel_start, c->sel_end - c->sel_start, bold);
    bk_fill(floorf(sx), y - 2.0f, BK_MIN(sw, clip_w - 2.0f), 2.0f, color);
  }
}

void bk_ime_set_candidate(BKImeComp *c, int x, int y)
{
  c->cand_x = x;
  c->cand_y = y;
  c->cand_valid = true;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name The node being edited
 * \{ */

BKImeComp *bk_ime_comp(BKNode *n)
{
  if (!n || !NODE_ENABLED(n)) {
    return NULL;
  }
  if (n->edit) {
    /* Blender's textedit_ime_begin skips number buttons; a password must not show the
     * composition in clear, and a hotkey field takes key codes, not text. */
    if (n->edit->mask[0] || n->cls == &bk_cls_number || n->cls == &bk_cls_slider ||
        n->cls == &bk_cls_hotkey)
    {
      return NULL;
    }
    return &n->edit->ime;
  }
  if (n->cls == &bk_cls_textarea) {
    return bk_textarea_ime(n);
  }
  return NULL;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Events from the window host (WM_IME_COMPOSITE_*)
 * \{ */

int BK_ime_event(BKContext *ctx, int type, const BKImeEvent *e)
{
  BKNode *f = ctx->focus;
  BKImeComp *c = bk_ime_comp(f);
  if (!c) {
    return 0;
  }
  static const BKImeEvent none = {"", "", -1, -1, -1};
  switch (type) {
    case BK_IME_COMPOSITION_START:
      c->composing = true;
      bk_ime_comp_set(c, &none);
      /* the composition replaces the selection (textedit_delete_selection) */
      if (f->edit) {
        bk_te_delete_selection(f->edit);
      }
      else {
        bk_textarea_delete_selection(f);
      }
      break;
    case BK_IME_COMPOSITION:
      bk_ime_comp_set(c, e);
      if (e && e->result && e->result[0]) {
        /* committed text goes in as typed text: the node's own key path inserts it, undoes it
         * and reports a live change like every other character */
        const bool was = c->composing;
        c->composing = false;
        BK_key(ctx, e->result, e->result, 0);
        c->composing = was;
      }
      break;
    case BK_IME_COMPOSITION_END:
      c->composing = false;
      bk_ime_comp_set(c, &none);
      break;
    default:
      return 0;
  }
  bk_invalidate(f);
  return 1;
}

void bk_ime_commit(BKContext *ctx)
{
  BKNode *f = ctx->focus;
  BKImeComp *c = bk_ime_comp(f);
  if (c && c->composing && c->len > 0) {
    char *s = (char *)malloc((size_t)c->len + 1);
    memcpy(s, c->text, (size_t)c->len + 1);
    c->composing = false;
    bk_ime_comp_set(c, &(BKImeEvent){"", "", -1, -1, -1});
    BK_key(ctx, s, s, 0);
    free(s);
  }
}

bool bk_ime_swallow_key(BKContext *ctx, const char *text, const char *keysym)
{
  const BKImeComp *c = bk_ime_comp(ctx->focus);
  return c && c->composing && ((text && text[0]) || bk_keysym_is(keysym, "Escape"));
}

int BK_ime_composing(BKContext *ctx)
{
  const BKImeComp *c = bk_ime_comp(ctx->focus);
  return c && c->composing;
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name What the host tells the platform
 * \{ */

int BK_ime_state(BKContext *ctx, BKImeState *state)
{
  BKNode *f = ctx->focus;
  const BKImeComp *c = bk_ime_comp(f);
  BKImeState s = {0, 0, 0, 0};
  if (c) {
    s.enabled = 1;
    s.restart = f != ctx->ime.node;
    /* before the first draw of a field: just below it */
    s.x = c->cand_valid ? c->cand_x : f->x;
    s.y = c->cand_valid ? c->cand_y : f->y + f->h + 1;
  }
  const int changed = s.enabled != ctx->ime.enabled || s.restart || s.x != ctx->ime.x ||
                      s.y != ctx->ime.y;
  ctx->ime.node = c ? f : NULL;
  ctx->ime.enabled = s.enabled;
  ctx->ime.x = s.x;
  ctx->ime.y = s.y;
  *state = s;
  return changed;
}

/** \} */
