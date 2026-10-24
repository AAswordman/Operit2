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

find_program(NUGET nuget
  HINTS
    "$ENV{CARGO_HOME}/bin"
    "${CMAKE_BINARY_DIR}"
)
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
set(DEPS_OUTPUT_DIR "${CMAKE_BINARY_DIR}/packages")
set(DEPS_STAMP "${DEPS_OUTPUT_DIR}/.webview_all_windows_deps.stamp")
set(WIL_TARGET_FILE
  "${DEPS_OUTPUT_DIR}/Microsoft.Windows.ImplementationLibrary/build/native/Microsoft.Windows.ImplementationLibrary.targets")
set(WEBVIEW_TARGET_FILE
  "${DEPS_OUTPUT_DIR}/Microsoft.Web.WebView2/build/native/Microsoft.Web.WebView2.targets")
set(WIL_INCLUDE_DIR
  "${DEPS_OUTPUT_DIR}/Microsoft.Windows.ImplementationLibrary/include")
set(WEBVIEW_INCLUDE_DIR
  "${DEPS_OUTPUT_DIR}/Microsoft.Web.WebView2/build/native/include")

string(TOLOWER "${CMAKE_VS_PLATFORM_NAME}" WEBVIEW_ARCH)
if(WEBVIEW_ARCH STREQUAL "win32")
  set(WEBVIEW_ARCH "x86")
elseif(NOT WEBVIEW_ARCH STREQUAL "x64" AND NOT WEBVIEW_ARCH STREQUAL "arm64")
  message(FATAL_ERROR "Unsupported WebView2 architecture: ${CMAKE_VS_PLATFORM_NAME}")
endif()

set(WEBVIEW_LIBRARY_DIR
  "${DEPS_OUTPUT_DIR}/Microsoft.Web.WebView2/build/native/${WEBVIEW_ARCH}")
set(WEBVIEW_LOADER_IMPORT_LIBRARY
  "${WEBVIEW_LIBRARY_DIR}/WebView2Loader.dll.lib")
set(WEBVIEW_LOADER_DLL
  "${WEBVIEW_LIBRARY_DIR}/WebView2Loader.dll")

# NuGet installation used to be a PRE_BUILD command on an ALL target, so it
# ran for every Flutter build even when both packages were already present.
# Track the package targets instead; a missing package or version change then
# still causes a fresh install.
add_custom_command(
  OUTPUT
    "${DEPS_STAMP}"
    "${WIL_TARGET_FILE}"
    "${WEBVIEW_TARGET_FILE}"
    "${WIL_INCLUDE_DIR}/wil/com.h"
    "${WEBVIEW_INCLUDE_DIR}/WebView2.h"
    "${WEBVIEW_INCLUDE_DIR}/WebView2EnvironmentOptions.h"
    "${WEBVIEW_LOADER_IMPORT_LIBRARY}"
    "${WEBVIEW_LOADER_DLL}"
  COMMAND ${CMAKE_COMMAND} -E make_directory "${DEPS_OUTPUT_DIR}"
  COMMAND ${CMAKE_COMMAND} -E echo "Downloading dependencies..."
  COMMAND ${NUGET} install Microsoft.Windows.ImplementationLibrary -Version ${WIL_VERSION} -ExcludeVersion -OutputDirectory "${DEPS_OUTPUT_DIR}"
  COMMAND ${NUGET} install Microsoft.Web.WebView2 -Version ${WEBVIEW_VERSION} -ExcludeVersion -OutputDirectory "${DEPS_OUTPUT_DIR}"
  COMMAND ${CMAKE_COMMAND} -E touch "${DEPS_STAMP}"
  DEPENDS "${CMAKE_CURRENT_LIST_FILE}"
  COMMENT "Downloading and installing NuGet packages"
  VERBATIM
)
add_custom_target(${DEPS_TARGET_NAME}
  DEPENDS
    "${DEPS_STAMP}"
    "${WIL_TARGET_FILE}"
    "${WEBVIEW_TARGET_FILE}"
    "${WIL_INCLUDE_DIR}/wil/com.h"
    "${WEBVIEW_INCLUDE_DIR}/WebView2.h"
    "${WEBVIEW_INCLUDE_DIR}/WebView2EnvironmentOptions.h"
    "${WEBVIEW_LOADER_IMPORT_LIBRARY}"
    "${WEBVIEW_LOADER_DLL}"
)

set(PLUGIN_SOURCES
  "generated/windows_webview_api.g.cpp"
  "src/plugin/webview_windows_plugin.cc"
  "src/plugin/windows_host_api.cc"
  "src/webview/webview.cc"
  "src/webview/webview_bridge.cc"
  "src/webview/webview_host.cc"
  "src/platform/webview_platform.cc"
  "src/platform/winrt_runtime.cc"
  "src/util/string_converter.cc"
)

add_library(${PLUGIN_NAME} SHARED ${PLUGIN_SOURCES})
add_dependencies(${PLUGIN_NAME} ${DEPS_TARGET_NAME})

if(MSVC)
  # Parallel CL processes share the compiler PDB in Debug builds.
  # Serialize those writes through mspdbsrv to avoid MSVC C1041.
  target_compile_options(${PLUGIN_NAME} PRIVATE "/await" "/FS")
endif()

apply_standard_settings(${PLUGIN_NAME})

target_compile_features(${PLUGIN_NAME} PUBLIC cxx_std_20)

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden
)

target_link_libraries(${PLUGIN_NAME} PRIVATE
  "${WEBVIEW_LOADER_IMPORT_LIBRARY}"
  flutter
  flutter_wrapper_plugin
  windowscodecs
  comctl32
  ole32
  shlwapi
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
  "${WIL_INCLUDE_DIR}"
  "${WEBVIEW_INCLUDE_DIR}"
)

set(webview_all_windows_bundled_libraries
  "${WEBVIEW_LOADER_DLL}"
  PARENT_SCOPE
)
