#include <windows.h>

#include <cstdlib>
#include <iostream>
#include <vector>

#include "multi_window_manager.h"
#include "multi_window_plugin_internal.h"
#include "include/desktop_multi_window/desktop_multi_window_plugin.h"

namespace {

int created = 0;
int unregistered = 0;
int destroyed = 0;
std::vector<HWND> child_windows;

/// Reports a failed lifetime invariant even in optimized test builds.
void Require(bool condition, const char* message) {
  if (!condition) {
    std::cerr << message << std::endl;
    std::exit(1);
  }
}

/// Marks the per-engine channels installed by the application's creation hook.
void OnCreated(void* value) {
  auto* controller = static_cast<flutter::FlutterViewController*>(value);
  controller->engine()->channels_registered = true;
  child_windows.push_back(GetAncestor(controller->view()->GetNativeWindow(), GA_ROOT));
  ++created;
}

/// Requires channel unregistration to happen before controller destruction starts.
void OnUnregister(void* value) {
  auto* controller = static_cast<flutter::FlutterViewController*>(value);
  Require(!controller->engine()->destroying, "Cleanup ran after engine teardown began");
  Require(controller->engine()->channels_registered, "Cleanup ran more than once");
  controller->engine()->channels_registered = false;
  ++unregistered;
}

/// Simulates engine teardown, plugin deregistration, and reentrant Win32 messages.
void OnDestroyed(flutter::FlutterViewController* controller) {
  Require(!controller->engine()->channels_registered, "Engine destroyed with registered channels");
  const auto parent = GetAncestor(controller->view()->GetNativeWindow(), GA_ROOT);
  SendMessage(parent, WM_FONTCHANGE, 0, 0);
  SendMessage(parent, WM_SIZE, 0, MAKELPARAM(16, 16));
  MultiWindowManager::Instance()->RemoveWindow(controller->engine()->window_id);
  ++destroyed;
}

/// Opens a hidden secondary window using the production manager and Win32 code.
std::string OpenWindow() {
  const flutter::EncodableMap arguments{
      {flutter::EncodableValue("hiddenAtLaunch"), flutter::EncodableValue(true)}};
  auto id = MultiWindowManager::Instance()->Create(&arguments);
  Require(!id.empty(), "Unable to create test window");
  return id;
}

}  // namespace

/// Models registrar-owned plugin destruction using each engine's window ID.
void InternalMultiWindowPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar, FlutterWindowWrapper* window) {
  reinterpret_cast<flutter::FlutterEngine*>(registrar)->window_id = window->GetWindowId();
}

/// Exercises individual closure, repeated closure, and application shutdown.
int main() {
  auto* manager = MultiWindowManager::Instance();
  DesktopMultiWindowSetWindowCreatedCallback(OnCreated);
  DesktopMultiWindowSetWindowDestroyedCallback(OnUnregister);
  flutter::FlutterViewController::on_destroy = OnDestroyed;

  const auto first = OpenWindow();
  const auto outside = CreateWindowW(L"STATIC", L"", 0, 0, 0, 1, 1,
                                     HWND_MESSAGE, nullptr, nullptr, nullptr);
  Require(outside != nullptr, "Unable to create an external focus target");
  SetFocus(outside);
  Require(GetFocus() == outside, "Unable to focus the external target");
  SendMessage(child_windows.back(), WM_ACTIVATE, WA_INACTIVE,
              reinterpret_cast<LPARAM>(outside));
  Require(GetFocus() == outside, "Inactive secondary window stole focus");
  DestroyWindow(outside);
  DestroyWindow(child_windows.back());
  Require(manager->GetWindow(first) == nullptr, "Closed window remains registered");
  Require(unregistered == 1 && destroyed == 1, "Single-window close did not clean its engine");

  // A later create reclaims the closed window object without repeating cleanup.
  OpenWindow();
  OpenWindow();
  Require(unregistered == 1 && destroyed == 1, "Deferred cleanup repeated engine destruction");
  DesktopMultiWindowShutdown();
  Require(created == 3 && unregistered == 3 && destroyed == 3, "Shutdown left live engines");
  Require(manager->GetAllWindowIds().empty(), "Shutdown left registered windows");
  for (const auto window : child_windows) {
    Require(!IsWindow(window), "Shutdown left a live secondary HWND");
  }
  DesktopMultiWindowShutdown();
  Require(unregistered == 3 && destroyed == 3, "Repeated shutdown repeated engine destruction");
  Require(manager->Create(nullptr).empty(), "Shutdown accepted a new window");
  std::cout << "Window lifetime checks passed" << std::endl;
}
