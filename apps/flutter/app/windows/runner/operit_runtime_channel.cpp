#include "operit_runtime_channel.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>

#include <memory>
#include <mutex>
#include <cstdint>
#include <string>
#include <thread>
#include <variant>
#include <vector>

namespace {

using BridgeHandle = void*;
using BridgeCreate = BridgeHandle (*)();
using BridgeCreateError = char* (*)();
using BridgeDestroy = void (*)(BridgeHandle);
using BridgeCall = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchSnapshot = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchStream = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgePollWatchStream = char* (*)(BridgeHandle, const char*);
using BridgeCloseWatchStream = char* (*)(BridgeHandle, const char*);
using BridgeHostDescriptor = char* (*)(BridgeHandle);
using BridgeCurrentPermissionRequest = char* (*)(BridgeHandle);
using BridgeHandlePermissionResult = char* (*)(BridgeHandle, const char*);
using BridgeNextBrowserAutomationRequest = char* (*)(BridgeHandle);
using BridgeHandleBrowserAutomationResult = char* (*)(BridgeHandle, const char*);
using BridgeNextWebVisitRequest = char* (*)(BridgeHandle);
using BridgeHandleWebVisitResult = char* (*)(BridgeHandle, const char*);
using BridgeStartTerminalPty = char* (*)(
    BridgeHandle, const char*, const char*, uint16_t, uint16_t);
using BridgeReadTerminalPty = char* (*)(BridgeHandle, const char*);
using BridgeWriteTerminalPty = char* (*)(BridgeHandle, const char*, const uint8_t*, size_t);
using BridgeResizeTerminalPty = char* (*)(BridgeHandle, const char*, uint16_t, uint16_t);
using BridgePollTerminalPtyExit = char* (*)(BridgeHandle, const char*);
using BridgeCloseTerminalPty = char* (*)(BridgeHandle, const char*);
using BridgeListTerminalSessions = char* (*)(BridgeHandle);
using BridgeGetTerminalSessionScreen = char* (*)(BridgeHandle, const char*);
using BridgeInputTerminalSession = char* (*)(BridgeHandle, const char*, const char*);
using BridgeFreeString = void (*)(char*);

class OperitRuntimeLibrary {
 public:
  OperitRuntimeLibrary() = default;
  ~OperitRuntimeLibrary() {
    if (handle_ != nullptr && destroy_ != nullptr) {
      destroy_(handle_);
      handle_ = nullptr;
    }
    if (library_ != nullptr) {
      FreeLibrary(library_);
      library_ = nullptr;
    }
  }

