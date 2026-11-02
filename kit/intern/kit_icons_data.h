/* Vector icon tables (generated into kit_icons_data.c).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

enum { BK_ICON_LINE, BK_ICON_POLY, BK_ICON_DISC, BK_ICON_RING, BK_ICON_RECT, BK_ICON_FRAME,
       BK_ICON_BOX };

typedef struct BKIconOp {
  unsigned char op;
  unsigned char color_mode; /* 0 pen, 1 pen * k alpha, 2 explicit rgba */
  float k;
  float rgba[4];
  float a[6];      /* op arguments in the unit box (widths are line-width multiples) */
  int pts_offset;  /* polygon points */
  int npts;
} BKIconOp;

typedef struct BKIconDef {
  const char *name;
  int first, count;
  int has_tint;
  float tint[4];
} BKIconDef;

extern const BKIconOp bk_icon_ops[];
extern const float bk_icon_pts[][2];
extern const BKIconDef bk_icon_defs[];
extern const int bk_icon_def_count;
