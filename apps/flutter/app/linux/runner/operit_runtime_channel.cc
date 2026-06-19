#include "operit_runtime_channel.h"

#include <dlfcn.h>
#include <stdint.h>
#include <string.h>

#include <gio/gio.h>

#include <chrono>
#include <condition_variable>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <mutex>
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
using BridgePollWatchStreams = char* (*)(BridgeHandle, const char*);
using BridgeCloseWatchStream = char* (*)(BridgeHandle, const char*);
using BridgeDispatchHostEvent = char* (*)(BridgeHandle, const char*, const char*);
using BridgeFreeString = void (*)(char*);

FlMethodChannel* g_operit_runtime_channel = nullptr;

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
      create_ =
          reinterpret_cast<BridgeCreate>(
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
      poll_watch_streams_ = reinterpret_cast<BridgePollWatchStreams>(
          dlsym(library_, "operit_flutter_bridge_poll_watch_streams"));
      close_watch_stream_ = reinterpret_cast<BridgeCloseWatchStream>(
          dlsym(library_, "operit_flutter_bridge_close_watch_stream"));
      dispatch_host_event_ = reinterpret_cast<BridgeDispatchHostEvent>(
          dlsym(library_, "operit_flutter_bridge_dispatch_host_event"));
      free_string_ = reinterpret_cast<BridgeFreeString>(
          dlsym(library_, "operit_flutter_bridge_free_string"));
      if (create_ == nullptr ||
          destroy_ == nullptr || call_ == nullptr ||
          watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
          poll_watch_stream_ == nullptr || poll_watch_streams_ == nullptr ||
          close_watch_stream_ == nullptr ||
          dispatch_host_event_ == nullptr ||
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

  bool PollWatchStreams(const std::string& subscriptions, std::string* response,
                        std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response = poll_watch_streams_(handle_, subscriptions.c_str());
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

  bool DispatchHostEvent(const std::string& source,
                         const std::string& payload,
                         std::string* response,
                         std::string* error) {
    if (!EnsureReady(error)) {
      return false;
    }
    char* raw_response =
        dispatch_host_event_(handle_, source.c_str(), payload.c_str());
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
  BridgePollWatchStreams poll_watch_streams_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeDispatchHostEvent dispatch_host_event_ = nullptr;
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
  if (strcmp(method, "pollWatchStreams") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_STRING) {
      respond_error(method_call, "INVALID_ARGS",
                    "pollWatchStreams expects a JSON string array");
      return;
    }
    const gchar* subscriptions = fl_value_get_string(args);
    if (g_operit_runtime_library->PollWatchStreams(
            subscriptions, &response_text, &error)) {
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
  if (strcmp(method, "dispatchHostEvent") == 0) {
    FlValue* args = fl_method_call_get_args(method_call);
    if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_MAP) {
      respond_error(method_call, "INVALID_ARGS",
                    "dispatchHostEvent expects source and payload");
      return;
    }
    const gchar* source = string_map_value(args, "source");
    const gchar* payload = string_map_value(args, "payload");
    if (source == nullptr || payload == nullptr) {
      respond_error(method_call, "INVALID_ARGS",
                    "dispatchHostEvent expects source and payload");
      return;
    }
    if (g_operit_runtime_library->DispatchHostEvent(
            source, payload, &response_text, &error)) {
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

// ── Linux D-Bus event monitor for ToolPkg host event hooks ──────────────

class LinuxEventMonitor {
 public:
  LinuxEventMonitor() = default;
  ~LinuxEventMonitor() { Stop(); }

  void Start(std::shared_ptr<class OperitRuntimeLibrary> library) {
    Stop();
    library_ = std::move(library);
    SetupDbus();
  }

  void Stop() {
    if (connection_) {
      if (login1_signal_id_) {
        g_dbus_connection_signal_unsubscribe(connection_, login1_signal_id_);
        login1_signal_id_ = 0;
      }
      if (nm_signal_id_) {
        g_dbus_connection_signal_unsubscribe(connection_, nm_signal_id_);
        nm_signal_id_ = 0;
      }
      if (bluez_prop_id_) {
        g_dbus_connection_signal_unsubscribe(connection_, bluez_prop_id_);
        bluez_prop_id_ = 0;
      }
      if (bluez_int_added_id_) {
        g_dbus_connection_signal_unsubscribe(connection_, bluez_int_added_id_);
        bluez_int_added_id_ = 0;
      }
      g_object_unref(connection_);
      connection_ = nullptr;
    }
    library_.reset();
  }

 private:
  std::shared_ptr<class OperitRuntimeLibrary> library_;
  GDBusConnection* connection_ = nullptr;
  guint login1_signal_id_ = 0;
  guint nm_signal_id_ = 0;
  guint bluez_prop_id_ = 0;
  guint bluez_int_added_id_ = 0;

  void SetupDbus() {
    GError* error = nullptr;
    connection_ = g_bus_get_sync(G_BUS_TYPE_SYSTEM, nullptr, &error);
    if (error != nullptr) {
      g_warning("operit: failed to connect to D-Bus system bus: %s", error->message);
      g_error_free(error);
      return;
    }

    login1_signal_id_ = g_dbus_connection_signal_subscribe(
        connection_,
        "org.freedesktop.login1",
        "org.freedesktop.login1.Manager",
        "PrepareForSleep",
        "/org/freedesktop/login1",
        nullptr,
        G_DBUS_SIGNAL_FLAGS_NONE,
        OnDbusSignal,
        this,
        nullptr);

    nm_signal_id_ = g_dbus_connection_signal_subscribe(
        connection_,
        "org.freedesktop.NetworkManager",
        "org.freedesktop.NetworkManager",
        "StateChanged",
        "/org/freedesktop/NetworkManager",
        nullptr,
        G_DBUS_SIGNAL_FLAGS_NONE,
        OnNetworkManagerSignal,
        this,
        nullptr);

    bluez_prop_id_ = g_dbus_connection_signal_subscribe(
        connection_,
        "org.bluez",
        "org.freedesktop.DBus.Properties",
        "PropertiesChanged",
        nullptr,
        "org.bluez.Device1",
        G_DBUS_SIGNAL_FLAGS_NONE,
        OnBluezPropertiesChanged,
        this,
        nullptr);

    bluez_int_added_id_ = g_dbus_connection_signal_subscribe(
        connection_,
        "org.bluez",
        "org.freedesktop.DBus.ObjectManager",
        "InterfacesAdded",
        "/",
        nullptr,
        G_DBUS_SIGNAL_FLAGS_NONE,
        OnBluezInterfacesAdded,
        this,
        nullptr);
  }

  static void OnDbusSignal(GDBusConnection* connection,
                            const gchar* sender_name,
                            const gchar* object_path,
                            const gchar* interface_name,
                            const gchar* signal_name,
                            GVariant* parameters,
                            gpointer user_data) {
    auto* self = static_cast<LinuxEventMonitor*>(user_data);
    if (g_strcmp0(signal_name, "PrepareForSleep") == 0) {
      gboolean preparing = FALSE;
      g_variant_get(parameters, "(b)", &preparing);
      self->DispatchTopic(
          preparing ? "system.power.sleep" : "system.power.wake",
          R"({"preparingForSleep":)" + std::string(preparing ? "true" : "false") + "}");
    }
  }

  static void OnNetworkManagerSignal(GDBusConnection* connection,
                                      const gchar* sender_name,
                                      const gchar* object_path,
                                      const gchar* interface_name,
                                      const gchar* signal_name,
                                      GVariant* parameters,
                                      gpointer user_data) {
    auto* self = static_cast<LinuxEventMonitor*>(user_data);
    guint32 state = 0;
    g_variant_get(parameters, "(u)", &state);
    self->DispatchTopic(
        "system.network.changed",
        R"({"state":)" + std::to_string(state) + "}");
  }

  static void OnBluezPropertiesChanged(GDBusConnection* connection,
                                        const gchar* sender_name,
                                        const gchar* object_path,
                                        const gchar* interface_name,
                                        const gchar* signal_name,
                                        GVariant* parameters,
                                        gpointer user_data) {
    auto* self = static_cast<LinuxEventMonitor*>(user_data);
    const gchar* iface = nullptr;
    GVariant* changed = nullptr;
    g_variant_get(parameters, "(sa{sv}as)", &iface, &changed, nullptr);
    if (g_strcmp0(iface, "org.bluez.Device1") != 0 || changed == nullptr) {
      if (changed) g_variant_unref(changed);
      return;
    }

    // Check for Connected property
    GVariant* connected_var = g_variant_lookup_value(changed, "Connected", G_VARIANT_TYPE_BOOLEAN);
    if (connected_var) {
      gboolean connected = g_variant_get_boolean(connected_var);
      std::string action = connected ? "device.connected" : "device.disconnected";
      // Extract device name from object path
      const char* name_start = g_strrstr(object_path, "dev_");
      std::string device_address = name_start ? (name_start + 4) : object_path;
      self->DispatchTopic(
          std::string("bluetooth.") + action,
          R"({"deviceAddress":")" + device_address + R"("})");
      g_variant_unref(connected_var);
    }
    g_variant_unref(changed);
  }

  static void OnBluezInterfacesAdded(GDBusConnection* connection,
                                      const gchar* sender_name,
                                      const gchar* object_path,
                                      const gchar* interface_name,
                                      const gchar* signal_name,
                                      GVariant* parameters,
                                      gpointer user_data) {
    auto* self = static_cast<LinuxEventMonitor*>(user_data);
    GVariant* interfaces = nullptr;
    g_variant_get(parameters, "(oa{sa{sv}})", nullptr, &interfaces);
    if (interfaces == nullptr) return;

    // Check if org.bluez.Device1 is in the added interfaces
    GVariantIter iter;
    const gchar* iface_name = nullptr;
    GVariant* iface_props = nullptr;
    g_variant_iter_init(&iter, interfaces);
    while (g_variant_iter_loop(&iter, "{s@a{sv}}", &iface_name, &iface_props)) {
      if (g_strcmp0(iface_name, "org.bluez.Device1") == 0) {
        const char* name_start = g_strrstr(object_path, "dev_");
        std::string device_address = name_start ? (name_start + 4) : object_path;
        self->DispatchTopic(
            "bluetooth.device.found",
            R"({"deviceAddress":")" + device_address + R"("})");
        break;
      }
    }
    g_variant_unref(interfaces);
  }

  void DispatchEvent(const std::string& source, const std::string& payload) {
    if (!library_) return;
    std::string response, error;
    library_->DispatchHostEvent(source, payload, &response, &error);
  }

  void DispatchTopic(const std::string& topic, const std::string& data_json) {
    DispatchEvent("broadcast", R"({"topic":")" + topic +
        R"(","platform":"linux","data":)" + data_json +
        R"(,"receivedAtMillis":)" + std::to_string(CurrentTimeMillis()) + "}");
  }

  static int64_t CurrentTimeMillis() {
    return static_cast<int64_t>(
        std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::system_clock::now().time_since_epoch()).count());
  }
};

std::shared_ptr<LinuxEventMonitor> g_linux_event_monitor;

}  // namespace

void register_operit_runtime_channel(FlView* view) {
  g_operit_runtime_library = std::make_shared<OperitRuntimeLibrary>();
  g_linux_event_monitor = std::make_shared<LinuxEventMonitor>();
  g_linux_event_monitor->Start(g_operit_runtime_library);
  FlBinaryMessenger* messenger =
      fl_engine_get_binary_messenger(fl_view_get_engine(view));
  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  g_operit_runtime_channel = fl_method_channel_new(
      messenger, "operit/runtime", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(
      g_operit_runtime_channel, operit_runtime_method_call_cb, nullptr,
      nullptr);
}
