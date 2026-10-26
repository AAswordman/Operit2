#include <flutter/method_result_functions.h>
#include "plugin/windows_host_api.h"

#include <windows.h>
#include <flutter/standard_method_codec.h>

#include <format>
#include <functional>
#include <optional>
#include <string>

#include "plugin/flutter_view_focus.h"
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
    flutter::PluginRegistrarWindows *registrar, FlutterDesktopViewRef view) {
  auto plugin = std::make_unique<WindowsHostApi>(view,
                                                 registrar->messenger());

  webview_all_windows::WindowsWebViewHostApi::SetUp(registrar->messenger(),
                                                    plugin.get());

  registrar->AddPlugin(std::move(plugin));
}

WindowsHostApi::WindowsHostApi(FlutterDesktopViewRef view,
                               flutter::BinaryMessenger *messenger)
    : view_(view), messenger_(messenger) {
  theme_channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      messenger, "operit/webview_theme", &flutter::StandardMethodCodec::GetInstance());
  // Applies the application preference to live views and retains it for new views.
  theme_channel_->SetMethodCallHandler([this](const auto &call, auto result) {
    if (call.method_name() != "setPreferredColorScheme") {
      result->NotImplemented();
      return;
    }
    const auto *scheme = call.arguments() == nullptr
        ? nullptr : std::get_if<std::string>(call.arguments());
    if (scheme == nullptr || (*scheme != "dark" && *scheme != "light")) {
      result->Error("invalid_color_scheme", "Expected dark or light");
      return;
    }
    const bool preferred_dark = *scheme == "dark";
    for (const auto &entry : instances_) {
      if (!entry.second->SetPreferredColorScheme(preferred_dark)) {
        result->Error("color_scheme_failed", "WebView2 rejected the color preference");
        return;
      }
    }
    result->Success();
  });
  resource_channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      messenger, "operit/webview_resources", &flutter::StandardMethodCodec::GetInstance());
  resource_channel_->SetMethodCallHandler([this](const auto& call, auto result) {
    using V = flutter::EncodableValue;
    using M = flutter::EncodableMap;
    if (call.method_name() != "configure") { result->NotImplemented(); return; }
    const auto* args = call.arguments() ? std::get_if<M>(call.arguments()) : nullptr;
    if (!args || !args->contains(V("identifier")) || !args->contains(V("enabled"))) {
      result->Error("invalid_arguments", "Missing view identifier"); return;
    }
    const int64_t id = args->at(V("identifier")).LongValue();
    const auto found = instances_.find(id);
    if (found == instances_.end()) { result->Error("unknown_view", "WebView no longer exists"); return; }
    Webview::LocalResourceCallback callback;
    if (std::get<bool>(args->at(V("enabled")))) {
      callback = [this, id](const std::string& url, const std::string& method,
          const std::map<std::string, std::string>& headers, bool main_frame,
          Webview::LocalResourceReply reply) {
        M request_headers;
        for (const auto& [name, value] : headers) request_headers[V(name)] = V(value);
        auto payload = std::make_unique<V>(M{
          {V("identifier"), V(id)}, {V("url"), V(url)}, {V("method"), V(method)},
          {V("headers"), V(request_headers)}, {V("isMainFrame"), V(main_frame)}});
        resource_channel_->InvokeMethod("request", std::move(payload),
          std::make_unique<flutter::MethodResultFunctions<V>>(
            [reply](const V* value) {
              const auto* map = value ? std::get_if<M>(value) : nullptr;
              if (!map || !map->contains(V("body"))) { reply(500, "Resource error", {}, {}); return; }
              std::map<std::string, std::string> response_headers;
              for (const auto& [name, item] : std::get<M>(map->at(V("headers")))) {
                response_headers[std::get<std::string>(name)] = std::get<std::string>(item);
              }
              bool has_type = false;
              for (const auto& [name, _] : response_headers) {
                if (_stricmp(name.c_str(), "Content-Type") == 0) has_type = true;
              }
              if (!has_type) response_headers["Content-Type"] =
                std::get<std::string>(map->at(V("mimeType"))) + "; charset=" +
                std::get<std::string>(map->at(V("encoding")));
              reply(static_cast<int>(map->at(V("statusCode")).LongValue()),
                std::get<std::string>(map->at(V("reasonPhrase"))), response_headers,
                std::get<std::vector<uint8_t>>(map->at(V("body"))));
            },
            [reply](const std::string&, const std::string&, const V*) { reply(500, "Resource error", {}, {}); },
            [reply]() { reply(501, "Not implemented", {}, {}); }));
      };
    }
    if (!found->second->SetLocalResourceHandler(std::move(callback))) {
      result->Error("resource_handler_failed", "WebView2 rejected the resource handler"); return;
    }
    result->Success();
  });
  window_class_.lpszClassName = L"FlutterWebviewMessage";
  window_class_.lpfnWndProc = &DefWindowProc;
  RegisterClass(&window_class_);
}

