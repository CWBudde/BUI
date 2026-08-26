/* Minimal stand-in for the {fmt} library, used only so the vendored BLI
 * headers compile inside BUI. Formatting behavior is NOT implemented;
 * replace with a real fmt build when needed.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <string>
#include <sstream>

namespace fmt {

template<typename... Args> inline std::string format(const char *fmt_str, Args &&...args)
{
  (void)fmt_str;
  (void)((int)sizeof...(args));
  return std::string(fmt_str);
}

template<typename T> inline std::string to_string(const T &value)
{
  std::ostringstream ss;
  ss << value;
  return ss.str();
}

}  // namespace fmt
