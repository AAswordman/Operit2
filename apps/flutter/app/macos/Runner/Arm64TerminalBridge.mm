#if defined(__x86_64__)
#include "OperitFlutterBridge.h"
#include <stdlib.h>
#include <string.h>

extern "C" char *operit_macos_arm64_terminal_call(const char *, const char *) {
  return strdup("{\"error\":\"ARM64 terminal is not available on Intel macOS\"}");
}

extern "C" char *operit_macos_arm64_terminal_execute_streaming(
    const char *command, const char *request, void *, operit_macos_arm64_terminal_output_callback) {
  return operit_macos_arm64_terminal_call(command, request);
}

extern "C" void operit_macos_arm64_terminal_free(char *value) { free(value); }
#else
// The Apple owners share session, cancellation and root configuration semantics.
#define operit_ios_arm64_terminal_call operit_macos_arm64_terminal_call
#define operit_ios_arm64_terminal_free operit_macos_arm64_terminal_free
#define operit_ios_arm64_terminal_execute_streaming operit_macos_arm64_terminal_execute_streaming
#include "../../ios/Runner/Arm64TerminalBridge.mm"
#endif
