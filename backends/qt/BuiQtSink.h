/* BUI Qt backend: forwards BUI primitives to a QPainter.
 *
 * Implements the BuiGpuSink contract (gpu/include/GPU_backend_sink.h) on top
 * of Qt's 2D raster engine, so the toolkit renders inside any Qt window
 * without OpenGL.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <QColor>
#include <QPainter>
#include <QPolygonF>

#include "GPU_backend_sink.h"

/* Renders BUI primitives with a QPainter. Install per paint pass:
 *
 *   QPainter p(widget);
 *   BUI::QtSink sink(&p);
 *   bui_set_render_sink(sink.gpuSink());
 *   ... draw BUI content ...
 *   bui_set_render_sink(nullptr);
 */
namespace BUI {

class QtSink {
 public:
  explicit QtSink(QPainter *painter, int viewport_w = 0, int viewport_h = 0);

  const BuiGpuSink *gpuSink() const
  {
    return &sink_;
  }

 private:
  static void begin_frame(void *user, int w, int h);
  static void scissor(void *user, int x, int y, int w, int h);
  static void triangles(void *user, const float (*coords)[2], const float (*colors)[4], int count);
  static void lines(void *user, const float (*coords)[2], const float (*colors)[4], int count);
  static void points(
      void *user, const float (*coords)[2], const float (*colors)[4], int count, float size);
  static void text(
      void *user, float x, float y, const char *text, float size, const float rgba[4]);
  static void end_frame(void *user);

  QPainter *painter_;
  BuiGpuSink sink_;
};

}  // namespace BUI
