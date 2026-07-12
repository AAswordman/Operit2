cmake_minimum_required(VERSION 3.15)

if(POLICY CMP0175)
  cmake_policy(SET CMP0175 NEW)
endif()

set(PROJECT_NAME "webview_all_windows")

set(WIL_VERSION "1.0.220914.1")
set(WEBVIEW_VERSION "1.0.1418.22")

message(VERBOSE "CMake system version is ${CMAKE_SYSTEM_VERSION} (using SDK ${CMAKE_VS_WINDOWS_TARGET_PLATFORM_VERSION})")

project(${PROJECT_NAME} LANGUAGES CXX)

# This value is used when generating builds using this plugin, so it must
# not be changed
set(PLUGIN_NAME "webview_all_windows_plugin")

set(NUGET_URL https://dist.nuget.org/win-x86-commandline/v5.10.0/nuget.exe)
set(NUGET_SHA256 852b71cc8c8c2d40d09ea49d321ff56fd2397b9d6ea9f96e532530307bbbafd3)

find_program(NUGET nuget)
if(NOT NUGET)
  message(NOTICE "Nuget is not installed.")
  set(NUGET ${CMAKE_BINARY_DIR}/nuget.exe)
  if (NOT EXISTS ${NUGET})
    message(NOTICE "Attempting to download nuget.")
    file(DOWNLOAD ${NUGET_URL} ${NUGET})
  endif()

  file(SHA256 ${NUGET} NUGET_DL_HASH)
  if (NOT NUGET_DL_HASH STREQUAL NUGET_SHA256)
    message(FATAL_ERROR "Integrity check for ${NUGET} failed.")
  endif()
endif()

set(DEPS_TARGET_NAME "webview_all_windows_deps")
add_custom_target(${DEPS_TARGET_NAME} ALL)

add_custom_command(
  TARGET ${DEPS_TARGET_NAME} PRE_BUILD
  COMMAND ${CMAKE_COMMAND} -E echo "Downloading dependencies..."
  COMMAND ${NUGET} install Microsoft.Windows.ImplementationLibrary -Version ${WIL_VERSION} -ExcludeVersion -OutputDirectory ${CMAKE_BINARY_DIR}/packages
  COMMAND ${NUGET} install Microsoft.Web.WebView2 -Version ${WEBVIEW_VERSION} -ExcludeVersion -OutputDirectory ${CMAKE_BINARY_DIR}/packages
  COMMENT "Downloading and installing NuGet packages"
)

set(PLUGIN_SOURCES
  "generated/windows_webview_api.g.cpp"
  "src/plugin/webview_windows_plugin.cc"
  "src/plugin/windows_host_api.cc"
  "src/webview/webview.cc"
  "src/webview/webview_bridge.cc"
  "src/webview/webview_host.cc"
  "src/rendering/graphics_context.cc"
  "src/rendering/texture_bridge.cc"
  "src/rendering/texture_bridge_gpu.cc"
  "src/platform/webview_platform.cc"
  "src/platform/winrt_runtime.cc"
  "src/util/direct3d11.interop.cc"
  "src/util/string_converter.cc"
)

add_library(${PLUGIN_NAME} SHARED ${PLUGIN_SOURCES})
add_dependencies(${PLUGIN_NAME} ${DEPS_TARGET_NAME})

if(MSVC)
  target_compile_options(${PLUGIN_NAME} PRIVATE "/await")
endif()

apply_standard_settings(${PLUGIN_NAME})

target_compile_features(${PLUGIN_NAME} PUBLIC cxx_std_20)

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden
)

target_link_libraries(${PLUGIN_NAME} PRIVATE
  ${CMAKE_BINARY_DIR}/packages/Microsoft.Web.WebView2/build/native/Microsoft.Web.WebView2.targets
  ${CMAKE_BINARY_DIR}/packages/Microsoft.Windows.ImplementationLibrary/build/native/Microsoft.Windows.ImplementationLibrary.targets
  flutter
  flutter_wrapper_plugin
)

target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)
target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
)
target_include_directories(${PLUGIN_NAME} PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}"
  "${CMAKE_CURRENT_SOURCE_DIR}/include"
  "${CMAKE_CURRENT_SOURCE_DIR}/generated"
  "${CMAKE_CURRENT_SOURCE_DIR}/src"
)

set(webview_all_windows_bundled_libraries
  PARENT_SCOPE
)
