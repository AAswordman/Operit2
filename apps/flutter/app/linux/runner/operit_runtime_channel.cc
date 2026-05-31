#include "operit_runtime_channel.h"

#include <dlfcn.h>
#include <string.h>

#include <memory>
#include <string>

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
      dlclose(library_);
      library_ = nullptr;
    }
  }

  bool EnsureReady(std::string* error) {
    if (handle_ != nullptr) {
      return true;
    }
    if (library_ == nullptr) {
      library_ = dlopen("liboperit_flutter_bridge.so", RTLD_NOW | RTLD_LOCAL);
      if (library_ == nullptr) {
        AssignError(error, dlerror());
        return false;
      }
      create_ = reinterpret_cast<BridgeCreate>(
          dlsym(library_, "operit_flutter_bridge_create"));
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
      poll_watch_stream_ = reinterpret_cast<BridgePollWatchStream>(
          dlsym(library_, "operit_flutter_bridge_poll_watch_stream"));
      close_watch_stream_ = reinterpret_cast<BridgeCloseWatchStream>(
          dlsym(library_, "operit_flutter_bridge_close_watch_stream"));
      host_descriptor_ = reinterpret_cast<BridgeHostDescriptor>(
          dlsym(library_, "operit_flutter_bridge_host_descriptor"));
      free_string_ = reinterpret_cast<BridgeFreeString>(
          dlsym(library_, "operit_flutter_bridge_free_string"));
      if (create_ == nullptr || destroy_ == nullptr || call_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          poll_watch_stream_ == nullptr || close_watch_stream_ == nullptr ||
          host_descriptor_ == nullptr || free_string_ == nullptr) {
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

  bool PollWatchStream(const std::string& subscription, std::string* response,
                       std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = poll_watch_stream_(handle_, subscription.c_str());
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

  bool HostDescriptor(std::string* response, std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = host_descriptor_(handle_);
    return TakeBridgeString(raw_response, response, error);
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
  BridgeCreate create_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
  BridgeCall call_ = nullptr;
  BridgeWatchSnapshot watch_snapshot_ = nullptr;
  BridgeWatchStream watch_stream_ = nullptr;
  BridgePollWatchStream poll_watch_stream_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeHostDescriptor host_descriptor_ = nullptr;
  BridgeFreeString free_string_ = nullptr;
};

std::shared_ptr<OperitRuntimeLibrary> g_operit_runtime_library;
FlMethodChannel* g_operit_runtime_channel = nullptr;

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

void operit_runtime_method_call_cb(FlMethodChannel* channel,
                                   FlMethodCall* method_call,
                                   gpointer user_data) {
  (void)channel;
  (void)user_data;
  const gchar* method = fl_method_call_get_name(method_call);
  std::string response_text;
  std::string error;
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
      respond_success(method_call, response_text);
    } else {
      respond_error(method_call, "RUNTIME_BRIDGE_ERROR", error);
    }
    return;
  }
  if (strcmp(method, "pollWatchStream") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "pollWatchStream expects a subscription id");
      return;
    }
    const gchar* subscription = fl_value_get_string(args);
    if (g_operit_runtime_library->PollWatchStream(subscription, &response_text,
                                                 &error)) {
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
  if (strcmp(method, "hostDescriptor") == 0) {
    if (g_operit_runtime_library->HostDescriptor(&response_text, &error)) {
      respond_success(method_call, response_text);
    } else {
      respond_error(method_call, "RUNTIME_BRIDGE_ERROR", error);
    }
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
