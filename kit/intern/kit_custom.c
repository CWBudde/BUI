/* Custom nodes ("custom"): drawing and input implemented by the host through
 * BKCustomClass callbacks (BUI_kit.h). Used to embed other UI systems, e.g.
 * Dear ImGui (bindings/imgui/imgui_impl_bui.cpp).
 *
 * The node takes keyboard focus when clicked (ctx->focus, like a text area),
 * keeps the pointer grabbed while any button is held (kit_core.c routes every
 * motion and extra button to it), and asks for `frame` calls that BK_render
 * runs before drawing: right away, paced to the display (BK_animating), or
 * after a delay through the host's timer (BK_TIMER_CUSTOM).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "kit_intern.h"

typedef struct BKCustom {
  BKCustomClass cls;
  void *data;
  BKNode *node;
  int want_frame; /* frame at the next render */
  double wake_at; /* frame due at this bk_time() (0 = none) */
  int framed;     /* frame ran at least once */
  int last_w, last_h;
  struct BKCustom *next; /* every custom node (any context) */
} BKCustom;

static BKCustom *g_customs = NULL;


static BKCustom *cust(const BKNode *n)
{
  return BK_IS_CUSTOM(n) ? (BKCustom *)n->ext : NULL;
}

static void custom_init(BKNode *n)
{
  BKCustom *c = (BKCustom *)calloc(1, sizeof(BKCustom));
  c->node = n;
  c->next = g_customs;
  g_customs = c;
  n->ext = c;
}

static void custom_free(BKNode *n)
{
  BKCustom *c = cust(n);
  if (!c) {
    return;
  }
  if (c->cls.free) {
    c->cls.free(c->data, n);
  }
  for (BKCustom **p = &g_customs; *p; p = &(*p)->next) {
    if (*p == c) {
      *p = c->next;
      break;
    }
  }
  free(c);
  n->ext = NULL;
}

static void custom_measure(BKNode *n, int *r_w, int *r_h)
{
  const BKCustom *c = cust(n);
  *r_w = c && c->cls.natural_w > 0 ? c->cls.natural_w : 8 * bk_unit();
  *r_h = c && c->cls.natural_h > 0 ? c->cls.natural_h : 8 * bk_unit();
}

static void custom_draw(BKNode *n)
{
  BKCustom *c = cust(n);
  if (c && c->cls.draw && bk_draw.r) {
    const int rect[4] = {n->x, n->y, n->w, n->h};
    bk_clip_push(n->x, n->y, n->w, n->h);
    if (bk_draw.clip[2] > bk_draw.clip[0] && bk_draw.clip[3] > bk_draw.clip[1]) {
      c->cls.draw(c->data, n, bk_draw.r, rect);
    }
    bk_clip_pop();
  }
}

int bk_custom_event(BKNode *n, BKCustomEvent *e, int wx, int wy)
{
  BKCustom *c = cust(n);
  if (!c || !c->cls.event) {
    return 0;
  }
  e->x = (float)(wx - n->x);
  e->y = (float)(wy - n->y);
  if (!e->text) {
    e->text = "";
  }
  if (!e->keysym) {
    e->keysym = "";
  }
  return c->cls.event(c->data, n, e);
}

static int pointer(BKNode *n, int type, int button, const BKEvent *e, int clicks)
{
  BKCustomEvent ce;
  memset(&ce, 0, sizeof(ce));
  ce.type = type;
  ce.button = button;
  ce.mods = e->mods;
  ce.clicks = clicks;
  return bk_custom_event(n, &ce, e->x, e->y);
}

static void take_focus(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (!ctx || ctx->focus == n) {
    return;
  }
  /* in_BK_press already ended the previous focus's edit */
  ctx->focus = n;
  BKCustomEvent ce;
  memset(&ce, 0, sizeof(ce));
  ce.type = BK_CUSTOM_FOCUS;
  bk_custom_event(n, &ce, n->x, n->y);
}

static int press_button(BKNode *n)
{
  const BKContext *ctx = bk_ctx(n);
  return ctx && ctx->press_button > 0 ? ctx->press_button : 1;
}

