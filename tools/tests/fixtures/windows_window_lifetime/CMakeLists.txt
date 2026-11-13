cmake_minimum_required(VERSION 3.14)
project(window_lifetime_test LANGUAGES CXX)

set(plugin "${OPERIT_REPO_ROOT}/apps/flutter/thirdparty/desktop_multi_window/windows")
set(flutter "${OPERIT_REPO_ROOT}/apps/flutter/app/windows/flutter/ephemeral")
add_executable(window_lifetime_test
  window_lifetime_test.cpp
  "${plugin}/multi_window_manager.cc"
  "${plugin}/flutter_window.cc"
  "${plugin}/win32_window.cpp"
)
target_compile_features(window_lifetime_test PRIVATE cxx_std_17)
target_compile_definitions(window_lifetime_test PRIVATE
  FLUTTER_PLUGIN_IMPL UNICODE _UNICODE NOMINMAX)
target_include_directories(window_lifetime_test PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}" "${plugin}"
  "${flutter}" "${flutter}/cpp_client_wrapper/include")
target_link_libraries(window_lifetime_test PRIVATE
  "${flutter}/flutter_windows.dll.lib" dwmapi shell32)

add_executable(keyboard_message_test keyboard_message_test.cpp)
target_compile_features(keyboard_message_test PRIVATE cxx_std_17)
target_compile_definitions(keyboard_message_test PRIVATE UNICODE _UNICODE NOMINMAX)
target_include_directories(keyboard_message_test PRIVATE
  "${OPERIT_REPO_ROOT}/apps/flutter/app/windows/runner")
target_link_libraries(keyboard_message_test PRIVATE user32)
