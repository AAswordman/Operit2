#include "operit_runtime_channel.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>
#include <shellapi.h>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <deque>
#include <filesystem>
#include <functional>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace {

using BridgeHandle = void*;
using BridgeCreate = BridgeHandle (*)();
using BridgeCreateWithStorageRoots = BridgeHandle (*)(const char*, const char*);
using BridgeCreateError = char* (*)();
using BridgeDestroy = void (*)(BridgeHandle);
struct OperitByteBuffer { unsigned char* ptr; size_t len; };
using BridgeNativeCall =
    OperitByteBuffer (*)(const void*, const unsigned char*, size_t);
using BridgePushOpen = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgePushItem = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgePushClose = OperitByteBuffer (*)(BridgeHandle, const char*);
using BridgeWatchSnapshot = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchStream = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeNextWatchChannelEvent = OperitByteBuffer (*)(BridgeHandle);
using BridgeCloseWatchStream = OperitByteBuffer (*)(BridgeHandle, const char*);
using BridgeFreeBytes = void (*)(OperitByteBuffer);
using BridgeStartWebAccessServer =
    char* (*)(BridgeHandle, const char*, const char*, const char*, const char*,
              const char*, const char*, const char*);
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
std::atomic_bool g_watch_channel_pump_running{false};

constexpr UINT kOperitRuntimePlatformTaskMessage = WM_APP + 0x520;

/// Builds a filesystem path from UTF-8 bytes under C++20 char8_t rules.
std::filesystem::path PathFromUtf8(const std::string& value) {
  std::u8string utf8;
  utf8.reserve(value.size());
  for (const unsigned char byte : value) {
    utf8.push_back(static_cast<char8_t>(byte));
  }
  return std::filesystem::path(utf8);
}

/// Converts a filesystem path into UTF-8 bytes under C++20 char8_t rules.
std::string PathToUtf8(const std::filesystem::path& value) {
  const std::u8string utf8 = value.u8string();
  std::string result;
  result.reserve(utf8.size());
  for (const char8_t byte : utf8) {
    result.push_back(static_cast<char>(byte));
  }
  return result;
}

/// Normalizes one caller-supplied Windows storage root.
bool NormalizeWindowsStorageRoot(const std::string& requested,
                                 const char* label,
                                 std::string* storage_root,
                                 std::string* error) {
  if (storage_root == nullptr || label == nullptr) {
    if (error != nullptr) {
      *error = "storage root output and label are required";
    }
    return false;
  }
  if (requested.empty()) {
    if (error != nullptr) {
      *error = std::string(label) + " is required";
    }
    return false;
  }
  const std::filesystem::path path = PathFromUtf8(requested).lexically_normal();
  if (!path.is_absolute()) {
    if (error != nullptr) {
      *error = std::string(label) + " must be an absolute path";
    }
    return false;
  }
  *storage_root = PathToUtf8(path);
  return true;
}

/// Resolves the default Windows runtime and workspace roots.
bool ResolveWindowsDefaultStorageRoots(std::string* runtime_root,
                                       std::string* workspace_root,
                                       std::string* error) {
  if (runtime_root == nullptr || workspace_root == nullptr) {
    if (error != nullptr) {
      *error = "runtime and workspace root outputs are required";
    }
    return false;
  }
  const DWORD required =
      ::GetEnvironmentVariableW(L"APPDATA", nullptr, 0);
  if (required == 0) {
    if (error != nullptr) {
      *error = "APPDATA is required for Operit2 runtime storage";
    }
    return false;
  }
  std::wstring app_data(required, L'\0');
  const DWORD written =
      ::GetEnvironmentVariableW(L"APPDATA", app_data.data(), required);
  if (written == 0 || written >= required) {
    if (error != nullptr) {
      *error = "Unable to read APPDATA for Operit2 runtime storage";
    }
    return false;
  }
  app_data.resize(written);
  const std::filesystem::path base =
      std::filesystem::path(app_data) / L"Operit2";
  *runtime_root = PathToUtf8(base / L"runtime");
  *workspace_root = PathToUtf8(base / L"workspaces");
  return true;
}

/// Builds Flutter storage path values for resolved Windows roots.
flutter::EncodableValue WindowsStoragePaths(const std::string& runtime_root,
                                            const std::string& workspace_root) {
  flutter::EncodableMap paths;
  paths[flutter::EncodableValue("runtimeRoot")] =
      flutter::EncodableValue(runtime_root);
  paths[flutter::EncodableValue("workspaceRoot")] =
      flutter::EncodableValue(workspace_root);
  return flutter::EncodableValue(paths);
}

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

