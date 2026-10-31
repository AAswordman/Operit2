#include "include/floating_windows/floating_windows_plugin.h"
#include <flutter/plugin_registrar_windows.h>

#include <dwmapi.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>

#include <cmath>
#include <memory>
#include <stdexcept>
#include <string>


namespace {
using Value = flutter::EncodableValue;
using Map = flutter::EncodableMap;

/// Owns a channel whose HWND belongs exclusively to one Flutter engine.
struct WidgetWindow {
  HWND window;
  HWND view;
  bool configured = false;
  bool topmost = true;
  bool focusable = false;
  std::unique_ptr<flutter::MethodChannel<Value>> channel;
};

/// Converts an explicit logical dimension to physical pixels at the window DPI.
int Dimension(const Map& arguments, const char* key, HWND window) {
  const auto entry = arguments.find(Value(key));
  if (entry == arguments.end()) throw std::invalid_argument("Missing window dimension");
  double number;
  if (const auto double_value = std::get_if<double>(&entry->second)) {
    number = *double_value;
  } else if (const auto integer_value = std::get_if<int32_t>(&entry->second)) {
    number = static_cast<double>(*integer_value);
  } else {
    throw std::invalid_argument("Window dimensions must be numeric");
  }
  if (!std::isfinite(number) || number < 16 || number > 4096) {
    throw std::invalid_argument("Window dimensions must be between 16 and 4096 logical pixels");
  }
  return static_cast<int>(std::lround(number * GetDpiForWindow(window) / 96.0));
}

/// Changes a window style while distinguishing a valid zero result from an error.
void SetStyle(HWND window, int index, LONG_PTR value) {
  SetLastError(ERROR_SUCCESS);
  const auto previous = SetWindowLongPtr(window, index, value);
  if (previous == 0 && GetLastError() != ERROR_SUCCESS) {
    throw std::runtime_error("Unable to configure floating window style");
  }
}

/// Enables alpha composition without color-keying or reducing content opacity.
void EnableTransparency(HWND window) {
  BOOL composition_enabled = FALSE;
  if (FAILED(DwmIsCompositionEnabled(&composition_enabled)) || !composition_enabled) {
    throw std::runtime_error("Desktop composition is required for transparent widgets");
  }
  HRGN region = CreateRectRgn(0, 0, -1, -1);
  if (region == nullptr) throw std::runtime_error("Unable to allocate desktop alpha region");
  DWM_BLURBEHIND blur{};
  blur.dwFlags = DWM_BB_ENABLE | DWM_BB_BLURREGION;
  blur.fEnable = TRUE;
  blur.hRgnBlur = region;
  const auto result = DwmEnableBlurBehindWindow(window, &blur);
  DeleteObject(region);
  if (FAILED(result)) throw std::runtime_error("Unable to enable transparent desktop composition");
  const DWMNCRENDERINGPOLICY policy = DWMNCRP_DISABLED;
  if (FAILED(DwmSetWindowAttribute(window, DWMWA_NCRENDERING_POLICY, &policy, sizeof(policy)))) {
    throw std::runtime_error("Unable to remove the floating window frame");
  }
}

/// Applies partial updates without resetting unrelated window state.
void Apply(WidgetWindow& owner, const Map& args) {
  RECT rect; GetWindowRect(owner.window, &rect);
  const auto has = [&](const char* key) { return args.find(Value(key)) != args.end(); };
  const auto flag = [&](const char* key, bool fallback) {
    const auto entry = args.find(Value(key));
    if (entry == args.end()) return fallback;
    const auto value = std::get_if<bool>(&entry->second);
    if (!value) throw std::invalid_argument("Window flag must be boolean");
    return *value;
  };
  const auto coordinate = [&](const char* key, int fallback) {
    const auto entry = args.find(Value(key));
    if (entry == args.end()) return fallback;
    double value;
    if (auto n = std::get_if<double>(&entry->second)) value = *n;
    else if (auto n = std::get_if<int32_t>(&entry->second)) value = *n;
    else throw std::invalid_argument("Coordinate must be numeric");
    if (!std::isfinite(value) || std::abs(value) > 1000000) throw std::invalid_argument("Invalid coordinate");
    return static_cast<int>(std::lround(value * GetDpiForWindow(owner.window) / 96.0));
  };
  int width = has("width") ? Dimension(args, "width", owner.window) : rect.right - rect.left;
  int height = has("height") ? Dimension(args, "height", owner.window) : rect.bottom - rect.top;
  owner.topmost = flag("alwaysOnTop", owner.topmost);
  owner.focusable = flag("focusable", owner.focusable);
  auto style = GetWindowLongPtr(owner.window, GWL_EXSTYLE);
  style = owner.focusable ? style & ~WS_EX_NOACTIVATE : style | WS_EX_NOACTIVATE;
  if (has("clickThrough")) {
    style = flag("clickThrough", false) ? style | WS_EX_TRANSPARENT | WS_EX_LAYERED
      : style & ~(WS_EX_TRANSPARENT | WS_EX_LAYERED);
  }
  SetStyle(owner.window, GWL_EXSTYLE, style);
  // A newly layered window needs an alpha policy before it can be displayed.
  // Keep content fully opaque while DWM retains the framebuffer's own alpha.
  if ((style & WS_EX_LAYERED) != 0 &&
      !SetLayeredWindowAttributes(owner.window, 0, 255, LWA_ALPHA)) {
    throw std::runtime_error("Unable to configure floating window alpha");
  }
  if (!SetWindowPos(owner.window, owner.topmost ? HWND_TOPMOST : HWND_NOTOPMOST,
      coordinate("x", rect.left), coordinate("y", rect.top), width, height,
      SWP_NOACTIVATE | SWP_FRAMECHANGED)) throw std::runtime_error("Unable to update floating window");
  if (!MoveWindow(owner.view, 0, 0, width, height, TRUE)) throw std::runtime_error("Unable to resize content");
}

void Configure(WidgetWindow& owner, const Map& arguments) {
  const auto role = arguments.find(Value("role"));
  if (role == arguments.end() || !std::holds_alternative<std::string>(role->second) ||
      std::get<std::string>(role->second) != "floating_windows.window") {
    throw std::invalid_argument("Explicit floating window role required");
  }
  EnableTransparency(owner.window);
  SetStyle(owner.window, GWL_STYLE,
    (GetWindowLongPtr(owner.window, GWL_STYLE) & ~static_cast<LONG_PTR>(WS_OVERLAPPEDWINDOW)) | WS_POPUP);
  SetStyle(owner.window, GWL_EXSTYLE,
    (GetWindowLongPtr(owner.window, GWL_EXSTYLE) & ~static_cast<LONG_PTR>(WS_EX_APPWINDOW)) | WS_EX_TOOLWINDOW);
  Apply(owner, arguments);
  owner.configured = true;
}
}  // namespace

