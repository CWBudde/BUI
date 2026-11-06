# The optional parts of BUI (features.json, the one table cargo and `bui build`
# read too): an option each, ON by default unless the table says otherwise.
# Off, a part's sources aren't compiled (it costs no binary size) and its
# API stays, reporting it missing. bui_features(<target>) defines every
# option that is ON for the target's sources (BUI_IMAGE_PNG, BUI_VIDEO, ...).
#
# SPDX-License-Identifier: GPL-2.0-or-later

file(READ "${CMAKE_CURRENT_LIST_DIR}/features.json" _bui_features_json)
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${CMAKE_CURRENT_LIST_DIR}/features.json")
string(JSON _bui_nfeatures LENGTH "${_bui_features_json}" features)
math(EXPR _bui_last "${_bui_nfeatures} - 1")
set(BUI_FEATURE_OPTIONS "")
foreach(_i RANGE ${_bui_last})
  string(JSON _opt GET "${_bui_features_json}" features ${_i} option)
  string(JSON _def GET "${_bui_features_json}" features ${_i} default)
  string(JSON _doc GET "${_bui_features_json}" features ${_i} doc)
  if(_opt STREQUAL "BUI_BUILD_IMGUI" AND NOT (CMAKE_CXX_COMPILER AND EXISTS "${CMAKE_CURRENT_LIST_DIR}/bindings/imgui/CMakeLists.txt"))
    set(_def OFF) # C-only source copies (the vendored crate sources) have no ImGui
  endif()
  option(${_opt} "${_doc}" ${_def})
  list(APPEND BUI_FEATURE_OPTIONS ${_opt})
  string(JSON _name GET "${_bui_features_json}" features ${_i} name)
  set(_bui_option_of_${_name} ${_opt})
endforeach()

# a part that needs another (video-dav1d needs video): checked here, at the boundary
foreach(_i RANGE ${_bui_last})
  string(JSON _opt GET "${_bui_features_json}" features ${_i} option)
  string(JSON _nreq ERROR_VARIABLE _noreq LENGTH "${_bui_features_json}" features ${_i} requires)
  if(${_opt} AND NOT _noreq AND _nreq GREATER 0)
    math(EXPR _rlast "${_nreq} - 1")
    foreach(_k RANGE ${_rlast})
      string(JSON _req GET "${_bui_features_json}" features ${_i} requires ${_k})
      if(NOT ${_bui_option_of_${_req}})
        message(FATAL_ERROR "${_opt} needs ${_bui_option_of_${_req}}")
      endif()
    endforeach()
  endif()
endforeach()

function(bui_features target)
  foreach(_opt ${BUI_FEATURE_OPTIONS})
    if(${_opt})
      target_compile_definitions(${target} PRIVATE ${_opt})
    endif()
  endforeach()
endfunction()
