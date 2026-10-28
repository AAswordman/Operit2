cmake_minimum_required(VERSION 3.14)
project(runner LANGUAGES CXX)

# Define the application target. To change its name, change BINARY_NAME in the
# top-level CMakeLists.txt, not the value here, or `flutter run` will no longer
# work.
#
# Any new source files that you add to the application should be added here.
add_executable(${BINARY_NAME} WIN32
  "crash_channel.cpp"
  "engine_channel_lifetime.cpp"
  "flutter_window.cpp"
  "main.cpp"
  "native_crash_report.cpp"
  "operit_runtime_channel.cpp"
  "system_audio_input_channel.cpp"
  "utils.cpp"
  "win32_window.cpp"
  "${FLUTTER_MANAGED_DIR}/generated_plugin_registrant.cc"
  "Runner.rc"
  "runner.exe.manifest"
)

# Apply the standard set of build settings. This can be removed for applications
# that need different build settings.
apply_standard_settings(${BINARY_NAME})

# Add preprocessor definitions for the build version.
target_compile_definitions(${BINARY_NAME} PRIVATE "FLUTTER_VERSION=\"${FLUTTER_VERSION}\"")
target_compile_definitions(${BINARY_NAME} PRIVATE "FLUTTER_VERSION_MAJOR=${FLUTTER_VERSION_MAJOR}")
target_compile_definitions(${BINARY_NAME} PRIVATE "FLUTTER_VERSION_MINOR=${FLUTTER_VERSION_MINOR}")
target_compile_definitions(${BINARY_NAME} PRIVATE "FLUTTER_VERSION_PATCH=${FLUTTER_VERSION_PATCH}")
target_compile_definitions(${BINARY_NAME} PRIVATE "FLUTTER_VERSION_BUILD=${FLUTTER_VERSION_BUILD}")

# Disable Windows macros that collide with C++ standard library functions.
target_compile_definitions(${BINARY_NAME} PRIVATE "NOMINMAX")

# Add dependency libraries and include directories. Add any application-specific
# dependencies here.
target_link_libraries(${BINARY_NAME} PRIVATE flutter flutter_wrapper_app)
target_link_libraries(${BINARY_NAME} PRIVATE "ole32.lib")
target_link_libraries(${BINARY_NAME} PRIVATE "propsys.lib" "shell32.lib")
target_link_libraries(${BINARY_NAME} PRIVATE "advapi32.lib")
target_link_libraries(${BINARY_NAME} PRIVATE "dwmapi.lib")
target_link_libraries(${BINARY_NAME} PRIVATE "wtsapi32.lib" "iphlpapi.lib")
target_link_libraries(${BINARY_NAME} PRIVATE "gdiplus.lib" "dbghelp.lib")
target_include_directories(${BINARY_NAME} PRIVATE "${CMAKE_SOURCE_DIR}")

