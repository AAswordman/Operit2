#include "operit_runtime_channel.h"

#include <dlfcn.h>
#include <gtk/gtk.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>

#include <atomic>
#include <algorithm>
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
using BridgeFreeString = void (*)(char*);
using BridgeRuntimeBootstrapRead = char* (*)(const char*);
using BridgeRuntimeBootstrapWrite = char* (*)(const char*, const char*);

std::vector<FlMethodChannel*> g_operit_runtime_channels;

/// Invokes one Runtime bootstrap storage export without creating a Core handle.
bool invoke_runtime_bootstrap_storage(const std::string& default_runtime_root,
                                      const std::string* content,
                                      std::string* response,
                                      std::string* error) {
  void* library = dlopen("liboperit_flutter_bridge.so", RTLD_NOW | RTLD_LOCAL);
  if (library == nullptr) {
    if (error != nullptr) {
      *error = dlerror();
    }
    return false;
  }
  const auto read = reinterpret_cast<BridgeRuntimeBootstrapRead>(
      dlsym(library, "operit_flutter_bridge_runtime_bootstrap_read"));
  const auto write = reinterpret_cast<BridgeRuntimeBootstrapWrite>(
      dlsym(library, "operit_flutter_bridge_runtime_bootstrap_write"));
  const auto free_string = reinterpret_cast<BridgeFreeString>(
      dlsym(library, "operit_flutter_bridge_free_string"));
  if (read == nullptr || write == nullptr || free_string == nullptr) {
    if (error != nullptr) {
      *error = "operit flutter bootstrap exports are incomplete";
    }
    dlclose(library);
    return false;
  }
  char* raw = content == nullptr
                  ? read(default_runtime_root.c_str())
                  : write(default_runtime_root.c_str(), content->c_str());
  if (raw == nullptr) {
    if (error != nullptr) {
      *error = "operit flutter bootstrap export returned null";
    }
    dlclose(library);
    return false;
  }
  if (response != nullptr) {
    *response = raw;
  }
  free_string(raw);
  dlclose(library);
  return true;
}

