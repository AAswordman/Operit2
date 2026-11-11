#pragma once

#include <windows.h>
#include <flutter/dart_project.h>
#include <flutter_plugin_registrar.h>

#include <cstdlib>
#include <functional>
#include <optional>
#include <string>

namespace flutter {

/// Models engine-owned application channels without starting a Dart isolate.
struct FlutterEngine {
  std::string window_id;
  bool channels_registered = false;
  bool destroying = false;

  /// Supplies a stable registrar identity for the native manager fixture.
  FlutterDesktopPluginRegistrarRef GetRegistrarForPlugin(const char*) {
    return reinterpret_cast<FlutterDesktopPluginRegistrarRef>(this);
  }

  /// Fails when a window message reaches an engine during its destruction.
  void ReloadSystemFonts() {
    if (destroying) std::abort();
  }
};

/// Provides real Win32 child content for production window message handling.
struct FlutterView {
  HWND handle = nullptr;

  /// Returns the child HWND hosted by the production Win32Window.
  HWND GetNativeWindow() { return handle; }
};

/// Exposes teardown timing while production code owns and destroys controllers.
class FlutterViewController {
 public:
  inline static std::function<void(FlutterViewController*)> on_destroy;

  /// Creates hidden content that is reparented by the production window.
  FlutterViewController(int, int, const DartProject&) {
    view_.handle = CreateWindowW(L"STATIC", L"", WS_POPUP, 0, 0, 16, 16,
                                nullptr, nullptr, GetModuleHandle(nullptr), nullptr);
  }

  /// Injects reentrant messages at the point the real Flutter engine shuts down.
  ~FlutterViewController() {
    engine_.destroying = true;
    on_destroy(this);
    DestroyWindow(view_.handle);
  }

  /// Returns the engine whose channels must be removed before destruction.
  FlutterEngine* engine() { return &engine_; }

  /// Returns the content view owned by this controller.
  FlutterView* view() { return &view_; }

  /// Detects dispatch through a controller whose destructor has already started.
  std::optional<LRESULT> HandleTopLevelWindowProc(HWND, UINT, WPARAM, LPARAM) {
    if (engine_.destroying) std::abort();
    return std::nullopt;
  }

 private:
  FlutterEngine engine_;
  FlutterView view_;
};

}  // namespace flutter