/// Owns move-only tasks executed by the persistent runtime worker threads.
class OperitRuntimeWorkerTask {
 public:
  virtual ~OperitRuntimeWorkerTask() = default;
  virtual void Run() = 0;
};

/// Stores one move-only callable for the runtime worker queue.
template <typename Callback>
class OperitRuntimeWorkerTaskImpl final : public OperitRuntimeWorkerTask {
 public:
  explicit OperitRuntimeWorkerTaskImpl(Callback callback)
      : callback_(std::move(callback)) {}

  void Run() override { callback_(); }

 private:
  Callback callback_;
};

/// Executes runtime bridge work on a fixed set of reusable native threads.
class OperitRuntimeWorkerQueue {
 public:
  /// Starts the requested number of reusable worker threads.
  explicit OperitRuntimeWorkerQueue(size_t worker_count) {
    workers_.reserve(worker_count);
    for (size_t index = 0; index < worker_count; ++index) {
      workers_.emplace_back([this]() { RunWorker(); });
    }
  }

  /// Stops the queue after every already-submitted task completes.
  ~OperitRuntimeWorkerQueue() { Shutdown(); }

  /// Adds one callable to the runtime worker queue.
  template <typename Callback>
  bool Post(Callback&& callback) {
    auto task = std::make_unique<
        OperitRuntimeWorkerTaskImpl<std::decay_t<Callback>>>(
        std::forward<Callback>(callback));
    {
      std::lock_guard<std::mutex> lock(mutex_);
      if (stopping_) {
        return false;
      }
      tasks_.push_back(std::move(task));
    }
    condition_.notify_one();
    return true;
  }

  /// Waits for workers to drain submitted work and terminate.
  void Shutdown() {
    {
      std::lock_guard<std::mutex> lock(mutex_);
      if (stopping_) {
        return;
      }
      stopping_ = true;
    }
    condition_.notify_all();
    for (auto& worker : workers_) {
      if (worker.joinable()) {
        worker.join();
      }
    }
    workers_.clear();
  }

 private:
  /// Runs the task loop for one persistent runtime worker.
  void RunWorker() {
    while (true) {
      std::unique_ptr<OperitRuntimeWorkerTask> task;
      {
        std::unique_lock<std::mutex> lock(mutex_);
        condition_.wait(lock, [this]() { return stopping_ || !tasks_.empty(); });
        if (stopping_ && tasks_.empty()) {
          return;
        }
        task = std::move(tasks_.front());
        tasks_.pop_front();
      }
      task->Run();
    }
  }

  std::mutex mutex_;
  std::condition_variable condition_;
  std::deque<std::unique_ptr<OperitRuntimeWorkerTask>> tasks_;
  std::vector<std::thread> workers_;
  bool stopping_ = false;
};

std::mutex g_operit_runtime_platform_tasks_mutex;
std::deque<std::unique_ptr<OperitRuntimePlatformTask>>
    g_operit_runtime_platform_tasks;
bool g_operit_runtime_platform_task_message_pending = false;
std::unique_ptr<OperitRuntimeWorkerQueue> g_operit_runtime_workers;

/// Queues a task for the Windows platform thread and coalesces wake-up messages.
template <typename Callback>
bool PostOperitRuntimePlatformTask(Callback&& callback) {
  auto task = std::make_unique<
      OperitRuntimePlatformTaskImpl<std::decay_t<Callback>>>(
      std::forward<Callback>(callback));
  std::lock_guard<std::mutex> lock(g_operit_runtime_platform_tasks_mutex);
  if (g_operit_runtime_window == nullptr) {
    return false;
  }
  g_operit_runtime_platform_tasks.push_back(std::move(task));
  if (g_operit_runtime_platform_task_message_pending) {
    return true;
  }
  if (::PostMessage(g_operit_runtime_window, kOperitRuntimePlatformTaskMessage,
                    0, 0) == 0) {
    g_operit_runtime_platform_tasks.pop_back();
    return false;
  }
  g_operit_runtime_platform_task_message_pending = true;
  return true;
}

