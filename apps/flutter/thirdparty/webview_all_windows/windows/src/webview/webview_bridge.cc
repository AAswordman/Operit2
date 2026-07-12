#include "webview/webview_bridge.h"

#include <flutter/event_stream_handler_functions.h>
#include <flutter/method_result_functions.h>

#include <format>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <utility>

#include "rendering/texture_bridge_gpu.h"

namespace webview_all_windows {
namespace {
constexpr auto kEventType = "type";
constexpr auto kEventValue = "value";

constexpr auto kChannelPrefix = "com.abandoft.webview_all_windows.webview";

static const std::string &GetCursorName(const HCURSOR cursor) {
  // The cursor names correspond to the Flutter Engine names:
  // in shell/platform/windows/flutter_window_win32.cc
  static const std::string kDefaultCursorName = "basic";
  static const std::pair<std::string, const wchar_t *> mappings[] = {
      {"allScroll", IDC_SIZEALL},
      {kDefaultCursorName, IDC_ARROW},
      {"click", IDC_HAND},
      {"forbidden", IDC_NO},
      {"help", IDC_HELP},
      {"move", IDC_SIZEALL},
      {"none", nullptr},
      {"noDrop", IDC_NO},
      {"precise", IDC_CROSS},
      {"progress", IDC_APPSTARTING},
      {"text", IDC_IBEAM},
      {"resizeColumn", IDC_SIZEWE},
      {"resizeDown", IDC_SIZENS},
      {"resizeDownLeft", IDC_SIZENESW},
      {"resizeDownRight", IDC_SIZENWSE},
      {"resizeLeft", IDC_SIZEWE},
      {"resizeLeftRight", IDC_SIZEWE},
      {"resizeRight", IDC_SIZEWE},
      {"resizeRow", IDC_SIZENS},
      {"resizeUp", IDC_SIZENS},
      {"resizeUpDown", IDC_SIZENS},
      {"resizeUpLeft", IDC_SIZENWSE},
      {"resizeUpRight", IDC_SIZENESW},
      {"resizeUpLeftDownRight", IDC_SIZENWSE},
      {"resizeUpRightDownLeft", IDC_SIZENESW},
      {"wait", IDC_WAIT},
  };

  static std::map<HCURSOR, std::string> cursors;
  static bool initialized = false;

  if (!initialized) {
    initialized = true;
    for (const auto &pair : mappings) {
      HCURSOR cursor_handle = LoadCursor(nullptr, pair.second);
      if (cursor_handle) {
        cursors[cursor_handle] = pair.first;
      }
    }
  }

  const auto it = cursors.find(cursor);
  if (it != cursors.end()) {
    return it->second;
  }
  return kDefaultCursorName;
}

static std::string JavaScriptDialogKindName(WebviewJavaScriptDialogKind kind) {
  switch (kind) {
  case WebviewJavaScriptDialogKind::Alert:
    return "alert";
  case WebviewJavaScriptDialogKind::Confirm:
    return "confirm";
  case WebviewJavaScriptDialogKind::Prompt:
    return "prompt";
  case WebviewJavaScriptDialogKind::BeforeUnload:
    return "beforeUnload";
  }
  return "alert";
}

flutter::EncodableMap
HeadersToEncodableMap(const std::map<std::string, std::string> &headers) {
  flutter::EncodableMap result;
  for (const auto &[name, value] : headers) {
    result[flutter::EncodableValue(name)] = flutter::EncodableValue(value);
  }
  return result;
}

} // namespace

WebviewBridge::WebviewBridge(flutter::BinaryMessenger *messenger,
                             flutter::TextureRegistrar *texture_registrar,
                             GraphicsContext *graphics_context,
                             std::unique_ptr<Webview> webview)
    : webview_(std::move(webview)), texture_registrar_(texture_registrar) {
  texture_bridge_ =
      std::make_unique<TextureBridgeGpu>(graphics_context, webview_->surface());

  flutter_texture_ =
      std::make_unique<flutter::TextureVariant>(flutter::GpuSurfaceTexture(
          kFlutterDesktopGpuSurfaceTypeDxgiSharedHandle,
          [bridge = static_cast<TextureBridgeGpu *>(texture_bridge_.get())](
              size_t width,
              size_t height) -> const FlutterDesktopGpuSurfaceDescriptor * {
            return bridge->GetSurfaceDescriptor(width, height);
          }));

  texture_id_ = texture_registrar->RegisterTexture(flutter_texture_.get());
  texture_bridge_->SetOnFrameAvailable([this]() { OnTextureFrameAvailable(); });
  // texture_bridge_->SetOnSurfaceSizeChanged([this](Size size) {
  //  webview_->SetSurfaceSize(size.width, size.height);
  //});

  const auto method_channel_name =
      std::format("{}/{}", kChannelPrefix, texture_id_);
  method_channel_ =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          messenger, method_channel_name,
          &flutter::StandardMethodCodec::GetInstance());
  method_channel_->SetMethodCallHandler(
      [this](const flutter::MethodCall<flutter::EncodableValue> &call,
             std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
                 result) {
        if (call.method_name() == "captureSurfaceFrame") {
          auto result_holder =
              std::make_shared<std::unique_ptr<
                  flutter::MethodResult<flutter::EncodableValue>>>(
                  std::move(result));
          CaptureSurfaceFrame(
              [result_holder](bool success, const std::vector<uint8_t> &data,
                              size_t width, size_t height) {
                auto result = std::move(*result_holder);
                if (!result) {
                  return;
                }
                if (!success) {
                  result->Error("method_failed",
                                "Capturing the surface frame failed");
                  return;
                }
                result->Success(flutter::EncodableValue(flutter::EncodableMap{
                    {flutter::EncodableValue("codec"),
                     flutter::EncodableValue("raw/bgra")},
                    {flutter::EncodableValue("data"),
                     flutter::EncodableValue(data)},
                    {flutter::EncodableValue("width"),
                     flutter::EncodableValue(static_cast<int64_t>(width))},
                    {flutter::EncodableValue("height"),
                     flutter::EncodableValue(static_cast<int64_t>(height))},
                }));
              });
          return;
        }
        if (call.method_name() != "dispatchKeyEvent") {
          result->NotImplemented();
          return;
        }
        const auto arguments = call.arguments();
        const auto event_json =
            arguments == nullptr ? nullptr : std::get_if<std::string>(arguments);
        if (event_json == nullptr) {
          result->Error("invalid_arguments",
                        "dispatchKeyEvent requires a JSON string payload");
          return;
        }
        if (!webview_->DispatchKeyEvent(*event_json)) {
          result->Error("method_failed", "Dispatching the key event failed");
          return;
        }
        result->Success();
      });

