#include "operit_runtime_channel.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>

#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <mutex>
#include <cstdint>
#include <string>
#include <thread>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace {

using BridgeHandle = void*;
using BridgeRuntimeHostRequestCallback =
    char* (*)(const char*, const unsigned char*, size_t, void*);
using BridgeRuntimeHostRequestFree = void (*)(char*, void*);
using BridgeCreateWithRuntimeHostBridge = BridgeHandle (*)(
    BridgeRuntimeHostRequestCallback, BridgeRuntimeHostRequestFree, void*);
using BridgeCreateError = char* (*)();
using BridgeDestroy = void (*)(BridgeHandle);
using BridgeCall = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchSnapshot = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchStream = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgePollWatchStream = char* (*)(BridgeHandle, const char*);
using BridgePollWatchStreams = char* (*)(BridgeHandle, const char*);
using BridgeCloseWatchStream = char* (*)(BridgeHandle, const char*);
using BridgeStartWebAccessServer =
    char* (*)(BridgeHandle, const char*, const char*, const char*, const char*,
              const char*, const char*, const char*, const char*);
using BridgeStopWebAccessServer = char* (*)(BridgeHandle);
using BridgeHostDescriptor = char* (*)(BridgeHandle);
using BridgeCurrentPermissionRequest = char* (*)(BridgeHandle);
using BridgeHandlePermissionResult = char* (*)(BridgeHandle, const char*);
using BridgeRemotePairStart =
    char* (*)(BridgeHandle, const char*, const char*, const char*);
using BridgeRemotePairFinish = char* (*)(BridgeHandle, const char*, const char*);
using BridgeFreeString = void (*)(char*);

std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>>
    g_operit_runtime_channel;
HWND g_operit_runtime_window = nullptr;
DWORD g_operit_runtime_platform_thread_id = 0;

constexpr UINT kOperitRuntimePlatformTaskMessage = WM_APP + 0x520;

class OperitRuntimePlatformTask {
 public:
  virtual ~OperitRuntimePlatformTask() = default;
  virtual void Run() = 0;
};

template <typename Callback>
class OperitRuntimePlatformTaskImpl final : public OperitRuntimePlatformTask {
 public:
  explicit OperitRuntimePlatformTaskImpl(Callback callback)
      : callback_(std::move(callback)) {}

  void Run() override { callback_(); }

 private:
  Callback callback_;
};

template <typename Callback>
bool PostOperitRuntimePlatformTask(Callback&& callback) {
  if (g_operit_runtime_window == nullptr) {
    return false;
  }
  auto task = std::make_unique<
      OperitRuntimePlatformTaskImpl<std::decay_t<Callback>>>(
      std::forward<Callback>(callback));
  auto raw_task = task.release();
  if (::PostMessage(g_operit_runtime_window, kOperitRuntimePlatformTaskMessage,
                    reinterpret_cast<WPARAM>(raw_task), 0) == 0) {
    delete raw_task;
    return false;
  }
  return true;
}

std::string JsonString(const std::string& value) {
  std::string output = "\"";
  for (char ch : value) {
    switch (ch) {
      case '\\':
        output += "\\\\";
        break;
      case '"':
        output += "\\\"";
        break;
      case '\b':
        output += "\\b";
        break;
      case '\f':
        output += "\\f";
        break;
      case '\n':
        output += "\\n";
        break;
      case '\r':
        output += "\\r";
        break;
      case '\t':
        output += "\\t";
        break;
      default:
        if (static_cast<unsigned char>(ch) < 0x20) {
          char buffer[7];
          std::snprintf(buffer, sizeof(buffer), "\\u%04x",
                        static_cast<unsigned char>(ch));
          output += buffer;
        } else {
          output += ch;
        }
        break;
    }
  }
  output += "\"";
  return output;
}

char* CopyRuntimeHostBridgeResponse(const std::string& value) {
  char* copy = static_cast<char*>(std::malloc(value.size() + 1));
  if (copy == nullptr) {
    return nullptr;
  }
  std::memcpy(copy, value.c_str(), value.size() + 1);
  return copy;
}

void FreeRuntimeHostBridgeResponse(char* value, void* user_data) {
  (void)user_data;
  std::free(value);
}

std::string RuntimeHostBridgeSuccess(const std::string& value) {
  return std::string("{\"ok\":true,\"value\":") + JsonString(value) + "}";
}

std::string RuntimeHostBridgeError(const std::string& error) {
  return std::string("{\"ok\":false,\"error\":") + JsonString(error) + "}";
}

struct BlockingMethodResultState {
  std::mutex mutex;
  std::condition_variable changed;
  bool completed = false;
  bool ok = false;
  std::string value;
  std::string error;
};

