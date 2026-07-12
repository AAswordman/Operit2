#include "plugin/windows_host_api.h"

#include <windows.h>

#include <format>
#include <functional>
#include <optional>
#include <string>

#include "util/string_converter.h"

#pragma comment(lib, "dxgi.lib")
#pragma comment(lib, "d3d11.lib")

namespace webview_all_windows {
namespace {

constexpr auto kErrorCodeInvalidId = "invalid_id";
constexpr auto kErrorCodeEnvironmentCreationFailed =
    "environment_creation_failed";
constexpr auto kErrorCodeEnvironmentAlreadyInitialized =
    "environment_already_initialized";
constexpr auto kErrorCodeWebviewCreationFailed = "webview_creation_failed";
constexpr auto kErrorUnsupportedPlatform = "unsupported_platform";
constexpr auto kErrorMethodFailed = "method_failed";
constexpr auto kErrorNotSupported = "not_supported";
constexpr auto kErrorScriptFailed = "script_failed";

} // namespace

// static
void WindowsHostApi::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows *registrar) {
  auto plugin = std::make_unique<WindowsHostApi>(registrar->texture_registrar(),
                                                 registrar->messenger());

  webview_all_windows::WindowsWebViewHostApi::SetUp(registrar->messenger(),
                                                    plugin.get());

  registrar->AddPlugin(std::move(plugin));
}

WindowsHostApi::WindowsHostApi(flutter::TextureRegistrar *textures,
                               flutter::BinaryMessenger *messenger)
    : textures_(textures), messenger_(messenger) {
  window_class_.lpszClassName = L"FlutterWebviewMessage";
  window_class_.lpfnWndProc = &DefWindowProc;
  RegisterClass(&window_class_);
}