  bool EnsureReady(std::string* error) {
    if (handle_ != nullptr) {
      return true;
    }
    if (library_ == nullptr) {
      library_ = LoadLibraryW(L"operit_flutter_bridge.dll");
      if (library_ == nullptr) {
        AssignError(error, "operit_flutter_bridge.dll was not found");
        return false;
      }
      create_ = reinterpret_cast<BridgeCreate>(
          GetProcAddress(library_, "operit_flutter_bridge_create"));
      create_error_ = reinterpret_cast<BridgeCreateError>(
          GetProcAddress(library_, "operit_flutter_bridge_create_error"));
      destroy_ = reinterpret_cast<BridgeDestroy>(
          GetProcAddress(library_, "operit_flutter_bridge_destroy"));
      call_ = reinterpret_cast<BridgeCall>(
          GetProcAddress(library_, "operit_flutter_bridge_call"));
      watch_snapshot_ = reinterpret_cast<BridgeWatchSnapshot>(
          GetProcAddress(library_, "operit_flutter_bridge_watch_snapshot"));
      watch_stream_ = reinterpret_cast<BridgeWatchStream>(
          GetProcAddress(library_, "operit_flutter_bridge_watch_stream"));
      poll_watch_stream_ = reinterpret_cast<BridgePollWatchStream>(
          GetProcAddress(library_, "operit_flutter_bridge_poll_watch_stream"));
      close_watch_stream_ = reinterpret_cast<BridgeCloseWatchStream>(
          GetProcAddress(library_, "operit_flutter_bridge_close_watch_stream"));
      host_descriptor_ = reinterpret_cast<BridgeHostDescriptor>(
          GetProcAddress(library_, "operit_flutter_bridge_host_descriptor"));
      current_permission_request_ = reinterpret_cast<BridgeCurrentPermissionRequest>(
          GetProcAddress(library_, "operit_flutter_bridge_current_permission_request"));
      handle_permission_result_ = reinterpret_cast<BridgeHandlePermissionResult>(
          GetProcAddress(library_, "operit_flutter_bridge_handle_permission_result"));
      next_browser_automation_request_ = reinterpret_cast<BridgeNextBrowserAutomationRequest>(
          GetProcAddress(library_, "operit_flutter_bridge_next_browser_automation_request"));
      handle_browser_automation_result_ = reinterpret_cast<BridgeHandleBrowserAutomationResult>(
          GetProcAddress(library_, "operit_flutter_bridge_handle_browser_automation_result"));
      next_web_visit_request_ = reinterpret_cast<BridgeNextWebVisitRequest>(
          GetProcAddress(library_, "operit_flutter_bridge_next_web_visit_request"));
      handle_web_visit_result_ = reinterpret_cast<BridgeHandleWebVisitResult>(
          GetProcAddress(library_, "operit_flutter_bridge_handle_web_visit_result"));
      start_terminal_pty_ = reinterpret_cast<BridgeStartTerminalPty>(
          GetProcAddress(library_, "operit_flutter_bridge_start_terminal_pty"));
      read_terminal_pty_ = reinterpret_cast<BridgeReadTerminalPty>(
          GetProcAddress(library_, "operit_flutter_bridge_read_terminal_pty"));
      write_terminal_pty_ = reinterpret_cast<BridgeWriteTerminalPty>(
          GetProcAddress(library_, "operit_flutter_bridge_write_terminal_pty"));
      resize_terminal_pty_ = reinterpret_cast<BridgeResizeTerminalPty>(
          GetProcAddress(library_, "operit_flutter_bridge_resize_terminal_pty"));
      poll_terminal_pty_exit_ = reinterpret_cast<BridgePollTerminalPtyExit>(
          GetProcAddress(library_, "operit_flutter_bridge_poll_terminal_pty_exit"));
      close_terminal_pty_ = reinterpret_cast<BridgeCloseTerminalPty>(
          GetProcAddress(library_, "operit_flutter_bridge_close_terminal_pty"));
      list_terminal_sessions_ = reinterpret_cast<BridgeListTerminalSessions>(
          GetProcAddress(library_, "operit_flutter_bridge_list_terminal_sessions"));
      get_terminal_session_screen_ = reinterpret_cast<BridgeGetTerminalSessionScreen>(
          GetProcAddress(library_, "operit_flutter_bridge_get_terminal_session_screen"));
      input_terminal_session_ = reinterpret_cast<BridgeInputTerminalSession>(
          GetProcAddress(library_, "operit_flutter_bridge_input_terminal_session"));
      free_string_ = reinterpret_cast<BridgeFreeString>(
          GetProcAddress(library_, "operit_flutter_bridge_free_string"));
      if (create_ == nullptr || destroy_ == nullptr || call_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          poll_watch_stream_ == nullptr || close_watch_stream_ == nullptr ||
          host_descriptor_ == nullptr || current_permission_request_ == nullptr ||
          handle_permission_result_ == nullptr ||
          next_browser_automation_request_ == nullptr ||
          handle_browser_automation_result_ == nullptr ||
          next_web_visit_request_ == nullptr ||
          handle_web_visit_result_ == nullptr ||
          start_terminal_pty_ == nullptr || read_terminal_pty_ == nullptr ||
          write_terminal_pty_ == nullptr || resize_terminal_pty_ == nullptr ||
          poll_terminal_pty_exit_ == nullptr || close_terminal_pty_ == nullptr ||
          list_terminal_sessions_ == nullptr ||
          get_terminal_session_screen_ == nullptr ||
          input_terminal_session_ == nullptr ||
          free_string_ == nullptr) {
        AssignError(error, "operit flutter bridge exports are incomplete");
        return false;
      }
    }
    handle_ = create_();
    if (handle_ == nullptr) {
      AssignError(error, ReadCreateError());
      return false;
    }
    return true;
  }

