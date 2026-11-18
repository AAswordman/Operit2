# Stage PowerShell 7 independently of Rust and Dart compilation.
if(FLUTTER_TARGET_PLATFORM STREQUAL "windows-x64")
  set(OPERIT_POWERSHELL_ARCH "x86_64")
elseif(FLUTTER_TARGET_PLATFORM STREQUAL "windows-arm64")
  set(OPERIT_POWERSHELL_ARCH "aarch64")
else()
  message(FATAL_ERROR "Unsupported Windows PowerShell target: ${FLUTTER_TARGET_PLATFORM}")
endif()

set(OPERIT_POWERSHELL_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/powershell")
set(OPERIT_POWERSHELL_OUTPUTS)
foreach(OPERIT_POWERSHELL_FILE IN ITEMS
    pwsh.exe pwsh.dll pwsh.deps.json pwsh.runtimeconfig.json
    System.Management.Automation.dll Microsoft.PowerShell.ConsoleHost.dll
    hostfxr.dll hostpolicy.dll coreclr.dll System.Private.CoreLib.dll
    Modules/Microsoft.PowerShell.Management/Microsoft.PowerShell.Management.psd1
    Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1
    LICENSE.txt ThirdPartyNotices.txt operit-bundle.json)
  list(APPEND OPERIT_POWERSHELL_OUTPUTS "${OPERIT_POWERSHELL_DIRECTORY}/${OPERIT_POWERSHELL_FILE}")
endforeach()
add_custom_command(
  OUTPUT ${OPERIT_POWERSHELL_OUTPUTS}
  COMMAND "${OPERIT_PLUGIN_SYNC_PYTHON}"
    "${OPERIT_REPO_ROOT}/tools/build_scripts/windows_powershell.py"
    --destination "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>"
    --arch "${OPERIT_POWERSHELL_ARCH}"
  COMMAND "${CMAKE_COMMAND}" -E touch_nocreate ${OPERIT_POWERSHELL_OUTPUTS}
  DEPENDS
    "${OPERIT_REPO_ROOT}/tools/build_scripts/windows_powershell.py"
    "${OPERIT_REPO_ROOT}/tools/build_scripts/common.py"
  COMMENT "Preparing the bundled PowerShell 7 runtime"
  VERBATIM
)
add_custom_target(operit_windows_powershell DEPENDS ${OPERIT_POWERSHELL_OUTPUTS})