static int custom_press(BKNode *n, BKEvent *e)
{
  take_focus(n);
  pointer(n, BK_CUSTOM_PRESS, press_button(n), e, 1);
  return 1;
}

static int custom_dbl(BKNode *n, BKEvent *e)
{
  take_focus(n);
  pointer(n, BK_CUSTOM_PRESS, press_button(n), e, 2);
  return 1;
}

static int custom_release(BKNode *n, BKEvent *e)
{
  pointer(n, BK_CUSTOM_RELEASE, press_button(n), e, 0);
  return 1;
}

static int custom_move(BKNode *n, BKEvent *e)
{
  pointer(n, BK_CUSTOM_MOTION, 0, e, 0);
  return 0;
}

static int custom_leave(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  if (ctx && ctx->grab == n) {
    return 0; /* still grabbed: motion keeps coming */
  }
  BKCustomEvent ce;
  memset(&ce, 0, sizeof(ce));
  ce.type = BK_CUSTOM_LEAVE;
  bk_custom_event(n, &ce, n->x - 100000, n->y - 100000);
  return 0;
}

static int custom_key(BKNode *n, BKEvent *e)
{
  BKCustomEvent ce;
  memset(&ce, 0, sizeof(ce));
  ce.type = BK_CUSTOM_KEY;
  ce.mods = e->mods;
  ce.text = e->text;
  ce.keysym = e->keysym;
  return bk_custom_event(n, &ce, e->x, e->y);
}

static int custom_wheel(BKNode *n, BKEvent *e)
{
  BKCustomEvent ce;
  memset(&ce, 0, sizeof(ce));
  ce.type = BK_CUSTOM_WHEEL;
  ce.mods = e->mods;
  if (e->wheel_x != 0.0f || e->wheel_y != 0.0f) {
    ce.wheel_x = e->wheel_x;
    ce.wheel_y = e->wheel_y;
  }
  else {
    ce.wheel_y = (float)e->delta;
  }
  bk_custom_event(n, &ce, e->x, e->y);
  return 1; /* never scroll the region under an embedded UI */
}

static void custom_finish_edit(BKNode *n, bool confirm)
{
  (void)confirm;
  BKCustomEvent ce;
  memset(&ce, 0, sizeof(ce));
  ce.type = BK_CUSTOM_BLUR;
  bk_custom_event(n, &ce, n->x, n->y);
}

static int custom_cursor_at(BKNode *n, int x, int y)
{
  (void)x;
  (void)y;
  return n->cursor_override > 0 ? n->cursor_override - 1 : BK_CURSOR_DEFAULT;
}

const BKClass bk_cls_custom = {.name = "custom",
                               .flags = CLS_INTERACTIVE | CLS_BUTTONS,
                               .init = custom_init,
                               .free_ext = custom_free,
                               .measure = custom_measure,
                               .draw = custom_draw,
                               .press = custom_press,
                               .dbl = custom_dbl,
                               .release = custom_release,
                               .move = custom_move,
                               .leave = custom_leave,
                               .key = custom_key,
                               .wheel = custom_wheel,
                               .cursor_at = custom_cursor_at,
                               .finish_edit = custom_finish_edit};

/* -- frames ------------------------------------------------------------------------------- */

/* Drawn by BK_render: in this context's tree (or a popup), visible, not empty. */
static bool live(const BKNode *n, BKContext *ctx)
{
  if (n->w <= 0 || n->h <= 0 || bk_ctx(n) != ctx) {
    return false;
  }
  for (const BKNode *p = n; p; p = p->parent) {
    if (!NODE_VISIBLE(p) || (p->cls == &bk_cls_panel && p != n && (p->flags & NF_COLLAPSED))) {
      return false;
    }
  }
  return true;
}

