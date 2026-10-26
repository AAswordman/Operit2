cmake_minimum_required(VERSION 3.10)
set(PROJECT_NAME "webview_all_linux")
project(${PROJECT_NAME} LANGUAGES CXX)

cmake_policy(VERSION 3.10...3.24)

set(PLUGIN_NAME "${PROJECT_NAME}_plugin")

find_package(PkgConfig REQUIRED)
pkg_check_modules(WEBKIT REQUIRED IMPORTED_TARGET webkit2gtk-4.1)

add_library(${PLUGIN_NAME} SHARED
  "src/common/method_channel_utils.cc"
  "src/platform/flutter_view.cc"
  "src/plugin/webview_all_linux_plugin.cc"
  "src/plugin/root_channel.cc"
  "src/webview/linux_webview.cc"
  "src/webview/webview_pointer.cc"
  "src/webview/download_policy.cc"
  "src/webview/local_resources.cc"
  "src/webview/webview_javascript.cc"
  "src/webview/webview_method_handler.cc"
)

apply_standard_settings(${PLUGIN_NAME})
set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden
)
target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)
target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
target_include_directories(${PLUGIN_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
  "${CMAKE_CURRENT_SOURCE_DIR}/src"
)
target_link_libraries(${PLUGIN_NAME} PRIVATE flutter)
target_link_libraries(${PLUGIN_NAME} PRIVATE PkgConfig::GTK)
target_link_libraries(${PLUGIN_NAME} PRIVATE PkgConfig::WEBKIT)