  const auto event_channel_name =
      std::format("{}/{}/events", kChannelPrefix, texture_id_);
  event_channel_ =
      std::make_unique<flutter::EventChannel<flutter::EncodableValue>>(
          messenger, event_channel_name,
          &flutter::StandardMethodCodec::GetInstance());

  auto handler = std::make_unique<
      flutter::StreamHandlerFunctions<flutter::EncodableValue>>(
      [this](const flutter::EncodableValue *arguments,
             std::unique_ptr<flutter::EventSink<flutter::EncodableValue>>
                 &&events) {
        event_sink_ = std::move(events);
        RegisterEventHandlers();
        return nullptr;
      },
      [this](const flutter::EncodableValue *arguments) {
        event_sink_ = nullptr;
        return nullptr;
      });

  event_channel_->SetStreamHandler(std::move(handler));
}

WebviewBridge::~WebviewBridge() {
  texture_bridge_->SetOnFrameAvailable({});
  texture_bridge_->StopFrameReadback();
  texture_registrar_->UnregisterTexture(texture_id_);
}

void WebviewBridge::RegisterEventHandlers() {
  webview_->OnUrlChanged([this](const std::string &url) {
    const auto event = flutter::EncodableValue(flutter::EncodableMap{
        {flutter::EncodableValue(kEventType),
         flutter::EncodableValue("urlChanged")},
        {flutter::EncodableValue(kEventValue), flutter::EncodableValue(url)},
    });
    EmitEvent(event);
  });

  webview_->OnLoadError([this](COREWEBVIEW2_WEB_ERROR_STATUS web_status) {
    const auto event = flutter::EncodableValue(flutter::EncodableMap{
        {flutter::EncodableValue(kEventType),
         flutter::EncodableValue("onLoadError")},
        {flutter::EncodableValue(kEventValue),
         flutter::EncodableValue(static_cast<int>(web_status))},
    });
    EmitEvent(event);
  });

  webview_->OnHttpResponseError(
      [this](const WebviewHttpResponseError &http_error) {
        flutter::EncodableMap value = {
            {flutter::EncodableValue("url"),
             flutter::EncodableValue(http_error.url)},
            {flutter::EncodableValue("method"),
             flutter::EncodableValue(http_error.method)},
            {flutter::EncodableValue("requestHeaders"),
             flutter::EncodableValue(
                 HeadersToEncodableMap(http_error.request_headers))},
            {flutter::EncodableValue("statusCode"),
             flutter::EncodableValue(http_error.status_code)},
            {flutter::EncodableValue("responseHeaders"),
             flutter::EncodableValue(
                 HeadersToEncodableMap(http_error.response_headers))},
        };
        if (http_error.reason_phrase.has_value()) {
          value[flutter::EncodableValue("reasonPhrase")] =
              flutter::EncodableValue(http_error.reason_phrase.value());
        }

        const auto event = flutter::EncodableValue(flutter::EncodableMap{
            {flutter::EncodableValue(kEventType),
             flutter::EncodableValue("httpError")},
            {flutter::EncodableValue(kEventValue),
             flutter::EncodableValue(value)},
        });
        EmitEvent(event);
      });

  webview_->OnLoadingStateChanged([this](WebviewLoadingState state) {
    const auto event = flutter::EncodableValue(flutter::EncodableMap{
        {flutter::EncodableValue(kEventType),
         flutter::EncodableValue("loadingStateChanged")},
        {flutter::EncodableValue(kEventValue),
         flutter::EncodableValue(static_cast<int>(state))},
    });
    EmitEvent(event);
  });

  webview_->OnDownloadEvent([this](WebviewDownloadEvent webviewDownloadEvent) {
    const auto event = flutter::EncodableValue(flutter::EncodableMap{
        {flutter::EncodableValue(kEventType),
         flutter::EncodableValue("downloadEvent")},
        {flutter::EncodableValue(kEventValue),
         flutter::EncodableValue(flutter::EncodableMap{
             {flutter::EncodableValue("kind"),
              flutter::EncodableValue(
                  static_cast<int>(webviewDownloadEvent.kind))},
             {flutter::EncodableValue("url"),
              flutter::EncodableValue(webviewDownloadEvent.url)},
             {flutter::EncodableValue("resultFilePath"),
              flutter::EncodableValue(webviewDownloadEvent.resultFilePath)},
             {flutter::EncodableValue("bytesReceived"),
              flutter::EncodableValue(webviewDownloadEvent.bytesReceived)},
             {flutter::EncodableValue("totalBytesToReceive"),
              flutter::EncodableValue(
                  webviewDownloadEvent.totalBytesToReceive)},
         })}});
    EmitEvent(event);
  });

  webview_->OnHistoryChanged([this](WebviewHistoryChanged historyChanged) {
    const auto event = flutter::EncodableValue(flutter::EncodableMap{
        {flutter::EncodableValue(kEventType),
         flutter::EncodableValue("historyChanged")},
        {flutter::EncodableValue(kEventValue),
         flutter::EncodableValue(flutter::EncodableMap{
             {flutter::EncodableValue("canGoBack"),
              flutter::EncodableValue(
                  static_cast<bool>(historyChanged.can_go_back))},
             {flutter::EncodableValue("canGoForward"),
              flutter::EncodableValue(
                  static_cast<bool>(historyChanged.can_go_forward))},
         })},
    });
    EmitEvent(event);
  });

  webview_->OnDevtoolsProtocolEvent([this](const std::string &json) {
    const auto event = flutter::EncodableValue(flutter::EncodableMap{
        {flutter::EncodableValue(kEventType),
         flutter::EncodableValue("securityStateChanged")},
        {flutter::EncodableValue(kEventValue), flutter::EncodableValue(json)}});
    EmitEvent(event);
  });

  webview_->OnDocumentTitleChanged([this](const std::string &title) {
    const auto event = flutter::EncodableValue(flutter::EncodableMap{
        {flutter::EncodableValue(kEventType),
         flutter::EncodableValue("titleChanged")},
        {flutter::EncodableValue(kEventValue), flutter::EncodableValue(title)},
    });
    EmitEvent(event);
  });

  webview_->OnSurfaceSizeChanged([this](size_t width, size_t height) {
    texture_bridge_->NotifySurfaceSizeChanged();
  });

  webview_->OnCursorChanged([this](const HCURSOR cursor) {
    const auto &name = GetCursorName(cursor);
    const auto event = flutter::EncodableValue(
        flutter::EncodableMap{{flutter::EncodableValue(kEventType),
                               flutter::EncodableValue("cursorChanged")},
                              {flutter::EncodableValue(kEventValue), name}});
    EmitEvent(event);
  });

  webview_->OnWebMessageReceived([this](const std::string &message) {
    const auto event = flutter::EncodableValue(
        flutter::EncodableMap{{flutter::EncodableValue(kEventType),
                               flutter::EncodableValue("webMessageReceived")},
                              {flutter::EncodableValue(kEventValue), message}});
    EmitEvent(event);
  });

  webview_->OnPermissionRequested(
      [this](const std::string &url, WebviewPermissionKind kind,
             bool is_user_initiated,
             Webview::WebviewPermissionRequestedCompleter completer) {
        OnPermissionRequested(url, kind, is_user_initiated, completer);
      });

  webview_->OnHttpAuthRequested(
      [this](const WebviewHttpAuthRequest &request,
             Webview::WebviewHttpAuthRequestedCompleter completer) {
        OnHttpAuthRequested(request, completer);
      });

  webview_->OnSslAuthError(
      [this](const WebviewSslAuthError &error,
             Webview::WebviewSslAuthErrorCompleter completer) {
        OnSslAuthError(error, completer);
      });

  webview_->OnJavaScriptDialogRequested(
      [this](const WebviewJavaScriptDialogRequest &request,
             Webview::WebviewJavaScriptDialogCompleter completer) {
        OnJavaScriptDialogRequested(request, completer);
      });

  webview_->OnContainsFullScreenElementChanged(
      [this](bool contains_fullscreen_element) {
        const auto event = flutter::EncodableValue(flutter::EncodableMap{
            {flutter::EncodableValue(kEventType),
             flutter::EncodableValue("containsFullScreenElementChanged")},
            {flutter::EncodableValue(kEventValue),
             contains_fullscreen_element}});
        EmitEvent(event);
      });
}

