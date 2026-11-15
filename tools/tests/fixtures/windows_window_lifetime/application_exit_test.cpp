#include <windows.h>

#include <cstdlib>
#include <iostream>
#include <thread>

#include "operit_runtime_channel.h"

bool close_intercepted = false;
bool destroyed = false;

/// Fails process-level exit checks in optimized builds too.
void Require(bool condition, const char* message) {
  if (!condition) {
    std::cerr << message << std::endl;
    std::exit(1);
  }
}

/// Models the tray plugin consuming close requests while retaining the engine.
LRESULT CALLBACK WindowMessage(HWND window, UINT message, WPARAM wparam,
                               LPARAM lparam) {
  if (message == WM_CLOSE) {
    close_intercepted = true;
    return 0;
  }
  if (message == WM_DESTROY) destroyed = true;
  return DefWindowProc(window, message, wparam, lparam);
}

/// Requires explicit application exit to reach teardown despite close interception.
int main() {
  WNDCLASSW window_class{};
  window_class.lpfnWndProc = WindowMessage;
  window_class.hInstance = GetModuleHandle(nullptr);
  window_class.lpszClassName = L"OperitApplicationExitTest";
  Require(RegisterClassW(&window_class) != 0, "Could not register test window");
  const HWND window = CreateWindowW(window_class.lpszClassName, L"", 0,
      0, 0, 1, 1, HWND_MESSAGE, nullptr, window_class.hInstance, nullptr);
  Require(window != nullptr, "Could not create test window");
  SendMessage(window, WM_CLOSE, 0, 0);
  Require(close_intercepted && IsWindow(window), "Tray close did not retain the window");
  Require(!RequestOperitApplicationExit(0), "Missing platform thread was accepted");

  const DWORD platform_thread = GetCurrentThreadId();
  bool requested = false;
  std::thread caller([&]() { requested = RequestOperitApplicationExit(platform_thread); });
  caller.join();
  Require(requested, "Could not request application exit from another thread");
  MSG message{};
  while (const auto status = GetMessage(&message, nullptr, 0, 0)) {
    Require(status != -1, "Message loop failed");
    TranslateMessage(&message);
    DispatchMessage(&message);
  }
  Require(message.message == WM_QUIT && message.wParam == 0, "Message loop did not quit successfully");
  DestroyWindow(window);
  Require(destroyed && !IsWindow(window), "Exit did not reach window teardown");
  std::cout << "Application exit bypasses close-to-tray and reaches teardown" << std::endl;
}