class BlockingStringMethodResult
    : public flutter::MethodResult<flutter::EncodableValue> {
 public:
  explicit BlockingStringMethodResult(
      std::shared_ptr<BlockingMethodResultState> state)
      : state_(std::move(state)) {}

 protected:
  void SuccessInternal(const flutter::EncodableValue* result) override {
    std::lock_guard<std::mutex> lock(state_->mutex);
    if (result != nullptr) {
      if (const std::string* value = std::get_if<std::string>(result)) {
        state_->value = *value;
        state_->ok = true;
      } else {
        state_->error = "runtime host handler returned a non-string value";
      }
    } else {
      state_->value = "";
      state_->ok = true;
    }
    state_->completed = true;
    state_->changed.notify_all();
  }

  void ErrorInternal(const std::string& error_code,
                     const std::string& error_message,
                     const flutter::EncodableValue* error_details) override {
    (void)error_details;
    std::lock_guard<std::mutex> lock(state_->mutex);
    state_->error = error_code + ": " + error_message;
    state_->completed = true;
    state_->changed.notify_all();
  }

  void NotImplementedInternal() override {
    std::lock_guard<std::mutex> lock(state_->mutex);
    state_->error = "runtime host method is not implemented";
    state_->completed = true;
    state_->changed.notify_all();
  }

 private:
  std::shared_ptr<BlockingMethodResultState> state_;
};

void InvokeRuntimeHostMethodOnPlatformThread(
    std::string method_name,
    std::string payload,
    std::shared_ptr<BlockingMethodResultState> state) {
  g_operit_runtime_channel->InvokeMethod(
      method_name,
      std::make_unique<flutter::EncodableValue>(std::move(payload)),
      std::make_unique<BlockingStringMethodResult>(std::move(state)));
}