/// Calls the offline ABI on a retained library mapping without starting Core.
bool invoke_data_rescue(const std::string& root, const std::string& request,
                        std::string* response, std::string* error) {
  static void* library = dlopen("liboperit_flutter_bridge.so", RTLD_NOW | RTLD_LOCAL);
  if (library == nullptr) { *error = "Data rescue bridge library is unavailable"; return false; }
  const auto call = reinterpret_cast<BridgeRuntimeBootstrapWrite>(dlsym(library, "operit_flutter_bridge_data_rescue"));
  const auto release = reinterpret_cast<BridgeFreeString>(dlsym(library, "operit_flutter_bridge_free_string"));
  if (call == nullptr || release == nullptr) { *error = "Data rescue ABI is incomplete"; return false; }
  char* raw = call(root.c_str(), request.c_str());
  if (raw == nullptr) { *error = "Data rescue returned null"; return false; }
  *response = raw;
  release(raw);
  return true;
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
  /// Releases the host runtime reference while preserving loaded FFI code.
  ~OperitRuntimeLibrary() {
    if (handle_ != nullptr && destroy_ != nullptr) {
      destroy_(handle_);
      handle_ = nullptr;
    }
    // FFI function pointers and VM finalizers retain this module until process exit.
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

      free_string_ = reinterpret_cast<BridgeFreeString>(
          dlsym(library_, "operit_flutter_bridge_free_string"));
      if (create_ == nullptr || create_with_storage_roots_ == nullptr ||
          destroy_ == nullptr || free_string_ == nullptr) {
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

  /// Creates a retained Dart FFI connection to the process-owned runtime.
  bool ConnectCoreFfi(std::vector<uint8_t>* response, std::string* error) {
    if (!EnsureReady(error)) return false;
    const auto connect = reinterpret_cast<char* (*)(BridgeHandle)>(
        dlsym(library_, "operit_flutter_bridge_ffi_connect"));
    if (connect == nullptr) {
      AssignError(error, "operit flutter FFI connect export is missing");
      return false;
    }
    std::string descriptor;
    if (!TakeBridgeString(connect(handle_), &descriptor, error)) return false;
    response->assign(descriptor.begin(), descriptor.end());
    return true;
  }

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

  void* library_ = nullptr;
  BridgeHandle handle_ = nullptr;
  std::string configured_runtime_root_;
  std::string configured_workspace_root_;
  std::mutex mutex_;
  BridgeCreate create_ = nullptr;
  BridgeCreateWithStorageRoots create_with_storage_roots_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
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

/// Reads one PNG payload from the Linux GTK clipboard.
FlValue* linux_clipboard_images(std::string* error) {
  FlValue* images = fl_value_new_list();
  GtkClipboard* clipboard = gtk_clipboard_get(GDK_SELECTION_CLIPBOARD);
  if (clipboard == nullptr) {
    return images;
  }
  g_autoptr(GdkPixbuf) pixbuf = gtk_clipboard_wait_for_image(clipboard);
  if (pixbuf == nullptr) {
    return images;
  }
  g_autofree gchar* png = nullptr;
  gsize png_size = 0;
  g_autoptr(GError) save_error = nullptr;
  if (!gdk_pixbuf_save_to_buffer(
          pixbuf, &png, &png_size, "png", &save_error, nullptr)) {
    if (error != nullptr) {
      *error = save_error != nullptr && save_error->message != nullptr
                   ? save_error->message
                   : "Linux clipboard image encoding failed";
    }
    fl_value_unref(images);
    return nullptr;
  }
  FlValue* item = fl_value_new_map();
  fl_value_set_string_take(item, "mimeType", fl_value_new_string("image/png"));
  fl_value_set_string_take(
      item, "bytes",
      fl_value_new_uint8_list(reinterpret_cast<const uint8_t*>(png), png_size));
  fl_value_append_take(images, item);
  return images;
}

struct RuntimeBytesResponse {
  FlMethodCall* method_call;
  bool ok;
  std::vector<uint8_t> response;
  std::string error;
};

/// Creates the FFI descriptor off the Linux platform thread.
template <typename Operation>
void respond_runtime_descriptor_async(FlMethodCall* method_call,
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
            g_autoptr(FlValue) value = fl_value_new_string(
                std::string(result->response.begin(), result->response.end()).c_str());
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

const gchar* string_map_value(FlValue* map, const char* key) {
  FlValue* value = fl_value_lookup_string(map, key);
  if (value == nullptr || fl_value_get_type(value) != FL_VALUE_TYPE_STRING) {
    return nullptr;
  }
  return fl_value_get_string(value);
}


void operit_runtime_method_call_cb(FlMethodChannel* channel,
                                   FlMethodCall* method_call,
                                   gpointer user_data) {
  (void)channel;
  (void)user_data;
  const gchar* method = fl_method_call_get_name(method_call);
  std::string error;
  if (strcmp(method, "restartApplication") == 0) {
    GApplication* application = g_application_get_default();
    if (application == nullptr) {
      respond_error(method_call, "APPLICATION_TERMINATION_ERROR",
                    "Linux application close request failed");
      return;
    }
    respond_success_value(method_call, nullptr);
    g_application_quit(application);
    return;
  }
  if (strcmp(method, "readClipboardImages") == 0) {
    g_autoptr(FlValue) result = linux_clipboard_images(&error);
    if (result == nullptr) {
      respond_error(method_call, "CLIPBOARD_IMAGE_READ_ERROR", error);
      return;
    }
    respond_success_value(method_call, result);
    return;
  }
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
  if (strcmp(method, "dataRescue") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS", "dataRescue expects JSON text"); return;
    }
    std::string root;
    std::string workspace;
    if (!resolve_linux_default_storage_roots(&root, &workspace, &error)) { respond_error(method_call, "DATA_RESCUE_ERROR", error); return; }
    const std::string request = fl_value_get_string(args);
    respond_runtime_descriptor_async(method_call, [root, request](std::vector<uint8_t>* bytes, std::string* error) {
      std::string response;
      if (!invoke_data_rescue(root, request, &response, error)) return false;
      bytes->assign(response.begin(), response.end());
      return true;
    });
    return;
  }
  if (strcmp(method, "runtimeBootstrapRead") == 0) {
    std::string runtime_root;
    std::string workspace_root;
    std::string response;
    if (!resolve_linux_default_storage_roots(
            &runtime_root, &workspace_root, &error) ||
        !invoke_runtime_bootstrap_storage(
            runtime_root, nullptr, &response, &error)) {
      respond_error(method_call, "RUNTIME_BOOTSTRAP_READ_ERROR", error);
      return;
    }
    g_autoptr(FlValue) result = fl_value_new_string(response.c_str());
    respond_success_value(method_call, result);
    return;
  }
  if (strcmp(method, "runtimeBootstrapWrite") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(
          method_call, "INVALID_ARGS",
          "runtimeBootstrapWrite expects JSON text");
      return;
    }
    std::string runtime_root;
    std::string workspace_root;
    std::string response;
    const std::string content = fl_value_get_string(args);
    if (!resolve_linux_default_storage_roots(
            &runtime_root, &workspace_root, &error) ||
        !invoke_runtime_bootstrap_storage(
            runtime_root, &content, &response, &error)) {
      respond_error(method_call, "RUNTIME_BOOTSTRAP_WRITE_ERROR", error);
      return;
    }
    g_autoptr(FlValue) result = fl_value_new_string(response.c_str());
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
  if (strcmp(method, "connectCoreFfi") == 0) {
    auto library = g_operit_runtime_library;
    respond_runtime_descriptor_async(
        method_call,
        [library](std::vector<uint8_t>* response, std::string* operation_error) {
          return library->ConnectCoreFfi(response, operation_error);
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

/// Removes only the destroyed view's channel while preserving other engine clients.
static void release_operit_runtime_channel(gpointer data) {
  auto* channel = FL_METHOD_CHANNEL(data);
  fl_method_channel_set_method_call_handler(channel, nullptr, nullptr, nullptr);
  auto& channels = g_operit_runtime_channels;
  channels.erase(std::remove(channels.begin(), channels.end(), channel), channels.end());
  g_object_unref(channel);
}

/// Attaches the process-level Runtime to the current Flutter view.
void register_operit_runtime_channel(FlView* view) {
  if (!g_operit_runtime_library) {
    g_operit_runtime_library = std::make_shared<OperitRuntimeLibrary>();
  }
  if (!g_operit_runtime_workers) {
    g_operit_runtime_workers = std::make_unique<OperitRuntimeWorkerQueue>(4);
  }
  FlBinaryMessenger* messenger =
      fl_engine_get_binary_messenger(fl_view_get_engine(view));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  FlMethodChannel* channel = fl_method_channel_new(
      messenger, "operit/runtime", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      channel, operit_runtime_method_call_cb, nullptr,
      nullptr);
  g_object_set_data_full(G_OBJECT(view), "operit-runtime-channel", channel,
                        release_operit_runtime_channel);
  g_operit_runtime_channels.push_back(channel);
}
