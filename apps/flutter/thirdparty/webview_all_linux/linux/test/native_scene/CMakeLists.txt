cmake_minimum_required(VERSION 3.16)
project(webview_native_scene_test LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(FLUTTER_ENGINE_ARTIFACTS "" CACHE PATH "Directory containing the patched Linux GTK library and headers")
if(NOT EXISTS "${FLUTTER_ENGINE_ARTIFACTS}/libflutter_linux_gtk.so" OR
   NOT EXISTS "${FLUTTER_ENGINE_ARTIFACTS}/flutter_linux/fl_view.h")
  message(FATAL_ERROR "FLUTTER_ENGINE_ARTIFACTS must contain the native-composition GTK engine and its headers")
endif()

find_package(PkgConfig REQUIRED)
pkg_check_modules(GTK REQUIRED IMPORTED_TARGET gtk+-3.0)
add_library(flutter SHARED IMPORTED GLOBAL)
set_target_properties(flutter PROPERTIES
  IMPORTED_LOCATION "${FLUTTER_ENGINE_ARTIFACTS}/libflutter_linux_gtk.so"
  INTERFACE_INCLUDE_DIRECTORIES "${FLUTTER_ENGINE_ARTIFACTS}"
)

# Applies the language level required by the plugin's standard build contract.
function(apply_standard_settings target)
  target_compile_features(${target} PRIVATE cxx_std_17)
endfunction()

add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/../.." webview_plugin)
add_executable(native_scene_driver ../native_scene_driver.cc)
target_link_libraries(native_scene_driver PRIVATE flutter webview_all_linux_plugin PkgConfig::GTK)
