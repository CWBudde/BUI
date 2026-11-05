/* C entry points to Blender C++ code vendored in core/ (blenlib):
 * - expr_pylike_eval.cc: the number-field expression evaluator,
 * - string_search.cc: Blender's fuzzy search (menu search, search fields).
 * Implemented in core/intern/bui_blender_c.cc.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/* Evaluate a Python-like expression (BLI_expr_pylike_eval): numbers, + - * / // % ** ^,
 * comparisons, and/or/not, x if c else y, pi e tau, sin cos sqrt ... min max.
 * Optional named parameters. Returns 1 and sets *r_value on success, 0 on a
 * parse error, division by zero or a non-finite result. */
int BUI_expr_eval(const char *expr,
                  const char **param_names,
                  const double *param_values,
                  int params_len,
                  double *r_value);

/* blender::string_search::StringSearch<void>. `recent_cache` is a
 * blender::string_search::RecentCache (or NULL), e.g. bk_recent_search_cache(). */
typedef struct BUIStringSearch BUIStringSearch;
enum { BUI_SEARCH_MAIN_FIRST = 0, BUI_SEARCH_MAIN_LAST = 1, BUI_SEARCH_MAIN_ALL = 2 };
BUIStringSearch *BUI_string_search_new(const void *recent_cache, int main_words_heuristic);
void BUI_string_search_add(BUIStringSearch *search, const char *str, void *user_data, int weight);
/* Filters and sorts the added items for `query`; writes at most `max` user
 * data pointers, best first. Returns the number of matches (may exceed max). */
int BUI_string_search_query(BUIStringSearch *search, const char *query, void **r_items, int max);
void BUI_string_search_free(BUIStringSearch *search);
/* blender::string_search::get_fuzzy_match_errors: -1 no match, else typo count. */
int BUI_string_search_fuzzy_errors(const char *query, const char *full);

#ifdef __cplusplus
}
#endif