void WebviewBridge::OnPermissionRequested(
    const std::string &url, WebviewPermissionKind permissionKind,
    bool isUserInitiated,
    Webview::WebviewPermissionRequestedCompleter completer) {
  auto args = std::make_unique<flutter::EncodableValue>(flutter::EncodableMap{
      {"url", url},
      {"isUserInitiated", isUserInitiated},
      {"permissionKind", static_cast<int>(permissionKind)}});

  method_channel_->InvokeMethod(
      "permissionRequested", std::move(args),
      std::make_unique<flutter::MethodResultFunctions<flutter::EncodableValue>>(
          [completer](const flutter::EncodableValue *result) {
            auto allow = std::get_if<bool>(result);
            if (allow != nullptr) {
              return completer(*allow ? WebviewPermissionState::Allow
                                      : WebviewPermissionState::Deny);
            }
            completer(WebviewPermissionState::Default);
          },
          [completer](const std::string &error_code,
                      const std::string &error_message,
                      const flutter::EncodableValue *error_details) {
            completer(WebviewPermissionState::Default);
          },
          [completer]() { completer(WebviewPermissionState::Default); }));
}

void WebviewBridge::OnHttpAuthRequested(
    const WebviewHttpAuthRequest &request,
    Webview::WebviewHttpAuthRequestedCompleter completer) {
  auto args = std::make_unique<flutter::EncodableValue>(flutter::EncodableMap{
      {"url", request.url},
      {"challenge", request.challenge},
  });

  method_channel_->InvokeMethod(
      "httpAuthRequested", std::move(args),
      std::make_unique<flutter::MethodResultFunctions<flutter::EncodableValue>>(
          [completer](const flutter::EncodableValue *result) {
            const auto response =
                result == nullptr ? nullptr
                                  : std::get_if<flutter::EncodableMap>(result);
            if (response == nullptr) {
              completer(false, "", "");
              return;
            }

            bool accepted = false;
            auto action_it = response->find(flutter::EncodableValue("action"));
            if (action_it != response->end()) {
              const auto action = std::get_if<std::string>(&action_it->second);
              accepted = action != nullptr && *action == "proceed";
            }

            std::string user;
            auto user_it = response->find(flutter::EncodableValue("user"));
            if (user_it != response->end()) {
              if (const auto response_user =
                      std::get_if<std::string>(&user_it->second)) {
                user = *response_user;
              }
            }

            std::string password;
            auto password_it =
                response->find(flutter::EncodableValue("password"));
            if (password_it != response->end()) {
              if (const auto response_password =
                      std::get_if<std::string>(&password_it->second)) {
                password = *response_password;
              }
            }

            completer(accepted, user, password);
          },
          [completer](const std::string &error_code,
                      const std::string &error_message,
                      const flutter::EncodableValue *error_details) {
            completer(false, "", "");
          },
          [completer]() { completer(false, "", ""); }));
}

