# Project-level configuration.
cmake_minimum_required(VERSION 3.14)
project(operit2 LANGUAGES CXX)

# The name of the executable created for the application. Change this to change
# the on-disk name of your application.
set(BINARY_NAME "operit2")

# Explicitly opt in to modern CMake behaviors to avoid warnings with recent
# versions of CMake.
cmake_policy(VERSION 3.14...3.25)

# Define build configuration option.
get_property(IS_MULTICONFIG GLOBAL PROPERTY GENERATOR_IS_MULTI_CONFIG)
if(IS_MULTICONFIG)
  set(CMAKE_CONFIGURATION_TYPES "Debug;Profile;Release"
    CACHE STRING "" FORCE)
else()
  if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE "Debug" CACHE
      STRING "Flutter build mode" FORCE)
    set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS
      "Debug" "Profile" "Release")
  endif()
endif()
# Define settings for the Profile build mode.
set(CMAKE_EXE_LINKER_FLAGS_PROFILE "${CMAKE_EXE_LINKER_FLAGS_RELEASE}")
set(CMAKE_SHARED_LINKER_FLAGS_PROFILE "${CMAKE_SHARED_LINKER_FLAGS_RELEASE}")
set(CMAKE_C_FLAGS_PROFILE "${CMAKE_C_FLAGS_RELEASE}")
set(CMAKE_CXX_FLAGS_PROFILE "${CMAKE_CXX_FLAGS_RELEASE}")

# Use Unicode for all projects.
add_definitions(-DUNICODE -D_UNICODE)

# Compilation settings that should be applied to most targets.
#
# Be cautious about adding new options here, as plugins use this function by
# default. In most cases, you should add new options to specific targets instead
# of modifying this function.
function(APPLY_STANDARD_SETTINGS TARGET)
  target_compile_features(${TARGET} PUBLIC cxx_std_17)
  target_compile_options(${TARGET} PRIVATE /W4 /WX /wd"4100")
  target_compile_options(${TARGET} PRIVATE /EHsc)
  target_compile_definitions(${TARGET} PRIVATE "_HAS_EXCEPTIONS=0")
  # Third-party Windows plugins still use the MSVC experimental coroutine header.
  target_compile_definitions(${TARGET} PRIVATE "_SILENCE_EXPERIMENTAL_COROUTINE_DEPRECATION_WARNINGS")
  target_compile_definitions(${TARGET} PRIVATE "$<$<CONFIG:Debug>:_DEBUG>")
endfunction()

# Flutter library and tool build rules.
set(FLUTTER_MANAGED_DIR "${CMAKE_CURRENT_SOURCE_DIR}/flutter")
find_program(OPERIT_CARGO_EXECUTABLE
  NAMES cargo cargo.exe
  HINTS "$ENV{USERPROFILE}/.cargo/bin" "$ENV{CARGO_HOME}/bin"
)
if(OPERIT_CARGO_EXECUTABLE)
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
  endif()
  if(OPERIT_CARGO_HOME)
    list(APPEND FLUTTER_TOOL_ENVIRONMENT "CARGO_HOME=${OPERIT_CARGO_HOME}")
  endif()
  if(OPERIT_RUSTUP_HOME)
    list(APPEND FLUTTER_TOOL_ENVIRONMENT "RUSTUP_HOME=${OPERIT_RUSTUP_HOME}")
  endif()
  if(DEFINED ENV{RUSTUP_TOOLCHAIN} AND NOT "$ENV{RUSTUP_TOOLCHAIN}" STREQUAL "")
    list(APPEND FLUTTER_TOOL_ENVIRONMENT "RUSTUP_TOOLCHAIN=$ENV{RUSTUP_TOOLCHAIN}")
  else()
    list(APPEND FLUTTER_TOOL_ENVIRONMENT "RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc")
  endif()
  if(DEFINED ENV{JAVA_HOME} AND NOT "$ENV{JAVA_HOME}" STREQUAL "")
    list(APPEND FLUTTER_TOOL_ENVIRONMENT "JAVA_HOME=$ENV{JAVA_HOME}")
  endif()
