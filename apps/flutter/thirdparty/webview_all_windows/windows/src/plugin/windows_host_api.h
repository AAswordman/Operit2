#pragma once

#include <flutter/plugin_registrar_windows.h>
#include <windows.h>

#include <memory>
#include <optional>
#include <string>
#include <unordered_map>

#include "platform/webview_platform.h"
#include "webview/webview_bridge.h"
#include "webview/webview_host.h"
#include "windows_webview_api.g.h"

namespace webview_all_windows {

class WindowsHostApi : public flutter::Plugin, public WindowsWebViewHostApi {
public:
  static void RegisterWithRegistrar(flutter::PluginRegistrarWindows *registrar);

  WindowsHostApi(flutter::TextureRegistrar *textures,
                 flutter::BinaryMessenger *messenger);
  ~WindowsHostApi() override;

private:
  std::unique_ptr<WebviewPlatform> platform_;
  std::unique_ptr<WebviewHost> webview_host_;
  std::unordered_map<int64_t, std::unique_ptr<WebviewBridge>> instances_;

  WNDCLASS window_class_ = {};
  flutter::TextureRegistrar *textures_;
  flutter::BinaryMessenger *messenger_;

  bool InitPlatform();

  std::optional<FlutterError>
  InitializeEnvironment(const WindowsEnvironmentOptions &options) override;
  ErrorOr<std::optional<std::string>> GetWebViewVersion() override;
  void CreateWebView(
      std::function<void(ErrorOr<WindowsCreateWebViewResult> reply)> result)
      override;
  std::optional<FlutterError> DisposeWebView(int64_t texture_id) override;
  std::optional<FlutterError> LoadUrl(int64_t texture_id,
                                      const std::string &url) override;
  std::optional<FlutterError>
  LoadRequest(int64_t texture_id,
              const WindowsLoadRequestData &request) override;
  std::optional<FlutterError>
  LoadStringContent(int64_t texture_id, const std::string &content) override;
  std::optional<FlutterError> Reload(int64_t texture_id) override;
  std::optional<FlutterError> Stop(int64_t texture_id) override;
  std::optional<FlutterError> GoBack(int64_t texture_id) override;
  std::optional<FlutterError> GoForward(int64_t texture_id) override;
  void AddScriptToExecuteOnDocumentCreated(
      int64_t texture_id, const std::string &script,
      std::function<void(ErrorOr<std::optional<std::string>> reply)> result)
      override;
  std::optional<FlutterError>
  RemoveScriptToExecuteOnDocumentCreated(int64_t texture_id,
                                         const std::string &script_id) override;
  void ExecuteScript(
      int64_t texture_id, const std::string &script,
      std::function<void(ErrorOr<std::string> reply)> result) override;
  std::optional<FlutterError>
  PostWebMessage(int64_t texture_id, const std::string &message) override;
  std::optional<FlutterError>
  SetUserAgent(int64_t texture_id, const std::string *user_agent) override;
  ErrorOr<std::optional<std::string>> GetUserAgent(int64_t texture_id) override;
  std::optional<FlutterError> SetJavaScriptEnabled(int64_t texture_id,
                                                   bool enabled) override;
  void ClearCookies(int64_t texture_id,
                    std::function<void(ErrorOr<bool> reply)> result) override;
  std::optional<FlutterError>
  SetCookie(int64_t texture_id, const WindowsCookieData &cookie) override;
  void GetCookies(int64_t texture_id, const std::string &url,
                  std::function<void(ErrorOr<flutter::EncodableList> reply)>
                      result) override;
  std::optional<FlutterError>
  DeleteCookie(int64_t texture_id, const WindowsCookieData &cookie) override;
  std::optional<FlutterError>
  DeleteCookiesWithNameAndUrl(int64_t texture_id, const std::string &name,
                              const std::string &url) override;
  std::optional<FlutterError> DeleteCookiesWithNameDomainAndPath(
      int64_t texture_id, const std::string &name, const std::string &domain,
      const std::string &path) override;
  std::optional<FlutterError> ClearCache(int64_t texture_id) override;
  void ClearLocalStorage(
      int64_t texture_id,
      std::function<void(std::optional<FlutterError> reply)> result) override;
  std::optional<FlutterError> SetCacheDisabled(int64_t texture_id,
                                               bool disabled) override;
  std::optional<FlutterError> OpenDevTools(int64_t texture_id) override;
  std::optional<FlutterError> SetBackgroundColor(int64_t texture_id,
                                                 int64_t color) override;
  std::optional<FlutterError> SetZoomControlEnabled(int64_t texture_id,
                                                    bool enabled) override;
  std::optional<FlutterError> SetZoomFactor(int64_t texture_id,
                                            double zoom_factor) override;
  std::optional<FlutterError> SetPopupWindowPolicy(int64_t texture_id,
                                                   int64_t policy) override;
  std::optional<FlutterError>
  SetJavaScriptDialogCallbacksEnabled(int64_t texture_id, bool alert,
                                      bool confirm, bool prompt) override;
  std::optional<FlutterError> Suspend(int64_t texture_id) override;
  std::optional<FlutterError> Resume(int64_t texture_id) override;
  std::optional<FlutterError> SetVirtualHostNameMapping(
      int64_t texture_id,
      const WindowsVirtualHostMappingData &mapping) override;
  std::optional<FlutterError>
  ClearVirtualHostNameMapping(int64_t texture_id,
                              const std::string &host_name) override;
  std::optional<FlutterError> SetFpsLimit(int64_t texture_id,
                                          int64_t max_fps) override;
  std::optional<FlutterError>
  SetPointerUpdate(int64_t texture_id,
                   const WindowsPointerUpdateData &update) override;
  std::optional<FlutterError>
  SetCursorPos(int64_t texture_id, const WindowsPointData &position) override;
  std::optional<FlutterError>
  SetPointerButton(int64_t texture_id,
                   const WindowsPointerButtonData &button) override;
  std::optional<FlutterError>
  SetScrollDelta(int64_t texture_id, const WindowsPointData &delta) override;
  std::optional<FlutterError> SetSize(int64_t texture_id,
                                      const WindowsSizeData &size) override;

  WebviewBridge *FindBridge(int64_t texture_id);
  std::optional<FlutterError> InvalidIdError();
  std::optional<FlutterError>
  MethodFailedError(const std::string &message = "");
};

} // namespace webview_all_windows
