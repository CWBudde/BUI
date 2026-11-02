/* BUI Qt widget: a QWidget that hosts the BUI toolkit's drawing.
 *
 * No Q_OBJECT is used, so no moc pass is required. Connect to Qt signals by
 * subclassing with Q_OBJECT if you prefer signals over callbacks.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include <QWidget>

#include <functional>

#include "BuiQtSink.h"

namespace BUI {

class BuiWidget : public QWidget {
 public:
  explicit BuiWidget(QWidget *parent = nullptr);

  /** Called during each paint pass with the widget's current size; draw your
   * BUI content here (or set render_func instead). */
  std::function<void(int width, int height)> render_func;

  /** Mouse position in widget pixels (useful to hit-test BUI layouts). */
  std::function<void(int x, int y)> mouse_moved;
  std::function<void(int x, int y, int button, bool pressed)> mouse_button;

 protected:
  void paintEvent(QPaintEvent *event) override;
  void resizeEvent(QResizeEvent *event) override;
  void mouseMoveEvent(QMouseEvent *event) override;
  void mousePressEvent(QMouseEvent *event) override;
  void mouseReleaseEvent(QMouseEvent *event) override;
};

}  // namespace BUI
