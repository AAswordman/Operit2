#include "operit_runtime_channel.h"

#include <flutter/encodable_value.h>
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <windows.h>
#include <shellapi.h>
#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <cwchar>
#include <deque>
#include <filesystem>
#include <functional>
#include <map>
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
std::atomic_uint64_t g_watch_channel_pump_generation{0};

constexpr UINT kOperitRuntimePlatformTaskMessage = WM_APP + 0x520;
#if !defined(NDEBUG)
constexpr uint8_t kProcessOpCreate = 1;
constexpr uint8_t kProcessOpCall = 2;
constexpr uint8_t kProcessOpPushOpen = 3;
constexpr uint8_t kProcessOpPushItem = 4;
constexpr uint8_t kProcessOpPushClose = 5;
constexpr uint8_t kProcessOpWatchSnapshot = 6;
constexpr uint8_t kProcessOpWatchStream = 7;
constexpr uint8_t kProcessOpCloseWatchStream = 8;
constexpr uint8_t kProcessOpStartWebAccessServer = 9;
constexpr uint8_t kProcessOpDiscoverDevices = 10;
constexpr uint8_t kProcessOpStopWebAccessServer = 11;
constexpr uint8_t kProcessOpRemotePairStart = 12;
constexpr uint8_t kProcessOpRemotePairFinish = 13;
constexpr uint8_t kProcessFrameResponse = 101;
constexpr uint8_t kProcessFrameWatchEvent = 102;
constexpr uint8_t kProcessStatusOk = 0;
constexpr uint8_t kProcessPayloadBytes = 1;
constexpr uint8_t kProcessPayloadString = 2;
#endif

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

#if !defined(NDEBUG)
struct OperitProcessPendingResponse {
  std::mutex mutex;
  std::condition_variable condition;
  bool completed = false;
  bool ok = false;
  uint8_t payload_kind = 0;
  std::vector<uint8_t> payload;
  std::string error;
};

class OperitRuntimeProcessLibrary {
 public:
  /// Creates a Debug bridge process client.
  OperitRuntimeProcessLibrary() = default;

  /// Stops the bridge child process and releases Windows handles.
  ~OperitRuntimeProcessLibrary() { Shutdown(); }

  /// Ensures the bridge child process has created its runtime instance.
  bool EnsureReady(std::string* error) {
    if (bridge_created_) {
      return true;
    }
    if (configured_runtime_root_.empty() || configured_workspace_root_.empty()) {
      AssignError(error,
                  "Runtime and workspace roots must be configured before runtime creation");
      return false;
    }
    if (!StartProcessLocked(error)) {
      return false;
    }
    std::vector<uint8_t> payload;
    AppendString(&payload, configured_runtime_root_);
    AppendString(&payload, configured_workspace_root_);
    std::string response;
    if (!SendRequest(kProcessOpCreate, payload, kProcessPayloadString, nullptr,
                     &response, error)) {
      return false;
    }
    bridge_created_ = true;
    return true;
  }

  /// Dispatches one compact core call through the bridge process.
  bool Call(const std::vector<uint8_t>& request, std::vector<uint8_t>* response,
            std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return SendBytesOperation(kProcessOpCall, request, response, error);
  }

  /// Opens one local Link push stream through the bridge process.
  bool PushOpen(const std::vector<uint8_t>& request,
                std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return SendBytesOperation(kProcessOpPushOpen, request, response, error);
  }

  /// Dispatches one local Link push item through the bridge process.
  bool PushItem(const std::vector<uint8_t>& item,
                std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return SendBytesOperation(kProcessOpPushItem, item, response, error);
  }

  /// Closes one local Link push stream through the bridge process.
  bool PushClose(const std::string& push_id,
                 std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    std::vector<uint8_t> payload;
    AppendString(&payload, push_id);
    return SendRequest(kProcessOpPushClose, payload, kProcessPayloadBytes,
                       response, nullptr, error);
  }

