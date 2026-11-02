# BUI Qt backend — renders the toolkit through QPainter.
# SPDX-License-Identifier: GPL-2.0-or-later

find_package(Qt6 QUIET COMPONENTS Widgets)
if(Qt6_FOUND)
  message(STATUS "BUI: Qt backend using Qt6 ${Qt6_VERSION}")
  set(BUI_QT_TARGET Qt6::Widgets)
else()
  find_package(Qt5 QUIET COMPONENTS Widgets)
  if(Qt5_FOUND)
    message(STATUS "BUI: Qt backend using Qt5 ${Qt5_VERSION}")
    set(BUI_QT_TARGET Qt5::Widgets)
  else()
    message(FATAL_ERROR "BUI_BUILD_QT_BACKEND is ON but no Qt5/Qt6 with Widgets was found")
  endif()
endif()

set(SRC
  BuiQtSink.h
  BuiQtSink.cpp
  BuiWidget.h
  BuiWidget.cpp
)

add_library(bui_qt STATIC ${SRC})
add_library(bui::qt ALIAS bui_qt)
target_include_directories(bui_qt PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(bui_qt PUBLIC bui_api bui_gpu ${BUI_QT_TARGET})
