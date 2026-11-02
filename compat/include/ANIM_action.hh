/* BUI seam: minimal animrig Action/Slot wrappers.
 * Only what the toolkit needs for search-item icons: the DNA structs carry
 * the data; the wrapper adds no members so pointers stay interchangeable.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "DNA_action_types.h"

namespace blender::animrig {

class Slot : public ActionSlot {
 public:
  Slot() = default;
};

}  // namespace blender::animrig

namespace blender {
inline animrig::Slot &ActionSlot::wrap()
{
  return *static_cast<animrig::Slot *>(this);
}
inline const animrig::Slot &ActionSlot::wrap() const
{
  return *static_cast<const animrig::Slot *>(this);
}
}  // namespace blender
