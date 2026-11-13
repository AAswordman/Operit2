#pragma once

#include <flutter/event_channel.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <flutter_windows.h>

#ifndef FLUTTER_WINDOWS_NATIVE_COMPOSITION_VERSION
#error This plugin requires the AAswordman/flutter-ohos native composition engine.
#endif

#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include "webview/webview.h"

namespace webview_all_windows {

class WebviewBridge {
public:
  using SurfaceFrameCaptureCallback =
      std::function<void(bool success, const std::vector<uint8_t> &data,
                         size_t width, size_t height)>;

  WebviewBridge(flutter::BinaryMessenger *messenger,
                FlutterDesktopNativeCompositionRef composition,
                std::unique_ptr<Webview> webview);
  ~WebviewBridge();

  /// Removes channel callbacks before an explicit Dart-side disposal.
  void DisposeChannels();

  int64_t view_id() const { return view_id_; }

  void SetCursorPos(double x, double y);
  void SetPointerUpdate(int64_t pointer, int64_t event, double x, double y,
                        double size, double pressure);
  void SetScrollDelta(double dx, double dy, bool control_key_pressed);
  void SetPointerButtonState(int64_t button, bool is_down);
  void SetSize(double width, double height, double scale_factor);
  /// Applies the owning top-level window visibility to the native browser.
  void SetHostVisible(bool visible);
  bool DispatchKeyEvent(const std::string &event_json);
  void CaptureSurfaceFrame(SurfaceFrameCaptureCallback result);

  bool SetLocalResourceHandler(Webview::LocalResourceCallback callback);
  void LoadUrl(const std::string &url);
  bool LoadRequest(const std::string &url, const std::string &method,
                   const std::string &headers,
                   const std::vector<uint8_t> *body);
  void LoadStringContent(const std::string &content);
  bool Reload();
  bool Stop();
  bool GoBack();
  bool GoForward();
  void Suspend();
  void Resume();

  void SetVirtualHostNameMapping(const std::string &host_name,
                                 const std::string &path, int64_t access_kind);
  bool ClearVirtualHostNameMapping(const std::string &host_name);

  void AddScriptToExecuteOnDocumentCreated(
      const std::string &script,
      std::function<void(bool success, const std::string &script_id)> result);
  void RemoveScriptToExecuteOnDocumentCreated(const std::string &script_id);
  void ExecuteScript(
      const std::string &script,
      std::function<void(bool success, const std::string &json_result)> result);
  bool PostWebMessage(const std::string &message);
  bool SetUserAgent(const std::string *user_agent);
  std::optional<std::string> GetUserAgent();
  bool SetJavaScriptEnabled(bool enabled);
  bool SetZoomControlEnabled(bool enabled);
  bool SetBackgroundColor(int64_t color);
  /// Forwards the app appearance to the owned browser profile.
  bool SetPreferredColorScheme(bool dark);
  bool SetZoomFactor(double zoom_factor);
  bool OpenDevTools();
  void SetJavaScriptDialogCallbacksEnabled(bool alert, bool confirm,
                                           bool prompt);

  void ClearCookies(std::function<void(bool success, bool had_cookies)> result);
  bool SetCookie(const WebviewCookie &cookie);
  void GetCookies(
      const std::string &url,
      std::function<void(bool success, std::vector<WebviewCookie> cookies)>
          result);
  bool DeleteCookie(const WebviewCookie &cookie);
  bool DeleteCookiesWithNameAndUrl(const std::string &name,
                                   const std::string &url);
  bool DeleteCookiesWithNameDomainAndPath(const std::string &name,
                                          const std::string &domain,
                                          const std::string &path);
  bool ClearCache();
  void ClearLocalStorage(Webview::OperationCompletedCallback callback);
  bool SetCacheDisabled(bool disabled);
  void SetPopupWindowPolicy(int64_t policy);

  /// Starts or stops compositor capture without suspending the page.
  void SetCaptureEnabled(bool enabled);

private:
  FlutterDesktopNativeCompositionRef composition_;
  std::unique_ptr<Webview> webview_;
  std::unique_ptr<flutter::EventSink<flutter::EncodableValue>> event_sink_;
  std::unique_ptr<flutter::EventChannel<flutter::EncodableValue>>
      event_channel_;
  std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>>
      method_channel_;

  int64_t view_id_;
  void RegisterEventHandlers();

  template <typename T> void EmitEvent(const T &value) {
    if (event_sink_) {
      event_sink_->Success(value);
    }
  }

  void
  OnPermissionRequested(const std::string &url,
                        WebviewPermissionKind permissionKind,
                        bool is_user_initiated,
                        Webview::WebviewPermissionRequestedCompleter completer);
  void
  OnHttpAuthRequested(const WebviewHttpAuthRequest &request,
                      Webview::WebviewHttpAuthRequestedCompleter completer);
  void OnSslAuthError(const WebviewSslAuthError &error,
                      Webview::WebviewSslAuthErrorCompleter completer);
  void OnJavaScriptDialogRequested(
      const WebviewJavaScriptDialogRequest &request,
      Webview::WebviewJavaScriptDialogCompleter completer);
};

} // namespace webview_all_windows