void bk_ctx_arm_timer(BKContext *ctx)
{
  /* the context's one frame timer: custom nodes' wakes and animated images */
  double due = bk_image_next_due(ctx);
  for (BKCustom *c = g_customs; c; c = c->next) {
    if (c->wake_at > 0.0 && bk_ctx(c->node) == ctx && (due == 0.0 || c->wake_at < due)) {
      due = c->wake_at;
    }
  }
  if (due == 0.0 || !ctx->host.schedule) {
    ctx->custom_timer_due = due;
    return;
  }
  if (ctx->custom_timer_due > 0.0 && ctx->custom_timer_due <= due &&
      ctx->custom_timer_due > bk_time())
  {
    return; /* an earlier timer is pending: it re-arms for this one */
  }
  ctx->custom_timer_due = due;
  const int ms = (int)ceil((due - bk_time()) * 1000.0);
  ctx->host.schedule(ctx->host.data, ms > 0 ? ms : 0, BK_TIMER_CUSTOM);
}

void bk_custom_timer(BKContext *ctx)
{
  ctx->custom_timer_due = 0.0;
  bk_image_tick(); /* animated images whose frame is due (they damage themselves) */
  const double now = bk_time() + 0.002; /* timers fire a little early */
  bool any = false;
  for (BKCustom *c = g_customs; c; c = c->next) {
    if (c->wake_at > 0.0 && c->wake_at <= now && bk_ctx(c->node) == ctx) {
      c->wake_at = 0.0;
      c->want_frame = 1;
      any = true;
    }
  }
  bk_ctx_arm_timer(ctx);
  if (any) {
    bk_ctx_request(ctx);
  }
}

void bk_custom_frames(BKContext *ctx)
{
  const double now = bk_time() + 0.002;
  ctx->in_custom_frame++;
  for (BKCustom *c = g_customs; c; c = c->next) {
    BKNode *n = c->node;
    if (!live(n, ctx)) {
      continue;
    }
    if (c->wake_at > 0.0 && c->wake_at <= now) {
      c->wake_at = 0.0;
      c->want_frame = 1;
    }
    if (!c->want_frame && c->framed && c->last_w == n->w && c->last_h == n->h) {
      continue;
    }
    c->want_frame = 0;
    c->framed = 1;
    c->last_w = n->w;
    c->last_h = n->h;
    if (c->cls.frame) {
      c->cls.frame(c->data, n);
    }
    else {
      bk_invalidate(n);
    }
  }
  ctx->in_custom_frame--;
}

bool bk_custom_animating(BKContext *ctx)
{
  for (BKCustom *c = g_customs; c; c = c->next) {
    if (c->want_frame && live(c->node, ctx)) {
      return true;
    }
  }
  return false;
}

/* -- public API ---------------------------------------------------------------------------- */

int BK_custom_set(BKNode *node, const BKCustomClass *cls, void *data)
{
  BKCustom *c = cust(node);
  if (!c) {
    return 0;
  }
  memset(&c->cls, 0, sizeof(c->cls));
  if (cls) {
    c->cls = *cls;
  }
  c->data = data;
  c->framed = 0; /* the first render calls frame */
  bk_relayout(node);
  return 1;
}

void *BK_custom_data(BKNode *node)
{
  const BKCustom *c = cust(node);
  return c ? c->data : NULL;
}

void BK_custom_request_frame(BKNode *node, int ms)
{
  BKCustom *c = cust(node);
  if (!c) {
    return;
  }
  BKContext *ctx = bk_ctx(node);
  if (ms <= 0) {
    c->want_frame = 1;
    if (ctx && !ctx->in_custom_frame) {
      bk_ctx_request(ctx);
    }
    return;
  }
  const double at = bk_time() + ms / 1000.0;
  if (c->wake_at == 0.0 || at < c->wake_at) {
    c->wake_at = at;
  }
  if (ctx) {
    bk_ctx_arm_timer(ctx);
  }
}

void BK_custom_set_cursor(BKNode *node, int cursor)
{
  if (!BK_IS_CUSTOM(node) || node->cursor_override == cursor + 1) {
    return;
  }
  node->cursor_override = cursor + 1;
  BKContext *ctx = bk_ctx(node);
  if (ctx && (ctx->hover == node || ctx->grab == node)) {
    bk_update_cursor(ctx);
  }
}

int BK_custom_has_focus(BKNode *node)
{
  BKContext *ctx = bk_ctx(node);
  return ctx && BK_IS_CUSTOM(node) && ctx->focus == node;
}

BKContext *BK_node_context(const BKNode *node)
{
  return node ? bk_ctx(node) : NULL;
}