set(OPERIT_REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../")
set(OPERIT_PLUGIN_SYNC_PYTHON "${OPERIT_REPO_ROOT}/.venv/Scripts/python.exe")
set(OPERIT_PLUGIN_SYNC_SCRIPT "${OPERIT_REPO_ROOT}/plugins/tools/sync_plugin_packages.py")
set(OPERIT_FLUTTER_BRIDGE_CRATE "${CMAKE_CURRENT_SOURCE_DIR}/../../../native/operit-flutter-bridge")
find_program(OPERIT_CARGO_EXECUTABLE
  NAMES cargo cargo.exe
  HINTS "$ENV{USERPROFILE}/.cargo/bin" "$ENV{CARGO_HOME}/bin"
  REQUIRED
)
get_filename_component(OPERIT_CARGO_BIN_DIR "${OPERIT_CARGO_EXECUTABLE}" DIRECTORY)
get_filename_component(OPERIT_DERIVED_CARGO_HOME "${OPERIT_CARGO_BIN_DIR}" DIRECTORY)
get_filename_component(OPERIT_TOOLCHAIN_ROOT "${OPERIT_DERIVED_CARGO_HOME}" DIRECTORY)
if(DEFINED ENV{CARGO_HOME} AND NOT "$ENV{CARGO_HOME}" STREQUAL "")
  set(OPERIT_CARGO_HOME "$ENV{CARGO_HOME}")
else()
  set(OPERIT_CARGO_HOME "${OPERIT_DERIVED_CARGO_HOME}")
endif()
if(DEFINED ENV{RUSTUP_HOME} AND NOT "$ENV{RUSTUP_HOME}" STREQUAL "")
  set(OPERIT_RUSTUP_HOME "$ENV{RUSTUP_HOME}")
elseif(EXISTS "${OPERIT_TOOLCHAIN_ROOT}/rustup")
  set(OPERIT_RUSTUP_HOME "${OPERIT_TOOLCHAIN_ROOT}/rustup")
elseif(EXISTS "${OPERIT_TOOLCHAIN_ROOT}/.rustup")
  set(OPERIT_RUSTUP_HOME "${OPERIT_TOOLCHAIN_ROOT}/.rustup")
else()
  set(OPERIT_RUSTUP_HOME "")
endif()
set(OPERIT_RUSTUP_ENV)
if(OPERIT_CARGO_HOME)
  list(APPEND OPERIT_RUSTUP_ENV "CARGO_HOME=${OPERIT_CARGO_HOME}")
endif()
if(OPERIT_RUSTUP_HOME)
  list(APPEND OPERIT_RUSTUP_ENV "RUSTUP_HOME=${OPERIT_RUSTUP_HOME}")
endif()
if(DEFINED ENV{RUSTUP_TOOLCHAIN} AND NOT "$ENV{RUSTUP_TOOLCHAIN}" STREQUAL "")
  set(OPERIT_RUSTUP_TOOLCHAIN "$ENV{RUSTUP_TOOLCHAIN}")
else()
  set(OPERIT_RUSTUP_TOOLCHAIN "stable-x86_64-pc-windows-msvc")
endif()
list(APPEND OPERIT_RUSTUP_ENV "RUSTUP_TOOLCHAIN=${OPERIT_RUSTUP_TOOLCHAIN}")

# Keep the plugin sync step incremental.  A custom target without an output is
# always considered out of date by MSBuild, which otherwise runs the plugin
# SDK code generator on every Flutter build.
file(GLOB_RECURSE OPERIT_PLUGIN_SYNC_INPUTS CONFIGURE_DEPENDS
  "${OPERIT_REPO_ROOT}/plugins/packages/buildin/*"
  "${OPERIT_REPO_ROOT}/plugins/packages/external/*"
  "${OPERIT_REPO_ROOT}/plugins/types/*"
  "${OPERIT_REPO_ROOT}/core/crates/plugin/codegen/*"
  "${OPERIT_REPO_ROOT}/core/crates/plugin/sdk/*"
)
# Node package contents are not inputs to the Python plugin synchronizer.
# Keeping pnpm's package links out of DEPENDS avoids stale/missing node_modules paths in MSBuild.
list(FILTER OPERIT_PLUGIN_SYNC_INPUTS EXCLUDE REGEX "(^|[/\\\\])node_modules([/\\\\]|$)")
list(APPEND OPERIT_PLUGIN_SYNC_INPUTS
  "${OPERIT_PLUGIN_SYNC_SCRIPT}"
  "${OPERIT_REPO_ROOT}/core/Cargo.toml"
  "${OPERIT_REPO_ROOT}/core/Cargo.lock"
)
set(OPERIT_PLUGIN_SYNC_STAMP
  "${CMAKE_CURRENT_BINARY_DIR}/CMakeFiles/sync_operit_plugins-$<CONFIG>.stamp")
add_custom_command(OUTPUT "${OPERIT_PLUGIN_SYNC_STAMP}"
  COMMAND ${CMAKE_COMMAND} -E env ${OPERIT_RUSTUP_ENV}
    "${OPERIT_PLUGIN_SYNC_PYTHON}" "${OPERIT_PLUGIN_SYNC_SCRIPT}" --source runtime --no-hot-reload
  WORKING_DIRECTORY "${OPERIT_REPO_ROOT}"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_CURRENT_BINARY_DIR}/CMakeFiles"
  COMMAND "${CMAKE_COMMAND}" -E touch "${OPERIT_PLUGIN_SYNC_STAMP}"
  DEPENDS ${OPERIT_PLUGIN_SYNC_INPUTS}
  COMMENT "Synchronizing Operit plugin packages"
  VERBATIM
)
add_custom_target(sync_operit_plugins DEPENDS "${OPERIT_PLUGIN_SYNC_STAMP}")

# Cargo already performs incremental compilation, but the previous custom
# target forced Cargo to start on every Flutter build.  Track the Rust/native
# inputs and the copied DLL instead, so Dart-only edits skip this whole step.
set(OPERIT_NATIVE_SOURCE_ROOTS
  "${OPERIT_FLUTTER_BRIDGE_CRATE}/src"
  "${OPERIT_REPO_ROOT}/core/crates"
  "${OPERIT_REPO_ROOT}/core/vendor"
  "${OPERIT_REPO_ROOT}/hosts"
)
set(OPERIT_NATIVE_INPUTS)
foreach(OPERIT_NATIVE_SOURCE_ROOT IN LISTS OPERIT_NATIVE_SOURCE_ROOTS)
  file(GLOB_RECURSE OPERIT_NATIVE_SOURCE_FILES CONFIGURE_DEPENDS
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.rs"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.toml"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.lock"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.c"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.cc"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.cpp"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.cmake"
    "${OPERIT_NATIVE_SOURCE_ROOT}/CMakeLists.txt"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.h"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.hh"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.hpp"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.inc"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.json"
    "${OPERIT_NATIVE_SOURCE_ROOT}/*.hjson"
  )
  list(APPEND OPERIT_NATIVE_INPUTS ${OPERIT_NATIVE_SOURCE_FILES})
