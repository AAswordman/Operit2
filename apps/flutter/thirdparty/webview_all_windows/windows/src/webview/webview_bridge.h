#pragma once

#include <flutter/event_channel.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <flutter/texture_registrar.h>

#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

#include "rendering/graphics_context.h"
#include "rendering/texture_bridge.h"
#include "webview/webview.h"

namespace webview_all_windows {

class WebviewBridge {
public:
  using SurfaceFrameCaptureCallback =
      std::function<void(bool success, const std::vector<uint8_t> &data,
                         size_t width, size_t height)>;

  WebviewBridge(flutter::BinaryMessenger *messenger,
                flutter::TextureRegistrar *texture_registrar,
                GraphicsContext *graphics_context,
                std::unique_ptr<Webview> webview);
  ~WebviewBridge();

  TextureBridge *texture_bridge() const { return texture_bridge_.get(); }

  int64_t texture_id() const { return texture_id_; }

  void SetCursorPos(double x, double y);
  void SetPointerUpdate(int64_t pointer, int64_t event, double x, double y,
                        double size, double pressure);
  void SetScrollDelta(double dx, double dy);
  void SetPointerButtonState(int64_t button, bool is_down);
  void SetSize(double width, double height, double scale_factor);
  bool DispatchKeyEvent(const std::string &event_json);
  void CaptureSurfaceFrame(SurfaceFrameCaptureCallback result);

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
  void SetFpsLimit(int64_t max_fps);

private:
  std::unique_ptr<flutter::TextureVariant> flutter_texture_;
  std::unique_ptr<TextureBridge> texture_bridge_;
  std::unique_ptr<Webview> webview_;
  std::unique_ptr<flutter::EventSink<flutter::EncodableValue>> event_sink_;
  std::unique_ptr<flutter::EventChannel<flutter::EncodableValue>>
      event_channel_;
  std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>>
      method_channel_;

  flutter::TextureRegistrar *texture_registrar_;
  int64_t texture_id_;
  std::mutex surface_frame_capture_mutex_;
  std::vector<SurfaceFrameCaptureCallback> pending_surface_frame_captures_;
  bool surface_frame_readback_active_ = false;

  void RegisterEventHandlers();
  void OnTextureFrameAvailable();

  /// Starts one worker readback for all pending method calls.
  void StartSurfaceFrameReadback();

  /// Resolves pending method calls after worker readback completes.
  void OnSurfaceFrameReadbackCompleted(bool success,
                                       const std::vector<uint8_t> &data,
                                       Size size);

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