void WebviewBridge::OnSslAuthError(
    const WebviewSslAuthError &error,
    Webview::WebviewSslAuthErrorCompleter completer) {
  auto args = std::make_unique<flutter::EncodableValue>(flutter::EncodableMap{
      {"url", error.url},
      {"errorStatus", static_cast<int>(error.status)},
  });

  method_channel_->InvokeMethod(
      "sslAuthError", std::move(args),
      std::make_unique<flutter::MethodResultFunctions<flutter::EncodableValue>>(
          [completer](const flutter::EncodableValue *result) {
            const auto response =
                result == nullptr ? nullptr
                                  : std::get_if<flutter::EncodableMap>(result);
            if (response == nullptr) {
              completer(false);
              return;
            }

            bool proceed = false;
            auto action_it = response->find(flutter::EncodableValue("action"));
            if (action_it != response->end()) {
              const auto action = std::get_if<std::string>(&action_it->second);
              proceed = action != nullptr && *action == "proceed";
            }
            completer(proceed);
          },
          [completer](const std::string &error_code,
                      const std::string &error_message,
                      const flutter::EncodableValue *error_details) {
            completer(false);
          },
          [completer]() { completer(false); }));
}

void WebviewBridge::OnJavaScriptDialogRequested(
    const WebviewJavaScriptDialogRequest &request,
    Webview::WebviewJavaScriptDialogCompleter completer) {
  flutter::EncodableMap dialog_request{
      {"dialogType", JavaScriptDialogKindName(request.kind)},
      {"url", request.url},
      {"message", request.message},
  };
  if (request.default_text) {
    dialog_request[flutter::EncodableValue("defaultText")] =
        flutter::EncodableValue(*request.default_text);
  }

  auto args =
      std::make_unique<flutter::EncodableValue>(std::move(dialog_request));
  method_channel_->InvokeMethod(
      "javaScriptDialogRequested", std::move(args),
      std::make_unique<flutter::MethodResultFunctions<flutter::EncodableValue>>(
          [completer](const flutter::EncodableValue *result) {
            const auto response =
                result == nullptr ? nullptr
                                  : std::get_if<flutter::EncodableMap>(result);
            if (response == nullptr) {
              completer(false, std::nullopt);
              return;
            }

            bool accepted = false;
            auto action_it = response->find(flutter::EncodableValue("action"));
            if (action_it != response->end()) {
              const auto action = std::get_if<std::string>(&action_it->second);
              accepted = action != nullptr &&
                         (*action == "accept" || *action == "confirm");
            }

            std::optional<std::string> text;
            auto text_it = response->find(flutter::EncodableValue("text"));
            if (text_it != response->end()) {
              const auto response_text =
                  std::get_if<std::string>(&text_it->second);
              if (response_text != nullptr) {
                text = *response_text;
              }
            }

            completer(accepted, text);
          },
          [completer](const std::string &error_code,
                      const std::string &error_message,
                      const flutter::EncodableValue *error_details) {
            completer(false, std::nullopt);
          },
          [completer]() { completer(false, std::nullopt); }));
}

