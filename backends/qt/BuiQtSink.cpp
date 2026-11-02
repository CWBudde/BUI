/* BUI Qt backend implementation.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "BuiQtSink.h"

#include <QPen>

#include <cmath>

namespace BUI {

static QColor to_qcolor(const float rgba[4])
{
  auto clamp01 = [](float v) {
    return static_cast<int>(std::max(0.0f, std::min(1.0f, v)) * 255.0f + 0.5f);
  };
  return QColor(clamp01(rgba[0]), clamp01(rgba[1]), clamp01(rgba[2]), clamp01(rgba[3]));
}

QtSink::QtSink(QPainter *painter, int viewport_w, int viewport_h) : painter_(painter)
{
  sink_.user = this;
  sink_.begin_frame = &QtSink::begin_frame;
  sink_.scissor = &QtSink::scissor;
  sink_.triangles = &QtSink::triangles;
  sink_.lines = &QtSink::lines;
  sink_.points = &QtSink::points;
  sink_.text = &QtSink::text;
  sink_.end_frame = &QtSink::end_frame;
  begin_frame(this, viewport_w, viewport_h);
}

void QtSink::begin_frame(void *user, int w, int h)
{
  auto *self = static_cast<QtSink *>(user);
  if (w > 0 && h > 0) {
    bui_set_viewport(w, h);
  }
  self->painter_->save();
}

void QtSink::scissor(void *user, int x, int y, int w, int h)
{
  auto *self = static_cast<QtSink *>(user);
  if (w > 0 && h > 0) {
    self->painter_->setClipRect(QRect(x, y, w, h), Qt::IntersectClip);
  }
  else {
    self->painter_->setClipRect(QRect(), Qt::NoClip);
  }
}

void QtSink::triangles(void *user, const float (*coords)[2], const float (*colors)[4], int count)
{
  auto *self = static_cast<QtSink *>(user);
  QPainter &p = *self->painter_;
  p.setPen(Qt::NoPen);
  for (int i = 0; i < count; i += 3) {
    p.setBrush(to_qcolor(colors[i]));
    QPolygonF tri;
    for (int v = 0; v < 3 && i + v < count; v++) {
      tri.append(QPointF(coords[i + v][0], coords[i + v][1]));
    }
    p.drawPolygon(tri);
  }
}

void QtSink::lines(void *user, const float (*coords)[2], const float (*colors)[4], int count)
{
  auto *self = static_cast<QtSink *>(user);
  QPainter &p = *self->painter_;
  for (int i = 0; i + 1 < count; i += 2) {
    p.setPen(QPen(to_qcolor(colors[i]), 1));
    p.drawLine(QPointF(coords[i][0], coords[i][1]), QPointF(coords[i + 1][0], coords[i + 1][1]));
  }
}

void QtSink::points(void *user, const float (*coords)[2], const float (*colors)[4], int count, float size)
{
  auto *self = static_cast<QtSink *>(user);
  QPainter &p = *self->painter_;
  p.setPen(Qt::NoPen);
  const float d = size > 0.0f ? size : 1.0f;
  for (int i = 0; i < count; i++) {
    p.setBrush(to_qcolor(colors[i]));
    p.drawEllipse(QRectF(coords[i][0] - d / 2.0f, coords[i][1] - d / 2.0f, d, d));
  }
}

void QtSink::text(void *user, float x, float y, const char *text, float size, const float rgba[4])
{
  auto *self = static_cast<QtSink *>(user);
  QPainter &p = *self->painter_;
  p.setPen(QPen(to_qcolor(rgba), 1));
  QFont f = p.font();
  f.setPixelSize(size > 0.0f ? int(size) : 12);
  p.setFont(f);
  p.drawText(QPointF(x, y), QString::fromUtf8(text));
}

void QtSink::end_frame(void *user)
{
  static_cast<QtSink *>(user)->painter_->restore();
}

}  // namespace BUI
