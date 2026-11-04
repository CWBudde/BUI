/* Blender's SVG icons (generated into kit_icons_svg.c by tools/gen_svg_icons.py).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

typedef struct BKSvgIcon {
  const char *name;
  /* -1: colored (DEF_ICON_COLOR); 0: mono, the text color; 1..8: mono with a
   * theme icon color (scene, collection, object, object data, modifier,
   * shading, folder, fund) */
  int kind;
  const char *svg;
} BKSvgIcon;

extern const BKSvgIcon bk_svg_icons[];
extern const int bk_svg_icon_count;
