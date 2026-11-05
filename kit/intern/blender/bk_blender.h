/* C declarations for the Blender sources copied into kit/intern/blender/
 * (C++ files edited for the kit; each keeps its Blender license header and
 * marks its edits with "BUI:").
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/* interface_string_search.cc: recent searches (ranked first for short queries).
 * Public: BK_recent_searches_* in BUI_kit.h (declared here too for C linkage). */
void BK_recent_searches_enable(int enabled);
void BK_recent_searches_clear(void);
void BK_recent_searches_load(const char *path);
void BK_recent_searches_save(const char *path);
void bk_search_recent_add(const char *chosen);
const void *bk_search_recent_cache(void); /* blender::string_search::RecentCache or NULL */

/* interface_undo.cc: the undo stack of an active text edit. */
typedef struct BKTextUndo BKTextUndo;
BKTextUndo *bk_textedit_undo_stack_create(void);
void bk_textedit_undo_stack_destroy(BKTextUndo *stack);
void bk_textedit_undo_push(BKTextUndo *stack, const char *text, int cursor_index);
/* direction -1 undo, 1 redo; NULL when there is nothing to step to */
const char *bk_textedit_undo(BKTextUndo *stack, int direction, int *r_cursor_index);

/* interface_utils.cc: UI_calc_float_precision (display decimals for small values). */
int bk_calc_float_precision(int prec, double value);

#ifdef __cplusplus
}
#endif
