/* SPDX-FileCopyrightText: 2023 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/* BUI: copied from editors/interface/interface_string_search.cc: the recent
 * searches cache that ranks recently chosen menu-search / search-field items
 * first. BUI edits, marked "BUI:": no user preferences (U.flag) — a runtime
 * switch instead; the file path is given by the caller (no BKE_appdir); plain
 * std::fstream; C entry points at the end (bk_blender.h). */

#include <algorithm>
#include <fstream>
#include <optional>
#include <string>

#include "BLI_map.hh"
#include "BLI_string_search.hh"
#include "BLI_vector.hh"

#include "bk_blender.h"

namespace blender::ui::string_search {

using blender::string_search::RecentCache;

/* BUI: replaces `U.flag & USER_FLAG_RECENT_SEARCHES_DISABLE`. */
static bool recent_searches_disabled = false;

struct RecentCacheStorage {
  /**
   * Is incremented every time a search item has been selected. This is used to keep track of the
   * order of recent searches.
   */
  int logical_clock = 0;
  RecentCache cache;
};

static RecentCacheStorage &get_recent_cache_storage()
{
  static RecentCacheStorage storage;
  return storage;
}

void add_recent_search(const StringRef chosen_str)
{
  RecentCacheStorage &storage = get_recent_cache_storage();
  storage.cache.logical_time_by_str.add_overwrite(chosen_str, storage.logical_clock);
  storage.logical_clock++;
}

const RecentCache *get_recent_cache_or_null()
{
  if (recent_searches_disabled) {
    return nullptr;
  }
  RecentCacheStorage &storage = get_recent_cache_storage();
  return &storage.cache;
}

/* BUI: the path is the caller's (Blender: BKE_appdir user config + recent-searches.txt). */
void write_recent_searches_file(const char *path)
{
  if (recent_searches_disabled || path == nullptr) {
    return;
  }

  const RecentCacheStorage &storage = get_recent_cache_storage();
  /* Avoid creating an empty file. */
  if (storage.cache.logical_time_by_str.is_empty()) {
    return;
  }

  Vector<std::pair<int, std::string>> values;
  for (const auto item : storage.cache.logical_time_by_str.items()) {
    values.append({item.value, item.key});
  }
  std::sort(values.begin(), values.end());

  std::fstream file(path, std::ios::out);
  for (const auto &item : values) {
    file.write(item.second.data(), item.second.size());
    file.write("\n", 1);
  }
}

void read_recent_searches_file(const char *path)
{
  if (recent_searches_disabled || path == nullptr) {
    return;
  }

  RecentCacheStorage &storage = get_recent_cache_storage();
  storage.logical_clock = 0;
  storage.cache.logical_time_by_str.clear();

  std::fstream file(path, std::ios::in);
  std::string line;
  while (std::getline(file, line)) {
    storage.cache.logical_time_by_str.add_overwrite(line, storage.logical_clock);
    storage.logical_clock++;
  }
}

}  // namespace blender::ui::string_search

/* BUI: C entry points for the kit. */
using namespace blender::ui::string_search;

void bk_search_recent_add(const char *chosen)
{
  if (chosen && !recent_searches_disabled) {
    add_recent_search(blender::StringRef(chosen));
  }
}

const void *bk_search_recent_cache(void)
{
  return get_recent_cache_or_null();
}

void BK_recent_searches_enable(int enabled)
{
  recent_searches_disabled = !enabled;
}

void BK_recent_searches_clear(void)
{
  RecentCacheStorage &storage = get_recent_cache_storage();
  storage.logical_clock = 0;
  storage.cache.logical_time_by_str.clear();
}

void BK_recent_searches_load(const char *path)
{
  read_recent_searches_file(path);
}

void BK_recent_searches_save(const char *path)
{
  write_recent_searches_file(path);
}