WindowsHostApi::~WindowsHostApi() {
  webview_all_windows::WindowsWebViewHostApi::SetUp(messenger_, nullptr);
  instances_.clear();
  UnregisterClass(window_class_.lpszClassName, nullptr);
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::InitializeEnvironment(
    const webview_all_windows::WindowsEnvironmentOptions &options) {
  if (webview_host_) {
    return webview_all_windows::FlutterError(
        kErrorCodeEnvironmentAlreadyInitialized,
        "The webview environment is already initialized");
  }

  if (!InitPlatform()) {
    return webview_all_windows::FlutterError(kErrorUnsupportedPlatform,
                                             "The platform is not supported");
  }

  std::optional<std::wstring> browser_exe_wpath = std::nullopt;
  if (options.browser_exe_path()) {
    browser_exe_wpath = util::Utf16FromUtf8(*options.browser_exe_path());
  }

  std::optional<std::wstring> user_data_wpath = std::nullopt;
  if (options.user_data_path()) {
    user_data_wpath = util::Utf16FromUtf8(*options.user_data_path());
  } else {
    user_data_wpath = platform_->GetDefaultDataDirectory();
  }

  std::optional<std::string> additional_args = std::nullopt;
  if (options.additional_arguments()) {
    additional_args = *options.additional_arguments();
  }

  webview_host_ = std::move(WebviewHost::Create(
      platform_.get(), user_data_wpath, browser_exe_wpath, additional_args));
  if (!webview_host_) {
    return webview_all_windows::FlutterError(
        kErrorCodeEnvironmentCreationFailed);
  }

  return std::nullopt;
}

webview_all_windows::ErrorOr<std::optional<std::string>>
WindowsHostApi::GetWebViewVersion() {
  LPWSTR version_info = nullptr;
  auto hr =
      GetAvailableCoreWebView2BrowserVersionString(nullptr, &version_info);
  if (SUCCEEDED(hr) && version_info != nullptr) {
    return std::optional<std::string>(util::Utf8FromUtf16(version_info));
  }
  return std::optional<std::string>();
}

void WindowsHostApi::CreateWebView(
    std::function<void(webview_all_windows::ErrorOr<
                       webview_all_windows::WindowsCreateWebViewResult>
                           reply)>
        result) {
  if (!InitPlatform()) {
    return result(webview_all_windows::FlutterError(
        kErrorUnsupportedPlatform, "The platform is not supported"));
  }

  if (!webview_host_) {
    webview_host_ = std::move(WebviewHost::Create(
        platform_.get(), platform_->GetDefaultDataDirectory()));
    if (!webview_host_) {
      return result(webview_all_windows::FlutterError(
          kErrorCodeEnvironmentCreationFailed));
    }
  }

  auto hwnd =
      CreateWindowEx(0, window_class_.lpszClassName, L"", 0, 0, 0, 0, 0,
                     HWND_MESSAGE, nullptr, window_class_.hInstance, nullptr);

  webview_host_->CreateWebview(
      hwnd, true, true,
      [result = std::move(result),
       this](std::unique_ptr<Webview> webview,
             std::unique_ptr<WebviewCreationError> error) mutable {
        if (!webview) {
          if (error) {
            return result(webview_all_windows::FlutterError(
                kErrorCodeWebviewCreationFailed,
                std::format(
                    "Creating the webview failed: {} (HRESULT: {:#010x})",
                    error->message, error->hr)));
          }
          return result(webview_all_windows::FlutterError(
              kErrorCodeWebviewCreationFailed, "Creating the webview failed."));
        }

        auto bridge = std::make_unique<WebviewBridge>(
            messenger_, textures_, platform_->graphics_context(),
            std::move(webview));
        auto texture_id = bridge->texture_id();
        instances_[texture_id] = std::move(bridge);

        result(webview_all_windows::WindowsCreateWebViewResult(texture_id));
      });
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::DisposeWebView(int64_t texture_id) {
  const auto it = instances_.find(texture_id);
  if (it != instances_.end()) {
    instances_.erase(it);
    return std::nullopt;
  }
  return webview_all_windows::FlutterError(kErrorCodeInvalidId);
}

WebviewBridge *WindowsHostApi::FindBridge(int64_t texture_id) {
  const auto it = instances_.find(texture_id);
  if (it == instances_.end()) {
    return nullptr;
  }
  return it->second.get();
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::InvalidIdError() {
  return webview_all_windows::FlutterError(kErrorCodeInvalidId);
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::MethodFailedError(const std::string &message) {
  return webview_all_windows::FlutterError(kErrorMethodFailed, message);
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::LoadUrl(int64_t texture_id, const std::string &url) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->LoadUrl(url);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::LoadRequest(
    int64_t texture_id,
    const webview_all_windows::WindowsLoadRequestData &request) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->LoadRequest(request.url(), request.method(), request.headers(),
                           request.body())) {
    return MethodFailedError("Loading the request failed.");
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::LoadStringContent(int64_t texture_id,
                                  const std::string &content) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->LoadStringContent(content);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Reload(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->Reload()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Stop(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->Stop()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::GoBack(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->GoBack()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::GoForward(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->GoForward()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

void WindowsHostApi::AddScriptToExecuteOnDocumentCreated(
    int64_t texture_id, const std::string &script,
    std::function<
        void(webview_all_windows::ErrorOr<std::optional<std::string>> reply)>
        result) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return result(webview_all_windows::FlutterError(kErrorCodeInvalidId));
  }
  bridge->AddScriptToExecuteOnDocumentCreated(
      script, [result = std::move(result)](
                  bool success, const std::string &script_id) mutable {
        if (success) {
          return result(std::optional<std::string>(script_id));
        }
        return result(webview_all_windows::FlutterError(
            kErrorScriptFailed, "Adding the document-created script failed."));
      });
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::RemoveScriptToExecuteOnDocumentCreated(
    int64_t texture_id, const std::string &script_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->RemoveScriptToExecuteOnDocumentCreated(script_id);
  return std::nullopt;
}

void WindowsHostApi::ExecuteScript(
    int64_t texture_id, const std::string &script,
    std::function<void(webview_all_windows::ErrorOr<std::string> reply)>
        result) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return result(webview_all_windows::FlutterError(kErrorCodeInvalidId));
  }
  bridge->ExecuteScript(
      script, [result = std::move(result)](
                  bool success, const std::string &json_result) mutable {
        if (success) {
          return result(json_result);
        }
        return result(webview_all_windows::FlutterError(
            kErrorScriptFailed, "Executing the script failed."));
      });
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::PostWebMessage(int64_t texture_id, const std::string &message) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->PostWebMessage(message)) {
    return webview_all_windows::FlutterError(kErrorNotSupported,
                                             "Posting the message failed.");
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetUserAgent(int64_t texture_id,
                             const std::string *user_agent) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->SetUserAgent(user_agent)) {
    return webview_all_windows::FlutterError(kErrorNotSupported,
                                             "Setting the user agent failed.");
  }
  return std::nullopt;
}

webview_all_windows::ErrorOr<std::optional<std::string>>
WindowsHostApi::GetUserAgent(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return webview_all_windows::FlutterError(kErrorCodeInvalidId);
  }
  return bridge->GetUserAgent();
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetJavaScriptEnabled(int64_t texture_id, bool enabled) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->SetJavaScriptEnabled(enabled)) {
    return webview_all_windows::FlutterError(kErrorNotSupported,
                                             "Setting JavaScript mode failed.");
  }
  return std::nullopt;
}

void WindowsHostApi::ClearCookies(
    int64_t texture_id,
    std::function<void(webview_all_windows::ErrorOr<bool> reply)> result) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return result(webview_all_windows::FlutterError(kErrorCodeInvalidId));
  }
  bridge->ClearCookies(
      [result = std::move(result)](bool success, bool had_cookies) mutable {
        if (success) {
          return result(had_cookies);
        }
        return result(webview_all_windows::FlutterError(kErrorMethodFailed));
      });
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::SetCookie(
    int64_t texture_id, const webview_all_windows::WindowsCookieData &cookie) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  WebviewCookie native_cookie{cookie.name(), cookie.value(), cookie.domain(),
                              cookie.path()};
  if (cookie.expires()) {
    native_cookie.expires = *cookie.expires();
  }
  if (cookie.is_http_only()) {
    native_cookie.is_http_only = *cookie.is_http_only();
  }
  if (cookie.is_secure()) {
    native_cookie.is_secure = *cookie.is_secure();
  }
  if (cookie.same_site()) {
    native_cookie.same_site = *cookie.same_site();
  }
  if (!bridge->SetCookie(native_cookie)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

void WindowsHostApi::GetCookies(
    int64_t texture_id, const std::string &url,
    std::function<
        void(webview_all_windows::ErrorOr<flutter::EncodableList> reply)>
        result) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return result(webview_all_windows::FlutterError(kErrorCodeInvalidId));
  }
  bridge->GetCookies(url, [result = std::move(result)](
                              bool success,
                              std::vector<WebviewCookie> cookies) mutable {
    if (!success) {
      return result(webview_all_windows::FlutterError(kErrorMethodFailed));
    }
    flutter::EncodableList encoded_cookies;
    encoded_cookies.reserve(cookies.size());
    for (const auto &cookie : cookies) {
      const double *expires =
          cookie.expires.has_value() ? &cookie.expires.value() : nullptr;
      const bool *is_http_only = cookie.is_http_only.has_value()
                                     ? &cookie.is_http_only.value()
                                     : nullptr;
      const bool *is_secure =
          cookie.is_secure.has_value() ? &cookie.is_secure.value() : nullptr;
      const int64_t *same_site =
          cookie.same_site.has_value() ? &cookie.same_site.value() : nullptr;
      const bool *is_session =
          cookie.is_session.has_value() ? &cookie.is_session.value() : nullptr;
      encoded_cookies.push_back(
          flutter::CustomEncodableValue(webview_all_windows::WindowsCookieData(
              cookie.name, cookie.value, cookie.domain, cookie.path, expires,
              is_http_only, is_secure, same_site, is_session)));
    }
    return result(std::move(encoded_cookies));
  });
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::DeleteCookie(
    int64_t texture_id, const webview_all_windows::WindowsCookieData &cookie) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  WebviewCookie native_cookie{cookie.name(), cookie.value(), cookie.domain(),
                              cookie.path()};
  if (cookie.expires()) {
    native_cookie.expires = *cookie.expires();
  }
  if (cookie.is_http_only()) {
    native_cookie.is_http_only = *cookie.is_http_only();
  }
  if (cookie.is_secure()) {
    native_cookie.is_secure = *cookie.is_secure();
  }
  if (cookie.same_site()) {
    native_cookie.same_site = *cookie.same_site();
  }
  if (!bridge->DeleteCookie(native_cookie)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::DeleteCookiesWithNameAndUrl(int64_t texture_id,
                                            const std::string &name,
                                            const std::string &url) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->DeleteCookiesWithNameAndUrl(name, url)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::DeleteCookiesWithNameDomainAndPath(int64_t texture_id,
                                                   const std::string &name,
                                                   const std::string &domain,
                                                   const std::string &path) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->DeleteCookiesWithNameDomainAndPath(name, domain, path)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::ClearCache(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->ClearCache()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

void WindowsHostApi::ClearLocalStorage(
    int64_t texture_id,
    std::function<void(std::optional<webview_all_windows::FlutterError> reply)>
        result) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return result(InvalidIdError());
  }

  bridge->ClearLocalStorage([result = std::move(result)](bool success) mutable {
    if (success) {
      return result(std::nullopt);
    }
    result(webview_all_windows::FlutterError(kErrorMethodFailed));
  });
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetCacheDisabled(int64_t texture_id, bool disabled) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->SetCacheDisabled(disabled)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::OpenDevTools(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->OpenDevTools()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetBackgroundColor(int64_t texture_id, int64_t color) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->SetBackgroundColor(color)) {
    return webview_all_windows::FlutterError(
        kErrorNotSupported, "Setting the background color failed.");
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetZoomControlEnabled(int64_t texture_id, bool enabled) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->SetZoomControlEnabled(enabled)) {
    return webview_all_windows::FlutterError(
        kErrorNotSupported, "Setting the zoom control mode failed.");
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetZoomFactor(int64_t texture_id, double zoom_factor) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->SetZoomFactor(zoom_factor)) {
    return webview_all_windows::FlutterError(kErrorNotSupported,
                                             "Setting the zoom factor failed.");
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetPopupWindowPolicy(int64_t texture_id, int64_t policy) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetPopupWindowPolicy(policy);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetJavaScriptDialogCallbacksEnabled(int64_t texture_id,
                                                    bool alert, bool confirm,
                                                    bool prompt) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetJavaScriptDialogCallbacksEnabled(alert, confirm, prompt);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Suspend(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->Suspend();
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Resume(int64_t texture_id) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->Resume();
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetVirtualHostNameMapping(
    int64_t texture_id,
    const webview_all_windows::WindowsVirtualHostMappingData &mapping) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetVirtualHostNameMapping(mapping.host_name(), mapping.path(),
                                    mapping.access_kind());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::ClearVirtualHostNameMapping(int64_t texture_id,
                                            const std::string &host_name) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->ClearVirtualHostNameMapping(host_name)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetFpsLimit(int64_t texture_id, int64_t max_fps) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetFpsLimit(max_fps);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetPointerUpdate(
    int64_t texture_id,
    const webview_all_windows::WindowsPointerUpdateData &update) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetPointerUpdate(update.pointer(), update.event(), update.x(),
                           update.y(), update.size(), update.pressure());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::SetCursorPos(
    int64_t texture_id, const webview_all_windows::WindowsPointData &position) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetCursorPos(position.x(), position.y());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetPointerButton(
    int64_t texture_id,
    const webview_all_windows::WindowsPointerButtonData &button) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetPointerButtonState(button.button(), button.is_down());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::SetScrollDelta(
    int64_t texture_id, const webview_all_windows::WindowsPointData &delta) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetScrollDelta(delta.x(), delta.y());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetSize(int64_t texture_id,
                        const webview_all_windows::WindowsSizeData &size) {
  auto bridge = FindBridge(texture_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetSize(size.width(), size.height(), size.scale_factor());
  return std::nullopt;
}

bool WindowsHostApi::InitPlatform() {
  if (!platform_) {
    platform_ = std::make_unique<WebviewPlatform>();
  }
  return platform_->IsSupported();
}

} // namespace webview_all_windows
