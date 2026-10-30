#include "flutter_window.h"

#include <optional>
#include <dwmapi.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>

#include <desktop_multi_window/desktop_multi_window_plugin.h>
#include <file_selector_windows/file_selector_windows.h>

#include "crash_channel.h"
#include "engine_channel_lifetime.h"
#include "flutter/generated_plugin_registrant.h"
#include "operit_runtime_channel.h"
#include "system_audio_input_channel.h"

namespace {

/// Keeps the system caption controls while joining its color to the sidebar.
void RegisterWindowChromeChannel(flutter::FlutterEngine* engine, HWND view) {
  using flutter::EncodableMap;
  using flutter::EncodableValue;
  auto channel = std::make_shared<flutter::MethodChannel<EncodableValue>>(
      engine->messenger(), "operit/window_chrome",
      &flutter::StandardMethodCodec::GetInstance());
  channel->SetMethodCallHandler(
      [view](const auto& call, auto result) {
        if (call.method_name() != "configure") {
          result->NotImplemented();
          return;
        }
        const auto* arguments = call.arguments()
            ? std::get_if<EncodableMap>(call.arguments()) : nullptr;
        if (!arguments) {
          result->Error("invalid_arguments", "Window appearance is required");
          return;
        }
        const auto dark_entry = arguments->find(EncodableValue("dark"));
        const auto background = arguments->find(EncodableValue("backgroundColor"));
        const auto foreground = arguments->find(EncodableValue("foregroundColor"));
        if (dark_entry == arguments->end() || background == arguments->end() ||
            foreground == arguments->end() ||
            !std::holds_alternative<bool>(dark_entry->second)) {
          result->Error("invalid_arguments", "Window appearance is incomplete");
          return;
        }
        const auto to_color = [](const EncodableValue& value) -> COLORREF {
          const auto* wide = std::get_if<int64_t>(&value);
          const auto* narrow = std::get_if<int32_t>(&value);
          const auto argb = static_cast<uint32_t>(wide ? *wide : narrow ? *narrow : 0);
          return RGB((argb >> 16) & 0xff, (argb >> 8) & 0xff, argb & 0xff);
        };
        const HWND window = GetAncestor(view, GA_ROOT);
        if (!IsWindow(window)) {
          result->Error("window_unavailable", "Window is unavailable");
          return;
        }
        const BOOL dark = std::get<bool>(dark_entry->second);
        const COLORREF caption_color = to_color(background->second);
        const COLORREF text_color = to_color(foreground->second);
        DwmSetWindowAttribute(window, DWMWA_USE_IMMERSIVE_DARK_MODE,
                              &dark, sizeof(dark));
        // Windows 11 caption/text colors. Older Windows still uses the native
        // light/dark caption selected above; its resizing and Snap stay native.
        DwmSetWindowAttribute(window, static_cast<DWMWINDOWATTRIBUTE>(35),
                              &caption_color, sizeof(caption_color));
        DwmSetWindowAttribute(window, static_cast<DWMWINDOWATTRIBUTE>(36),
                              &text_color, sizeof(text_color));
        result->Success(EncodableValue(0.0));
      });
  RegisterOperitEngineChannelShutdown(engine, [channel]() {
    channel->SetMethodCallHandler(nullptr);
  });
}

}  // namespace

/// Creates a window that owns one Flutter view controller.
FlutterWindow::FlutterWindow(const flutter::DartProject& project, bool data_rescue)
    : project_(project), data_rescue_(data_rescue) {}

/// Destroys the window after Win32 teardown has released hosted content.
FlutterWindow::~FlutterWindow() {}

/// Initializes Flutter, plugins, and native channels for the window.
bool FlutterWindow::OnCreate() {
  if (!Win32Window::OnCreate()) {
    return false;
  }

  RECT frame = GetClientArea();

  // The size here must match the window dimensions to avoid unnecessary surface
  // creation / destruction in the startup path.
  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  // Ensure that basic setup of the controller was successful.
  if (!flutter_controller_->engine() || !flutter_controller_->view()) {
    return false;
  }
  if (data_rescue_) {
    FileSelectorWindowsRegisterWithRegistrar(
        flutter_controller_->engine()->GetRegistrarForPlugin("FileSelectorWindows"));
  } else {
    RegisterPlugins(flutter_controller_->engine());
  }
  RegisterOperitRuntimeChannel(flutter_controller_->engine(), GetHandle());
  if (!data_rescue_) {
    RegisterOperitCrashChannel(flutter_controller_->engine());
    RegisterSystemAudioInputChannel(flutter_controller_->engine());
    RegisterWindowChromeChannel(flutter_controller_->engine(), GetHandle());
    DesktopMultiWindowSetWindowCreatedCallback([](void* controller) {
    auto* flutter_view_controller =
        reinterpret_cast<flutter::FlutterViewController*>(controller);
    RegisterPlugins(flutter_view_controller->engine());
    RegisterOperitRuntimeChannel(
        flutter_view_controller->engine(),
        flutter_view_controller->view()->GetNativeWindow());
    RegisterSystemAudioInputChannel(flutter_view_controller->engine());
    RegisterWindowChromeChannel(
        flutter_view_controller->engine(),
        flutter_view_controller->view()->GetNativeWindow());
    });
  }
  SetChildContent(flutter_controller_->view()->GetNativeWindow());

  flutter_controller_->engine()->SetNextFrameCallback([&]() {
    this->Show();
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();

  return true;
}

/// Releases native channels before destroying the Flutter controller.
void FlutterWindow::OnDestroy() {
  if (flutter_controller_) {
    ShutdownOperitRuntimeChannel();
    ShutdownAllOperitEngineChannels();
    ShutdownSystemAudioInputChannel();
    ShutdownOperitCrashChannel();
    flutter_controller_ = nullptr;
  }

  Win32Window::OnDestroy();
}

/// Dispatches Windows messages through runtime hooks, Flutter, and Win32.
LRESULT
FlutterWindow::MessageHandler(HWND hwnd, UINT const message,
                              WPARAM const wparam,
                              LPARAM const lparam) noexcept {
  LRESULT operit_runtime_result = 0;
  if (HandleOperitNotificationActivationWindowMessage(
          message, wparam, lparam, &operit_runtime_result)) {
    return operit_runtime_result;
  }
  if (HandleOperitRuntimeChannelWindowMessage(message, wparam, lparam,
                                              &operit_runtime_result)) {
    return operit_runtime_result;
  }

  // Give Flutter, including plugins, an opportunity to handle window messages.
  if (flutter_controller_) {
    std::optional<LRESULT> result =
        flutter_controller_->HandleTopLevelWindowProc(hwnd, message, wparam,
                                                      lparam);
    if (result) {
      return *result;
    }

    switch (message) {
      case WM_FONTCHANGE:
        flutter_controller_->engine()->ReloadSystemFonts();
        break;
    }
  }

  return Win32Window::MessageHandler(hwnd, message, wparam, lparam);
}
