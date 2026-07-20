# Builds and copies the Rust artifacts required by the selected Windows runner configuration.
if(OPERIT_BUILD_CONFIG STREQUAL "Debug")
  execute_process(
    COMMAND "${CMAKE_COMMAND}" -E env
      "RUSTFLAGS=-Awarnings"
      "CARGO_TERM_COLOR=never"
      "${OPERIT_CARGO_EXECUTABLE}" build --quiet --features process-stdio --manifest-path "${OPERIT_FLUTTER_BRIDGE_CRATE}/Cargo.toml"
    WORKING_DIRECTORY "${OPERIT_FLUTTER_BRIDGE_CRATE}"
    RESULT_VARIABLE OPERIT_CARGO_RESULT
  )
  if(NOT OPERIT_CARGO_RESULT EQUAL 0)
    message(FATAL_ERROR "operit flutter bridge Debug build failed: ${OPERIT_CARGO_RESULT}")
  endif()
  file(COPY_FILE
    "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/debug/operit_flutter_bridge.dll"
    "${OPERIT_OUTPUT_DIRECTORY}/operit_flutter_bridge.dll"
    ONLY_IF_DIFFERENT
  )
  file(COPY_FILE
    "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/debug/operit_flutter_bridge_process.exe"
    "${OPERIT_OUTPUT_DIRECTORY}/operit_flutter_bridge_process.exe"
    ONLY_IF_DIFFERENT
  )
else()
  execute_process(
    COMMAND "${CMAKE_COMMAND}" -E env
      "RUSTFLAGS=-Awarnings"
      "CARGO_TERM_COLOR=never"
      "${OPERIT_CARGO_EXECUTABLE}" build --quiet --manifest-path "${OPERIT_FLUTTER_BRIDGE_CRATE}/Cargo.toml" --release
    WORKING_DIRECTORY "${OPERIT_FLUTTER_BRIDGE_CRATE}"
    RESULT_VARIABLE OPERIT_CARGO_RESULT
  )
  if(NOT OPERIT_CARGO_RESULT EQUAL 0)
    message(FATAL_ERROR "operit flutter bridge release build failed: ${OPERIT_CARGO_RESULT}")
  endif()
  file(COPY_FILE
    "${OPERIT_FLUTTER_BRIDGE_CRATE}/target/release/operit_flutter_bridge.dll"
    "${OPERIT_OUTPUT_DIRECTORY}/operit_flutter_bridge.dll"
    ONLY_IF_DIFFERENT
  )
endif()