void WebviewBridge::SetCursorPos(double x, double y) {
  webview_->SetCursorPos(x, y);
}

void WebviewBridge::SetPointerUpdate(int64_t pointer, int64_t event, double x,
                                     double y, double size, double pressure) {
  webview_->SetPointerUpdate(static_cast<int32_t>(pointer),
                             static_cast<WebviewPointerEventKind>(event), x, y,
                             size, pressure);
}

void WebviewBridge::SetScrollDelta(double dx, double dy) {
  webview_->SetScrollDelta(dx, dy);
}

void WebviewBridge::SetPointerButtonState(int64_t button, bool is_down) {
  webview_->SetPointerButtonState(static_cast<WebviewPointerButton>(button),
                                  is_down);
}

void WebviewBridge::SetSize(double width, double height, double scale_factor) {
  webview_->SetSurfaceSize(static_cast<size_t>(width),
                           static_cast<size_t>(height),
                           static_cast<float>(scale_factor));
  texture_bridge_->Start();
}

bool WebviewBridge::DispatchKeyEvent(const std::string &event_json) {
  return webview_->DispatchKeyEvent(event_json);
}

// Completes frame captures waiting for the compositor's first available frame.
void WebviewBridge::OnTextureFrameAvailable() {
  texture_registrar_->MarkTextureFrameAvailable(texture_id_);

  bool start_readback = false;
  {
    const std::lock_guard<std::mutex> lock(surface_frame_capture_mutex_);
    if (!pending_surface_frame_captures_.empty() &&
        !surface_frame_readback_active_) {
      surface_frame_readback_active_ = true;
      start_readback = true;
    }
  }
  if (start_readback) {
    StartSurfaceFrameReadback();
  }
}

