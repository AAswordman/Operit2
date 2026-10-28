# Builds and copies the Rust artifacts required by the selected Windows runner configuration.
file(MAKE_DIRECTORY "${OPERIT_OUTPUT_DIRECTORY}")

if(NOT OPERIT_RUSTUP_TOOLCHAIN)
  if(DEFINED ENV{RUSTUP_TOOLCHAIN} AND NOT "$ENV{RUSTUP_TOOLCHAIN}" STREQUAL "")
    set(OPERIT_RUSTUP_TOOLCHAIN "$ENV{RUSTUP_TOOLCHAIN}")
  else()
    set(OPERIT_RUSTUP_TOOLCHAIN "stable-x86_64-pc-windows-msvc")
  endif()
endif()

set(OPERIT_CARGO_ENV
  "RUSTFLAGS=-Awarnings"
  "CARGO_TERM_COLOR=never"
)
if(OPERIT_CARGO_HOME)
  list(APPEND OPERIT_CARGO_ENV "CARGO_HOME=${OPERIT_CARGO_HOME}")
endif()
if(OPERIT_RUSTUP_HOME)
  list(APPEND OPERIT_CARGO_ENV "RUSTUP_HOME=${OPERIT_RUSTUP_HOME}")
endif()
if(OPERIT_RUSTUP_TOOLCHAIN)
  list(APPEND OPERIT_CARGO_ENV "RUSTUP_TOOLCHAIN=${OPERIT_RUSTUP_TOOLCHAIN}")
endif()
if(DEFINED ENV{JAVA_HOME} AND NOT "$ENV{JAVA_HOME}" STREQUAL "")
  list(APPEND OPERIT_CARGO_ENV "JAVA_HOME=$ENV{JAVA_HOME}")
endif()

if(OPERIT_BUILD_CONFIG STREQUAL "Debug")
  list(APPEND OPERIT_CARGO_ENV "CARGO_PROFILE_DEV_DEBUG=2")
  set(OPERIT_CARGO_ARGS build --manifest-path "${OPERIT_FLUTTER_BRIDGE_CRATE}/Cargo.toml")
  set(OPERIT_CARGO_DLL "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/debug/operit_flutter_bridge.dll")
  set(OPERIT_CARGO_LABEL Debug)
else()
  set(OPERIT_CARGO_ARGS build --manifest-path "${OPERIT_FLUTTER_BRIDGE_CRATE}/Cargo.toml" --release)
  set(OPERIT_CARGO_DLL "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/release/operit_flutter_bridge.dll")
  set(OPERIT_CARGO_LABEL release)
endif()

# Deploy the matching debug symbols beside the loaded DLL for Rust stack resolution.
if(OPERIT_BUILD_CONFIG STREQUAL "Debug")
  set(OPERIT_CARGO_PDB "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/debug/operit_flutter_bridge.pdb")
endif()

execute_process(
  COMMAND "${CMAKE_COMMAND}" -E env ${OPERIT_CARGO_ENV}
    "${OPERIT_PLUGIN_SYNC_PYTHON}" "${OPERIT_REPO_ROOT}/tools/build_scripts/cargo_build.py"
    "${OPERIT_CARGO_EXECUTABLE}" ${OPERIT_CARGO_ARGS}
  WORKING_DIRECTORY "${OPERIT_FLUTTER_BRIDGE_CRATE}"
  RESULT_VARIABLE OPERIT_CARGO_RESULT
)
if(NOT OPERIT_CARGO_RESULT EQUAL 0)
  message(FATAL_ERROR "operit flutter bridge ${OPERIT_CARGO_LABEL} build failed: ${OPERIT_CARGO_RESULT}")
endif()
get_filename_component(OPERIT_CARGO_OUTPUT_DIRECTORY "${OPERIT_CARGO_DLL}" DIRECTORY)
foreach(OPERIT_RUNTIME_DLL IN ITEMS
    "${OPERIT_CARGO_DLL}"
    "${OPERIT_CARGO_OUTPUT_DIRECTORY}/operit_local_llm.dll")
  execute_process(
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
      "${OPERIT_RUNTIME_DLL}" "${OPERIT_OUTPUT_DIRECTORY}/"
    RESULT_VARIABLE OPERIT_COPY_RESULT
  )
  if(NOT OPERIT_COPY_RESULT EQUAL 0)
    message(FATAL_ERROR
      "Failed to deploy ${OPERIT_RUNTIME_DLL} into ${OPERIT_OUTPUT_DIRECTORY}: ${OPERIT_COPY_RESULT}")
  endif()
endforeach()
if(OPERIT_BUILD_CONFIG STREQUAL "Debug")
  execute_process(
    COMMAND "${CMAKE_COMMAND}" -E copy_if_different
      "${OPERIT_CARGO_PDB}"
      "${OPERIT_OUTPUT_DIRECTORY}/operit_flutter_bridge.pdb"
    RESULT_VARIABLE OPERIT_SYMBOL_COPY_RESULT
  )
  if(NOT OPERIT_SYMBOL_COPY_RESULT EQUAL 0)
    message(FATAL_ERROR "Failed to deploy Rust debug symbols: ${OPERIT_CARGO_PDB}")
  endif()
endif()
