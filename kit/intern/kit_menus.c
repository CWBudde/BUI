/* Menus (dropdown, context, menu bar, submenus, type-to-search), pie menus
 * and popover panels; popup placement.
 *
 * As in Blender: menus open on press and pick on release, hovering a
 * submenu item opens it, typing searches every item including submenus,
 * Up/Down/Return/Right/Left navigate; pies pick by direction. Backgrounds
 * use widget_menu_back (soft shadow + menu_back colors), rows use
 * widget_menu_itembut with widget_state_menu_item.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <ctype.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "kit_intern.h"

static int u(void)
{
  return bk_unit();
}

/* -------------------------------------------------------------------- */
/** \name Menu specs (deep copies with retained refs)
 * \{ */

static bool is_sep(const BKMenuSpec *s)
{
  return !s->text || strcmp(s->text, "-") == 0;
}

BKMenuSpec *bk_menuspec_copy(const BKMenuSpec *items, int n)
{
  if (n <= 0) {
    return NULL;
  }
  BKMenuSpec *out = (BKMenuSpec *)calloc((size_t)n, sizeof(BKMenuSpec));
  for (int i = 0; i < n; i++) {
    out[i] = items[i];
    out[i].text = bk_strdup(items[i].text ? items[i].text : "-");
    out[i].shortcut = items[i].shortcut ? bk_strdup(items[i].shortcut) : NULL;
    out[i].sub = items[i].nsub ? bk_menuspec_copy(items[i].sub, items[i].nsub) : NULL;
    bk_retain(items[i].ref);
  }
  return out;
}