// Starts one asynchronous readback for all currently pending captures.
void WebviewBridge::StartSurfaceFrameReadback() {
  texture_bridge_->CopyLatestFrameAsync(
      [this](bool success, const std::vector<uint8_t> &data, Size size) {
        OnSurfaceFrameReadbackCompleted(success, data, size);
      });
}

// Completes pending method calls with one asynchronously read compositor frame.
void WebviewBridge::OnSurfaceFrameReadbackCompleted(
    bool success, const std::vector<uint8_t> &data, Size size) {
  std::vector<SurfaceFrameCaptureCallback> pending_captures;
  {
    const std::lock_guard<std::mutex> lock(surface_frame_capture_mutex_);
    surface_frame_readback_active_ = false;
    pending_captures.swap(pending_surface_frame_captures_);
  }
  for (auto &capture : pending_captures) {
    capture(success, data, size.width, size.height);
  }
}

// Captures one compositor frame or waits for the first frame to arrive.
void WebviewBridge::CaptureSurfaceFrame(SurfaceFrameCaptureCallback result) {
  bool start_readback = false;
  {
    const std::lock_guard<std::mutex> lock(surface_frame_capture_mutex_);
    pending_surface_frame_captures_.push_back(std::move(result));
    if (!surface_frame_readback_active_ && texture_bridge_->HasLatestFrame()) {
      surface_frame_readback_active_ = true;
      start_readback = true;
    }
  }
  if (start_readback) {
    StartSurfaceFrameReadback();
  }
}

void WebviewBridge::LoadUrl(const std::string &url) { webview_->LoadUrl(url); }

bool WebviewBridge::LoadRequest(const std::string &url,
                                const std::string &method,
                                const std::string &headers,
                                const std::vector<uint8_t> *body) {
  return webview_->LoadRequest(url, method, headers, body);
}

void WebviewBridge::LoadStringContent(const std::string &content) {
  webview_->LoadStringContent(content);
}

bool WebviewBridge::Reload() { return webview_->Reload(); }

bool WebviewBridge::Stop() { return webview_->Stop(); }

bool WebviewBridge::GoBack() { return webview_->GoBack(); }

bool WebviewBridge::GoForward() { return webview_->GoForward(); }

void WebviewBridge::Suspend() {
  texture_bridge_->Stop();
  webview_->Suspend();
}

void WebviewBridge::Resume() {
  webview_->Resume();
  texture_bridge_->Start();
}

void WebviewBridge::SetVirtualHostNameMapping(const std::string &host_name,
                                              const std::string &path,
                                              int64_t access_kind) {
  webview_->SetVirtualHostNameMapping(
      host_name, path, static_cast<WebviewHostResourceAccessKind>(access_kind));
}