endif()
add_subdirectory(${FLUTTER_MANAGED_DIR})

# Application build; see runner/CMakeLists.txt.
add_subdirectory("runner")


# Generated plugin build rules, which manage building the plugins and adding
# them to the application.
include(flutter/generated_plugins.cmake)


# === Installation ===
# Support files are copied into place next to the executable, so that it can
# run in place. This is done instead of making a separate bundle (as on Linux)
# so that building and running from within Visual Studio will work.
set(BUILD_BUNDLE_DIR "$<TARGET_FILE_DIR:${BINARY_NAME}>")
# Make the "install" step default, as it's required to run.
set(CMAKE_VS_INCLUDE_INSTALL_TO_DEFAULT_BUILD 1)
if(CMAKE_INSTALL_PREFIX_INITIALIZED_TO_DEFAULT)
  set(CMAKE_INSTALL_PREFIX "${BUILD_BUNDLE_DIR}" CACHE PATH "..." FORCE)
endif()

set(INSTALL_BUNDLE_DATA_DIR "${CMAKE_INSTALL_PREFIX}/data")
set(INSTALL_BUNDLE_LIB_DIR "${CMAKE_INSTALL_PREFIX}")

install(TARGETS ${BINARY_NAME} RUNTIME DESTINATION "${CMAKE_INSTALL_PREFIX}"
  COMPONENT Runtime)

install(FILES "${FLUTTER_ICU_DATA_FILE}" DESTINATION "${INSTALL_BUNDLE_DATA_DIR}"
  COMPONENT Runtime)

install(FILES "${FLUTTER_LIBRARY}" DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
  COMPONENT Runtime)

set(OPERIT_FLUTTER_BRIDGE_CRATE "${CMAKE_CURRENT_SOURCE_DIR}/../../native/operit-flutter-bridge")
install(FILES
  "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/$<IF:$<CONFIG:Debug>,debug,release>/operit_flutter_bridge.dll"
  "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/$<IF:$<CONFIG:Debug>,debug,release>/operit_local_llm.dll"
  DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
  COMPONENT Runtime)

if(PLUGIN_BUNDLED_LIBRARIES)
  install(FILES "${PLUGIN_BUNDLED_LIBRARIES}"
    DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
    COMPONENT Runtime)
endif()

# Copy the native assets provided by the build.dart from all packages.
set(NATIVE_ASSETS_DIR "${PROJECT_BUILD_DIR}native_assets/windows/")
install(DIRECTORY "${NATIVE_ASSETS_DIR}"
   DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
   COMPONENT Runtime)

# Release installs clean the assets directory so removed assets cannot remain
# in a package. Debug/Profile installs keep the existing directory and let
# CMake update changed files in place; deleting and re-copying the full bundle
# on every edit is needlessly expensive during development.
set(FLUTTER_ASSET_DIR_NAME "flutter_assets")
install(CODE "
  if(NOT \"\${CMAKE_INSTALL_CONFIG_NAME}\" STREQUAL \"Debug\" AND
     NOT \"\${CMAKE_INSTALL_CONFIG_NAME}\" STREQUAL \"Profile\")
    file(REMOVE_RECURSE \"${INSTALL_BUNDLE_DATA_DIR}/${FLUTTER_ASSET_DIR_NAME}\")
  endif()
  " COMPONENT Runtime)
install(DIRECTORY "${PROJECT_BUILD_DIR}/${FLUTTER_ASSET_DIR_NAME}"
  DESTINATION "${INSTALL_BUNDLE_DATA_DIR}" COMPONENT Runtime)

# Install the AOT library on non-Debug builds only.
install(FILES "${AOT_LIBRARY}" DESTINATION "${INSTALL_BUNDLE_DATA_DIR}"
  CONFIGURATIONS Profile;Release
  COMPONENT Runtime)
