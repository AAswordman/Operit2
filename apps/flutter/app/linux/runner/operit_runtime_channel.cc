#include "operit_runtime_channel.h"

#include <dlfcn.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>

#include <atomic>
#include <condition_variable>
#include <cstdlib>
#include <cstring>
#include <deque>
#include <filesystem>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <type_traits>
#include <utility>
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
using BridgeCall = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgePushOpen = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgePushItem = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgePushClose = OperitByteBuffer (*)(BridgeHandle, const char*);
using BridgeWatchSnapshot = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchStream = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeNextWatchChannelEvent = OperitByteBuffer (*)(BridgeHandle);
using BridgeCloseWatchStream = OperitByteBuffer (*)(BridgeHandle, const char*);
using BridgeFreeBytes = void (*)(OperitByteBuffer);
using BridgeFreeString = void (*)(char*);

FlMethodChannel* g_operit_runtime_channel = nullptr;
std::atomic_bool g_watch_channel_pump_running{false};

std::string json_string(const std::string& value) {
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
        output += ch;
        break;
    }
  }
  output += "\"";
  return output;
}

/// Normalizes one caller-supplied Linux storage root.
bool normalize_linux_storage_root(const std::string& requested,
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
  const std::filesystem::path path =
      std::filesystem::path(requested).lexically_normal();
  if (!path.is_absolute()) {
    if (error != nullptr) {
      *error = std::string(label) + " must be an absolute path";
    }
    return false;
  }
  *storage_root = path.string();
  return true;
}

/// Resolves the default Linux runtime and workspace roots.
bool resolve_linux_default_storage_roots(std::string* runtime_root,
                                         std::string* workspace_root,
                                         std::string* error) {
  if (runtime_root == nullptr || workspace_root == nullptr) {
    if (error != nullptr) {
      *error = "runtime and workspace root outputs are required";
    }
    return false;
  }
  const gchar* user_data_dir = g_get_user_data_dir();
  if (user_data_dir == nullptr || user_data_dir[0] == '\0') {
    if (error != nullptr) {
      *error = "Linux user data directory is required for Operit2 storage";
    }
    return false;
  }
  const std::filesystem::path base =
      std::filesystem::path(user_data_dir) / "operit2";
  *runtime_root = (base / "runtime").string();
  *workspace_root = (base / "workspaces").string();
  return true;
}