endforeach()
list(APPEND OPERIT_NATIVE_INPUTS
  "${OPERIT_FLUTTER_BRIDGE_CRATE}/Cargo.toml"
  "${OPERIT_FLUTTER_BRIDGE_CRATE}/Cargo.lock"
)
# Embedded plugin bytes must invalidate the copied bridge DLL when repackaged.
file(GLOB_RECURSE OPERIT_PLUGIN_ASSET_INPUTS CONFIGURE_DEPENDS
  "${OPERIT_REPO_ROOT}/core/crates/runtime/application/assets/plugins/*"
)
list(APPEND OPERIT_NATIVE_INPUTS ${OPERIT_PLUGIN_ASSET_INPUTS})
set(OPERIT_FLUTTER_BRIDGE_OUTPUT_DLL
  "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/operit_flutter_bridge.dll")
set(OPERIT_LOCAL_LLM_OUTPUT_DLL
  "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/operit_local_llm.dll")
set(OPERIT_FLUTTER_BRIDGE_STAMP
  "${CMAKE_CURRENT_BINARY_DIR}/CMakeFiles/operit_flutter_bridge-$<CONFIG>.stamp")
add_custom_command(
  OUTPUT
    "${OPERIT_FLUTTER_BRIDGE_OUTPUT_DLL}"
    "${OPERIT_LOCAL_LLM_OUTPUT_DLL}"
    "${OPERIT_FLUTTER_BRIDGE_STAMP}"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${CMAKE_CURRENT_BINARY_DIR}/CMakeFiles"
  COMMAND ${CMAKE_COMMAND}
    "-DOPERIT_BUILD_CONFIG=$<CONFIG>"
    "-DOPERIT_REPO_ROOT=${OPERIT_REPO_ROOT}"
    "-DOPERIT_PLUGIN_SYNC_PYTHON=${OPERIT_PLUGIN_SYNC_PYTHON}"
    "-DOPERIT_CARGO_EXECUTABLE=${OPERIT_CARGO_EXECUTABLE}"
    "-DOPERIT_CARGO_HOME=${OPERIT_CARGO_HOME}"
    "-DOPERIT_RUSTUP_HOME=${OPERIT_RUSTUP_HOME}"
    "-DOPERIT_RUSTUP_TOOLCHAIN=${OPERIT_RUSTUP_TOOLCHAIN}"
    "-DOPERIT_FLUTTER_BRIDGE_CRATE=${OPERIT_FLUTTER_BRIDGE_CRATE}"
    "-DOPERIT_OUTPUT_DIRECTORY=${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>"
    -P "${CMAKE_CURRENT_SOURCE_DIR}/operit_flutter_bridge_build.cmake"
  # Only mark the custom command complete after Cargo and the DLL copy both
  # succeed.  This allows a later build to retry if the running app locked the
  # previous DLL during the copy step.
  COMMAND "${CMAKE_COMMAND}" -E touch "${OPERIT_FLUTTER_BRIDGE_STAMP}"
  DEPENDS
    "${OPERIT_PLUGIN_SYNC_STAMP}"
    "${CMAKE_CURRENT_SOURCE_DIR}/operit_flutter_bridge_build.cmake"
    "${OPERIT_REPO_ROOT}/tools/build_scripts/cargo_build.py"
    ${OPERIT_NATIVE_INPUTS}
  COMMENT "Building the Operit Flutter Rust bridge"
  VERBATIM
)
add_custom_target(operit_flutter_bridge
  DEPENDS
    "${OPERIT_FLUTTER_BRIDGE_OUTPUT_DLL}"
    "${OPERIT_LOCAL_LLM_OUTPUT_DLL}"
    "${OPERIT_FLUTTER_BRIDGE_STAMP}"
)
add_dependencies(operit_flutter_bridge sync_operit_plugins)
add_dependencies(${BINARY_NAME} operit_flutter_bridge)

# Run the Flutter tool portions of the build. This must not be removed.
# Rust's build script generates the Dart proxy contract consumed by assembly.
add_dependencies(flutter_assemble operit_flutter_bridge)
add_dependencies(${BINARY_NAME} flutter_assemble)
