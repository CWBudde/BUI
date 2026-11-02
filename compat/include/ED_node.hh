/* BUI seam: node-socket drawing entry points used by the widget cache.
 * A host (or the node editor add-on) can provide real implementations; the
 * stubs draw nothing with the null renderer.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "DNA_node_types.h"
#include "DNA_vec_types.h"

namespace blender::ed::space_node {

void node_socket_draw(bNodeSocket *sock, const rcti *rect, const float color[4], float scale);
void node_draw_nodesocket(const rctf *rect,
                          const float color_inner[4],
                          const float color_outline[4],
                          float outline_thickness,
                          int shape,
                          float aspect);

}  // namespace blender::ed::space_node
