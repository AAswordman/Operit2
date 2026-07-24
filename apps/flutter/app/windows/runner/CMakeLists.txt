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
target_link_libraries(${BINARY_NAME} PRIVATE "dwmapi.lib")
target_link_libraries(${BINARY_NAME} PRIVATE "wtsapi32.lib" "iphlpapi.lib")
target_include_directories(${BINARY_NAME} PRIVATE "${CMAKE_SOURCE_DIR}")

set(OPERIT_REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../")
set(OPERIT_PLUGIN_SYNC_PYTHON "${OPERIT_REPO_ROOT}/.venv/Scripts/python.exe")
set(OPERIT_PLUGIN_SYNC_SCRIPT "${OPERIT_REPO_ROOT}/plugins/tools/sync_plugin_packages.py")
add_custom_target(sync_operit_plugins ALL
  COMMAND "${OPERIT_PLUGIN_SYNC_PYTHON}" "${OPERIT_PLUGIN_SYNC_SCRIPT}" --source buildin
  WORKING_DIRECTORY "${OPERIT_REPO_ROOT}"
)

set(OPERIT_FLUTTER_BRIDGE_CRATE "${CMAKE_CURRENT_SOURCE_DIR}/../../../native/operit-flutter-bridge")
find_program(OPERIT_CARGO_EXECUTABLE
  NAMES cargo cargo.exe
  HINTS "$ENV{USERPROFILE}/.cargo/bin" "$ENV{CARGO_HOME}/bin"
  REQUIRED
)
add_custom_target(operit_flutter_bridge ALL
  COMMAND ${CMAKE_COMMAND}
    "-DOPERIT_BUILD_CONFIG=$<CONFIG>"
    "-DOPERIT_CARGO_EXECUTABLE=${OPERIT_CARGO_EXECUTABLE}"
    "-DOPERIT_FLUTTER_BRIDGE_CRATE=${OPERIT_FLUTTER_BRIDGE_CRATE}"
    "-DOPERIT_OUTPUT_DIRECTORY=${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>"
    -P "${CMAKE_CURRENT_SOURCE_DIR}/operit_flutter_bridge_build.cmake"
)
add_dependencies(operit_flutter_bridge sync_operit_plugins)
add_dependencies(${BINARY_NAME} operit_flutter_bridge)

# Run the Flutter tool portions of the build. This must not be removed.
add_dependencies(${BINARY_NAME} flutter_assemble)
