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
using BridgeCreateWithStorageRoot = BridgeHandle (*)(const char*);
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

/// Resolves the requested Linux storage root into a normalized path.
bool resolve_linux_storage_root(const std::string& requested,
                                std::string* storage_root,
                                std::string* error) {
  if (storage_root == nullptr) {
    if (error != nullptr) {
      *error = "storage root output is required";
    }
    return false;
  }
  if (!requested.empty()) {
    *storage_root =
        std::filesystem::path(requested).lexically_normal().string();
    return true;
  }
  const gchar* user_data_dir = g_get_user_data_dir();
  if (user_data_dir == nullptr || user_data_dir[0] == '\0') {
    if (error != nullptr) {
      *error = "Linux user data directory is required for Operit2 storage";
    }
    return false;
  }
  *storage_root =
      (std::filesystem::path(user_data_dir) / "operit2").string();
  return true;
}

/// Builds Flutter storage path values for a resolved Linux root.
FlValue* linux_storage_paths(const std::string& storage_root) {
  const std::filesystem::path root(storage_root);
  FlValue* paths = fl_value_new_map();
  fl_value_set_string_take(
      paths, "storageRoot", fl_value_new_string(root.string().c_str()));
  fl_value_set_string_take(
      paths, "runtimeRoot",
      fl_value_new_string((root / "runtime").string().c_str()));
  fl_value_set_string_take(
      paths, "workspaceRoot",
      fl_value_new_string((root / "workspaces").string().c_str()));
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
      create_with_storage_root_ =
          reinterpret_cast<BridgeCreateWithStorageRoot>(
              dlsym(
                  library_,
                  "operit_flutter_bridge_create_with_storage_root"));
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
      if (create_ == nullptr || create_with_storage_root_ == nullptr ||
          destroy_ == nullptr || call_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          next_watch_channel_event_ == nullptr ||
          close_watch_stream_ == nullptr || free_string_ == nullptr) {
        AssignError(error, "operit flutter bridge exports are incomplete");
        return false;
      }
    }
    std::string storage_root;
    if (!resolve_linux_storage_root(
            configured_storage_root_, &storage_root, error)) {
      return false;
    }
    handle_ = create_with_storage_root_(storage_root.c_str());
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

  /// Sets the storage root used when the runtime handle is created.
  bool SetStorageRoot(const std::string& storage_root, std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (handle_ != nullptr) {
      AssignError(
          error,
          "Runtime storage root cannot change after runtime creation");
      return false;
    }
    std::string resolved;
    if (!resolve_linux_storage_root(storage_root, &resolved, error)) {
      return false;
    }
    configured_storage_root_ = std::move(resolved);
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
  std::string configured_storage_root_;
  std::mutex mutex_;
  BridgeCreate create_ = nullptr;
  BridgeCreateWithStorageRoot create_with_storage_root_ = nullptr;
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
  std::string response_text;
  std::string error;
  if (strcmp(method, "localRuntimeStoragePaths") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    const gchar* requested_root = nullptr;
    if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
      requested_root = string_map_value(args, "storageRoot");
    }
    if (requested_root == nullptr) {
      respond_error(
          method_call, "INVALID_ARGS",
          "localRuntimeStoragePaths expects storageRoot");
      return;
    }
    std::string storage_root;
    if (!resolve_linux_storage_root(
            requested_root, &storage_root, &error)) {
      respond_error(
          method_call, "RUNTIME_STORAGE_PATHS_ERROR", error);
      return;
    }
    g_autoptr(FlValue) result = linux_storage_paths(storage_root);
    respond_success_value(method_call, result);
    return;
  }
  if (strcmp(method, "setLocalRuntimeStorage") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    const gchar* storage_root = nullptr;
    if (args != nullptr && fl_value_get_type(args) == FL_VALUE_TYPE_MAP) {
      storage_root = string_map_value(args, "storageRoot");
    }
    if (storage_root == nullptr) {
      respond_error(
          method_call, "INVALID_ARGS",
          "setLocalRuntimeStorage expects storageRoot");
      return;
    }
    if (!g_operit_runtime_library->SetStorageRoot(storage_root, &error)) {
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
    const gchar* request = fl_value_get_string(args);
    if (g_operit_runtime_library->Call(request, &response_text, &error)) {
      respond_success(method_call, response_text);
    } else {
      respond_error(method_call, "RUNTIME_BRIDGE_ERROR", error);
    }
    return;
  }
  if (strcmp(method, "watchSnapshot") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "watchSnapshot expects a JSON string");
      return;
    }
    const gchar* request = fl_value_get_string(args);
    if (g_operit_runtime_library->WatchSnapshot(request, &response_text, &error)) {
      respond_success(method_call, response_text);
    } else {
      respond_error(method_call, "RUNTIME_BRIDGE_ERROR", error);
    }
    return;
  }
  if (strcmp(method, "watchStream") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "watchStream expects a JSON string");
      return;
    }
    const gchar* request = fl_value_get_string(args);
    if (g_operit_runtime_library->WatchStream(request, &response_text, &error)) {
      ensure_watch_channel_pump();
      respond_success(method_call, response_text);
    } else {
      respond_error(method_call, "RUNTIME_BRIDGE_ERROR", error);
    }
    return;
  }
  if (strcmp(method, "closeWatchStream") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "closeWatchStream expects a subscription id");
      return;
    }
    const gchar* subscription = fl_value_get_string(args);
    if (g_operit_runtime_library->CloseWatchStream(subscription, &response_text,
                                                  &error)) {
      respond_success(method_call, response_text);
    } else {
      respond_error(method_call, "RUNTIME_BRIDGE_ERROR", error);
    }
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

void register_operit_runtime_channel(FlView* view) {
  g_operit_runtime_library = std::make_shared<OperitRuntimeLibrary>();
  FlBinaryMessenger* messenger =
      fl_engine_get_binary_messenger(fl_view_get_engine(view));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_operit_runtime_channel = fl_method_channel_new(
      messenger, "operit/runtime", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      g_operit_runtime_channel, operit_runtime_method_call_cb, nullptr,
      nullptr);
}