/// Builds Flutter storage path values for resolved Linux roots.
FlValue* linux_storage_paths(const std::string& runtime_root,
                             const std::string& workspace_root) {
  FlValue* paths = fl_value_new_map();
  fl_value_set_string_take(
      paths, "runtimeRoot",
      fl_value_new_string(runtime_root.c_str()));
  fl_value_set_string_take(
      paths, "workspaceRoot",
      fl_value_new_string(workspace_root.c_str()));
  return paths;
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
      dlclose(library_);
      library_ = nullptr;
    }
  }

  bool EnsureReady(std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (handle_ != nullptr) {
      return true;
    }
    if (library_ == nullptr) {
      library_ = dlopen("liboperit_flutter_bridge.so", RTLD_NOW | RTLD_LOCAL);
      if (library_ == nullptr) {
        AssignError(error, dlerror());
        return false;
      }
      create_ =
          reinterpret_cast<BridgeCreate>(
              dlsym(library_, "operit_flutter_bridge_create"));
      create_with_storage_roots_ =
          reinterpret_cast<BridgeCreateWithStorageRoots>(
              dlsym(
                  library_,
                  "operit_flutter_bridge_create_with_storage_roots"));
      create_error_ = reinterpret_cast<BridgeCreateError>(
          dlsym(library_, "operit_flutter_bridge_create_error"));
      destroy_ = reinterpret_cast<BridgeDestroy>(
          dlsym(library_, "operit_flutter_bridge_destroy"));
      native_call_ = reinterpret_cast<BridgeNativeCall>(
          dlsym(library_, "operit_flutter_bridge_native_call"));
      push_open_ = reinterpret_cast<BridgePushOpen>(
          dlsym(library_, "operit_flutter_bridge_push_open"));
      push_item_ = reinterpret_cast<BridgePushItem>(
          dlsym(library_, "operit_flutter_bridge_push_item"));
      push_close_ = reinterpret_cast<BridgePushClose>(
          dlsym(library_, "operit_flutter_bridge_push_close"));
      watch_snapshot_ = reinterpret_cast<BridgeWatchSnapshot>(
          dlsym(library_, "operit_flutter_bridge_watch_snapshot"));
      watch_stream_ = reinterpret_cast<BridgeWatchStream>(
          dlsym(library_, "operit_flutter_bridge_watch_stream"));
      next_watch_channel_event_ = reinterpret_cast<BridgeNextWatchChannelEvent>(
          dlsym(library_, "operit_flutter_bridge_next_watch_channel_event"));
      close_watch_stream_ = reinterpret_cast<BridgeCloseWatchStream>(
          dlsym(library_, "operit_flutter_bridge_close_watch_stream"));
      free_bytes_ = reinterpret_cast<BridgeFreeBytes>(
          dlsym(library_, "operit_flutter_bridge_free_bytes"));
      free_string_ = reinterpret_cast<BridgeFreeString>(
          dlsym(library_, "operit_flutter_bridge_free_string"));
      if (create_ == nullptr || create_with_storage_roots_ == nullptr ||
          destroy_ == nullptr || native_call_ == nullptr || push_open_ == nullptr ||
          push_item_ == nullptr || push_close_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          next_watch_channel_event_ == nullptr ||
          close_watch_stream_ == nullptr || free_bytes_ == nullptr ||
          free_string_ == nullptr) {
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
    if (!EnsureReady(error)) {
      return false;
    }
    return TakeBridgeBytes(
        native_call_(handle_, request.data(), request.size()), response, error);
  }

  /// Opens one local Link push stream.
  bool PushOpen(const std::vector<uint8_t>& request, std::vector<uint8_t>* response,
                std::string* error) {
    if (!EnsureReady(error)) return false;
    return TakeBridgeBytes(push_open_(handle_, request.data(), request.size()), response, error);
  }

  /// Dispatches one local Link push item.
  bool PushItem(const std::vector<uint8_t>& item, std::vector<uint8_t>* response,
                std::string* error) {
    if (!EnsureReady(error)) return false;
    return TakeBridgeBytes(push_item_(handle_, item.data(), item.size()), response, error);
  }

  /// Closes one local Link push stream.
  bool PushClose(const std::string& push_id, std::vector<uint8_t>* response,
                 std::string* error) {
    if (!EnsureReady(error)) return false;
    return TakeBridgeBytes(push_close_(handle_, push_id.c_str()), response, error);
  }

  bool WatchSnapshot(const std::vector<uint8_t>& request, std::vector<uint8_t>* response,
                     std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    return TakeBridgeBytes(watch_snapshot_(handle_, request.data(), request.size()), response, error);
  }

  bool WatchStream(const std::vector<uint8_t>& request, std::vector<uint8_t>* response,
                   std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    return TakeBridgeBytes(watch_stream_(handle_, request.data(), request.size()), response, error);
  }

  bool NextWatchChannelEvent(std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    return TakeBridgeBytes(next_watch_channel_event_(handle_), response, error);
  }

  bool CloseWatchStream(const std::string& subscription, std::vector<uint8_t>* response,
                        std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    return TakeBridgeBytes(close_watch_stream_(handle_, subscription.c_str()), response, error);
  }

  /// Sets the runtime and workspace roots used when the runtime handle is created.
  bool SetStorageRoots(const std::string& runtime_root,
                       const std::string& workspace_root,
                       std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    std::string resolved_runtime_root;
    std::string resolved_workspace_root;
    if (!normalize_linux_storage_root(
            runtime_root, "runtimeRoot", &resolved_runtime_root, error)) {
      return false;
    }
    if (!normalize_linux_storage_root(
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
  static void AssignError(std::string* target, const char* value) {
    if (target != nullptr) {
      *target = value == nullptr ? "" : value;
    }
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

  /// Copies and releases one Rust-owned binary response.
  bool TakeBridgeBytes(OperitByteBuffer value, std::vector<uint8_t>* output,
                       std::string* error) {
    if (value.ptr == nullptr && value.len != 0) {
      AssignError(error, "operit flutter bridge returned invalid bytes");
      return false;
    }
    if (output != nullptr) {
      if (value.len == 0) {
        output->clear();
      } else {
        output->assign(value.ptr, value.ptr + value.len);
      }
    }
    free_bytes_(value);
    return true;
  }

  void* library_ = nullptr;
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
  BridgeFreeBytes free_bytes_ = nullptr;
  BridgeFreeString free_string_ = nullptr;
};

std::shared_ptr<OperitRuntimeLibrary> g_operit_runtime_library;

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

std::unique_ptr<OperitRuntimeWorkerQueue> g_operit_runtime_workers;

void respond_error(FlMethodCall* method_call,
                   const char* code,
                   const std::string& message) {
  g_autoptr(FlMethodErrorResponse) response =
      fl_method_error_response_new(code, message.c_str(), nullptr);
  fl_method_call_respond(method_call, FL_METHOD_RESPONSE(response), nullptr);
}

void respond_success(FlMethodCall* method_call, const std::string& value) {
  g_autoptr(FlValue) result = fl_value_new_string(value.c_str());
  g_autoptr(FlMethodSuccessResponse) response =
      fl_method_success_response_new(result);
  fl_method_call_respond(method_call, FL_METHOD_RESPONSE(response), nullptr);
}

void respond_success_value(FlMethodCall* method_call, FlValue* value) {
  g_autoptr(FlMethodSuccessResponse) response =
      fl_method_success_response_new(value);
  fl_method_call_respond(method_call, FL_METHOD_RESPONSE(response), nullptr);
}

FlValue* linux_root_requirement_snapshot() {
  g_autoptr(FlValue) item = fl_value_new_map();
  fl_value_set_string_take(item, "id", fl_value_new_string("linux.root"));
  fl_value_set_string_take(
      item, "status",
      fl_value_new_string(geteuid() == 0 ? "Satisfied" : "Missing"));

  FlValue* result = fl_value_new_map();
  fl_value_set_string_take(result, "linux.root", fl_value_ref(item));
  return result;
}

void dispatch_watch_channel_event(std::vector<uint8_t> frame) {
  g_main_context_invoke(
      nullptr,
      [](gpointer data) -> gboolean {
        std::unique_ptr<std::vector<uint8_t>> frame(
            static_cast<std::vector<uint8_t>*>(data));
        if (g_operit_runtime_channel != nullptr) {
          g_autoptr(FlValue) args =
              fl_value_new_uint8_list(frame->data(), frame->size());
          fl_method_channel_invoke_method(g_operit_runtime_channel,
                                          "watchChannelEvent", args, nullptr,
                                          nullptr, nullptr);
        }
        return G_SOURCE_REMOVE;
      },
      new std::vector<uint8_t>(std::move(frame)));
}

void ensure_watch_channel_pump() {
  bool expected = false;
  if (!g_watch_channel_pump_running.compare_exchange_strong(expected, true)) {
    return;
  }
  auto library = g_operit_runtime_library;
  std::thread([library]() {
    while (g_watch_channel_pump_running.load()) {
      std::vector<uint8_t> frame;
      std::string error;
      if (!library->NextWatchChannelEvent(&frame, &error)) {
        break;
      }
      dispatch_watch_channel_event(std::move(frame));
    }
    g_watch_channel_pump_running.store(false);
  }).detach();
}

struct RuntimeStringResponse {
  FlMethodCall* method_call;
  bool ok;
  std::string response;
  std::string error;
};

struct RuntimeBytesResponse {
  FlMethodCall* method_call;
  bool ok;
  std::vector<uint8_t> response;
  std::string error;
};

/// Runs one binary Rust bridge operation off the Linux platform thread.
template <typename Operation>
void respond_runtime_bytes_async(FlMethodCall* method_call,
                                 Operation operation) {
  auto* workers = g_operit_runtime_workers.get();
  if (workers == nullptr) {
    respond_error(method_call, "RUNTIME_WORKER_QUEUE_CLOSED",
                  "runtime worker queue is not available");
    return;
  }
  g_object_ref(method_call);
  const bool submitted = workers->Post(
      [method_call, operation = std::move(operation)]() mutable {
    std::vector<uint8_t> response;
    std::string error;
    const bool ok = operation(&response, &error);
    auto* result = new RuntimeBytesResponse{
        method_call, ok, std::move(response), std::move(error)};
    g_main_context_invoke(
        nullptr,
        [](gpointer data) -> gboolean {
          std::unique_ptr<RuntimeBytesResponse> result(
              static_cast<RuntimeBytesResponse*>(data));
          if (result->ok) {
            g_autoptr(FlValue) value = fl_value_new_uint8_list(
                result->response.data(), result->response.size());
            respond_success_value(result->method_call, value);
          } else {
            respond_error(result->method_call, "RUNTIME_BRIDGE_ERROR", result->error);
          }
          g_object_unref(result->method_call);
          return G_SOURCE_REMOVE;
        },
        result);
  });
  if (!submitted) {
    g_object_unref(method_call);
    respond_error(method_call, "RUNTIME_WORKER_QUEUE_CLOSED",
                  "runtime worker queue is not accepting work");
  }
}

/// Runs one Rust bridge operation off the Linux platform thread.
template <typename Operation>
void respond_runtime_string_async(FlMethodCall* method_call,
                                  Operation operation) {
  auto* workers = g_operit_runtime_workers.get();
  if (workers == nullptr) {
    respond_error(method_call, "RUNTIME_WORKER_QUEUE_CLOSED",
                  "runtime worker queue is not available");
    return;
  }
  g_object_ref(method_call);
  const bool submitted = workers->Post(
      [method_call, operation = std::move(operation)]() mutable {
    std::string response;
    std::string error;
    const bool ok = operation(&response, &error);
    auto* result = new RuntimeStringResponse{
        method_call, ok, std::move(response), std::move(error)};
    g_main_context_invoke(
        nullptr,
        [](gpointer data) -> gboolean {
          std::unique_ptr<RuntimeStringResponse> result(
              static_cast<RuntimeStringResponse*>(data));
          if (result->ok) {
            respond_success(result->method_call, result->response);
          } else {
            respond_error(
                result->method_call, "RUNTIME_BRIDGE_ERROR", result->error);
          }
          g_object_unref(result->method_call);
          return G_SOURCE_REMOVE;
        },
        result);
  });
  if (!submitted) {
    g_object_unref(method_call);
    respond_error(method_call, "RUNTIME_WORKER_QUEUE_CLOSED",
                  "runtime worker queue is not accepting work");
  }
}

const gchar* string_map_value(FlValue* map, const char* key) {
  FlValue* value = fl_value_lookup_string(map, key);
  if (value == nullptr || fl_value_get_type(value) != FL_VALUE_TYPE_STRING) {
    return nullptr;
  }
  return fl_value_get_string(value);
}

/// Copies a Flutter uint8 list into an owned byte vector.
bool bytes_value(FlValue* value, std::vector<uint8_t>* output) {
  if (value == nullptr || output == nullptr ||
      fl_value_get_type(value) != FL_VALUE_TYPE_UINT8_LIST) {
    return false;
  }
  const uint8_t* bytes = fl_value_get_uint8_list(value);
  output->assign(bytes, bytes + fl_value_get_length(value));
  return true;
}

void operit_runtime_method_call_cb(FlMethodChannel* channel,
                                   FlMethodCall* method_call,
                                   gpointer user_data) {
  (void)channel;
  (void)user_data;
  const gchar* method = fl_method_call_get_name(method_call);
  std::string error;
  if (strcmp(method, "localRuntimeStorageDefaults") == 0) {
    std::string runtime_root;
    std::string workspace_root;
    if (!resolve_linux_default_storage_roots(
            &runtime_root, &workspace_root, &error)) {
      respond_error(
          method_call, "RUNTIME_STORAGE_DEFAULTS_ERROR", error);
      return;
    }
    g_autoptr(FlValue) result =
        linux_storage_paths(runtime_root, workspace_root);
    respond_success_value(method_call, result);
    return;
  }
  if (strcmp(method, "localRuntimeStoragePaths") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    const gchar* requested_runtime_root = nullptr;
    const gchar* requested_workspace_root = nullptr;
    if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
      requested_runtime_root = string_map_value(args, "runtimeRoot");
      requested_workspace_root = string_map_value(args, "workspaceRoot");
    }
    if (requested_runtime_root == nullptr ||
        requested_workspace_root == nullptr) {
      respond_error(
          method_call, "INVALID_ARGS",
          "localRuntimeStoragePaths expects runtimeRoot and workspaceRoot");
      return;
    }
    std::string runtime_root;
    std::string workspace_root;
    if (!normalize_linux_storage_root(
            requested_runtime_root, "runtimeRoot", &runtime_root, &error) ||
        !normalize_linux_storage_root(
            requested_workspace_root,
            "workspaceRoot",
            &workspace_root,
            &error)) {
      respond_error(
          method_call, "RUNTIME_STORAGE_PATHS_ERROR", error);
      return;
    }
    g_autoptr(FlValue) result =
        linux_storage_paths(runtime_root, workspace_root);
    respond_success_value(method_call, result);
    return;
  }
  if (strcmp(method, "setLocalRuntimeStorage") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    const gchar* runtime_root = nullptr;
    const gchar* workspace_root = nullptr;
    if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
      runtime_root = string_map_value(args, "runtimeRoot");
      workspace_root = string_map_value(args, "workspaceRoot");
    }
    if (runtime_root == nullptr || workspace_root == nullptr) {
      respond_error(
          method_call, "INVALID_ARGS",
          "setLocalRuntimeStorage expects runtimeRoot and workspaceRoot");
      return;
    }
    if (!g_operit_runtime_library->SetStorageRoots(
            runtime_root, workspace_root, &error)) {
      respond_error(method_call, "RUNTIME_STORAGE_SET_ERROR", error);
      return;
    }
    respond_success_value(method_call, nullptr);
    return;
  }
  if (strcmp(method, "call") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    std::vector<uint8_t> request;
    if (!bytes_value(args, &request)) {
      respond_error(method_call, "INVALID_ARGS", "call expects MessagePack bytes");
      return;
    }
    auto library = g_operit_runtime_library;
    respond_runtime_bytes_async(
        method_call,
        [library, request = std::move(request)](
            std::vector<uint8_t>* response, std::string* operation_error) {
          return library->Call(request, response, operation_error);
        });
    return;
  }
  if (strcmp(method, "pushOpen") == 0 || strcmp(method, "pushItem") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    std::vector<uint8_t> request;
    if (!bytes_value(args, &request)) {
      respond_error(method_call, "INVALID_ARGS", "push operation expects MessagePack bytes");
      return;
    }
    auto library = g_operit_runtime_library;
    const bool opening = strcmp(method, "pushOpen") == 0;
    respond_runtime_bytes_async(
        method_call,
        [library, request = std::move(request), opening](
            std::vector<uint8_t>* response, std::string* operation_error) {
          return opening
              ? library->PushOpen(request, response, operation_error)
              : library->PushItem(request, response, operation_error);
        });
    return;
  }
  if (strcmp(method, "pushClose") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS", "pushClose expects a push id");
      return;
    }
    std::string push_id = fl_value_get_string(args);
    auto library = g_operit_runtime_library;
    respond_runtime_bytes_async(
        method_call,
        [library, push_id = std::move(push_id)](
            std::vector<uint8_t>* response, std::string* operation_error) {
          return library->PushClose(push_id, response, operation_error);
        });
    return;
  }
  if (strcmp(method, "watchSnapshot") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    std::vector<uint8_t> request;
    if (!bytes_value(args, &request)) {
      respond_error(method_call, "INVALID_ARGS",
                    "watchSnapshot expects MessagePack bytes");
      return;
    }
    auto library = g_operit_runtime_library;
    respond_runtime_bytes_async(
        method_call,
        [library, request = std::move(request)](
            std::vector<uint8_t>* response, std::string* operation_error) {
          return library->WatchSnapshot(
              request, response, operation_error);
        });
    return;
  }
  if (strcmp(method, "watchStream") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    std::vector<uint8_t> request;
    if (!bytes_value(args, &request)) {
      respond_error(method_call, "INVALID_ARGS",
                    "watchStream expects MessagePack bytes");
      return;
    }
    auto library = g_operit_runtime_library;
    respond_runtime_bytes_async(
        method_call,
        [library, request = std::move(request)](
            std::vector<uint8_t>* response, std::string* operation_error) {
          if (!library->WatchStream(request, response, operation_error)) {
            return false;
          }
          ensure_watch_channel_pump();
          return true;
        });
    return;
  }
  if (strcmp(method, "closeWatchStream") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "closeWatchStream expects a subscription id");
      return;
    }
    std::string subscription = fl_value_get_string(args);
    auto library = g_operit_runtime_library;
    respond_runtime_bytes_async(
        method_call,
        [library, subscription = std::move(subscription)](
            std::vector<uint8_t>* response, std::string* operation_error) {
          return library->CloseWatchStream(
              subscription, response, operation_error);
        });
    return;
  }
  if (strcmp(method, "hostOnboardingPermissionSnapshot") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    const gchar* host_id = nullptr;
    if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
      host_id = string_map_value(args, "hostId");
    }
    if (host_id == nullptr || strcmp(host_id, "linux") != 0) {
      respond_error(method_call, "INVALID_HOST", "Invalid onboarding host");
      return;
    }
    g_autoptr(FlValue) result = linux_root_requirement_snapshot();
    respond_success_value(method_call, result);
    return;
  }
  if (strcmp(method, "hostOnboardingRequestPermission") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    const gchar* host_id = nullptr;
    const gchar* requirement_id = nullptr;
    if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
      host_id = string_map_value(args, "hostId");
      requirement_id = string_map_value(args, "requirementId");
    }
    if (host_id != nullptr && strcmp(host_id, "linux") != 0) {
      respond_error(method_call, "INVALID_HOST", "Invalid onboarding host");
      return;
    }
    if (requirement_id == nullptr || strcmp(requirement_id, "linux.root") != 0) {
      respond_error(method_call, "INVALID_ONBOARDING_REQUIREMENT",
                    "Invalid onboarding requirement");
      return;
    }
    respond_error(method_call, "HOST_AUTHORIZATION_MANAGED",
                  "Restart Operit Host as root or through the service manager");
    return;
  }
  g_autoptr(FlMethodNotImplementedResponse) response =
      fl_method_not_implemented_response_new();
  fl_method_call_respond(method_call, FL_METHOD_RESPONSE(response), nullptr);
}

}  // namespace

/// Attaches the process-level Runtime to the current Flutter view.
void register_operit_runtime_channel(FlView* view) {
  if (!g_operit_runtime_library) {
    g_operit_runtime_library = std::make_shared<OperitRuntimeLibrary>();
  }
  if (!g_operit_runtime_workers) {
    g_operit_runtime_workers = std::make_unique<OperitRuntimeWorkerQueue>(4);
  }
  if (g_operit_runtime_channel != nullptr) {
    fl_method_channel_set_method_call_handler(
        g_operit_runtime_channel, nullptr, nullptr, nullptr);
    g_clear_object(&g_operit_runtime_channel);
  }
  FlBinaryMessenger* messenger =
      fl_engine_get_binary_messenger(fl_view_get_engine(view));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_operit_runtime_channel = fl_method_channel_new(
      messenger, "operit/runtime", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      g_operit_runtime_channel, operit_runtime_method_call_cb, nullptr,
      nullptr);
}