void bk_menuspec_free(BKMenuSpec *items, int n)
{
  if (!items) {
    return;
  }
  for (int i = 0; i < n; i++) {
    free((char *)items[i].text);
    free((char *)items[i].shortcut);
    bk_menuspec_free((BKMenuSpec *)items[i].sub, items[i].nsub);
    bk_release(items[i].ref);
  }
  free(items);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Placement
 * \{ */

#define PLACE_NONE (-100000)

static BKPopupPlace *popup_place_of(BKNode *p)
{
  return (BKPopupPlace *)p->ext; /* every popup's ext starts with BKPopupPlace */
}

static void place_init(BKPopupPlace *pl)
{
  pl->at_x = pl->at_y = PLACE_NONE;
  pl->side_x = pl->side_y = pl->side_xl = PLACE_NONE;
}

void bk_popup_place(BKNode *p, int win_w, int win_h)
{
  if (p->cls == &bk_cls_pie) {
    bk_arrange(p, 0, 0, win_w, win_h);
    return;
  }
  BKPopupPlace *pl = popup_place_of(p);
  int w, h;
  bk_natural(p, &w, &h);
  int x, y;
  const BKNode *o = p->parent;
  if (pl->side_x != PLACE_NONE) { /* submenu beside its row */
    x = (pl->side_x + w <= win_w) ? pl->side_x : BK_MAX(0, pl->side_xl - w);
    y = BK_MAX(0, BK_MIN(pl->side_y, win_h - h));
  }
  else if (pl->at_x != PLACE_NONE) { /* at a point (context menus) */
    x = BK_MAX(0, BK_MIN(pl->at_x, win_w - w));
    y = pl->at_y;
    if (y + h > win_h) {
      y = BK_MAX(0, y - h);
    }
  }
  else { /* below the owner; flipped above when there is no room */
    w = BK_MAX(w, o ? o->w : 0);
    const int ox = o ? o->x : 0, oy = o ? o->y : 0, oh = o ? o->h : 0;
    y = oy + oh;
    if (y + h > win_h && oy - h >= 0) {
      y = oy - h;
    }
    x = BK_MAX(0, BK_MIN(ox, win_w - w));
  }
  bk_arrange(p, x, y, w, h);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Menu
 * \{ */

static BKRect pie_item_rect_pub(BKNode *n, int i);

typedef struct MenuData {
  BKPopupPlace place;
  BKMenuSpec *all;
  int nall;
  BKMenuSpec *filtered; /* search results (owned) */
  int nfiltered;
  char *title, *current;
  char query[128];
  int from_row; /* row of the parent menu that opened this submenu */
} MenuData;

#define MENU(n) ((MenuData *)(n)->ext)

static BKMenuSpec *menu_items(BKNode *n, int *r_count)
{
  MenuData *m = MENU(n);
  if (m->query[0]) {
    *r_count = m->nfiltered;
    return m->filtered;
  }
  *r_count = m->nall;
  return m->all;
}

static int menu_pad(void)
{
  return bk_iround(0.2f * u()); /* UI_MENU_PADDING */
}
static int menu_sep_h(void)
{
  return bk_iround(0.5f * u());
}
static int menu_title_h(BKNode *n)
{
  return (MENU(n)->title || MENU(n)->query[0]) ? u() : 0;
}

static void menu_row_geom(BKNode *n, int row, int *r_y, int *r_h)
{
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  int y = menu_pad() + menu_title_h(n);
  for (int i = 0; i < count; i++) {
    const int h = is_sep(&items[i]) ? menu_sep_h() : u();
    if (i == row) {
      *r_y = n->y + y;
      *r_h = h;
      return;
    }
    y += h;
  }
  *r_y = n->y + y;
  *r_h = 0;
}

static int menu_row_at(BKNode *n, int y)
{
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  for (int i = 0; i < count; i++) {
    int ry, rh;
    menu_row_geom(n, i, &ry, &rh);
    if (y >= ry && y < ry + rh) {
      return (!is_sep(&items[i]) && items[i].enabled) ? i : -1;
    }
  }
  return -1;
}

static bool menu_has_icons(BKMenuSpec *items, int count)
{
  for (int i = 0; i < count; i++) {
    if (!is_sep(&items[i]) && (items[i].icon || items[i].checked >= 0)) {
      return true;
    }
  }
  return false;
}

static void menu_measure(BKNode *n, int *rw, int *rh)
{
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  const float uu = (float)u();
  float text_w = 0, sc_w = 0;
  bool sub = false;
  int h = 0;
  for (int i = 0; i < count; i++) {
    if (is_sep(&items[i])) {
      h += menu_sep_h();
      continue;
    }
    h += u();
    text_w = BK_MAX(text_w, bk_text_width(items[i].text, -1, false));
    if (items[i].shortcut) {
      sc_w = BK_MAX(sc_w, bk_text_width(items[i].shortcut, -1, false));
    }
    sub |= items[i].nsub > 0;
  }
  float w = text_w + 2 * bk_pad() + (menu_has_icons(items, count) ? bk_icon_size() + 0.3f * uu : 0);
  w += sc_w > 0 ? sc_w + 1.5f * uu : 0;
  w += sub ? uu : 0;
  if (MENU(n)->title) {
    w = BK_MAX(w, bk_text_width(MENU(n)->title, -1, false) + 2 * bk_pad());
  }
  w = BK_MAX(w, 9 * uu); /* UI_MENU_WIDTH_MIN */
  if (count == 0 && MENU(n)->query[0]) {
    h = u(); /* "No results found" */
  }
  *rw = (int)w + 2 * menu_pad();
  *rh = BK_MAX(h, u()) + menu_title_h(n) + 2 * menu_pad();
}

static int menu_child_row(BKNode *n)
{
  BKContext *ctx = bk_ctx(n);
  const int k = ctx ? bk_popup_index(ctx, n) : -1;
  if (k >= 0 && k + 1 < ctx->npopups && ctx->popups[k + 1]->parent == n) {
    return MENU(ctx->popups[k + 1])->from_row;
  }
  return -1;
}

static void menu_open_sub(BKNode *n, int row)
{
  BKContext *ctx = bk_ctx(n);
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  if (!ctx || row < 0 || row >= count || !items[row].nsub || menu_child_row(n) == row) {
    return;
  }
  BKNode *sub = bk_menu_new(items[row].sub, items[row].nsub, NULL, NULL);
  int ry, rh;
  menu_row_geom(n, row, &ry, &rh);
  MENU(sub)->place.side_x = n->x + n->w;
  MENU(sub)->place.side_y = ry - menu_pad();
  MENU(sub)->place.side_xl = n->x;
  MENU(sub)->from_row = row;
  bk_open_popup(ctx, sub, n, n);
}

static BKNode *menu_root(BKNode *n)
{
  while (n->parent && n->parent->cls == &bk_cls_menu) {
    n = n->parent;
  }
  return n;
}

static void menu_pick(BKNode *n, int row)
{
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  if (row < 0 || row >= count || is_sep(&items[row]) || !items[row].enabled) {
    return;
  }
  if (items[row].nsub) {
    menu_open_sub(n, row);
    return;
  }
  BKContext *ctx = bk_ctx(n);
  if (!ctx) {
    return;
  }
  /* take what we need before the menus (and their copies) are freed */
  void (*fn)(void *) = items[row].fn;
  void *data = items[row].data;
  void *ref = items[row].ref;
  BKNode *owner = menu_root(n)->parent;
  bk_retain(ref);
  BK_close_popups(ctx);
  if (fn) {
    fn(data);
  }
  else if (ref) {
    if (ctx->host.emit) {
      ctx->host.emit(ctx->host.data, owner, BK_EV_CALL, ref, 0, 0);
    }
  }
  else if (owner && (owner->cls == &bk_cls_dropdown || owner->cls == &bk_cls_enum)) {
    bk_set_value(owner, (double)(intptr_t)data); /* enum menu: the item index */
  }
  bk_release(ref);
}

static void menu_filter_walk(MenuData *m, const BKMenuSpec *items, int count, const char *path,
                             const char *q, BKMenuSpec **out, int *nout, int *cap)
{
  for (int i = 0; i < count; i++) {
    const BKMenuSpec *it = &items[i];
    if (is_sep(it)) {
      continue;
    }
    char full[512];
    if (path[0]) {
      snprintf(full, sizeof(full), "%s \xe2\x96\xb8 %s", path, it->text);
    }
    else {
      snprintf(full, sizeof(full), "%s", it->text);
    }
    if (it->nsub) {
      menu_filter_walk(m, it->sub, it->nsub, full, q, out, nout, cap);
      continue;
    }
    /* case-insensitive substring */
    bool hit = false;
    for (const char *s = it->text; *s && !hit; s++) {
      size_t k = 0;
      while (q[k] && s[k] && tolower((unsigned char)s[k]) == tolower((unsigned char)q[k])) {
        k++;
      }
      hit = q[k] == 0;
    }
    if (!hit) {
      continue;
    }
    if (*nout == *cap) {
      *cap = *cap ? *cap * 2 : 16;
      *out = (BKMenuSpec *)realloc(*out, sizeof(BKMenuSpec) * (size_t)*cap);
    }
    BKMenuSpec s = *it;
    s.text = bk_strdup(full);
    s.shortcut = it->shortcut ? bk_strdup(it->shortcut) : NULL;
    s.sub = NULL;
    s.nsub = 0;
    bk_retain(s.ref);
    (*out)[(*nout)++] = s;
  }
}

static void menu_filter(BKNode *n)
{
  MenuData *m = MENU(n);
  bk_menuspec_free(m->filtered, m->nfiltered);
  m->filtered = NULL;
  m->nfiltered = 0;
  if (m->query[0]) {
    int cap = 0;
    menu_filter_walk(m, m->all, m->nall, "", m->query, &m->filtered, &m->nfiltered, &cap);
  }
  n->hot = (m->query[0] && m->nfiltered) ? 0 : -1;
  BKContext *ctx = bk_ctx(n);
  if (ctx) {
    bk_close_popups_after(ctx, n);
    bk_invalidate(n); /* old size */
    bk_popup_place(n, ctx->win_w, ctx->win_h);
  }
  bk_relayout(n);
  bk_invalidate(n);
}

static int menu_move(BKNode *n, BKEvent *e)
{
  const int hot = menu_row_at(n, e->y);
  if (hot == n->hot) {
    return 0;
  }
  n->hot = hot;
  bk_invalidate(n);
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  if (hot >= 0 && items[hot].nsub) {
    menu_open_sub(n, hot);
  }
  else if (hot >= 0 && menu_child_row(n) >= 0) {
    bk_close_popups_after(bk_ctx(n), n);
  }
  return 1;
}

static int menu_press(BKNode *n, BKEvent *e)
{
  return 1;
}

static int menu_release(BKNode *n, BKEvent *e)
{
  const int row = menu_row_at(n, e->y);
  if (bk_contains(n, e->x, e->y) && row >= 0) {
    menu_pick(n, row);
  }
  return 1;
}

static void menu_step(BKNode *n, int d)
{
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  int i = n->hot;
  for (int k = 0; k < count; k++) {
    i = i >= 0 ? (i + d + count) % count : (d > 0 ? 0 : count - 1);
    if (!is_sep(&items[i]) && items[i].enabled) {
      n->hot = i;
      bk_invalidate(n);
      return;
    }
  }
}

static int menu_search_key(BKNode *n, BKEvent *e)
{
  MenuData *m = MENU(n);
  if (bk_keysym_is(e->keysym, "Escape") && m->query[0]) {
    m->query[0] = 0;
    menu_filter(n);
    return 1;
  }
  if (bk_keysym_is(e->keysym, "BackSpace") && m->query[0]) {
    size_t l = strlen(m->query);
    do {
      l--;
    } while (l > 0 && ((unsigned char)m->query[l] & 0xC0) == 0x80);
    m->query[l] = 0;
    menu_filter(n);
    return 1;
  }
  if (e->text && (unsigned char)e->text[0] >= 0x20 && !EV_CTRL(e) &&
      strlen(m->query) + strlen(e->text) < sizeof(m->query) - 1)
  {
    strcat(m->query, e->text);
    menu_filter(n);
    return 1;
  }
  return 0;
}

static int menu_key(BKNode *n, BKEvent *e)
{
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  if (bk_keysym_is(e->keysym, "Down") || bk_keysym_is(e->keysym, "Up")) {
    menu_step(n, bk_keysym_is(e->keysym, "Down") ? 1 : -1);
    return 1;
  }
  if (bk_keysym_is(e->keysym, "Return") || bk_keysym_is(e->keysym, "KP_Enter")) {
    menu_pick(n, n->hot);
    return 1;
  }
  if (bk_keysym_is(e->keysym, "Right") && n->hot >= 0 && n->hot < count && items[n->hot].nsub) {
    menu_open_sub(n, n->hot);
    return 1;
  }
  if (bk_keysym_is(e->keysym, "Left") && n->parent && n->parent->cls == &bk_cls_menu) {
    bk_close_popups_after(bk_ctx(n), n->parent);
    return 1;
  }
  /* typing always searches from the top menu (Blender menu search) */
  return menu_search_key(menu_root(n), e);
}

static void menu_check_icon(float cx, float cy, bool on)
{
  const int s = bk_iround(0.6f * u());
  BKRect r = bk_rect((int)(cx - s * 0.5f), (int)(cy - s * 0.5f), s, s);
  BKWCol w;
  bk_widget_state(&w, &bk_theme.option, on ? ST_SELECT : 0);
  bk_widget_optionbut(&w, &r, on ? ST_SELECT : 0);
}

static void menu_draw(BKNode *n)
{
  MenuData *m = MENU(n);
  BKRect r = bk_node_rect(n);
  bk_widget_menu_back(&r, CNR_ALL);
  const int p = menu_pad(), th = menu_title_h(n);
  const float uu = (float)u();
  const BKWCol *mi = &bk_theme.menu_item;
  if (m->query[0]) {
    uchar ic[4];
    memcpy(ic, mi->text, 4);
    ic[3] = 180;
    bk_icon(bk_icons_lookup("SEARCH"), n->x + p + 0.2f * uu + bk_icon_size() * 0.5f,
            n->y + p + th * 0.5f, ic, 1.0f, 0);
    BKRect qr = bk_rect(n->x + p + bk_icon_size() + (int)(0.3f * uu) - bk_pad(), n->y + p,
                        n->w - 2 * p, th);
    bk_text_in(&qr, m->query, mi->text, BK_ALIGN_LEFT, false);
  }
  else if (m->title) {
    BKRect tr = bk_rect(n->x + p, n->y + p, n->w - 2 * p, th);
    uchar tc[4];
    memcpy(tc, bk_theme.menu_back.text, 4);
    bk_text_in(&tr, m->title, tc, BK_ALIGN_LEFT, false);
  }
  int count;
  BKMenuSpec *items = menu_items(n, &count);
  if (count == 0 && m->query[0]) {
    BKRect nr = bk_rect(n->x + p, n->y + p + th, n->w - 2 * p, u());
    uchar tc[4];
    memcpy(tc, mi->text, 4);
    tc[3] = 128;
    bk_text_in(&nr, "No results found", tc, BK_ALIGN_LEFT, false);
  }
  const bool icons = menu_has_icons(items, count);
  const int open_child = menu_child_row(n);
  for (int i = 0; i < count; i++) {
    int ry, rh;
    menu_row_geom(n, i, &ry, &rh);
    BKMenuSpec *it = &items[i];
    BKRect row = bk_rect(n->x + p, ry, n->w - 2 * p, rh);
    if (is_sep(it)) {
      float c[4];
      bk_rgba_alpha(mi->text, 0.15f, c);
      bk_fill_f(row.xmin + bk_pad(), ry + rh / 2, bk_rw(&row) - 2 * bk_pad(), 1, c);
      continue;
    }
    int st = it->enabled ? 0 : ST_DISABLED;
    if ((i == n->hot || i == open_child) && it->enabled) {
      st |= ST_HOVER;
    }
    if (m->current && strcmp(it->text, m->current) == 0) {
      st |= ST_ACTIVE_DEFAULT; /* the current enum value */
    }
    BKWCol w;
    bk_widget_state_menu_item(&w, mi, st);
    BKRect wr = row;
    if (st & (ST_HOVER | ST_ACTIVE_DEFAULT)) {
      bk_widget_menu_itembut(&w, &wr);
    }
    float tx = row.xmin + bk_pad();
    if (icons) {
      const float cx = tx + bk_icon_size() * 0.5f - 0.1f * uu;
      if (it->checked >= 0) {
        menu_check_icon(cx, ry + rh * 0.5f, it->checked > 0);
      }
      else if (it->icon) {
        bk_icon(it->icon, cx, ry + rh * 0.5f, NULL, it->enabled ? 1.0f : 0.5f, 0);
      }
      tx += bk_icon_size() + 0.2f * uu;
    }
    bk_clip_push(row.xmin, row.ymin, bk_rw(&row), rh);
    BKRect lr = bk_rect(row.xmin, ry, bk_rw(&row), rh);
    bk_text(tx, bk_text_baseline(&lr), it->text, -1, w.text, false);
    if (it->shortcut) {
      uchar sc[4];
      memcpy(sc, w.text, 4);
      sc[3] = (uchar)(sc[3] * 0.55f);
      const float sw = bk_text_width(it->shortcut, -1, false);
      bk_text(row.xmax - bk_pad() - sw - (it->nsub ? uu : 0), bk_text_baseline(&lr), it->shortcut,
              -1, sc, false);
    }
    bk_clip_pop();
    if (it->nsub) {
      uchar ac[4];
      memcpy(ac, w.text, 4);
      ac[3] = (uchar)(ac[3] * 0.8f);
      bk_icon(bk_icons_lookup("RIGHTARROW"), row.xmax - 0.5f * uu, ry + rh * 0.5f, ac, 1.0f,
              0.6f * uu);
    }
  }
}

static void menu_free(BKNode *n)
{
  MenuData *m = MENU(n);
  bk_menuspec_free(m->all, m->nall);
  bk_menuspec_free(m->filtered, m->nfiltered);
  free(m->title);
  free(m->current);
  free(m);
}

void bk_menu_on_close(BKNode *menu)
{
  (void)menu;
}

const BKClass bk_cls_menu = {.name = "menu",
                             .flags = CLS_INTERACTIVE | CLS_POPUP,
                             .free_ext = menu_free,
                             .measure = menu_measure,
                             .draw = menu_draw,
                             .press = menu_press,
                             .release = menu_release,
                             .move = menu_move,
                             .key = menu_key};

BKNode *bk_menu_new(const BKMenuSpec *items, int n, const char *title, const char *current)
{
  BKNode *menu = bk_node_new_cls(&bk_cls_menu);
  MenuData *m = (MenuData *)calloc(1, sizeof(MenuData));
  place_init(&m->place);
  m->all = bk_menuspec_copy(items, n);
  m->nall = n;
  m->title = title ? bk_strdup(title) : NULL;
  m->current = current ? bk_strdup(current) : NULL;
  m->from_row = -1;
  menu->ext = m;
  menu->hot = -1;
  if (current) {
    for (int i = 0; i < n; i++) {
      if (!is_sep(&items[i]) && strcmp(items[i].text, current) == 0) {
        menu->hot = i;
      }
    }
  }
  return menu;
}

void BK_open_menu(BKContext *ctx, BKNode *owner, const BKMenuSpec *items, int n, const char *title,
                  int at_x, int at_y)
{
  BKNode *menu = bk_menu_new(items, n, title, NULL);
  if (at_x > PLACE_NONE && at_y > PLACE_NONE && (at_x || at_y)) {
    MENU(menu)->place.at_x = at_x;
    MENU(menu)->place.at_y = at_y;
  }
  bk_open_popup(ctx, menu, owner ? owner : ctx->root, NULL);
}

int BK_menu_row_count(BKNode *menu)
{
  int count = 0;
  if (menu->cls == &bk_cls_menu || menu->cls == &bk_cls_pie) {
    menu_items(menu, &count);
  }
  return count;
}

void BK_menu_row_rect(BKNode *menu, int i, int r[4])
{
  if (menu->cls == &bk_cls_pie) {
    BKRect rr = pie_item_rect_pub(menu, i);
    r[0] = rr.xmin;
    r[1] = rr.ymin;
    r[2] = bk_rw(&rr);
    r[3] = bk_rh(&rr);
    return;
  }
  int ry, rh;
  menu_row_geom(menu, i, &ry, &rh);
  r[0] = menu->x + menu_pad();
  r[1] = ry;
  r[2] = menu->w - 2 * menu_pad();
  r[3] = rh;
}

const char *BK_menu_row_text(BKNode *menu, int i)
{
  int count = 0;
  BKMenuSpec *items = menu_items(menu, &count);
  return (i >= 0 && i < count) ? items[i].text : "";
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Pie menu (Blender order: W, E, S, N, NW, NE, SW, SE)
 * \{ */

static const float g_pie_dirs[8][2] = {{-1, 0},         {1, 0},        {0, 1},
                                       {0, -1},         {-0.71f, -0.71f}, {0.71f, -0.71f},
                                       {-0.71f, 0.71f}, {0.71f, 0.71f}};

typedef struct PieData {
  MenuData menu; /* items in menu.all */
  float cx, cy;
  int at_x, at_y;
} PieData;

#define PIE(n) ((PieData *)(n)->ext)

static float pie_radius(void)
{
  return 4.2f * u();
}

static BKRect pie_item_rect(BKNode *n, int i)
{
  PieData *pd = PIE(n);
  const BKMenuSpec *it = &pd->menu.all[i];
  const float uu = (float)u();
  float w = bk_text_width(it->text, -1, false) + 2 * bk_pad() +
            (it->icon ? bk_icon_size() + 0.3f * uu : 0);
  w = BK_MAX(w, 3 * uu);
  const float px = pd->cx + g_pie_dirs[i][0] * pie_radius();
  const float py = pd->cy + g_pie_dirs[i][1] * pie_radius() * 0.8f;
  float x;
  if (g_pie_dirs[i][0] < -0.1f) {
    x = px - w;
  }
  else if (g_pie_dirs[i][0] > 0.1f) {
    x = px;
  }
  else {
    x = px - w * 0.5f;
  }
  return bk_rect((int)x, (int)(py - uu * 0.5f), (int)w, (int)uu);
}

static BKRect pie_item_rect_pub(BKNode *n, int i)
{
  return pie_item_rect(n, i);
}

static int pie_dir_hot(BKNode *n, int x, int y)
{
  PieData *pd = PIE(n);
  const int count = BK_MIN(8, pd->menu.nall);
  for (int i = 0; i < count; i++) {
    BKRect r = pie_item_rect(n, i);
    if (x >= r.xmin && x < r.xmax && y >= r.ymin && y < r.ymax) {
      return i;
    }
  }
  const float dx = x - pd->cx, dy = y - pd->cy;
  if (hypotf(dx, dy) < 0.8f * u()) {
    return -1;
  }
  const float a = atan2f(dy, dx);
  int best = -1;
  float best_d = 9.0f;
  for (int i = 0; i < count; i++) {
    const float da = a - atan2f(g_pie_dirs[i][1], g_pie_dirs[i][0]);
    const float d = fabsf(atan2f(sinf(da), cosf(da)));
    if (d < best_d) {
      best = i;
      best_d = d;
    }
  }
  return best;
}

static void pie_arrange(BKNode *n)
{
  PieData *pd = PIE(n);
  const float r = pie_radius(), uu = (float)u();
  const float mx = r + 6 * uu, my = r * 0.8f + uu;
  const float ax = pd->at_x > PLACE_NONE ? pd->at_x : n->w * 0.5f;
  const float ay = pd->at_y > PLACE_NONE ? pd->at_y : n->h * 0.5f;
  pd->cx = BK_MAX(mx, BK_MIN(n->w - mx, ax));
  pd->cy = BK_MAX(my, BK_MIN(n->h - my, ay));
}

static BKNode *pie_hit(BKNode *n, int x, int y)
{
  return n; /* modal: owns the whole window while open */
}

static void pie_pick(BKNode *n, int i)
{
  PieData *pd = PIE(n);
  BKContext *ctx = bk_ctx(n);
  if (!ctx) {
    return;
  }
  if (i < 0 || i >= BK_MIN(8, pd->menu.nall) || !pd->menu.all[i].enabled) {
    BK_close_popups(ctx);
    return;
  }
  void (*fn)(void *) = pd->menu.all[i].fn;
  void *data = pd->menu.all[i].data, *ref = pd->menu.all[i].ref;
  bk_retain(ref);
  BK_close_popups(ctx);
  if (fn) {
    fn(data);
  }
  else if (ref && ctx->host.emit) {
    ctx->host.emit(ctx->host.data, NULL, BK_EV_CALL, ref, 0, 0);
  }
  bk_release(ref);
}

static int pie_move(BKNode *n, BKEvent *e)
{
  const int hot = pie_dir_hot(n, e->x, e->y);
  if (hot != n->hot) {
    n->hot = hot;
    bk_invalidate(n);
    return 1;
  }
  return 0;
}

static int pie_press(BKNode *n, BKEvent *e)
{
  pie_pick(n, pie_dir_hot(n, e->x, e->y));
  return 1;
}

static int pie_key(BKNode *n, BKEvent *e)
{
  if (e->text && e->text[0] >= '1' && e->text[0] <= '8' && !e->text[1]) {
    pie_pick(n, e->text[0] - '1');
    return 1;
  }
  if (bk_keysym_is(e->keysym, "Return") || bk_keysym_is(e->keysym, "KP_Enter")) {
    pie_pick(n, n->hot);
    return 1;
  }
  return 0;
}

static void pie_draw(BKNode *n)
{
  PieData *pd = PIE(n);
  const float uu = (float)u();
  float c[4];
  /* center disc with a direction indicator (draw_pie_center) */
  bk_rgba_alpha(bk_theme.pie_menu.inner, 0.85f, c);
  bk_circle(pd->cx, pd->cy, 0.9f * uu, c);
  bk_rgba(bk_theme.menu_back.outline, c);
  bk_ring(pd->cx, pd->cy, 0.9f * uu, c, 1.0f);
  if (n->hot >= 0) {
    const float a = atan2f(g_pie_dirs[n->hot][1], g_pie_dirs[n->hot][0]);
    float ic[4];
    bk_rgba(bk_theme.pie_menu.item, ic);
    for (int t = -4; t < 4; t++) {
      bk_line(pd->cx + 0.9f * uu * cosf(a + t / 10.0f), pd->cy + 0.9f * uu * sinf(a + t / 10.0f),
              pd->cx + 0.9f * uu * cosf(a + (t + 1) / 10.0f),
              pd->cy + 0.9f * uu * sinf(a + (t + 1) / 10.0f), ic, BK_MAX(2.0f, 0.15f * uu));
    }
  }
  if (pd->menu.title) {
    const float tw = bk_text_width(pd->menu.title, -1, false) + 2 * bk_pad();
    BKRect tr = bk_rect((int)(pd->cx - tw * 0.5f), (int)(pd->cy - pie_radius() * 0.8f - 2.2f * uu),
                        (int)tw, (int)uu);
    bk_widget_menu_back(&tr, CNR_ALL);
    bk_text_in(&tr, pd->menu.title, bk_theme.menu_back.text, BK_ALIGN_CENTER, false);
  }
  for (int i = 0; i < BK_MIN(8, pd->menu.nall); i++) {
    BKRect r = pie_item_rect(n, i);
    const BKMenuSpec *it = &pd->menu.all[i];
    int st = (i == n->hot ? ST_HOVER : 0) | (it->enabled ? 0 : ST_DISABLED);
    BKWCol w = bk_theme.pie_menu;
    /* widget_state_pie_menu_item */
    if (st & ST_HOVER) {
      memcpy(w.text, w.text_sel, 3);
      memcpy(w.inner, w.item, 4);
    }
    bk_shadow(&r, bk_widget_radius(&w));
    BKRect wr = r;
    bk_widget_but(&w, &wr, 0, CNR_ALL);
    bk_text_icon(&r, it->text, it->icon, w.text, BK_ALIGN_CENTER, it->enabled);
  }
}

static void pie_free(BKNode *n)
{
  PieData *pd = PIE(n);
  bk_menuspec_free(pd->menu.all, pd->menu.nall);
  free(pd->menu.title);
  free(pd);
}

const BKClass bk_cls_pie = {.name = "pie",
                            .flags = CLS_INTERACTIVE | CLS_POPUP,
                            .free_ext = pie_free,
                            .arrange = pie_arrange,
                            .draw = pie_draw,
                            .hit = pie_hit,
                            .press = pie_press,
                            .move = pie_move,
                            .key = pie_key};

void BK_open_pie(BKContext *ctx, const BKMenuSpec *items, int n, const char *title)
{
  BKNode *pie = bk_node_new_cls(&bk_cls_pie);
  PieData *pd = (PieData *)calloc(1, sizeof(PieData));
  place_init(&pd->menu.place);
  pd->menu.all = bk_menuspec_copy(items, BK_MIN(n, 8));
  pd->menu.nall = BK_MIN(n, 8);
  pd->menu.title = title ? bk_strdup(title) : NULL;
  pd->at_x = ctx->pointer_x;
  pd->at_y = ctx->pointer_y;
  pie->ext = pd;
  pie->hot = -1;
  bk_open_popup(ctx, pie, ctx->root, NULL);
}

/** \} */

/* -------------------------------------------------------------------- */
/** \name Popover panel (holds the Popover's body while open)
 * \{ */

typedef struct PopoverData {
  BKPopupPlace place;
} PopoverData;

static int popover_pad(void)
{
  return 2 * bk_iround(0.2f * u());
}

static void popover_panel_measure(BKNode *n, int *rw, int *rh)
{
  int bw = 0, bh = 0;
  if (n->nkids) {
    bk_natural(n->kids[0], &bw, &bh);
  }
  const int p = popover_pad();
  *rw = BK_MAX(bw, 10 * u()) + 2 * p;
  *rh = bh + 2 * p;
}

static void popover_panel_arrange(BKNode *n)
{
  const int p = popover_pad();
  if (n->nkids) {
    bk_arrange(n->kids[0], n->x + p, n->y + p, n->w - 2 * p, n->h - 2 * p);
  }
}

static BKNode *popover_panel_hit(BKNode *n, int x, int y)
{
  BKNode *f = n->nkids ? bk_hit(n->kids[0], x, y) : NULL;
  return f ? f : n;
}

static void popover_panel_draw(BKNode *n)
{
  BKRect r = bk_node_rect(n);
  bk_widget_menu_back(&r, CNR_ALL);
}

static void popover_panel_free(BKNode *n)
{
  free(n->ext);
}

const BKClass bk_cls_popover_panel = {.name = "popover_panel",
                                      .flags = CLS_INTERACTIVE | CLS_POPUP,
                                      .free_ext = popover_panel_free,
                                      .measure = popover_panel_measure,
                                      .arrange = popover_panel_arrange,
                                      .draw = popover_panel_draw,
                                      .hit = popover_panel_hit};

BKNode *bk_popover_panel_new(BKNode *owner, BKNode *body)
{
  BKNode *p = bk_node_new_cls(&bk_cls_popover_panel);
  PopoverData *d = (PopoverData *)calloc(1, sizeof(PopoverData));
  place_init(&d->place);
  p->ext = d;
  /* borrow the body (it stays owned by the popover button) */
  p->kids = (BKNode **)malloc(sizeof(BKNode *));
  p->kids[0] = body;
  p->nkids = 1;
  p->capkids = 1;
  body->parent = p;
  return p;
}

/** \} */
