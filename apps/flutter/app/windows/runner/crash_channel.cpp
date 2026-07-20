#include "crash_channel.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>

#include <memory>
#include <string>
#include <variant>

namespace {

std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>>
    g_operit_crash_channel;

/// Converts UTF-8 crash details into a Windows wide string.
std::wstring Utf8ToWide(const std::string& value) {
  const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                                       value.data(), static_cast<int>(value.size()),
                                       nullptr, 0);
  std::wstring result(size, L'\0');
  MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, value.data(),
                      static_cast<int>(value.size()), result.data(), size);
  return result;
}

/// Reads the crash details string from a Flutter method call.
const std::string* CrashDetails(
    const flutter::MethodCall<flutter::EncodableValue>& method_call) {
  const auto* arguments = std::get_if<flutter::EncodableMap>(method_call.arguments());
  if (arguments == nullptr) {
    return nullptr;
  }
  const auto found = arguments->find(flutter::EncodableValue("details"));
  if (found == arguments->end()) {
    return nullptr;
  }
  return std::get_if<std::string>(&found->second);
}

}  // namespace

/// Presents a native Windows crash dialog with the supplied details.
void ShowOperitWindowsCrashScreen(const std::string& details) {
  const std::wstring message = Utf8ToWide(details);
  MessageBoxW(nullptr, message.c_str(), L"Operit2 has stopped",
              MB_OK | MB_ICONERROR | MB_TASKMODAL | MB_SETFOREGROUND);
}

/// Registers the Flutter crash presentation channel for this engine.
void RegisterOperitCrashChannel(flutter::FlutterEngine* engine) {
  g_operit_crash_channel = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      engine->messenger(), "operit/crash",
      &flutter::StandardMethodCodec::GetInstance());
  g_operit_crash_channel->SetMethodCallHandler(
      [](const flutter::MethodCall<flutter::EncodableValue>& method_call,
         std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
        if (method_call.method_name() != "present") {
          result->NotImplemented();
          return;
        }
        const std::string* details = CrashDetails(method_call);
        if (details == nullptr) {
          result->Error("INVALID_ARGS", "present requires crash details");
          return;
        }
        ShowOperitWindowsCrashScreen(*details);
        result->Success();
      });
}

/// Releases the crash channel while the Flutter engine messenger is valid.
void ShutdownOperitCrashChannel() {
  g_operit_crash_channel.reset();
}