/// Releases WebView resources after the engine has stopped dispatching messages.
WindowsHostApi::~WindowsHostApi() {
  // Engine teardown owns channel cleanup. Its messenger is already detached
  // when the registrar destroys this plugin; channel handlers must not be
  // changed here, including the theme channel handler.
  *alive_ = false;
  instances_.clear();
  if (focus_host_window_ != nullptr) {
    RemoveWindowSubclass(focus_host_window_, FlutterViewFocusProc,
                         reinterpret_cast<UINT_PTR>(this));
  }
  UnregisterClass(window_class_.lpszClassName, nullptr);
}

// Creates an explicitly configured environment once per plugin instance.
void WindowsHostApi::InitializeEnvironment(
    const WindowsEnvironmentOptions& options,
    std::function<void(std::optional<FlutterError>)> result) {
  if (webview_host_ || environment_initializing_) {
    result(FlutterError(kErrorCodeEnvironmentAlreadyInitialized));
    return;
  }
  if (!InitPlatform()) {
    result(FlutterError(kErrorUnsupportedPlatform));
    return;
  }
  environment_waiters_.push_back(std::move(result));
  StartEnvironment(options);
}

// Coalesces browser creation requests while the shared environment initializes.
void WindowsHostApi::StartEnvironment(const WindowsEnvironmentOptions& options) {
  environment_initializing_ = true;
  std::optional<std::wstring> browser;
  if (options.browser_exe_path()) browser = util::Utf16FromUtf8(*options.browser_exe_path());
  const auto directory = options.user_data_path()
      ? std::optional<std::wstring>(util::Utf16FromUtf8(*options.user_data_path()))
      : platform_->GetDefaultDataDirectory();
  std::optional<std::string> arguments;
  if (options.additional_arguments()) arguments = *options.additional_arguments();
  WebviewHost::Create(directory, browser, arguments,
      [this, alive = alive_](std::shared_ptr<WebviewHost> host, HRESULT error) {
        if (!*alive) return;
        environment_initializing_ = false;
        webview_host_ = std::move(host);
        auto pending = std::move(environment_waiters_);
        environment_waiters_.clear();
        for (auto& reply : pending) {
          if (FAILED(error)) {
            reply(FlutterError(kErrorCodeEnvironmentCreationFailed,
                std::format("WebView2 environment HRESULT: {:#010x}", error)));
          } else {
            reply(std::nullopt);
          }
        }
      });
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

/// Creates a browser whose native focus belongs to its containing Flutter view.
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
    environment_waiters_.push_back([this, alive = alive_, result](std::optional<FlutterError> error) {
      if (!*alive) return;
      if (error) { result(*error); return; }
      CreateWebView(result);
    });
    if (!environment_initializing_) StartEnvironment(WindowsEnvironmentOptions());
    return;
  }

  const auto hwnd = FlutterDesktopViewGetHWND(view_);
  if (focus_host_window_ == nullptr) {
    if (!SetWindowSubclass(hwnd, FlutterViewFocusProc,
                           reinterpret_cast<UINT_PTR>(this), 0)) {
      return result(FlutterError("native_focus_failed",
                                "Could not install the Flutter native focus host"));
    }
    focus_host_window_ = hwnd;
  }
  webview_host_->CreateWebview(
      hwnd, true, false,
      [result = std::move(result), host = webview_host_, alive = alive_,
       hwnd, this](std::unique_ptr<Webview> webview,
             std::unique_ptr<WebviewCreationError> error) mutable {
        if (!*alive) return;
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

        // Native descendants do not send WM_KILLFOCUS to the Flutter HWND.
        // Forward a real departure so app deactivation still parks its scope.
        webview->OnFocusChanged([hwnd, alive](bool focused) {
          if (!*alive || focused) return;
          NotifyFlutterViewFocusLost(hwnd);
        });

        auto composition = FlutterDesktopViewCreateNativeComposition(view_);
        if (!composition) {
          return result(FlutterError("native_composition_failed",
                                    "The custom Flutter engine could not create a native visual"));
        }
        if (!webview->AttachVisual(FlutterDesktopNativeCompositionGetVisual(composition))) {
          FlutterDesktopNativeCompositionDestroy(composition);
          return result(FlutterError("native_composition_failed", "WebView2 rejected the native visual"));
        }
        auto bridge = std::make_unique<WebviewBridge>(
            messenger_, composition, std::move(webview));
        auto view_id = bridge->view_id();
        instances_[view_id] = std::move(bridge);

        result(webview_all_windows::WindowsCreateWebViewResult(view_id));
      });
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::DisposeWebView(int64_t view_id) {
  const auto it = instances_.find(view_id);
  if (it != instances_.end()) {
    it->second->DisposeChannels();
    instances_.erase(it);
    return std::nullopt;
  }
  return webview_all_windows::FlutterError(kErrorCodeInvalidId);
}

