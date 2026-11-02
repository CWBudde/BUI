/* BUI seam: stand-in for the generated RNA prototypes. Only symbols actually
 * referenced by the toolkit are declared here.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#ifndef __cplusplus
#error "RNA prototypes are C++-only in BUI"
#endif

#include "RNA_types.hh"

namespace blender {
extern StructRNA *const RNA_NodeTree;
extern StructRNA *const RNA_ActionSlot;
}  // namespace blender

using blender::RNA_NodeTree;
using blender::RNA_ActionSlot;
