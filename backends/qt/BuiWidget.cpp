/* BUI Qt widget implementation.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "BuiWidget.h"

#include "bui_c_api.h"

namespace BUI {

BuiWidget::BuiWidget(QWidget *parent) : QWidget(parent)
{
  setMouseTracking(true);
  setMinimumSize(200, 150);
}

void BuiWidget::paintEvent(QPaintEvent * /*event*/)
{
  QPainter painter(this);
  QtSink sink(&painter, width(), height());
  bui_set_render_sink(sink.gpuSink());
  if (render_func) {
    render_func(width(), height());
  }
  bui_set_render_sink(nullptr);
}

void BuiWidget::resizeEvent(QResizeEvent *event)
{
  QWidget::resizeEvent(event);
  bui_set_viewport(width(), height());
}

void BuiWidget::mouseMoveEvent(QMouseEvent *event)
{
  QWidget::mouseMoveEvent(event);
  if (mouse_moved) {
    mouse_moved(event->position().x(), event->position().y());
  }
}

void BuiWidget::mousePressEvent(QMouseEvent *event)
{
  QWidget::mousePressEvent(event);
  if (mouse_button) {
    mouse_button(event->position().x(), event->position().y(), int(event->button()), true);
  }
}

void BuiWidget::mouseReleaseEvent(QMouseEvent *event)
{
  QWidget::mouseReleaseEvent(event);
  if (mouse_button) {
    mouse_button(event->position().x(), event->position().y(), int(event->button()), false);
  }
}

}  // namespace BUI