/// Drops platform tasks that can no longer run during process shutdown.
void ClearOperitRuntimePlatformTasks() {
  std::lock_guard<std::mutex> lock(g_operit_runtime_platform_tasks_mutex);
  g_operit_runtime_platform_tasks.clear();
  g_operit_runtime_platform_task_message_pending = false;
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
      create_with_storage_roots_ =
          reinterpret_cast<BridgeCreateWithStorageRoots>(
              GetProcAddress(
                  library_,
                  "operit_flutter_bridge_create_with_storage_roots"));
      create_error_ = reinterpret_cast<BridgeCreateError>(
          GetProcAddress(library_, "operit_flutter_bridge_create_error"));
      destroy_ = reinterpret_cast<BridgeDestroy>(
          GetProcAddress(library_, "operit_flutter_bridge_destroy"));
      native_call_ = reinterpret_cast<BridgeNativeCall>(
          GetProcAddress(library_, "operit_flutter_bridge_native_call"));
      push_open_ = reinterpret_cast<BridgePushOpen>(
          GetProcAddress(library_, "operit_flutter_bridge_push_open"));
      push_item_ = reinterpret_cast<BridgePushItem>(
          GetProcAddress(library_, "operit_flutter_bridge_push_item"));
      push_close_ = reinterpret_cast<BridgePushClose>(
          GetProcAddress(library_, "operit_flutter_bridge_push_close"));
      watch_snapshot_ = reinterpret_cast<BridgeWatchSnapshot>(
          GetProcAddress(library_, "operit_flutter_bridge_watch_snapshot"));
      watch_stream_ = reinterpret_cast<BridgeWatchStream>(
          GetProcAddress(library_, "operit_flutter_bridge_watch_stream"));
      next_watch_channel_event_ =
          reinterpret_cast<BridgeNextWatchChannelEvent>(
              GetProcAddress(library_,
                             "operit_flutter_bridge_next_watch_channel_event"));
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
      free_bytes_ = reinterpret_cast<BridgeFreeBytes>(
          GetProcAddress(library_, "operit_flutter_bridge_free_bytes"));
      if (create_ == nullptr || create_with_storage_roots_ == nullptr ||
          destroy_ == nullptr || native_call_ == nullptr || push_open_ == nullptr ||
          push_item_ == nullptr || push_close_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          next_watch_channel_event_ == nullptr ||
          close_watch_stream_ == nullptr ||
          start_web_access_server_ == nullptr || stop_web_access_server_ == nullptr ||
          remote_pair_start_ == nullptr || remote_pair_finish_ == nullptr ||
          free_string_ == nullptr || free_bytes_ == nullptr) {
        AssignError(error, "operit flutter bridge exports are incomplete");
        return false;
      }
    }
    if (configured_runtime_root_.empty() || configured_workspace_root_.empty()) {
      AssignError(error, "Runtime and workspace roots must be configured before runtime creation");
      return false;
    }
    handle_ = create_with_storage_roots_(
        configured_runtime_root_.c_str(),
        configured_workspace_root_.c_str());
    if (handle_ == nullptr) {
      AssignError(error, ReadCreateError());
      return false;
    }
    return true;
  }

  bool Call(const std::vector<uint8_t>& request, std::vector<uint8_t>* response,
            std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return TakeBridgeBytes(
        native_call_(handle_, request.data(), request.size()), response, error);
  }

  /// Opens one local Link push stream.
  bool PushOpen(const std::vector<uint8_t>& request,
                std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) return false;
    return TakeBridgeBytes(push_open_(handle_, request.data(), request.size()), response, error);
  }

  /// Dispatches one local Link push item.
  bool PushItem(const std::vector<uint8_t>& item,
                std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) return false;
    return TakeBridgeBytes(push_item_(handle_, item.data(), item.size()), response, error);
  }

  /// Closes one local Link push stream.
  bool PushClose(const std::string& push_id,
                 std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) return false;
    return TakeBridgeBytes(push_close_(handle_, push_id.c_str()), response, error);
  }

  bool WatchSnapshot(const std::vector<uint8_t>& request, std::vector<uint8_t>* response,
                     std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return TakeBridgeBytes(watch_snapshot_(handle_, request.data(), request.size()), response, error);
  }

  bool WatchStream(const std::vector<uint8_t>& request, std::vector<uint8_t>* response,
                   std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return TakeBridgeBytes(watch_stream_(handle_, request.data(), request.size()), response, error);
  }

  bool NextWatchChannelEvent(std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return TakeBridgeBytes(next_watch_channel_event_(handle_), response, error);
  }

  bool CloseWatchStream(const std::string& subscription, std::vector<uint8_t>* response,
                        std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return TakeBridgeBytes(close_watch_stream_(handle_, subscription.c_str()), response, error);
  }

  bool StartWebAccessServer(const std::string& bind_address,
                            const std::string& token,
                            const std::string& shutdown_token,
                            const std::string& web_root,
                            const std::string& device_info,
                            const std::string& enable_web_access,
                            const std::string& enable_discovery,
                            std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
      char* raw_response = start_web_access_server_(
          handle_, bind_address.c_str(), token.c_str(), shutdown_token.c_str(),
          web_root.c_str(), device_info.c_str(), enable_web_access.c_str(),
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

  /// Sets the runtime and workspace roots used when the runtime handle is created.
  bool SetStorageRoots(const std::string& runtime_root,
                       const std::string& workspace_root,
                       std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    std::string resolved_runtime_root;
    std::string resolved_workspace_root;
    if (!NormalizeWindowsStorageRoot(
            runtime_root, "runtimeRoot", &resolved_runtime_root, error)) {
      return false;
    }
    if (!NormalizeWindowsStorageRoot(
            workspace_root, "workspaceRoot", &resolved_workspace_root, error)) {
      return false;
    }
    if (handle_ != nullptr) {
      if (configured_runtime_root_ == resolved_runtime_root &&
          configured_workspace_root_ == resolved_workspace_root) {
        return true;
      }
      AssignError(
          error,
          "Runtime and workspace roots cannot change after runtime creation");
      return false;
    }
    configured_runtime_root_ = std::move(resolved_runtime_root);
    configured_workspace_root_ = std::move(resolved_workspace_root);
    return true;
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

  /// Copies one owned Rust Link buffer and releases its native allocation.
  bool TakeBridgeBytes(OperitByteBuffer value, std::vector<uint8_t>* output,
                       std::string* error) {
    if (value.ptr == nullptr) {
      AssignError(error, "operit flutter bridge returned an empty byte buffer");
      return false;
    }
    output->assign(value.ptr, value.ptr + value.len);
    free_bytes_(value);
    return true;
  }

  HMODULE library_ = nullptr;
  BridgeHandle handle_ = nullptr;
  std::string configured_runtime_root_;
  std::string configured_workspace_root_;
  std::mutex mutex_;
  BridgeCreate create_ = nullptr;
  BridgeCreateWithStorageRoots create_with_storage_roots_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
  BridgeNativeCall native_call_ = nullptr;
  BridgePushOpen push_open_ = nullptr;
  BridgePushItem push_item_ = nullptr;
  BridgePushClose push_close_ = nullptr;
  BridgeWatchSnapshot watch_snapshot_ = nullptr;
  BridgeWatchStream watch_stream_ = nullptr;
  BridgeNextWatchChannelEvent next_watch_channel_event_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeStartWebAccessServer start_web_access_server_ = nullptr;
  BridgeDiscoverDevices discover_devices_ = nullptr;
  BridgeStopWebAccessServer stop_web_access_server_ = nullptr;
  BridgeRemotePairStart remote_pair_start_ = nullptr;
  BridgeRemotePairFinish remote_pair_finish_ = nullptr;
  BridgeFreeString free_string_ = nullptr;
  BridgeFreeBytes free_bytes_ = nullptr;
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

// Reads an integer value from a Flutter method argument map.
bool IntegerMapValue(
    const flutter::MethodCall<flutter::EncodableValue>& method_call,
    const char* key,
    int64_t* value) {
  const flutter::EncodableValue* arguments = method_call.arguments();
  if (arguments == nullptr || value == nullptr) {
    return false;
  }
  const auto* map =
      std::get_if<flutter::EncodableMap>(arguments);
  if (map == nullptr) {
    return false;
  }
  auto item = map->find(flutter::EncodableValue(std::string(key)));
  if (item == map->end()) {
    return false;
  }
  const auto* int32_value = std::get_if<int32_t>(&item->second);
  if (int32_value != nullptr) {
    *value = *int32_value;
    return true;
  }
  const auto* int64_value = std::get_if<int64_t>(&item->second);
  if (int64_value != nullptr) {
    *value = *int64_value;
    return true;
  }
  return false;
}

bool IsCurrentProcessElevated() {
  HANDLE token = nullptr;
  if (::OpenProcessToken(::GetCurrentProcess(), TOKEN_QUERY, &token) == 0) {
    return false;
  }
  TOKEN_ELEVATION elevation{};
  DWORD size = 0;
  const BOOL ok = ::GetTokenInformation(token, TokenElevation, &elevation,
                                        sizeof(elevation), &size);
  ::CloseHandle(token);
  return ok != 0 && elevation.TokenIsElevated != 0;
}

flutter::EncodableValue WindowsAdminRequirementSnapshot() {
  flutter::EncodableMap item;
  item[flutter::EncodableValue("id")] =
      flutter::EncodableValue("windows.admin");
  item[flutter::EncodableValue("status")] = flutter::EncodableValue(
      IsCurrentProcessElevated() ? "Satisfied" : "Missing");

  flutter::EncodableMap result;
  result[flutter::EncodableValue("windows.admin")] =
      flutter::EncodableValue(item);
  return flutter::EncodableValue(result);
}

void RequestWindowsAdminAuthorization(
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  wchar_t exe_path[MAX_PATH];
  const DWORD path_length =
      ::GetModuleFileNameW(nullptr, exe_path, static_cast<DWORD>(MAX_PATH));
  if (path_length == 0 || path_length >= MAX_PATH) {
    result->Error("HOST_AUTHORIZATION_ERROR",
                  "Unable to read current executable path");
    return;
  }

  HINSTANCE instance =
      ::ShellExecuteW(nullptr, L"runas", exe_path, nullptr, nullptr,
                      SW_SHOWNORMAL);
  if (reinterpret_cast<INT_PTR>(instance) <= 32) {
    result->Error("HOST_AUTHORIZATION_DENIED",
                  "Administrator launch was not approved");
    return;
  }
  result->Success();
}

void DispatchWatchChannelEvent(std::vector<uint8_t> frame) {
  PostOperitRuntimePlatformTask([frame = std::move(frame)]() {
    for (const auto& channel : g_operit_runtime_channels) {
      channel->InvokeMethod(
          "watchChannelEvent",
          std::make_unique<flutter::EncodableValue>(frame));
    }
  });
}

void EnsureWatchChannelPump(std::shared_ptr<OperitRuntimeLibrary> library) {
  bool expected = false;
  if (!g_watch_channel_pump_running.compare_exchange_strong(expected, true)) {
    return;
  }
  std::thread([library = std::move(library)]() {
    while (g_watch_channel_pump_running.load()) {
      std::vector<uint8_t> frame;
      std::string error;
      if (!library->NextWatchChannelEvent(&frame, &error)) {
        break;
      }
      DispatchWatchChannelEvent(std::move(frame));
    }
    g_watch_channel_pump_running.store(false);
  }).detach();
}

/// Runs one Rust bridge operation off the Windows platform thread.
template <typename Operation>
void RespondRuntimeStringAsync(
    Operation operation,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  auto* workers = g_operit_runtime_workers.get();
  if (workers == nullptr) {
    result->Error("RUNTIME_WORKER_QUEUE_CLOSED",
                  "runtime worker queue is not available");
    return;
  }
  auto result_holder = std::make_shared<
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>>(
      std::move(result));
  const bool submitted = workers->Post(
      [operation = std::move(operation), result_holder]() mutable {
    std::string response;
    std::string error;
    const bool ok = operation(&response, &error);
    auto platform_result = std::move(*result_holder);
    PostOperitRuntimePlatformTask(
        [result = std::move(platform_result), ok, response = std::move(response),
         error = std::move(error)]() mutable {
          if (ok) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
        });
  });
  if (!submitted) {
    auto platform_result = std::move(*result_holder);
    platform_result->Error("RUNTIME_WORKER_QUEUE_CLOSED",
                           "runtime worker queue is not accepting work");
  }
}

/// Runs one binary Link bridge operation off the Windows platform thread.
template <typename Operation>
void RespondRuntimeBytesAsync(
    Operation operation,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  auto* workers = g_operit_runtime_workers.get();
  if (workers == nullptr) {
    result->Error("RUNTIME_WORKER_QUEUE_CLOSED",
                  "runtime worker queue is not available");
    return;
  }
  auto result_holder = std::make_shared<
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>>(
      std::move(result));
  const bool submitted = workers->Post(
      [operation = std::move(operation), result_holder]() mutable {
    std::vector<uint8_t> response;
    std::string error;
    const bool ok = operation(&response, &error);
    auto platform_result = std::move(*result_holder);
    PostOperitRuntimePlatformTask(
        [result = std::move(platform_result), ok, response = std::move(response), error = std::move(error)]() mutable {
          if (ok) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
        });
  });
  if (!submitted) {
    auto platform_result = std::move(*result_holder);
    platform_result->Error("RUNTIME_WORKER_QUEUE_CLOSED",
                           "runtime worker queue is not accepting work");
  }
}

/// Runs a core proxy call off the Windows platform thread.
void RespondRuntimeCallAsync(
    std::vector<uint8_t> request,
    std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result) {
  auto* workers = g_operit_runtime_workers.get();
  if (workers == nullptr) {
    result->Error("RUNTIME_WORKER_QUEUE_CLOSED",
                  "runtime worker queue is not available");
    return;
  }
  auto library = g_operit_runtime_library;
  auto result_holder = std::make_shared<
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>>(
      std::move(result));
  const bool submitted = workers->Post(
      [library, request = std::move(request), result_holder]() mutable {
    std::vector<uint8_t> response;
    std::string error;
    const bool ok = library->Call(request, &response, &error);
    auto platform_result = std::move(*result_holder);
    PostOperitRuntimePlatformTask(
        [result = std::move(platform_result), ok, response = std::move(response),
         error = std::move(error)]() mutable {
          if (ok) {
            result->Success(flutter::EncodableValue(response));
          } else {
            result->Error("RUNTIME_BRIDGE_ERROR", error);
          }
        });
  });
  if (!submitted) {
    auto platform_result = std::move(*result_holder);
    platform_result->Error("RUNTIME_WORKER_QUEUE_CLOSED",
                           "runtime worker queue is not accepting work");
  }
}

}  // namespace

bool HandleOperitRuntimeChannelWindowMessage(UINT message,
                                             WPARAM wparam,
                                             LPARAM lparam,
                                             LRESULT* result) {
  if (message != kOperitRuntimePlatformTaskMessage) {
    return false;
  }
  std::deque<std::unique_ptr<OperitRuntimePlatformTask>> tasks;
  {
    std::lock_guard<std::mutex> lock(g_operit_runtime_platform_tasks_mutex);
    g_operit_runtime_platform_task_message_pending = false;
    tasks.swap(g_operit_runtime_platform_tasks);
  }
  for (const auto& task : tasks) {
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
  if (!g_operit_runtime_workers) {
    g_operit_runtime_workers = std::make_unique<OperitRuntimeWorkerQueue>(4);
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
        std::string error;
        if (method_call.method_name().compare(
                "localRuntimeStorageDefaults") == 0) {
          std::string runtime_root;
          std::string workspace_root;
          if (!ResolveWindowsDefaultStorageRoots(
                  &runtime_root, &workspace_root, &error)) {
            result->Error("RUNTIME_STORAGE_DEFAULTS_ERROR", error);
            return;
          }
          result->Success(WindowsStoragePaths(runtime_root, workspace_root));
          return;
        }
        if (method_call.method_name().compare(
                "localRuntimeStoragePaths") == 0) {
          const std::string* requested_runtime_root =
              StringMapValue(method_call, "runtimeRoot");
          const std::string* requested_workspace_root =
              StringMapValue(method_call, "workspaceRoot");
          if (requested_runtime_root == nullptr ||
              requested_workspace_root == nullptr) {
            result->Error(
                "INVALID_ARGS",
                "localRuntimeStoragePaths expects runtimeRoot and workspaceRoot");
            return;
          }
          std::string runtime_root;
          std::string workspace_root;
          if (!NormalizeWindowsStorageRoot(
                  *requested_runtime_root, "runtimeRoot", &runtime_root, &error) ||
              !NormalizeWindowsStorageRoot(
                  *requested_workspace_root,
                  "workspaceRoot",
                  &workspace_root,
                  &error)) {
            result->Error("RUNTIME_STORAGE_PATHS_ERROR", error);
            return;
          }
          result->Success(WindowsStoragePaths(runtime_root, workspace_root));
          return;
        }
        if (method_call.method_name().compare(
                "setLocalRuntimeStorage") == 0) {
          const std::string* runtime_root =
              StringMapValue(method_call, "runtimeRoot");
          const std::string* workspace_root =
              StringMapValue(method_call, "workspaceRoot");
          if (runtime_root == nullptr || workspace_root == nullptr) {
            result->Error(
                "INVALID_ARGS",
                "setLocalRuntimeStorage expects runtimeRoot and workspaceRoot");
            return;
          }
          if (!runtime_library->SetStorageRoots(
                  *runtime_root, *workspace_root, &error)) {
            result->Error("RUNTIME_STORAGE_SET_ERROR", error);
            return;
          }
          result->Success();
          return;
        }
        if (method_call.method_name().compare("call") == 0) {
          const std::vector<uint8_t>* request =
              std::get_if<std::vector<uint8_t>>(method_call.arguments());
          if (request == nullptr) {
            result->Error("INVALID_ARGS", "call expects MessagePack bytes");
            return;
          }
          RespondRuntimeCallAsync(*request, std::move(result));
          return;
        }
        if (method_call.method_name().compare("pushOpen") == 0 ||
            method_call.method_name().compare("pushItem") == 0) {
          const std::vector<uint8_t>* request =
              std::get_if<std::vector<uint8_t>>(method_call.arguments());
          if (request == nullptr) {
            result->Error("INVALID_ARGS", "push operation expects MessagePack bytes");
            return;
          }
          const bool opening = method_call.method_name().compare("pushOpen") == 0;
          RespondRuntimeBytesAsync(
              [runtime_library, request = *request, opening](
                  std::vector<uint8_t>* response, std::string* operation_error) {
                return opening
                    ? runtime_library->PushOpen(request, response, operation_error)
                    : runtime_library->PushItem(request, response, operation_error);
              },
              std::move(result));
          return;
        }
        if (method_call.method_name().compare("pushClose") == 0) {
          const std::string* push_id = StringArgument(method_call);
          if (push_id == nullptr) {
            result->Error("INVALID_ARGS", "pushClose expects a push id");
            return;
          }
          RespondRuntimeBytesAsync(
              [runtime_library, push_id = *push_id](
                  std::vector<uint8_t>* response, std::string* operation_error) {
                return runtime_library->PushClose(push_id, response, operation_error);
              },
              std::move(result));
          return;
        }
        if (method_call.method_name().compare("watchSnapshot") == 0) {
          const std::vector<uint8_t>* request =
              std::get_if<std::vector<uint8_t>>(method_call.arguments());
          if (request == nullptr) {
            result->Error("INVALID_ARGS", "watchSnapshot expects MessagePack bytes");
            return;
          }
          RespondRuntimeBytesAsync(
              [runtime_library, request = *request](
                  std::vector<uint8_t>* response, std::string* operation_error) {
                return runtime_library->WatchSnapshot(
                    request, response, operation_error);
              },
              std::move(result));
          return;
        }
        if (method_call.method_name().compare("watchStream") == 0) {
          const std::vector<uint8_t>* request =
              std::get_if<std::vector<uint8_t>>(method_call.arguments());
          if (request == nullptr) {
            result->Error("INVALID_ARGS", "watchStream expects MessagePack bytes");
            return;
          }
          RespondRuntimeBytesAsync(
              [runtime_library, request = *request](
                  std::vector<uint8_t>* response, std::string* operation_error) {
                if (!runtime_library->WatchStream(
                        request, response, operation_error)) {
                  return false;
                }
                EnsureWatchChannelPump(runtime_library);
                return true;
              },
              std::move(result));
          return;
        }
        if (method_call.method_name().compare("closeWatchStream") == 0) {
          const std::string* subscription = StringArgument(method_call);
          if (subscription == nullptr) {
            result->Error("INVALID_ARGS",
                          "closeWatchStream expects a subscription id");
            return;
          }
          RespondRuntimeBytesAsync(
              [runtime_library, subscription = *subscription](
                  std::vector<uint8_t>* response, std::string* operation_error) {
                return runtime_library->CloseWatchStream(
                    subscription, response, operation_error);
              },
              std::move(result));
          return;
        }
        if (method_call.method_name().compare("startWebAccessServer") == 0) {
          const std::string* bind_address =
              StringMapValue(method_call, "bindAddress");
          const std::string* token = StringMapValue(method_call, "token");
          const std::string* shutdown_token =
              StringMapValue(method_call, "shutdownToken");
          const std::string* web_root = StringMapValue(method_call, "webRoot");
          const std::string* device_info =
              StringMapValue(method_call, "deviceInfo");
          const std::string* enable_web_access =
              StringMapValue(method_call, "enableWebAccess");
          const std::string* enable_discovery =
              StringMapValue(method_call, "enableDiscovery");
          if (bind_address == nullptr || token == nullptr ||
                shutdown_token == nullptr || web_root == nullptr ||
                device_info == nullptr ||
                enable_web_access == nullptr || enable_discovery == nullptr) {
              result->Error("INVALID_ARGS",
                           "startWebAccessServer expects bindAddress, token, shutdownToken, webRoot, deviceInfo, enableWebAccess and enableDiscovery");
              return;
            }
          RespondRuntimeStringAsync(
              [runtime_library,
               bind_address = *bind_address,
               token = *token,
               shutdown_token = *shutdown_token,
               web_root = *web_root,
               device_info = *device_info,
               enable_web_access = *enable_web_access,
               enable_discovery = *enable_discovery](
                  std::string* response, std::string* operation_error) {
                return runtime_library->StartWebAccessServer(
                    bind_address, token, shutdown_token, web_root, device_info, enable_web_access,
                    enable_discovery, response, operation_error);
              },
              std::move(result));
          return;
        }
        if (method_call.method_name().compare("discoverDevices") == 0) {
          int64_t timeout_ms = 0;
          if (!IntegerMapValue(method_call, "timeoutMs", &timeout_ms)) {
            result->Error("INVALID_ARGS",
                          "discoverDevices expects timeoutMs");
            return;
          }
          std::string timeout = std::to_string(timeout_ms);
          RespondRuntimeStringAsync(
              [runtime_library, timeout = std::move(timeout)](
                  std::string* response, std::string* operation_error) {
                return runtime_library->DiscoverDevices(
                    timeout, response, operation_error);
              },
              std::move(result));
          return;
        }
        if (method_call.method_name().compare(
                "hostOnboardingPermissionSnapshot") == 0) {
          const std::string* host_id = StringMapValue(method_call, "hostId");
          if (host_id == nullptr || *host_id != "windows") {
            result->Error("INVALID_HOST", "Invalid onboarding host");
            return;
          }
          result->Success(WindowsAdminRequirementSnapshot());
          return;
        }
        if (method_call.method_name().compare(
                "hostOnboardingRequestPermission") == 0) {
          const std::string* host_id = StringMapValue(method_call, "hostId");
          const std::string* requirement_id =
              StringMapValue(method_call, "requirementId");
          if (host_id != nullptr && *host_id != "windows") {
            result->Error("INVALID_HOST", "Invalid onboarding host");
            return;
          }
          if (requirement_id == nullptr ||
              *requirement_id != "windows.admin") {
            result->Error("INVALID_ONBOARDING_REQUIREMENT",
                          "Invalid onboarding requirement");
            return;
          }
          RequestWindowsAdminAuthorization(std::move(result));
          return;
        }
        if (method_call.method_name().compare("stopWebAccessServer") == 0) {
          RespondRuntimeStringAsync(
              [runtime_library](
                  std::string* response, std::string* operation_error) {
                return runtime_library->StopWebAccessServer(
                    response, operation_error);
              },
              std::move(result));
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
          RespondRuntimeStringAsync(
              [runtime_library,
               base_url = *base_url,
               token_hash = *token_hash,
               client_device_info = *client_device_info](
                  std::string* response, std::string* operation_error) {
                return runtime_library->RemotePairStart(
                    base_url, token_hash, client_device_info, response,
                    operation_error);
              },
              std::move(result));
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
          RespondRuntimeStringAsync(
              [runtime_library,
               pairing_id = *pairing_id,
               pairing_code = *pairing_code](
                  std::string* response, std::string* operation_error) {
                return runtime_library->RemotePairFinish(
                    pairing_id, pairing_code, response, operation_error);
              },
              std::move(result));
          return;
        }
        result->NotImplemented();
      });
  g_operit_runtime_channels.push_back(std::move(channel));
}

void ShutdownOperitRuntimeChannel() {
  g_watch_channel_pump_running.store(false);
  g_operit_runtime_workers.reset();
  g_operit_runtime_channels.clear();
  ClearOperitRuntimePlatformTasks();
  g_operit_runtime_library.reset();
  g_operit_runtime_window = nullptr;
  g_operit_runtime_platform_thread_id = 0;
}
