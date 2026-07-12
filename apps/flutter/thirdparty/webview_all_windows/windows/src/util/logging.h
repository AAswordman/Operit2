#pragma once

#include <windows.h>

#include <string>
#include <string_view>

namespace webview_all_windows::util {

inline void LogWarning(std::string_view message) {
  std::string output = "[webview_all_windows] ";
  output.append(message);
  output.append("\n");
  OutputDebugStringA(output.c_str());
}

} // namespace webview_all_windows::util