  /// Reads one watch snapshot through the bridge process.
  bool WatchSnapshot(const std::vector<uint8_t>& request,
                     std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return SendBytesOperation(kProcessOpWatchSnapshot, request, response, error);
  }

  /// Opens one watch stream through the bridge process.
  bool WatchStream(const std::vector<uint8_t>& request,
                   std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return SendBytesOperation(kProcessOpWatchStream, request, response, error);
  }

  /// Pops one watch event delivered by the bridge process reader thread.
  bool NextWatchChannelEvent(std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    std::unique_lock<std::mutex> lock(watch_events_mutex_);
    watch_events_condition_.wait(
        lock, [this]() { return !watch_events_.empty() || !running_; });
    if (watch_events_.empty()) {
      AssignError(error, "runtime bridge process stopped");
      return false;
    }
    *response = std::move(watch_events_.front());
    watch_events_.pop_front();
    return true;
  }

  /// Closes one watch stream through the bridge process.
  bool CloseWatchStream(const std::string& subscription,
                        std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    std::vector<uint8_t> payload;
    AppendString(&payload, subscription);
    return SendRequest(kProcessOpCloseWatchStream, payload, kProcessPayloadBytes,
                       response, nullptr, error);
  }

  /// Starts the local web access server inside the bridge process.
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
    std::vector<uint8_t> payload;
    AppendString(&payload, bind_address);
    AppendString(&payload, token);
    AppendString(&payload, shutdown_token);
    AppendString(&payload, web_root);
    AppendString(&payload, device_info);
    AppendString(&payload, enable_web_access);
    AppendString(&payload, enable_discovery);
    return SendRequest(kProcessOpStartWebAccessServer, payload,
                       kProcessPayloadString, nullptr, response, error);
  }

  /// Discovers link devices from inside the bridge process.
  bool DiscoverDevices(const std::string& timeout_ms,
                       std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    std::vector<uint8_t> payload;
    AppendString(&payload, timeout_ms);
    return SendRequest(kProcessOpDiscoverDevices, payload, kProcessPayloadString,
                       nullptr, response, error);
  }

  /// Stops the local web access server inside the bridge process.
  bool StopWebAccessServer(std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    return SendRequest(kProcessOpStopWebAccessServer, {}, kProcessPayloadString,
                       nullptr, response, error);
  }

  /// Starts one remote pairing flow inside the bridge process.
  bool RemotePairStart(const std::string& base_url,
                       const std::string& token_hash,
                       const std::string& client_device_info,
                       std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    std::vector<uint8_t> payload;
    AppendString(&payload, base_url);
    AppendString(&payload, token_hash);
    AppendString(&payload, client_device_info);
    return SendRequest(kProcessOpRemotePairStart, payload,
                       kProcessPayloadString, nullptr, response, error);
  }

  /// Finishes one remote pairing flow inside the bridge process.
  bool RemotePairFinish(const std::string& pairing_id,
                        const std::string& pairing_code,
                        std::string* response, std::string* error) {
    if (!EnsureReadyThreadSafe(error)) {
      return false;
    }
    std::vector<uint8_t> payload;
    AppendString(&payload, pairing_id);
    AppendString(&payload, pairing_code);
    return SendRequest(kProcessOpRemotePairFinish, payload,
                       kProcessPayloadString, nullptr, response, error);
  }

  /// Rebuilds and replaces the Debug child process before creating a new runtime.
  bool RebuildAndRestart(std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (!BuildBridgeProcess(error)) {
      return false;
    }
    ShutdownLocked();
    if (!CopyBuiltBridgeProcess(error)) {
      return false;
    }
    return EnsureReady(error);
  }

  /// Sets the runtime and workspace roots used by the bridge process.
  bool SetStorageRoots(const std::string& runtime_root,
                       const std::string& workspace_root,
                       std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    std::string resolved_runtime_root;
    std::string resolved_workspace_root;
    if (!NormalizeWindowsStorageRoot(runtime_root, "runtimeRoot",
                                     &resolved_runtime_root, error)) {
      return false;
    }
    if (!NormalizeWindowsStorageRoot(workspace_root, "workspaceRoot",
                                     &resolved_workspace_root, error)) {
      return false;
    }
    if (bridge_created_) {
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
  /// Ensures the bridge process is ready while holding the root mutex.
  bool EnsureReadyThreadSafe(std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    return EnsureReady(error);
  }

  /// Assigns an optional error output.
  static void AssignError(std::string* target, const std::string& value) {
    if (target != nullptr) {
      *target = value;
    }
  }

  /// Sends one operation whose payload is a single byte vector.
  bool SendBytesOperation(uint8_t operation, const std::vector<uint8_t>& bytes,
                          std::vector<uint8_t>* response,
                          std::string* error) {
    std::vector<uint8_t> payload;
    AppendBytes(&payload, bytes);
    return SendRequest(operation, payload, kProcessPayloadBytes, response,
                       nullptr, error);
  }

  /// Starts the stdio bridge child process.
  bool StartProcessLocked(std::string* error) {
    if (process_.hProcess != nullptr) {
      return true;
    }
    SECURITY_ATTRIBUTES security_attributes{};
    security_attributes.nLength = sizeof(SECURITY_ATTRIBUTES);
    security_attributes.bInheritHandle = TRUE;

    HANDLE stdout_read = nullptr;
    HANDLE stdout_write = nullptr;
    HANDLE stdin_read = nullptr;
    HANDLE stdin_write = nullptr;
    if (::CreatePipe(&stdout_read, &stdout_write, &security_attributes, 0) == 0) {
      AssignError(error, "failed to create bridge process stdout pipe");
      return false;
    }
    if (::SetHandleInformation(stdout_read, HANDLE_FLAG_INHERIT, 0) == 0) {
      CloseOwnedHandle(stdout_read);
      CloseOwnedHandle(stdout_write);
      AssignError(error, "failed to mark bridge process stdout pipe");
      return false;
    }
    if (::CreatePipe(&stdin_read, &stdin_write, &security_attributes, 0) == 0) {
      CloseOwnedHandle(stdout_read);
      CloseOwnedHandle(stdout_write);
      AssignError(error, "failed to create bridge process stdin pipe");
      return false;
    }
    if (::SetHandleInformation(stdin_write, HANDLE_FLAG_INHERIT, 0) == 0) {
      CloseOwnedHandle(stdout_read);
      CloseOwnedHandle(stdout_write);
      CloseOwnedHandle(stdin_read);
      CloseOwnedHandle(stdin_write);
      AssignError(error, "failed to mark bridge process stdin pipe");
      return false;
    }

    std::wstring executable = BridgeProcessExecutablePath(error);
    if (executable.empty()) {
      CloseOwnedHandle(stdout_read);
      CloseOwnedHandle(stdout_write);
      CloseOwnedHandle(stdin_read);
      CloseOwnedHandle(stdin_write);
      return false;
    }

    STARTUPINFOW startup_info{};
    startup_info.cb = sizeof(startup_info);
    startup_info.dwFlags = STARTF_USESTDHANDLES;
    startup_info.hStdInput = stdin_read;
    startup_info.hStdOutput = stdout_write;
    startup_info.hStdError = ::GetStdHandle(STD_ERROR_HANDLE);
    PROCESS_INFORMATION process_info{};
    std::wstring command_line = L"\"" + executable + L"\"";
    const BOOL created = ::CreateProcessW(
        executable.c_str(), command_line.data(), nullptr, nullptr, TRUE,
        CREATE_NO_WINDOW, nullptr, nullptr, &startup_info, &process_info);
    CloseOwnedHandle(stdin_read);
    CloseOwnedHandle(stdout_write);
    if (created == 0) {
      CloseOwnedHandle(stdout_read);
      CloseOwnedHandle(stdin_write);
      AssignError(error, "failed to start operit_flutter_bridge_process.exe");
      return false;
    }

    child_stdout_read_ = stdout_read;
    child_stdin_write_ = stdin_write;
    process_ = process_info;
    running_ = true;
    reader_thread_ = std::thread([this]() { ReaderLoop(); });
    return true;
  }

  /// Stops the child process and releases handles from the current thread.
  void Shutdown() {
    std::lock_guard<std::mutex> lock(mutex_);
    ShutdownLocked();
  }

  /// Stops the child process while the root mutex is held.
  void ShutdownLocked() {
    running_ = false;
    g_watch_channel_pump_running.store(false);
    g_watch_channel_pump_generation.fetch_add(1);
    CloseOwnedHandle(child_stdin_write_);
    CloseOwnedHandle(child_stdout_read_);
    if (process_.hProcess != nullptr) {
      const DWORD wait_result = ::WaitForSingleObject(process_.hProcess, 2000);
      if (wait_result != WAIT_OBJECT_0) {
        ::TerminateProcess(process_.hProcess, 0);
        ::WaitForSingleObject(process_.hProcess, 2000);
      }
    }
    if (reader_thread_.joinable()) {
      reader_thread_.join();
    }
    CloseOwnedHandle(process_.hThread);
    CloseOwnedHandle(process_.hProcess);
    bridge_created_ = false;
    {
      std::lock_guard<std::mutex> watch_lock(watch_events_mutex_);
      watch_events_.clear();
    }
    MarkProcessStopped("runtime bridge process stopped");
  }

  /// Builds the Debug stdio bridge process with Cargo.
  bool BuildBridgeProcess(std::string* error) {
    std::vector<wchar_t> environment;
    if (!BuildRustEnvironment(&environment, error)) {
      return false;
    }
    const std::wstring cargo_path(OPERIT_CARGO_EXECUTABLE_PATH);
    const std::wstring crate_path(OPERIT_FLUTTER_BRIDGE_CRATE_PATH);
    std::wstring command_line =
        L"\"" + cargo_path + L"\" build --quiet --features process-stdio "
        L"--manifest-path \"" + crate_path + L"\\Cargo.toml\"";
    STARTUPINFOW startup_info{};
    startup_info.cb = sizeof(startup_info);
    startup_info.dwFlags = STARTF_USESHOWWINDOW;
    startup_info.wShowWindow = SW_HIDE;
    PROCESS_INFORMATION process_info{};
    if (::CreateProcessW(cargo_path.c_str(), command_line.data(), nullptr,
                         nullptr, FALSE, CREATE_NO_WINDOW, environment.data(),
                         crate_path.c_str(), &startup_info, &process_info) == 0) {
      AssignError(error, "failed to start Cargo for Debug Rust runtime build");
      return false;
    }
    ::CloseHandle(process_info.hThread);
    ::WaitForSingleObject(process_info.hProcess, INFINITE);
    DWORD exit_code = 0;
    const BOOL read_exit_code =
        ::GetExitCodeProcess(process_info.hProcess, &exit_code);
    ::CloseHandle(process_info.hProcess);
    if (read_exit_code == 0 || exit_code != 0) {
      AssignError(error, "Debug Rust runtime build failed");
      return false;
    }
    return true;
  }

  /// Copies the rebuilt Debug bridge process beside the Flutter runner.
  bool CopyBuiltBridgeProcess(std::string* error) {
    const std::filesystem::path source =
        std::filesystem::path(OPERIT_FLUTTER_BRIDGE_CRATE_PATH) / L"target" /
        L"debug" / L"operit_flutter_bridge_process.exe";
    const std::wstring destination = BridgeProcessExecutablePath(error);
    if (destination.empty()) {
      return false;
    }
    if (::CopyFileW(source.c_str(), destination.c_str(), FALSE) == 0) {
      AssignError(error, "failed to update operit_flutter_bridge_process.exe");
      return false;
    }
    return true;
  }

  /// Builds an inherited environment that disables Rust compiler warnings.
  static bool BuildRustEnvironment(std::vector<wchar_t>* environment,
                                   std::string* error) {
    LPWCH current_environment = ::GetEnvironmentStringsW();
    if (current_environment == nullptr) {
      AssignError(error, "failed to read the process environment");
      return false;
    }
    for (const wchar_t* current = current_environment; *current != L'\0';) {
      const size_t length = std::wcslen(current);
      if (!HasEnvironmentVariableName(current, L"RUSTFLAGS")) {
        environment->insert(environment->end(), current, current + length + 1);
      }
      current += length + 1;
    }
    ::FreeEnvironmentStringsW(current_environment);
    const std::wstring rust_flags = L"RUSTFLAGS=-Awarnings";
    environment->insert(environment->end(), rust_flags.begin(), rust_flags.end());
    environment->push_back(L'\0');
    environment->push_back(L'\0');
    return true;
  }

  /// Returns whether one environment entry declares the supplied variable.
  static bool HasEnvironmentVariableName(const wchar_t* entry,
                                         const wchar_t* variable_name) {
    const size_t variable_length = std::wcslen(variable_name);
    return std::wcslen(entry) > variable_length &&
           entry[variable_length] == L'=' &&
           ::CompareStringOrdinal(entry, static_cast<int>(variable_length),
                                  variable_name,
                                  static_cast<int>(variable_length), TRUE) ==
               CSTR_EQUAL;
  }

  /// Sends one request and waits for its matching response.
  bool SendRequest(uint8_t operation, const std::vector<uint8_t>& payload,
                   uint8_t expected_payload_kind,
                   std::vector<uint8_t>* bytes_response,
                   std::string* string_response, std::string* error) {
    const uint64_t request_id = NextRequestId();
    auto pending = std::make_shared<OperitProcessPendingResponse>();
    {
      std::lock_guard<std::mutex> lock(pending_responses_mutex_);
      pending_responses_[request_id] = pending;
    }
    const std::vector<uint8_t> frame =
        BuildRequestFrame(operation, request_id, payload);
    if (!WriteProcessFrame(frame, error)) {
      RemovePendingResponse(request_id);
      return false;
    }
    std::unique_lock<std::mutex> response_lock(pending->mutex);
    pending->condition.wait(response_lock,
                            [&pending]() { return pending->completed; });
    if (!pending->ok) {
      AssignError(error, pending->error);
      return false;
    }
    if (pending->payload_kind != expected_payload_kind) {
      AssignError(error, "bridge process returned an unexpected payload kind");
      return false;
    }
    if (expected_payload_kind == kProcessPayloadBytes) {
      if (bytes_response != nullptr) {
        *bytes_response = std::move(pending->payload);
      }
      return true;
    }
    if (string_response != nullptr) {
      string_response->assign(pending->payload.begin(), pending->payload.end());
    }
    return true;
  }

  /// Generates the next process request id.
  uint64_t NextRequestId() {
    std::lock_guard<std::mutex> lock(request_id_mutex_);
    return next_request_id_++;
  }

  /// Removes one pending response entry.
  void RemovePendingResponse(uint64_t request_id) {
    std::lock_guard<std::mutex> lock(pending_responses_mutex_);
    pending_responses_.erase(request_id);
  }

  /// Reads process frames and dispatches responses or watch events.
  void ReaderLoop() {
    while (running_) {
      std::vector<uint8_t> frame;
      if (!ReadProcessFrame(&frame)) {
        break;
      }
      ProcessOutputFrame(frame);
    }
    MarkProcessStopped("runtime bridge process stopped");
  }

  /// Reads one length-prefixed frame from the child stdout pipe.
  bool ReadProcessFrame(std::vector<uint8_t>* frame) {
    uint8_t length_bytes[4]{};
    if (!ReadExactPipe(child_stdout_read_, length_bytes, sizeof(length_bytes))) {
      return false;
    }
    const uint32_t length = ReadUint32(length_bytes);
    frame->assign(length, 0);
    if (length == 0) {
      return true;
    }
    return ReadExactPipe(child_stdout_read_, frame->data(), frame->size());
  }

  /// Dispatches one decoded child process output frame.
  void ProcessOutputFrame(const std::vector<uint8_t>& frame) {
    size_t offset = 0;
    uint8_t frame_kind = 0;
    if (!ReadFrameByte(frame, &offset, &frame_kind)) {
      return;
    }
    if (frame_kind == kProcessFrameResponse) {
      ProcessResponseFrame(frame, offset);
      return;
    }
    if (frame_kind == kProcessFrameWatchEvent) {
      ProcessWatchFrame(frame, offset);
    }
  }

  /// Dispatches one process response frame.
  void ProcessResponseFrame(const std::vector<uint8_t>& frame, size_t offset) {
    uint64_t request_id = 0;
    uint8_t status = 0;
    uint8_t payload_kind = 0;
    std::vector<uint8_t> payload;
    if (!ReadFrameUint64(frame, &offset, &request_id) ||
        !ReadFrameByte(frame, &offset, &status) ||
        !ReadFrameByte(frame, &offset, &payload_kind) ||
        !ReadFrameBytes(frame, &offset, &payload)) {
      return;
    }
    std::shared_ptr<OperitProcessPendingResponse> pending;
    {
      std::lock_guard<std::mutex> lock(pending_responses_mutex_);
      auto found = pending_responses_.find(request_id);
      if (found == pending_responses_.end()) {
        return;
      }
      pending = found->second;
      pending_responses_.erase(found);
    }
    {
      std::lock_guard<std::mutex> lock(pending->mutex);
      pending->completed = true;
      pending->ok = status == kProcessStatusOk;
      pending->payload_kind = payload_kind;
      pending->payload = std::move(payload);
      if (!pending->ok) {
        pending->error.assign(pending->payload.begin(), pending->payload.end());
      }
    }
    pending->condition.notify_all();
  }

  /// Dispatches one watch event frame from the child process.
  void ProcessWatchFrame(const std::vector<uint8_t>& frame, size_t offset) {
    std::vector<uint8_t> payload;
    if (!ReadFrameBytes(frame, &offset, &payload)) {
      return;
    }
    {
      std::lock_guard<std::mutex> lock(watch_events_mutex_);
      watch_events_.push_back(std::move(payload));
    }
    watch_events_condition_.notify_all();
  }

  /// Marks the process as stopped for all blocked callers.
  void MarkProcessStopped(const std::string& message) {
    running_ = false;
    std::vector<std::shared_ptr<OperitProcessPendingResponse>> pending;
    {
      std::lock_guard<std::mutex> lock(pending_responses_mutex_);
      for (const auto& item : pending_responses_) {
        pending.push_back(item.second);
      }
      pending_responses_.clear();
    }
    for (const auto& response : pending) {
      {
        std::lock_guard<std::mutex> lock(response->mutex);
        response->completed = true;
        response->ok = false;
        response->error = message;
      }
      response->condition.notify_all();
    }
    watch_events_condition_.notify_all();
  }

  /// Writes one length-prefixed frame to the child stdin pipe.
  bool WriteProcessFrame(const std::vector<uint8_t>& frame, std::string* error) {
    if (child_stdin_write_ == nullptr) {
      AssignError(error, "runtime bridge process stdin is closed");
      return false;
    }
    const uint32_t length = static_cast<uint32_t>(frame.size());
    const uint8_t* length_bytes =
        reinterpret_cast<const uint8_t*>(&length);
    if (!WriteExactPipe(child_stdin_write_, length_bytes, sizeof(length))) {
      AssignError(error, "failed to write bridge process frame length");
      return false;
    }
    if (!frame.empty() &&
        !WriteExactPipe(child_stdin_write_, frame.data(), frame.size())) {
      AssignError(error, "failed to write bridge process frame payload");
      return false;
    }
    return true;
  }

  /// Builds one request frame payload.
  static std::vector<uint8_t> BuildRequestFrame(
      uint8_t operation, uint64_t request_id,
      const std::vector<uint8_t>& payload) {
    std::vector<uint8_t> frame;
    frame.push_back(operation);
    AppendUint64(&frame, request_id);
    frame.insert(frame.end(), payload.begin(), payload.end());
    return frame;
  }

  /// Appends one little-endian u64 to a frame.
  static void AppendUint64(std::vector<uint8_t>* frame, uint64_t value) {
    for (size_t index = 0; index < sizeof(value); ++index) {
      frame->push_back(static_cast<uint8_t>((value >> (index * 8)) & 0xff));
    }
  }

  /// Appends one length-prefixed byte vector to a frame.
  static void AppendBytes(std::vector<uint8_t>* frame,
                          const std::vector<uint8_t>& value) {
    AppendUint32(frame, static_cast<uint32_t>(value.size()));
    frame->insert(frame->end(), value.begin(), value.end());
  }

  /// Appends one length-prefixed string to a frame.
  static void AppendString(std::vector<uint8_t>* frame,
                           const std::string& value) {
    AppendUint32(frame, static_cast<uint32_t>(value.size()));
    frame->insert(frame->end(), value.begin(), value.end());
  }

  /// Appends one little-endian u32 to a frame.
  static void AppendUint32(std::vector<uint8_t>* frame, uint32_t value) {
    for (size_t index = 0; index < sizeof(value); ++index) {
      frame->push_back(static_cast<uint8_t>((value >> (index * 8)) & 0xff));
    }
  }

  /// Reads one little-endian u32 from raw bytes.
  static uint32_t ReadUint32(const uint8_t* value) {
    return static_cast<uint32_t>(value[0]) |
           (static_cast<uint32_t>(value[1]) << 8) |
           (static_cast<uint32_t>(value[2]) << 16) |
           (static_cast<uint32_t>(value[3]) << 24);
  }

  /// Reads one byte from a frame.
  static bool ReadFrameByte(const std::vector<uint8_t>& frame, size_t* offset,
                            uint8_t* value) {
    if (*offset >= frame.size()) {
      return false;
    }
    *value = frame[*offset];
    ++(*offset);
    return true;
  }

  /// Reads one little-endian u64 from a frame.
  static bool ReadFrameUint64(const std::vector<uint8_t>& frame,
                              size_t* offset, uint64_t* value) {
    if (*offset + sizeof(uint64_t) > frame.size()) {
      return false;
    }
    uint64_t result = 0;
    for (size_t index = 0; index < sizeof(uint64_t); ++index) {
      result |= static_cast<uint64_t>(frame[*offset + index]) << (index * 8);
    }
    *offset += sizeof(uint64_t);
    *value = result;
    return true;
  }

  /// Reads one length-prefixed byte vector from a frame.
  static bool ReadFrameBytes(const std::vector<uint8_t>& frame, size_t* offset,
                             std::vector<uint8_t>* value) {
    if (*offset + sizeof(uint32_t) > frame.size()) {
      return false;
    }
    const uint32_t length = ReadUint32(frame.data() + *offset);
    *offset += sizeof(uint32_t);
    if (*offset + length > frame.size()) {
      return false;
    }
    value->assign(frame.begin() + *offset, frame.begin() + *offset + length);
    *offset += length;
    return true;
  }

  /// Writes every byte to a Windows pipe.
  static bool WriteExactPipe(HANDLE pipe, const uint8_t* data, size_t size) {
    size_t offset = 0;
    while (offset < size) {
      DWORD written = 0;
      const DWORD chunk =
          static_cast<DWORD>(std::min<size_t>(size - offset, 1 << 20));
      if (::WriteFile(pipe, data + offset, chunk, &written, nullptr) == 0 ||
          written == 0) {
        return false;
      }
      offset += written;
    }
    return true;
  }

  /// Reads an exact byte count from a Windows pipe.
  static bool ReadExactPipe(HANDLE pipe, uint8_t* data, size_t size) {
    size_t offset = 0;
    while (offset < size) {
      DWORD read = 0;
      const DWORD chunk =
          static_cast<DWORD>(std::min<size_t>(size - offset, 1 << 20));
      if (::ReadFile(pipe, data + offset, chunk, &read, nullptr) == 0 ||
          read == 0) {
        return false;
      }
      offset += read;
    }
    return true;
  }

  /// Closes one owned Windows handle.
  static void CloseOwnedHandle(HANDLE& handle) {
    if (handle != nullptr) {
      ::CloseHandle(handle);
      handle = nullptr;
    }
  }

  /// Resolves the bridge process executable beside the Flutter runner.
  static std::wstring BridgeProcessExecutablePath(std::string* error) {
    wchar_t module_path[MAX_PATH];
    const DWORD length =
        ::GetModuleFileNameW(nullptr, module_path, static_cast<DWORD>(MAX_PATH));
    if (length == 0 || length >= MAX_PATH) {
      AssignError(error, "failed to resolve Flutter runner path");
      return std::wstring();
    }
    std::filesystem::path path(module_path);
    path = path.parent_path() / L"operit_flutter_bridge_process.exe";
    return path.wstring();
  }

  bool bridge_created_ = false;
  std::string configured_runtime_root_;
  std::string configured_workspace_root_;
  std::mutex mutex_;
  PROCESS_INFORMATION process_{};
  HANDLE child_stdin_write_ = nullptr;
  HANDLE child_stdout_read_ = nullptr;
  std::thread reader_thread_;
  std::atomic_bool running_{false};
  std::mutex request_id_mutex_;
  uint64_t next_request_id_ = 1;
  std::mutex pending_responses_mutex_;
  std::map<uint64_t, std::shared_ptr<OperitProcessPendingResponse>>
      pending_responses_;
  std::mutex watch_events_mutex_;
  std::condition_variable watch_events_condition_;
  std::deque<std::vector<uint8_t>> watch_events_;
};

using OperitRuntimeActiveLibrary = OperitRuntimeProcessLibrary;
#else
using OperitRuntimeActiveLibrary = OperitRuntimeLibrary;
#endif

std::shared_ptr<OperitRuntimeActiveLibrary> g_operit_runtime_library;

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

void EnsureWatchChannelPump(std::shared_ptr<OperitRuntimeActiveLibrary> library) {
  bool expected = false;
  if (!g_watch_channel_pump_running.compare_exchange_strong(expected, true)) {
    return;
  }
  const uint64_t generation = g_watch_channel_pump_generation.load();
  std::thread([library = std::move(library), generation]() {
    while (g_watch_channel_pump_running.load() &&
           g_watch_channel_pump_generation.load() == generation) {
      std::vector<uint8_t> frame;
      std::string error;
      if (!library->NextWatchChannelEvent(&frame, &error)) {
        break;
      }
      DispatchWatchChannelEvent(std::move(frame));
    }
    if (g_watch_channel_pump_generation.load() == generation) {
      g_watch_channel_pump_running.store(false);
    }
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

/// Runs one void Rust bridge operation off the Windows platform thread.
template <typename Operation>
void RespondRuntimeVoidAsync(
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
    std::string error;
    const bool ok = operation(&error);
    auto platform_result = std::move(*result_holder);
    PostOperitRuntimePlatformTask(
        [result = std::move(platform_result), ok, error = std::move(error)]() mutable {
          if (ok) {
            result->Success();
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
    g_operit_runtime_library = std::make_shared<OperitRuntimeActiveLibrary>();
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
#if !defined(NDEBUG)
        if (method_call.method_name().compare(
                "debugRebuildAndRestartLocalRuntime") == 0) {
          RespondRuntimeVoidAsync(
              [runtime_library](std::string* operation_error) {
                return runtime_library->RebuildAndRestart(operation_error);
              },
              std::move(result));
          return;
        }
#endif
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
  g_watch_channel_pump_generation.fetch_add(1);
  g_operit_runtime_workers.reset();
  g_operit_runtime_channels.clear();
  ClearOperitRuntimePlatformTasks();
  g_operit_runtime_library.reset();
  g_operit_runtime_window = nullptr;
  g_operit_runtime_platform_thread_id = 0;
}
