#include "operit_runtime_channel.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>
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
using BridgeCreate = BridgeHandle (*)();
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
              const char*, const char*, const char*, const char*, const char*,
              const char*, const char*);
using BridgeDiscoverDevices =
    char* (*)(BridgeHandle, const char*);
using BridgeStopWebAccessServer = char* (*)(BridgeHandle);
using BridgeRemotePairStart =
    char* (*)(BridgeHandle, const char*, const char*, const char*);
using BridgeRemotePairFinish = char* (*)(BridgeHandle, const char*, const char*);
using BridgeFreeString = void (*)(char*);

std::vector<std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>>>
    g_operit_runtime_channels;
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
      poll_watch_streams_ = reinterpret_cast<BridgePollWatchStreams>(
          GetProcAddress(library_, "operit_flutter_bridge_poll_watch_streams"));
      close_watch_stream_ = reinterpret_cast<BridgeCloseWatchStream>(
          GetProcAddress(library_, "operit_flutter_bridge_close_watch_stream"));
      discover_devices_ = reinterpret_cast<BridgeDiscoverDevices>(
          GetProcAddress(library_, "operit_flutter_bridge_discover_devices"));
      start_web_access_server_ = reinterpret_cast<BridgeStartWebAccessServer>(
          GetProcAddress(library_, "operit_flutter_bridge_start_web_access_server"));
      stop_web_access_server_ = reinterpret_cast<BridgeStopWebAccessServer>(
          GetProcAddress(library_, "operit_flutter_bridge_stop_web_access_server"));
      remote_pair_start_ = reinterpret_cast<BridgeRemotePairStart>(
          GetProcAddress(library_, "operit_flutter_bridge_remote_pair_start"));
      remote_pair_finish_ = reinterpret_cast<BridgeRemotePairFinish>(
          GetProcAddress(library_, "operit_flutter_bridge_remote_pair_finish"));
      free_string_ = reinterpret_cast<BridgeFreeString>(
          GetProcAddress(library_, "operit_flutter_bridge_free_string"));
      if (create_ == nullptr ||
          destroy_ == nullptr || call_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          poll_watch_stream_ == nullptr || poll_watch_streams_ == nullptr ||
          close_watch_stream_ == nullptr ||
          start_web_access_server_ == nullptr || stop_web_access_server_ == nullptr ||
          remote_pair_start_ == nullptr || remote_pair_finish_ == nullptr ||
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
                            const std::string& device_id,
                            const std::string& accepted_sessions,
                            const std::string& accepted_session_store_path,
                            const std::string& pairing_code_path,
                            const std::string& device_info,
                            const std::string& enable_web_access,
                            const std::string& enable_discovery,
                            std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
      char* raw_response = start_web_access_server_(
          handle_, bind_address.c_str(), token.c_str(), shutdown_token.c_str(),
          web_root.c_str(), device_id.c_str(), accepted_sessions.c_str(),
          accepted_session_store_path.c_str(), pairing_code_path.c_str(),
          device_info.c_str(), enable_web_access.c_str(),
          enable_discovery.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool DiscoverDevices(const std::string& timeout_ms,
                       std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = discover_devices_(handle_, timeout_ms.c_str());
    return TakeBridgeString(raw_response, response, error);
  }

  bool StopWebAccessServer(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response = stop_web_access_server_(handle_);
    return TakeBridgeString(raw_response, response, error);
  }

  bool RemotePairStart(const std::string& base_url, const std::string& token_hash,
                       const std::string& client_device_info,
                       std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    char* raw_response =
        remote_pair_start_(handle_, base_url.c_str(), token_hash.c_str(),
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
  BridgeCreate create_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
  BridgeCall call_ = nullptr;
  BridgeWatchSnapshot watch_snapshot_ = nullptr;
  BridgeWatchStream watch_stream_ = nullptr;
  BridgePollWatchStream poll_watch_stream_ = nullptr;
  BridgePollWatchStreams poll_watch_streams_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeStartWebAccessServer start_web_access_server_ = nullptr;
  BridgeDiscoverDevices discover_devices_ = nullptr;
  BridgeStopWebAccessServer stop_web_access_server_ = nullptr;
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
  if (g_operit_runtime_window == nullptr) {
    g_operit_runtime_window = window;
    g_operit_runtime_platform_thread_id = ::GetCurrentThreadId();
  }
  if (!g_operit_runtime_library) {
    g_operit_runtime_library = std::make_shared<OperitRuntimeLibrary>();
  }
  auto channel =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          engine->messenger(), "operit/runtime",
          &flutter::StandardMethodCodec::GetInstance());
  auto runtime_library = g_operit_runtime_library;

  channel->SetMethodCallHandler(
      [runtime_library](const flutter::MethodCall<flutter::EncodableValue>& method_call,
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
          if (runtime_library->WatchSnapshot(*request, &response,
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
          if (runtime_library->WatchStream(*request, &response,
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
          if (runtime_library->PollWatchStream(
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
          if (runtime_library->PollWatchStreams(
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
          if (runtime_library->CloseWatchStream(
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
            const std::string* device_id =
                StringMapValue(method_call, "deviceId");
            const std::string* accepted_sessions =
                StringMapValue(method_call, "acceptedSessions");
          const std::string* accepted_session_store_path =
              StringMapValue(method_call, "acceptedSessionStorePath");
          const std::string* pairing_code_path =
              StringMapValue(method_call, "pairingCodePath");
          const std::string* device_info =
              StringMapValue(method_call, "deviceInfo");
          const std::string* enable_web_access =
              StringMapValue(method_call, "enableWebAccess");
          const std::string* enable_discovery =
              StringMapValue(method_call, "enableDiscovery");
          if (bind_address == nullptr || token == nullptr ||
                shutdown_token == nullptr || web_root == nullptr ||
                device_id == nullptr ||
                accepted_sessions == nullptr ||
                accepted_session_store_path == nullptr ||
                pairing_code_path == nullptr || device_info == nullptr ||
                enable_web_access == nullptr || enable_discovery == nullptr) {
              result->Error("INVALID_ARGS",
                           "startWebAccessServer expects bindAddress, token, shutdownToken, webRoot, deviceId, acceptedSessions, acceptedSessionStorePath, pairingCodePath, deviceInfo, enableWebAccess and enableDiscovery");
              return;
            }
            if (runtime_library->StartWebAccessServer(
                    *bind_address, *token, *shutdown_token, *web_root,
                    *device_id, *accepted_sessions, *accepted_session_store_path,
                    *pairing_code_path, *device_info, *enable_web_access,
                    *enable_discovery, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("discoverDevices") == 0) {
          const std::string* timeout_ms =
              StringMapValue(method_call, "timeoutMs");
          if (timeout_ms == nullptr) {
            result->Error("INVALID_ARGS",
                          "discoverDevices expects timeoutMs");
            return;
          }
          std::string timeout = *timeout_ms;
          auto library = runtime_library;
          std::thread([library, timeout = std::move(timeout),
                       result = std::move(result)]() mutable {
            std::string response;
            std::string error;
            const bool ok = library->DiscoverDevices(timeout, &response, &error);
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
          return;
        }
        if (method_call.method_name().compare("stopWebAccessServer") == 0) {
          if (runtime_library->StopWebAccessServer(&response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        if (method_call.method_name().compare("remotePairStart") == 0) {
          const std::string* base_url = StringMapValue(method_call, "baseUrl");
          const std::string* token_hash =
              StringMapValue(method_call, "tokenHash");
          const std::string* client_device_info =
              StringMapValue(method_call, "clientDeviceInfo");
          if (base_url == nullptr || token_hash == nullptr ||
              client_device_info == nullptr) {
            result->Error("INVALID_ARGS",
                          "remotePairStart expects baseUrl, tokenHash and clientDeviceInfo");
            return;
          }
          if (runtime_library->RemotePairStart(
                  *base_url, *token_hash, *client_device_info, &response, &error)) {
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
          if (runtime_library->RemotePairFinish(
                  *pairing_id, *pairing_code, &response, &error)) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
          return;
        }
        result->NotImplemented();
      });
  g_operit_runtime_channels.push_back(std::move(channel));
}

void ShutdownOperitRuntimeChannel() {
  g_operit_runtime_channels.clear();
  g_operit_runtime_library.reset();
  g_operit_runtime_window = nullptr;
  g_operit_runtime_platform_thread_id = 0;
}