/// Installs host operations and releases their messenger references with the engine.
class FloatingWindowsPlugin : public flutter::Plugin {
 public:
  /// Creates the host-owned plugin and its per-engine method channel.
  explicit FloatingWindowsPlugin(flutter::PluginRegistrarWindows* registrar) {
  auto owner = std::make_shared<WidgetWindow>();
  owner->view = registrar->GetView()->GetNativeWindow();
  owner->window = GetAncestor(owner->view, GA_ROOT);
  owner->channel = std::make_unique<flutter::MethodChannel<Value>>(
      registrar->messenger(), "floating_windows/window",
      &flutter::StandardMethodCodec::GetInstance());
  const std::weak_ptr<WidgetWindow> weak_owner = owner;
  owner->channel->SetMethodCallHandler(
      [weak_owner](const flutter::MethodCall<Value>& call,
                   std::unique_ptr<flutter::MethodResult<Value>> result) {
        auto current = weak_owner.lock();
        if (!current || !IsWindow(current->window)) {
          result->Error("WIDGET_WINDOW_CLOSED", "The floating window is closed");
          return;
        }
        try {
          const auto& method = call.method_name();
          if (method == "capabilities") {
            BOOL enabled = FALSE;
            const bool supported = SUCCEEDED(DwmIsCompositionEnabled(&enabled)) && enabled;
            result->Success(Value(Map{{Value("supported"),Value(supported)},
              {Value("permissionGranted"),Value(true)}, {Value("positioning"),Value(true)},
              {Value("alwaysOnTop"),Value(true)}, {Value("clickThrough"),Value(true)}, {Value("focusable"),Value(true)}}));
            return;
          }
          if (method == "requestPermission") { result->Success(Value(true)); return; }
          if (method == "configure" || method == "update") {
            const auto* args = call.arguments() ? std::get_if<Map>(call.arguments()) : nullptr;
            if (!args) throw std::invalid_argument("Expected window options");
            if (method == "configure") Configure(*current, *args);
            else {
              if (!current->configured) throw std::runtime_error("Configure the floating window first");
              Apply(*current, *args);
            }
            result->Success(); return;
          }
          if (!current->configured) throw std::runtime_error("Configure the floating window first");
          if (method == "show") {
            ShowWindow(current->window, current->focusable ? SW_SHOW : SW_SHOWNOACTIVATE);
            result->Success();
          } else if (method == "hide") {
            ShowWindow(current->window, SW_HIDE); result->Success();
          } else if (method == "close") {
            result->Success(); PostMessage(current->window, WM_CLOSE, 0, 0);
          } else { result->NotImplemented(); }
        } catch (const std::exception& error) {
          result->Error("FLOATING_WINDOW_ERROR", error.what());
        }
      });
  owner_ = std::move(owner);
  }

  /// Releases ownership while engine teardown clears its detached messenger.
  ~FloatingWindowsPlugin() override = default;

 private:
  std::shared_ptr<WidgetWindow> owner_;
};

/// Registers one independent widget-window host for each Flutter engine.
void FloatingWindowsPluginRegisterWithRegistrar(FlutterDesktopPluginRegistrarRef registrar) {
  auto* windows_registrar = flutter::PluginRegistrarManager::GetInstance()
      ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar);
  windows_registrar->AddPlugin(std::make_unique<FloatingWindowsPlugin>(windows_registrar));
}