char* HandleRuntimeHostRequest(const char* method_name,
                               const unsigned char* payload,
                               size_t payload_length,
                               void* user_data) {
  (void)user_data;
  if (method_name == nullptr) {
    return CopyRuntimeHostBridgeResponse(
        RuntimeHostBridgeError("runtime host method pointer is null"));
  }
  if (payload == nullptr) {
    return CopyRuntimeHostBridgeResponse(
        RuntimeHostBridgeError("runtime host payload pointer is null"));
  }
  if (!g_operit_runtime_channel) {
    return CopyRuntimeHostBridgeResponse(
        RuntimeHostBridgeError("operit/runtime channel is not initialized"));
  }
  auto state = std::make_shared<BlockingMethodResultState>();
  if (::GetCurrentThreadId() == g_operit_runtime_platform_thread_id) {
    return CopyRuntimeHostBridgeResponse(RuntimeHostBridgeError(
        "runtime host request cannot block the platform thread"));
  }
  std::string method_name_string(method_name);
  std::string payload_string(reinterpret_cast<const char*>(payload),
                             payload_length);
  if (!PostOperitRuntimePlatformTask(
          [method_name_string = std::move(method_name_string),
           payload_string = std::move(payload_string), state]() mutable {
            InvokeRuntimeHostMethodOnPlatformThread(
                std::move(method_name_string), std::move(payload_string),
                std::move(state));
          })) {
    return CopyRuntimeHostBridgeResponse(RuntimeHostBridgeError(
        "operit/runtime platform task could not be posted"));
  }
  std::unique_lock<std::mutex> lock(state->mutex);
  state->changed.wait(lock, [&state]() { return state->completed; });
  return CopyRuntimeHostBridgeResponse(
      state->ok ? RuntimeHostBridgeSuccess(state->value)
                : RuntimeHostBridgeError(state->error));
}

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
      create_with_runtime_host_bridge_ =
          reinterpret_cast<BridgeCreateWithRuntimeHostBridge>(GetProcAddress(
              library_, "operit_flutter_bridge_create_with_runtime_host_bridge"));
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
      poll_watch_streams_ = reinterpret_cast<BridgePollWatchStreams>(
          GetProcAddress(library_, "operit_flutter_bridge_poll_watch_streams"));
      close_watch_stream_ = reinterpret_cast<BridgeCloseWatchStream>(
          GetProcAddress(library_, "operit_flutter_bridge_close_watch_stream"));
      start_web_access_server_ = reinterpret_cast<BridgeStartWebAccessServer>(
          GetProcAddress(library_, "operit_flutter_bridge_start_web_access_server"));
      stop_web_access_server_ = reinterpret_cast<BridgeStopWebAccessServer>(
          GetProcAddress(library_, "operit_flutter_bridge_stop_web_access_server"));
      host_descriptor_ = reinterpret_cast<BridgeHostDescriptor>(
          GetProcAddress(library_, "operit_flutter_bridge_host_descriptor"));
      current_permission_request_ = reinterpret_cast<BridgeCurrentPermissionRequest>(
          GetProcAddress(library_, "operit_flutter_bridge_current_permission_request"));
      handle_permission_result_ = reinterpret_cast<BridgeHandlePermissionResult>(
          GetProcAddress(library_, "operit_flutter_bridge_handle_permission_result"));
      remote_pair_start_ = reinterpret_cast<BridgeRemotePairStart>(
          GetProcAddress(library_, "operit_flutter_bridge_remote_pair_start"));
      remote_pair_finish_ = reinterpret_cast<BridgeRemotePairFinish>(
          GetProcAddress(library_, "operit_flutter_bridge_remote_pair_finish"));
      free_string_ = reinterpret_cast<BridgeFreeString>(
          GetProcAddress(library_, "operit_flutter_bridge_free_string"));
      if (create_with_runtime_host_bridge_ == nullptr ||
          destroy_ == nullptr || call_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          poll_watch_stream_ == nullptr || poll_watch_streams_ == nullptr ||
          close_watch_stream_ == nullptr ||
          start_web_access_server_ == nullptr || stop_web_access_server_ == nullptr ||
          host_descriptor_ == nullptr || current_permission_request_ == nullptr ||
          handle_permission_result_ == nullptr ||
          remote_pair_start_ == nullptr || remote_pair_finish_ == nullptr ||
          free_string_ == nullptr) {
        AssignError(error, "operit flutter bridge exports are incomplete");
        return false;
      }
    }
    handle_ = create_with_runtime_host_bridge_(
        HandleRuntimeHostRequest, FreeRuntimeHostBridgeResponse, nullptr);
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

  bool PollWatchStreams(const std::string& subscriptions, std::string* response,
                        std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = poll_watch_streams_(handle_, subscriptions.c_str());
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

  bool StartWebAccessServer(const std::string& bind_address,
                            const std::string& token,
                            const std::string& shutdown_token,
                            const std::string& web_root,
                            const std::string& accepted_sessions,
                            const std::string& accepted_session_store_path,
                            const std::string& pairing_code_path,
                            const std::string& device_info,
                            std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = start_web_access_server_(
        handle_, bind_address.c_str(), token.c_str(), shutdown_token.c_str(),
        web_root.c_str(), accepted_sessions.c_str(),
        accepted_session_store_path.c_str(), pairing_code_path.c_str(),
        device_info.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool StopWebAccessServer(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = stop_web_access_server_(handle_);
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

  bool RemotePairStart(const std::string& base_url, const std::string& token,
                       const std::string& client_device_info,
                       std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response =
        remote_pair_start_(handle_, base_url.c_str(), token.c_str(),
                           client_device_info.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool RemotePairFinish(const std::string& pairing_id,
                        const std::string& pairing_code,
                        std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = remote_pair_finish_(
        handle_, pairing_id.c_str(), pairing_code.c_str());
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
  BridgeCreateWithRuntimeHostBridge create_with_runtime_host_bridge_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
  BridgeCall call_ = nullptr;
  BridgeWatchSnapshot watch_snapshot_ = nullptr;
  BridgeWatchStream watch_stream_ = nullptr;
  BridgePollWatchStream poll_watch_stream_ = nullptr;
  BridgePollWatchStreams poll_watch_streams_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeStartWebAccessServer start_web_access_server_ = nullptr;
  BridgeStopWebAccessServer stop_web_access_server_ = nullptr;
  BridgeHostDescriptor host_descriptor_ = nullptr;
  BridgeCurrentPermissionRequest current_permission_request_ = nullptr;
  BridgeHandlePermissionResult handle_permission_result_ = nullptr;
  BridgeRemotePairStart remote_pair_start_ = nullptr;
  BridgeRemotePairFinish remote_pair_finish_ = nullptr;
  BridgeFreeString free_string_ = nullptr;
};

std::shared_ptr<OperitRuntimeLibrary> g_operit_runtime_library;

const std::string* StringArgument(
    const flutter::MethodCall<flutter::EncodableValue>& method_call) {
  const flutter::EncodableValue* arguments = method_call.arguments();
  if (arguments == nullptr) {
    return nullptr;
  }
  return std::get_if<std::string>(arguments);
}

const std::string* StringMapValue(
    const flutter::MethodCall<flutter::EncodableValue>& method_call,
    const char* key) {
  const flutter::EncodableValue* arguments = method_call.arguments();
  if (arguments == nullptr) {
    return nullptr;
  }
  const auto* map =
      std::get_if<flutter::EncodableMap>(arguments);
  if (map == nullptr) {
    return nullptr;
  }
  auto item = map->find(flutter::EncodableValue(std::string(key)));
  if (item == map->end()) {
    return nullptr;
  }
  return std::get_if<std::string>(&item->second);
}

void RespondRuntimeCallAsync(
    std::string request,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  auto library = g_operit_runtime_library;
  std::thread([library, request = std::move(request),
               result = std::move(result)]() mutable {
    std::string response;
    std::string error;
    const bool ok = library->Call(request, &response, &error);
    PostOperitRuntimePlatformTask(
        [result = std::move(result), ok, response = std::move(response),
         error = std::move(error)]() mutable {
          if (ok) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
        });
  }).detach();
}

}  // namespace

bool HandleOperitRuntimeChannelWindowMessage(UINT message,
                                             WPARAM wparam,
                                             LPARAM lparam,
                                             LRESULT* result) {
  (void)lparam;
  if (message != kOperitRuntimePlatformTaskMessage) {
    return false;
  }
  std::unique_ptr<OperitRuntimePlatformTask> task(
      reinterpret_cast<OperitRuntimePlatformTask*>(wparam));
  if (task) {
    task->Run();
  }
  if (result != nullptr) {
    *result = 0;
  }
  return true;
}

void RegisterOperitRuntimeChannel(flutter::FlutterEngine* engine, HWND window) {
  g_operit_runtime_window = window;
  g_operit_runtime_platform_thread_id = ::GetCurrentThreadId();
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
        if (method_call.method_name().compare("pollWatchStreams") == 0) {
          const std::string* subscriptions = StringArgument(method_call);
          if (subscriptions == nullptr) {
            result->Error("INVALID_ARGS",
                          "pollWatchStreams expects a JSON string array");
            return;
          }
          if (g_operit_runtime_library->PollWatchStreams(
                  *subscriptions, &response, &error)) {
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
        if (method_call.method_name().compare("startWebAccessServer") == 0) {
          const std::string* bind_address =
              StringMapValue(method_call, "bindAddress");
          const std::string* token = StringMapValue(method_call, "token");
          const std::string* shutdown_token =
              StringMapValue(method_call, "shutdownToken");
          const std::string* web_root = StringMapValue(method_call, "webRoot");
          const std::string* accepted_sessions =
              StringMapValue(method_call, "acceptedSessions");
          const std::string* accepted_session_store_path =
              StringMapValue(method_call, "acceptedSessionStorePath");
          const std::string* pairing_code_path =
              StringMapValue(method_call, "pairingCodePath");
          const std::string* device_info =
              StringMapValue(method_call, "deviceInfo");
          if (bind_address == nullptr || token == nullptr ||
              shutdown_token == nullptr || web_root == nullptr ||
              accepted_sessions == nullptr ||
              accepted_session_store_path == nullptr ||
              pairing_code_path == nullptr || device_info == nullptr) {
            result->Error("INVALID_ARGS",
                          "startWebAccessServer expects bindAddress, token, shutdownToken, webRoot, acceptedSessions, acceptedSessionStorePath, pairingCodePath and deviceInfo");
            return;
          }
          if (g_operit_runtime_library->StartWebAccessServer(
                  *bind_address, *token, *shutdown_token, *web_root,
                  *accepted_sessions, *accepted_session_store_path,
                  *pairing_code_path, *device_info,
                  &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("stopWebAccessServer") == 0) {
          if (g_operit_runtime_library->StopWebAccessServer(&response, &error)) {
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
        if (method_call.method_name().compare("remotePairStart") == 0) {
          const std::string* base_url = StringMapValue(method_call, "baseUrl");
          const std::string* token = StringMapValue(method_call, "token");
          const std::string* client_device_info =
              StringMapValue(method_call, "clientDeviceInfo");
          if (base_url == nullptr || token == nullptr ||
              client_device_info == nullptr) {
            result->Error("INVALID_ARGS",
                          "remotePairStart expects baseUrl, token and clientDeviceInfo");
            return;
          }
          if (g_operit_runtime_library->RemotePairStart(
                  *base_url, *token, *client_device_info, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("remotePairFinish") == 0) {
          const std::string* pairing_id =
              StringMapValue(method_call, "pairingId");
          const std::string* pairing_code =
              StringMapValue(method_call, "pairingCode");
          if (pairing_id == nullptr || pairing_code == nullptr) {
            result->Error("INVALID_ARGS",
                          "remotePairFinish expects pairingId and pairingCode");
            return;
          }
          if (g_operit_runtime_library->RemotePairFinish(
                  *pairing_id, *pairing_code, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        result->NotImplemented();
      });
}
