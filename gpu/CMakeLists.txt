# bui_gpu - drawing abstraction with a null backend
#
# The UI toolkit draws exclusively through these seam headers. Ship a real
# renderer by implementing the same API (GL / Metal / software rasterizer).
#
# SPDX-License-Identifier: GPL-2.0-or-later

set(SRC
  intern/gpu_null.c
  intern/gpu_raster.c
  intern/gpu_d3d11.c # without BUI_GPU (features.json), the stubs that keep rasters on the CPU
)

add_library(bui_gpu STATIC ${SRC})
add_library(bui::gpu ALIAS bui_gpu)
bui_features(bui_gpu)

target_include_directories(bui_gpu PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
  ${BUI_SOURCE_DIR}/core/include
)
target_link_libraries(bui_gpu PUBLIC bui::core)
if(NOT MSVC)
  target_link_libraries(bui_gpu PRIVATE m)
endif()
# Text: the portable stb_truetype engine everywhere (Inter embedded as the
# default face, like Blender; glyph outlines feed the GPU's Slug renderer).
# BUI_TEXT_GDI=ON switches Windows to the native GDI engine (system fonts,
# ClearType-style hinting, no Slug). BUI_TEXT_STB is kept as a no-op.
option(BUI_TEXT_GDI "Use the native GDI text engine on Windows" OFF)
option(BUI_TEXT_STB "Deprecated: the portable engine is the default" ON)
# GPU text: Slug (glyph outlines in the pixel shader) or the per-glyph atlas.
# Runtime: BUI_gpu_set_slug() / BUI_SLUG=1|0; this sets the default.
option(BUI_SLUG "GPU renderer draws text with Slug by default (else a glyph atlas)" ON)
target_compile_definitions(bui_gpu PRIVATE BUI_SLUG_DEFAULT=$<BOOL:${BUI_SLUG}>)
if(BUI_TEXT_GDI)
  target_compile_definitions(bui_gpu PRIVATE BUI_TEXT_GDI=1)
endif()
# FreeType (extern/freetype, 2.13.3 as Blender uses): loads, hints and renders
# glyphs the way Blender's BLF does (light hinting, the variable Inter at the
# text's optical size, gamma 1.43). Compiled into bui_gpu itself
# (BUI_TEXT_FREETYPE, features.json).
if(BUI_TEXT_FREETYPE AND NOT BUI_TEXT_GDI)
  set(FT ${CMAKE_CURRENT_SOURCE_DIR}/extern/freetype)
  set(FT_SRC
    ${FT}/src/base/ftsystem.c ${FT}/src/base/ftinit.c ${FT}/src/base/ftdebug.c
    ${FT}/src/base/ftbase.c ${FT}/src/base/ftmm.c ${FT}/src/base/ftbitmap.c ${FT}/src/base/ftbbox.c
    ${FT}/src/truetype/truetype.c ${FT}/src/sfnt/sfnt.c ${FT}/src/autofit/autofit.c
    ${FT}/src/smooth/smooth.c ${FT}/src/psnames/psnames.c ${FT}/src/cff/cff.c
    ${FT}/src/psaux/psaux.c ${FT}/src/pshinter/pshinter.c
  )
  target_sources(bui_gpu PRIVATE ${FT_SRC})
  set_source_files_properties(${FT_SRC} PROPERTIES COMPILE_DEFINITIONS FT2_BUILD_LIBRARY)
  if(MSVC)
    set_source_files_properties(${FT_SRC} PROPERTIES COMPILE_OPTIONS "/w")
  else()
    set_source_files_properties(${FT_SRC} PROPERTIES COMPILE_OPTIONS "-w")
  endif()
  target_include_directories(bui_gpu PRIVATE ${FT}/bui ${FT}/include)
  target_compile_definitions(bui_gpu PRIVATE
    BUI_TEXT_FT=1
    "FT_CONFIG_OPTIONS_H=<bui_ftoption.h>"
    "FT_CONFIG_MODULES_H=<bui_ftmodule.h>"
  )
endif()
if(WIN32)
  # gpu_raster.c: GDI glyph-run text engine + native window present.
  target_link_libraries(bui_gpu PUBLIC gdi32 user32)
elseif(NOT APPLE AND BUI_GPU)
  # GPU renderer: OpenGL 3.3 (GLX windows, EGL offscreen) in gpu_gl.c instead
  # of Direct3D 11. libGL / libEGL / libX11 are dlopen()ed at run time, never
  # linked, so nothing GL is needed to build or import.
  target_sources(bui_gpu PRIVATE intern/gpu_gl.c)
  target_compile_definitions(bui_gpu PRIVATE BUI_GPU_GL=1)
  target_link_libraries(bui_gpu PRIVATE ${CMAKE_DL_LIBS})
endif()
