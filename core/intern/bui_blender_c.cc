/* C wrappers over the Blender C++ code vendored in core/ (see BUI_blender_c.h).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <cmath>

#include "BLI_expr_pylike_eval.h"
#include "BLI_string_search.hh"

#include "BUI_blender_c.h"

using namespace blender;

int BUI_expr_eval(const char *expr,
                  const char **param_names,
                  const double *param_values,
                  int params_len,
                  double *r_value)
{
  if (expr == nullptr) {
    return 0;
  }
  ExprPyLike_Parsed *parsed = BLI_expr_pylike_parse(expr, param_names, params_len);
  double value = 0.0;
  int ok = 0;
  if (BLI_expr_pylike_is_valid(parsed)) {
    ok = BLI_expr_pylike_eval(parsed, param_values, params_len, &value) == EXPR_PYLIKE_SUCCESS &&
         std::isfinite(value);
  }
  BLI_expr_pylike_free(parsed);
  if (ok) {
    *r_value = value;
  }
  return ok;
}

struct BUIStringSearch {
  string_search::StringSearch<void> search;
  BUIStringSearch(const string_search::RecentCache *cache, string_search::MainWordsHeuristic h)
      : search(cache, h)
  {
  }
};

BUIStringSearch *BUI_string_search_new(const void *recent_cache, int main_words_heuristic)
{
  const string_search::MainWordsHeuristic h =
      main_words_heuristic == BUI_SEARCH_MAIN_LAST ? string_search::MainWordsHeuristic::LastGroup :
      main_words_heuristic == BUI_SEARCH_MAIN_ALL  ? string_search::MainWordsHeuristic::All :
                                                     string_search::MainWordsHeuristic::FirstGroup;
  return new BUIStringSearch(static_cast<const string_search::RecentCache *>(recent_cache), h);
}

void BUI_string_search_add(BUIStringSearch *search, const char *str, void *user_data, int weight)
{
  search->search.add(StringRef(str ? str : ""), user_data, weight);
}

int BUI_string_search_query(BUIStringSearch *search, const char *query, void **r_items, int max)
{
  const Vector<void *> found = search->search.query(StringRef(query ? query : ""));
  for (int i = 0; i < int(found.size()) && i < max; i++) {
    r_items[i] = found[i];
  }
  return int(found.size());
}

void BUI_string_search_free(BUIStringSearch *search)
{
  delete search;
}

int BUI_string_search_fuzzy_errors(const char *query, const char *full)
{
  return string_search::get_fuzzy_match_errors(StringRef(query), StringRef(full));
}