  bool Call(const std::string& request, std::string* response,
            std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = call_(
        handle_, reinterpret_cast<const unsigned char*>(request.data()),
        request.size());
    return TakeBridgeString(raw_response, response, error);
  }

  bool WatchSnapshot(const std::string& request, std::string* response,
                     std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = watch_snapshot_(
        handle_, reinterpret_cast<const unsigned char*>(request.data()),
        request.size());
    return TakeBridgeString(raw_response, response, error);
  }

  bool WatchStream(const std::string& request, std::string* response,
                   std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = watch_stream_(
        handle_, reinterpret_cast<const unsigned char*>(request.data()),
        request.size());
    return TakeBridgeString(raw_response, response, error);
  }

  bool PollWatchStream(const std::string& subscription, std::string* response,
                       std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = poll_watch_stream_(handle_, subscription.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool CloseWatchStream(const std::string& subscription, std::string* response,
                        std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = close_watch_stream_(handle_, subscription.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool HostDescriptor(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = host_descriptor_(handle_);
    return TakeBridgeString(raw_response, response, error);
  }

  bool CurrentPermissionRequest(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = current_permission_request_(handle_);
    return TakeBridgeString(raw_response, response, error);
  }

  bool HandlePermissionResult(const std::string& permission_result,
                              std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = handle_permission_result_(handle_, permission_result.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool NextBrowserAutomationRequest(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = next_browser_automation_request_(handle_);
    return TakeBridgeString(raw_response, response, error);
  }

  bool HandleBrowserAutomationResult(const std::string& browser_result,
                                     std::string* response,
                                     std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response =
        handle_browser_automation_result_(handle_, browser_result.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool NextWebVisitRequest(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = next_web_visit_request_(handle_);
    return TakeBridgeString(raw_response, response, error);
  }

  bool HandleWebVisitResult(const std::string& web_visit_result,
                            std::string* response,
                            std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response =
        handle_web_visit_result_(handle_, web_visit_result.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool StartTerminalPty(const std::string& session_name,
                        const std::string& working_directory, int rows,
                        int columns, std::string* response,
                        std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = start_terminal_pty_(
        handle_, session_name.c_str(), working_directory.c_str(),
        static_cast<uint16_t>(rows), static_cast<uint16_t>(columns));
    return TakeBridgeString(raw_response, response, error);
  }

  bool ReadTerminalPty(const std::string& session_id, std::string* response,
                       std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = read_terminal_pty_(handle_, session_id.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool WriteTerminalPty(const std::string& session_id,
                        const std::vector<uint8_t>& data,
                        std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response =
        write_terminal_pty_(handle_, session_id.c_str(), data.data(), data.size());
    return TakeBridgeString(raw_response, response, error);
  }

  bool ResizeTerminalPty(const std::string& session_id, int rows, int columns,
                         std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = resize_terminal_pty_(
        handle_, session_id.c_str(), static_cast<uint16_t>(rows),
        static_cast<uint16_t>(columns));
    return TakeBridgeString(raw_response, response, error);
  }

  bool PollTerminalPtyExit(const std::string& session_id,
                           std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = poll_terminal_pty_exit_(handle_, session_id.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool CloseTerminalPty(const std::string& session_id,
                        std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = close_terminal_pty_(handle_, session_id.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool ListTerminalSessions(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = list_terminal_sessions_(handle_);
    return TakeBridgeString(raw_response, response, error);
  }

  bool GetTerminalSessionScreen(const std::string& session_id,
                                std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response =
        get_terminal_session_screen_(handle_, session_id.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool InputTerminalSession(const std::string& session_id,
                            const std::string& input,
                            std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response =
        input_terminal_session_(handle_, session_id.c_str(), input.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

 private:
  bool EnsureReadyThreadSafe(std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    return EnsureReady(error);
  }

  static void AssignError(std::string* target, const std::string& value) {
    if (target != nullptr) {
      *target = value;
    }
  }

  std::string ReadCreateError() {
    if (create_error_ == nullptr || free_string_ == nullptr) {
      return "failed to initialize operit flutter bridge";
    }
    char* raw_error = create_error_();
    std::string error;
    std::string ignored;
    if (TakeBridgeString(raw_error, &error, &ignored) && !error.empty()) {
      return error;
    }
    return "failed to initialize operit flutter bridge";
  }

  bool TakeBridgeString(char* value, std::string* output, std::string* error) {
    if (value == nullptr) {
      AssignError(error, "operit flutter bridge returned null");
      return false;
    }
    if (output != nullptr) {
      *output = value;
    }
    free_string_(value);
    return true;
  }

  HMODULE library_ = nullptr;
  BridgeHandle handle_ = nullptr;
  std::mutex mutex_;
  BridgeCreate create_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
  BridgeCall call_ = nullptr;
  BridgeWatchSnapshot watch_snapshot_ = nullptr;
  BridgeWatchStream watch_stream_ = nullptr;
  BridgePollWatchStream poll_watch_stream_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeHostDescriptor host_descriptor_ = nullptr;
  BridgeCurrentPermissionRequest current_permission_request_ = nullptr;
  BridgeHandlePermissionResult handle_permission_result_ = nullptr;
  BridgeNextBrowserAutomationRequest next_browser_automation_request_ = nullptr;
  BridgeHandleBrowserAutomationResult handle_browser_automation_result_ = nullptr;
  BridgeNextWebVisitRequest next_web_visit_request_ = nullptr;
  BridgeHandleWebVisitResult handle_web_visit_result_ = nullptr;
  BridgeStartTerminalPty start_terminal_pty_ = nullptr;
  BridgeReadTerminalPty read_terminal_pty_ = nullptr;
  BridgeWriteTerminalPty write_terminal_pty_ = nullptr;
  BridgeResizeTerminalPty resize_terminal_pty_ = nullptr;
  BridgePollTerminalPtyExit poll_terminal_pty_exit_ = nullptr;
  BridgeCloseTerminalPty close_terminal_pty_ = nullptr;
  BridgeListTerminalSessions list_terminal_sessions_ = nullptr;
  BridgeGetTerminalSessionScreen get_terminal_session_screen_ = nullptr;
  BridgeInputTerminalSession input_terminal_session_ = nullptr;
  BridgeFreeString free_string_ = nullptr;
};

std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>>
    g_operit_runtime_channel;
std::shared_ptr<OperitRuntimeLibrary> g_operit_runtime_library;

const std::string* StringArgument(
    const flutter::MethodCall<flutter::EncodableValue>& method_call) {
  const flutter::EncodableValue* arguments = method_call.arguments();
  if (arguments == nullptr) {
    return nullptr;
  }
  return std::get_if<std::string>(arguments);
}

const flutter::EncodableMap* MapArgument(
    const flutter::MethodCall<flutter::EncodableValue>& method_call) {
  const flutter::EncodableValue* arguments = method_call.arguments();
  if (arguments == nullptr) {
    return nullptr;
  }
  return std::get_if<flutter::EncodableMap>(arguments);
}

const flutter::EncodableValue* MapValue(const flutter::EncodableMap& map,
                                        const char* key) {
  auto iterator = map.find(flutter::EncodableValue(std::string(key)));
  if (iterator == map.end()) {
    return nullptr;
  }
  return &iterator->second;
}

const std::string* StringMapValue(const flutter::EncodableMap& map,
                                  const char* key) {
  const flutter::EncodableValue* value = MapValue(map, key);
  if (value == nullptr) {
    return nullptr;
  }
  return std::get_if<std::string>(value);
}

bool IntMapValue(const flutter::EncodableMap& map, const char* key, int* output) {
  const flutter::EncodableValue* value = MapValue(map, key);
  if (value == nullptr || output == nullptr) {
    return false;
  }
  if (const int32_t* int32_value = std::get_if<int32_t>(value)) {
    *output = *int32_value;
    return true;
  }
  if (const int64_t* int64_value = std::get_if<int64_t>(value)) {
    *output = static_cast<int>(*int64_value);
    return true;
  }
  return false;
}

const std::vector<uint8_t>* Uint8ListMapValue(const flutter::EncodableMap& map,
                                              const char* key) {
  const flutter::EncodableValue* value = MapValue(map, key);
  if (value == nullptr) {
    return nullptr;
  }
  return std::get_if<std::vector<uint8_t>>(value);
}

void RespondRuntimeCallAsync(
    std::string request,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  auto library = g_operit_runtime_library;
  std::thread([library, request = std::move(request),
               result = std::move(result)]() mutable {
    std::string response;
    std::string error;
    if (library->Call(request, &response, &error)) {
      result->Success(flutter::EncodableValue(response));
    } else {
      result->Error("RUNTIME_BRIDGE_ERROR", error);
    }
  }).detach();
}

}  // namespace

void RegisterOperitRuntimeChannel(flutter::FlutterEngine* engine) {
  g_operit_runtime_library = std::make_shared<OperitRuntimeLibrary>();
  g_operit_runtime_channel =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          engine->messenger(), "operit/runtime",
          &flutter::StandardMethodCodec::GetInstance());

  g_operit_runtime_channel->SetMethodCallHandler(
      [](const flutter::MethodCall<flutter::EncodableValue>& method_call,
         std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
             result) {
        std::string response;
        std::string error;
        if (method_call.method_name().compare("call") == 0) {
          const std::string* request = StringArgument(method_call);
          if (request == nullptr) {
            result->Error("INVALID_ARGS", "call expects a JSON string");
            return;
          }
          RespondRuntimeCallAsync(*request, std::move(result));
          return;
        }
        if (method_call.method_name().compare("watchSnapshot") == 0) {
          const std::string* request = StringArgument(method_call);
          if (request == nullptr) {
            result->Error("INVALID_ARGS", "watchSnapshot expects a JSON string");
            return;
          }
          if (g_operit_runtime_library->WatchSnapshot(*request, &response,
                                                      &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("watchStream") == 0) {
          const std::string* request = StringArgument(method_call);
          if (request == nullptr) {
            result->Error("INVALID_ARGS", "watchStream expects a JSON string");
            return;
          }
          if (g_operit_runtime_library->WatchStream(*request, &response,
                                                    &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("pollWatchStream") == 0) {
          const std::string* subscription = StringArgument(method_call);
          if (subscription == nullptr) {
            result->Error("INVALID_ARGS",
                          "pollWatchStream expects a subscription id");
            return;
          }
          if (g_operit_runtime_library->PollWatchStream(
                  *subscription, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("closeWatchStream") == 0) {
          const std::string* subscription = StringArgument(method_call);
          if (subscription == nullptr) {
            result->Error("INVALID_ARGS",
                          "closeWatchStream expects a subscription id");
            return;
          }
          if (g_operit_runtime_library->CloseWatchStream(
                  *subscription, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("hostDescriptor") == 0) {
          if (g_operit_runtime_library->HostDescriptor(&response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("currentPermissionRequest") == 0) {
          if (g_operit_runtime_library->CurrentPermissionRequest(&response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("handlePermissionResult") == 0) {
          const std::string* permission_result = StringArgument(method_call);
          if (permission_result == nullptr) {
            result->Error("INVALID_ARGS", "handlePermissionResult expects a result string");
            return;
          }
          if (g_operit_runtime_library->HandlePermissionResult(
                  *permission_result, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("nextBrowserAutomationRequest") == 0) {
          if (g_operit_runtime_library->NextBrowserAutomationRequest(
                  &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("handleBrowserAutomationResult") == 0) {
          const std::string* browser_result = StringArgument(method_call);
          if (browser_result == nullptr) {
            result->Error("INVALID_ARGS",
                          "handleBrowserAutomationResult expects a JSON string");
            return;
          }
          if (g_operit_runtime_library->HandleBrowserAutomationResult(
                  *browser_result, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("nextWebVisitRequest") == 0) {
          if (g_operit_runtime_library->NextWebVisitRequest(
                  &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("handleWebVisitResult") == 0) {
          const std::string* web_visit_result = StringArgument(method_call);
          if (web_visit_result == nullptr) {
            result->Error("INVALID_ARGS",
                          "handleWebVisitResult expects a JSON string");
            return;
          }
          if (g_operit_runtime_library->HandleWebVisitResult(
                  *web_visit_result, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("startTerminalPty") == 0) {
          const flutter::EncodableMap* args = MapArgument(method_call);
          const std::string* session_name =
              args == nullptr ? nullptr : StringMapValue(*args, "sessionName");
          const std::string* working_directory =
              args == nullptr ? nullptr : StringMapValue(*args, "workingDirectory");
          int rows = 0;
          int columns = 0;
          if (args == nullptr || session_name == nullptr ||
              working_directory == nullptr ||
              !IntMapValue(*args, "rows", &rows) ||
              !IntMapValue(*args, "columns", &columns)) {
            result->Error("INVALID_ARGS",
                          "startTerminalPty expects sessionName, workingDirectory, rows, columns");
            return;
          }
          if (g_operit_runtime_library->StartTerminalPty(
                  *session_name, *working_directory, rows, columns, &response,
                  &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("readTerminalPty") == 0) {
          const std::string* session_id = StringArgument(method_call);
          if (session_id == nullptr) {
            result->Error("INVALID_ARGS", "readTerminalPty expects a session id");
            return;
          }
          if (g_operit_runtime_library->ReadTerminalPty(
                  *session_id, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("writeTerminalPty") == 0) {
          const flutter::EncodableMap* args = MapArgument(method_call);
          const std::string* session_id =
              args == nullptr ? nullptr : StringMapValue(*args, "sessionId");
          const std::vector<uint8_t>* data =
              args == nullptr ? nullptr : Uint8ListMapValue(*args, "data");
          if (args == nullptr || session_id == nullptr || data == nullptr) {
            result->Error("INVALID_ARGS",
                          "writeTerminalPty expects sessionId and data");
            return;
          }
          if (g_operit_runtime_library->WriteTerminalPty(
                  *session_id, *data, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("resizeTerminalPty") == 0) {
          const flutter::EncodableMap* args = MapArgument(method_call);
          const std::string* session_id =
              args == nullptr ? nullptr : StringMapValue(*args, "sessionId");
          int rows = 0;
          int columns = 0;
          if (args == nullptr || session_id == nullptr ||
              !IntMapValue(*args, "rows", &rows) ||
              !IntMapValue(*args, "columns", &columns)) {
            result->Error("INVALID_ARGS",
                          "resizeTerminalPty expects sessionId, rows, columns");
            return;
          }
          if (g_operit_runtime_library->ResizeTerminalPty(
                  *session_id, rows, columns, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("pollTerminalPtyExit") == 0) {
          const std::string* session_id = StringArgument(method_call);
          if (session_id == nullptr) {
            result->Error("INVALID_ARGS",
                          "pollTerminalPtyExit expects a session id");
            return;
          }
          if (g_operit_runtime_library->PollTerminalPtyExit(
                  *session_id, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("closeTerminalPty") == 0) {
          const std::string* session_id = StringArgument(method_call);
          if (session_id == nullptr) {
            result->Error("INVALID_ARGS", "closeTerminalPty expects a session id");
            return;
          }
          if (g_operit_runtime_library->CloseTerminalPty(
                  *session_id, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("listTerminalSessions") == 0) {
          if (g_operit_runtime_library->ListTerminalSessions(&response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("getTerminalSessionScreen") == 0) {
          const std::string* session_id = StringArgument(method_call);
          if (session_id == nullptr) {
            result->Error("INVALID_ARGS",
                          "getTerminalSessionScreen expects a session id");
            return;
          }
          if (g_operit_runtime_library->GetTerminalSessionScreen(
                  *session_id, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("inputTerminalSession") == 0) {
          const flutter::EncodableMap* args = MapArgument(method_call);
          const std::string* session_id =
              args == nullptr ? nullptr : StringMapValue(*args, "sessionId");
          const std::string* input =
              args == nullptr ? nullptr : StringMapValue(*args, "input");
          if (args == nullptr || session_id == nullptr || input == nullptr) {
            result->Error("INVALID_ARGS",
                          "inputTerminalSession expects sessionId and input");
            return;
          }
          if (g_operit_runtime_library->InputTerminalSession(
                  *session_id, *input, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        result->NotImplemented();
      });
}