bool WebviewBridge::ClearVirtualHostNameMapping(const std::string &host_name) {
  return webview_->ClearVirtualHostNameMapping(host_name);
}

void WebviewBridge::AddScriptToExecuteOnDocumentCreated(
    const std::string &script,
    std::function<void(bool success, const std::string &script_id)> result) {
  webview_->AddScriptToExecuteOnDocumentCreated(script, std::move(result));
}

void WebviewBridge::RemoveScriptToExecuteOnDocumentCreated(
    const std::string &script_id) {
  webview_->RemoveScriptToExecuteOnDocumentCreated(script_id);
}

void WebviewBridge::ExecuteScript(
    const std::string &script,
    std::function<void(bool success, const std::string &json_result)> result) {
  webview_->ExecuteScript(script, std::move(result));
}

bool WebviewBridge::PostWebMessage(const std::string &message) {
  return webview_->PostWebMessage(message);
}

bool WebviewBridge::SetUserAgent(const std::string *user_agent) {
  return webview_->SetUserAgent(user_agent);
}

std::optional<std::string> WebviewBridge::GetUserAgent() {
  return webview_->GetUserAgent();
}

bool WebviewBridge::SetJavaScriptEnabled(bool enabled) {
  return webview_->SetJavaScriptEnabled(enabled);
}

bool WebviewBridge::SetZoomControlEnabled(bool enabled) {
  return webview_->SetZoomControlEnabled(enabled);
}

bool WebviewBridge::SetBackgroundColor(int64_t color) {
  return webview_->SetBackgroundColor(static_cast<int32_t>(color));
}

bool WebviewBridge::SetZoomFactor(double zoom_factor) {
  return webview_->SetZoomFactor(zoom_factor);
}

bool WebviewBridge::OpenDevTools() { return webview_->OpenDevTools(); }

void WebviewBridge::SetJavaScriptDialogCallbacksEnabled(bool alert,
                                                        bool confirm,
                                                        bool prompt) {
  webview_->SetJavaScriptDialogCallbacksEnabled(alert, confirm, prompt);
}

void WebviewBridge::ClearCookies(
    std::function<void(bool success, bool had_cookies)> result) {
  webview_->ClearCookies(std::move(result));
}

bool WebviewBridge::SetCookie(const WebviewCookie &cookie) {
  return webview_->SetCookie(cookie);
}

void WebviewBridge::GetCookies(
    const std::string &url,
    std::function<void(bool success, std::vector<WebviewCookie> cookies)>
        result) {
  webview_->GetCookies(url, std::move(result));
}

bool WebviewBridge::DeleteCookie(const WebviewCookie &cookie) {
  return webview_->DeleteCookie(cookie);
}

bool WebviewBridge::DeleteCookiesWithNameAndUrl(const std::string &name,
                                                const std::string &url) {
  return webview_->DeleteCookiesWithNameAndUrl(name, url);
}

bool WebviewBridge::DeleteCookiesWithNameDomainAndPath(
    const std::string &name, const std::string &domain,
    const std::string &path) {
  return webview_->DeleteCookiesWithNameDomainAndPath(name, domain, path);
}

bool WebviewBridge::ClearCache() { return webview_->ClearCache(); }

void WebviewBridge::ClearLocalStorage(
    Webview::OperationCompletedCallback callback) {
  webview_->ClearLocalStorage(std::move(callback));
}

bool WebviewBridge::SetCacheDisabled(bool disabled) {
  return webview_->SetCacheDisabled(disabled);
}

void WebviewBridge::SetPopupWindowPolicy(int64_t policy) {
  switch (policy) {
  case 1:
    webview_->SetPopupWindowPolicy(WebviewPopupWindowPolicy::Deny);
    break;
  case 2:
    webview_->SetPopupWindowPolicy(WebviewPopupWindowPolicy::ShowInSameWindow);
    break;
  default:
    webview_->SetPopupWindowPolicy(WebviewPopupWindowPolicy::Allow);
    break;
  }
}

void WebviewBridge::SetFpsLimit(int64_t max_fps) {
  if (max_fps == 0) {
    texture_bridge_->SetFpsLimit(std::nullopt);
    return;
  }
  texture_bridge_->SetFpsLimit(
      std::make_optional<int>(static_cast<int>(max_fps)));
}

} // namespace webview_all_windows
