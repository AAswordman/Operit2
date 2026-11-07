#include "OperitFlutterBridge.h"
#include <cassert>
#include <cstring>

int main() {
  const char *expected = "{\"error\":\"ARM64 terminal is not available on Intel macOS\"}";
  char *response = operit_macos_arm64_terminal_call("terminalCreate", "{}");
  assert(response && std::strcmp(response, expected) == 0);
  operit_macos_arm64_terminal_free(response);
  response = operit_macos_arm64_terminal_execute_streaming("terminalExecute", "{}", nullptr, nullptr);
  assert(response && std::strcmp(response, expected) == 0);
  operit_macos_arm64_terminal_free(response);
}