WebviewBridge *WindowsHostApi::FindBridge(int64_t view_id) {
  const auto it = instances_.find(view_id);
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
WindowsHostApi::LoadUrl(int64_t view_id, const std::string &url) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->LoadUrl(url);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::LoadRequest(
    int64_t view_id,
    const webview_all_windows::WindowsLoadRequestData &request) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::LoadStringContent(int64_t view_id,
                                  const std::string &content) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->LoadStringContent(content);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Reload(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->Reload()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Stop(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->Stop()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::GoBack(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->GoBack()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::GoForward(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->GoForward()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

void WindowsHostApi::AddScriptToExecuteOnDocumentCreated(
    int64_t view_id, const std::string &script,
    std::function<
        void(webview_all_windows::ErrorOr<std::optional<std::string>> reply)>
        result) {
  auto bridge = FindBridge(view_id);
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
    int64_t view_id, const std::string &script_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->RemoveScriptToExecuteOnDocumentCreated(script_id);
  return std::nullopt;
}

void WindowsHostApi::ExecuteScript(
    int64_t view_id, const std::string &script,
    std::function<void(webview_all_windows::ErrorOr<std::string> reply)>
        result) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::PostWebMessage(int64_t view_id, const std::string &message) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::SetUserAgent(int64_t view_id,
                             const std::string *user_agent) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::GetUserAgent(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return webview_all_windows::FlutterError(kErrorCodeInvalidId);
  }
  return bridge->GetUserAgent();
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetJavaScriptEnabled(int64_t view_id, bool enabled) {
  auto bridge = FindBridge(view_id);
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
    int64_t view_id,
    std::function<void(webview_all_windows::ErrorOr<bool> reply)> result) {
  auto bridge = FindBridge(view_id);
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
    int64_t view_id, const webview_all_windows::WindowsCookieData &cookie) {
  auto bridge = FindBridge(view_id);
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
    int64_t view_id, const std::string &url,
    std::function<
        void(webview_all_windows::ErrorOr<flutter::EncodableList> reply)>
        result) {
  auto bridge = FindBridge(view_id);
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
    int64_t view_id, const webview_all_windows::WindowsCookieData &cookie) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::DeleteCookiesWithNameAndUrl(int64_t view_id,
                                            const std::string &name,
                                            const std::string &url) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->DeleteCookiesWithNameAndUrl(name, url)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::DeleteCookiesWithNameDomainAndPath(int64_t view_id,
                                                   const std::string &name,
                                                   const std::string &domain,
                                                   const std::string &path) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->DeleteCookiesWithNameDomainAndPath(name, domain, path)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::ClearCache(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->ClearCache()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

void WindowsHostApi::ClearLocalStorage(
    int64_t view_id,
    std::function<void(std::optional<webview_all_windows::FlutterError> reply)>
        result) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::SetCacheDisabled(int64_t view_id, bool disabled) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->SetCacheDisabled(disabled)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::OpenDevTools(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->OpenDevTools()) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetBackgroundColor(int64_t view_id, int64_t color) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::SetZoomControlEnabled(int64_t view_id, bool enabled) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::SetZoomFactor(int64_t view_id, double zoom_factor) {
  auto bridge = FindBridge(view_id);
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
WindowsHostApi::SetPopupWindowPolicy(int64_t view_id, int64_t policy) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetPopupWindowPolicy(policy);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetJavaScriptDialogCallbacksEnabled(int64_t view_id,
                                                    bool alert, bool confirm,
                                                    bool prompt) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetJavaScriptDialogCallbacksEnabled(alert, confirm, prompt);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Suspend(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->Suspend();
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::Resume(int64_t view_id) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->Resume();
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetVirtualHostNameMapping(
    int64_t view_id,
    const webview_all_windows::WindowsVirtualHostMappingData &mapping) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetVirtualHostNameMapping(mapping.host_name(), mapping.path(),
                                    mapping.access_kind());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::ClearVirtualHostNameMapping(int64_t view_id,
                                            const std::string &host_name) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (!bridge->ClearVirtualHostNameMapping(host_name)) {
    return MethodFailedError();
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetFpsLimit(int64_t view_id, int64_t max_fps) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  if (max_fps != 0) {
    return FlutterError(kErrorNotSupported, "Native WebView2 composition is paced by Windows");
  }
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetPointerUpdate(
    int64_t view_id,
    const webview_all_windows::WindowsPointerUpdateData &update) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetPointerUpdate(update.pointer(), update.event(), update.x(),
                           update.y(), update.size(), update.pressure());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::SetCursorPos(
    int64_t view_id, const webview_all_windows::WindowsPointData &position) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetCursorPos(position.x(), position.y());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetPointerButton(
    int64_t view_id,
    const webview_all_windows::WindowsPointerButtonData &button) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetPointerButtonState(button.button(), button.is_down());
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError> WindowsHostApi::SetScrollDelta(
    int64_t view_id,
    const webview_all_windows::WindowsPointData &delta,
    bool control_key_pressed) {
  auto bridge = FindBridge(view_id);
  if (!bridge) {
    return InvalidIdError();
  }
  bridge->SetScrollDelta(delta.x(), delta.y(), control_key_pressed);
  return std::nullopt;
}

std::optional<webview_all_windows::FlutterError>
WindowsHostApi::SetSize(int64_t view_id,
                        const webview_all_windows::WindowsSizeData &size) {
  auto bridge = FindBridge(view_id);
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
