#include "operit_runtime_channel.h"

#include <dlfcn.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>

#include <atomic>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <utility>

namespace {

using BridgeHandle = void*;
using BridgeCreate = BridgeHandle (*)();
using BridgeCreateWithStorageRoots = BridgeHandle (*)(const char*, const char*);
using BridgeCreateError = char* (*)();
using BridgeDestroy = void (*)(BridgeHandle);
using BridgeCall = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchSnapshot = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeWatchStream = char* (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeNextWatchChannelEvent = char* (*)(BridgeHandle);
using BridgeCloseWatchStream = char* (*)(BridgeHandle, const char*);
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
      call_ = reinterpret_cast<BridgeCall>(
          dlsym(library_, "operit_flutter_bridge_call"));
      watch_snapshot_ = reinterpret_cast<BridgeWatchSnapshot>(
          dlsym(library_, "operit_flutter_bridge_watch_snapshot"));
      watch_stream_ = reinterpret_cast<BridgeWatchStream>(
          dlsym(library_, "operit_flutter_bridge_watch_stream"));
      next_watch_channel_event_ = reinterpret_cast<BridgeNextWatchChannelEvent>(
          dlsym(library_, "operit_flutter_bridge_next_watch_channel_event"));
      close_watch_stream_ = reinterpret_cast<BridgeCloseWatchStream>(
          dlsym(library_, "operit_flutter_bridge_close_watch_stream"));
      free_string_ = reinterpret_cast<BridgeFreeString>(
          dlsym(library_, "operit_flutter_bridge_free_string"));
      if (create_ == nullptr || create_with_storage_roots_ == nullptr ||
          destroy_ == nullptr || call_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          next_watch_channel_event_ == nullptr ||
          close_watch_stream_ == nullptr || free_string_ == nullptr) {
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

  bool Call(const std::string& request, std::string* response,
            std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = call_(
        handle_, reinterpret_cast<const unsigned char*>(request.data()),
        request.size());
    return TakeBridgeString(raw_response, response, error);
  }

  bool WatchSnapshot(const std::string& request, std::string* response,
                     std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = watch_snapshot_(
        handle_, reinterpret_cast<const unsigned char*>(request.data()),
        request.size());
    return TakeBridgeString(raw_response, response, error);
  }

  bool WatchStream(const std::string& request, std::string* response,
                   std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = watch_stream_(
        handle_, reinterpret_cast<const unsigned char*>(request.data()),
        request.size());
    return TakeBridgeString(raw_response, response, error);
  }

  bool NextWatchChannelEvent(std::string* response, std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = next_watch_channel_event_(handle_);
    return TakeBridgeString(raw_response, response, error);
  }

  bool CloseWatchStream(const std::string& subscription, std::string* response,
                        std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = close_watch_stream_(handle_, subscription.c_str());
    return TakeBridgeString(raw_response, response, error);
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

  void* library_ = nullptr;
  BridgeHandle handle_ = nullptr;
  std::string configured_runtime_root_;
  std::string configured_workspace_root_;
  std::mutex mutex_;
  BridgeCreate create_ = nullptr;
  BridgeCreateWithStorageRoots create_with_storage_roots_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
  BridgeCall call_ = nullptr;
  BridgeWatchSnapshot watch_snapshot_ = nullptr;
  BridgeWatchStream watch_stream_ = nullptr;
  BridgeNextWatchChannelEvent next_watch_channel_event_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeFreeString free_string_ = nullptr;
};

std::shared_ptr<OperitRuntimeLibrary> g_operit_runtime_library;

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

void dispatch_watch_channel_event(std::string frame) {
  g_main_context_invoke(
      nullptr,
      [](gpointer data) -> gboolean {
        std::unique_ptr<std::string> frame(static_cast<std::string*>(data));
        if (g_operit_runtime_channel != nullptr) {
          g_autoptr(FlValue) args = fl_value_new_string(frame->c_str());
          fl_method_channel_invoke_method(g_operit_runtime_channel,
                                          "watchChannelEvent", args, nullptr,
                                          nullptr, nullptr);
        }
        return G_SOURCE_REMOVE;
      },
      new std::string(std::move(frame)));
}

void ensure_watch_channel_pump() {
  bool expected = false;
  if (!g_watch_channel_pump_running.compare_exchange_strong(expected, true)) {
    return;
  }
  auto library = g_operit_runtime_library;
  std::thread([library]() {
    while (g_watch_channel_pump_running.load()) {
      std::string frame;
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

/// Runs one Rust bridge operation off the Linux platform thread.
template <typename Operation>
void respond_runtime_string_async(FlMethodCall* method_call,
                                  Operation operation) {
  g_object_ref(method_call);
  std::thread([method_call, operation = std::move(operation)]() mutable {
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
  }).detach();
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
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS", "call expects a JSON string");
      return;
    }
    std::string request = fl_value_get_string(args);
    auto library = g_operit_runtime_library;
    respond_runtime_string_async(
        method_call,
        [library, request = std::move(request)](
            std::string* response, std::string* operation_error) {
          return library->Call(request, response, operation_error);
        });
    return;
  }
  if (strcmp(method, "watchSnapshot") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "watchSnapshot expects a JSON string");
      return;
    }
    std::string request = fl_value_get_string(args);
    auto library = g_operit_runtime_library;
    respond_runtime_string_async(
        method_call,
        [library, request = std::move(request)](
            std::string* response, std::string* operation_error) {
          return library->WatchSnapshot(
              request, response, operation_error);
        });
    return;
  }
  if (strcmp(method, "watchStream") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "watchStream expects a JSON string");
      return;
    }
    std::string request = fl_value_get_string(args);
    auto library = g_operit_runtime_library;
    respond_runtime_string_async(
        method_call,
        [library, request = std::move(request)](
            std::string* response, std::string* operation_error) {
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
    respond_runtime_string_async(
        method_call,
        [library, subscription = std::move(subscription)](
            std::string* response, std::string* operation_error) {
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
