# Stage the same bundled interpreter for normal Flutter builds and release packaging.
if(FLUTTER_TARGET_PLATFORM STREQUAL "windows-x64")
  set(OPERIT_BASH_ARCH "x86_64")
elseif(FLUTTER_TARGET_PLATFORM STREQUAL "windows-arm64")
  set(OPERIT_BASH_ARCH "aarch64")
else()
  message(FATAL_ERROR "Unsupported Windows Bash target: ${FLUTTER_TARGET_PLATFORM}")
endif()

set(OPERIT_BASH_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/bash")
set(OPERIT_BASH_OUTPUTS)
foreach(OPERIT_BASH_FILE IN ITEMS
    bin/bash.exe usr/bin/bash.exe usr/bin/msys-2.0.dll
    usr/bin/base64.exe usr/bin/tr.exe usr/bin/uname.exe LICENSE.txt operit-bundle.json)
  list(APPEND OPERIT_BASH_OUTPUTS "${OPERIT_BASH_DIRECTORY}/${OPERIT_BASH_FILE}")
endforeach()
add_custom_command(
  OUTPUT ${OPERIT_BASH_OUTPUTS}
  COMMAND "${OPERIT_PLUGIN_SYNC_PYTHON}"
    "${OPERIT_REPO_ROOT}/tools/build_scripts/windows_bash.py"
    --destination "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>"
    --arch "${OPERIT_BASH_ARCH}"
  # Upstream archive timestamps predate our build inputs. Mark the verified
  # outputs current so MSBuild does not repeat preparation on every Dart edit.
  COMMAND "${CMAKE_COMMAND}" -E touch_nocreate ${OPERIT_BASH_OUTPUTS}
  DEPENDS
    "${OPERIT_REPO_ROOT}/tools/build_scripts/windows_bash.py"
    "${OPERIT_REPO_ROOT}/tools/build_scripts/common.py"
  COMMENT "Preparing the bundled Windows Bash runtime"
  VERBATIM
)
add_custom_target(operit_windows_bash DEPENDS ${OPERIT_BASH_OUTPUTS})

# Relative installation also supports a custom prefix outside the runner directory.
install(DIRECTORY "${OPERIT_BASH_DIRECTORY}" DESTINATION "." COMPONENT Runtime)
