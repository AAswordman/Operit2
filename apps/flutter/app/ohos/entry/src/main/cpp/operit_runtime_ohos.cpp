#include <dlfcn.h>
#include <node_api.h>

#include <cstdint>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

namespace {

using BridgeHandle = void*;
using BridgeCreateWithStorageRootsAndSystemLanguage =
    BridgeHandle (*)(const char*, const char*, const char*);
using BridgeCreateError = char* (*)();
using BridgeDestroy = void (*)(BridgeHandle);

struct OperitByteBuffer {
  unsigned char* ptr;
  size_t len;
};

using BridgeCall = OperitByteBuffer (*)(BridgeHandle, const unsigned char*, size_t);
using BridgeNativeCall =
    OperitByteBuffer (*)(const void*, const unsigned char*, size_t);
using BridgePushClose = OperitByteBuffer (*)(BridgeHandle, const char*);
using BridgeNextWatchChannelEvent = OperitByteBuffer (*)(BridgeHandle);
using BridgeCloseWatchStream = OperitByteBuffer (*)(BridgeHandle, const char*);
using BridgeFreeBytes = void (*)(OperitByteBuffer);
using BridgeFreeString = void (*)(char*);
using BridgeStartWebAccessServer = char* (*)(
    BridgeHandle, const char*, const char*, const char*, const char*, const char*, const char*);
using BridgeDiscoverDevices = char* (*)(BridgeHandle, const char*);
using BridgeStopWebAccessServer = char* (*)(BridgeHandle);
using BridgeRemotePairStart = char* (*)(BridgeHandle, const char*, const char*, const char*);
using BridgeRemotePairFinish = char* (*)(BridgeHandle, const char*, const char*);
using BridgeEmitRuntimeEvent = char* (*)(BridgeHandle, const char*);

class OperitBridgeLibrary {
 public:
  /// Releases the loaded bridge library handle.
  ~OperitBridgeLibrary() {
    if (library_ != nullptr) {
      dlclose(library_);
      library_ = nullptr;
    }
  }

  /// Loads the Rust bridge library and resolves all exported symbols.
  bool EnsureReady(std::string* error) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (library_ != nullptr) {
      return true;
    }
    library_ = dlopen("liboperit_flutter_bridge.so", RTLD_NOW | RTLD_LOCAL);
    if (library_ == nullptr) {
      AssignError(error, dlerror());
      return false;
    }
    create_with_storage_roots_ =
        Load<BridgeCreateWithStorageRootsAndSystemLanguage>(
            "operit_flutter_bridge_create_with_storage_roots_and_system_language");
    create_error_ = Load<BridgeCreateError>("operit_flutter_bridge_create_error");
    destroy_ = Load<BridgeDestroy>("operit_flutter_bridge_destroy");
    native_call_ = Load<BridgeNativeCall>("operit_flutter_bridge_native_call");
    push_open_ = Load<BridgeCall>("operit_flutter_bridge_push_open");
    push_item_ = Load<BridgeCall>("operit_flutter_bridge_push_item");
    push_close_ = Load<BridgePushClose>("operit_flutter_bridge_push_close");
    watch_snapshot_ = Load<BridgeCall>("operit_flutter_bridge_watch_snapshot");
    watch_stream_ = Load<BridgeCall>("operit_flutter_bridge_watch_stream");
    next_watch_channel_event_ =
        Load<BridgeNextWatchChannelEvent>("operit_flutter_bridge_next_watch_channel_event");
    close_watch_stream_ = Load<BridgeCloseWatchStream>("operit_flutter_bridge_close_watch_stream");
    start_web_access_server_ =
        Load<BridgeStartWebAccessServer>("operit_flutter_bridge_start_web_access_server");
    discover_devices_ = Load<BridgeDiscoverDevices>("operit_flutter_bridge_discover_devices");
    stop_web_access_server_ =
        Load<BridgeStopWebAccessServer>("operit_flutter_bridge_stop_web_access_server");
    remote_pair_start_ = Load<BridgeRemotePairStart>("operit_flutter_bridge_remote_pair_start");
    remote_pair_finish_ = Load<BridgeRemotePairFinish>("operit_flutter_bridge_remote_pair_finish");
    emit_runtime_event_ = Load<BridgeEmitRuntimeEvent>("operit_flutter_bridge_emit_runtime_event");
    free_bytes_ = Load<BridgeFreeBytes>("operit_flutter_bridge_free_bytes");
    free_string_ = Load<BridgeFreeString>("operit_flutter_bridge_free_string");
    if (create_with_storage_roots_ == nullptr || create_error_ == nullptr || destroy_ == nullptr ||
        native_call_ == nullptr || push_open_ == nullptr || push_item_ == nullptr ||
        push_close_ == nullptr || watch_snapshot_ == nullptr || watch_stream_ == nullptr ||
        next_watch_channel_event_ == nullptr || close_watch_stream_ == nullptr ||
        start_web_access_server_ == nullptr || discover_devices_ == nullptr ||
        stop_web_access_server_ == nullptr || remote_pair_start_ == nullptr ||
        remote_pair_finish_ == nullptr || emit_runtime_event_ == nullptr ||
        free_bytes_ == nullptr || free_string_ == nullptr) {
      AssignError(error, "operit flutter bridge exports are incomplete");
      return false;
    }
    return true;
  }

  /// Creates one Rust runtime bridge handle.
  BridgeHandle Create(const std::string& runtime_root, const std::string& workspace_root,
                      const std::string& system_language_code) {
    return create_with_storage_roots_(runtime_root.c_str(), workspace_root.c_str(),
                                      system_language_code.c_str());
  }

  /// Returns the last Rust runtime bridge creation error.
  std::string CreateError() { return TakeString(create_error_()); }

  /// Destroys one Rust runtime bridge handle.
  void Destroy(BridgeHandle handle) { destroy_(handle); }

  /// Dispatches a Core Link call through the Rust bridge.
  OperitByteBuffer Call(BridgeHandle handle, const unsigned char* data, size_t len) {
    return native_call_(handle, data, len);
  }

  /// Opens a local Link push stream through the Rust bridge.
  OperitByteBuffer PushOpen(BridgeHandle handle, const unsigned char* data, size_t len) {
    return push_open_(handle, data, len);
  }

  /// Sends one local Link push item through the Rust bridge.
  OperitByteBuffer PushItem(BridgeHandle handle, const unsigned char* data, size_t len) {
    return push_item_(handle, data, len);
  }

  /// Closes one local Link push stream through the Rust bridge.
  OperitByteBuffer PushClose(BridgeHandle handle, const std::string& push_id) {
    return push_close_(handle, push_id.c_str());
  }

  /// Reads a watch snapshot through the Rust bridge.
  OperitByteBuffer WatchSnapshot(BridgeHandle handle, const unsigned char* data, size_t len) {
    return watch_snapshot_(handle, data, len);
  }

  /// Opens a watch stream through the Rust bridge.
  OperitByteBuffer WatchStream(BridgeHandle handle, const unsigned char* data, size_t len) {
    return watch_stream_(handle, data, len);
  }

  /// Blocks until the Rust bridge produces one watch-channel event.
  OperitByteBuffer NextWatchChannelEvent(BridgeHandle handle) {
    return next_watch_channel_event_(handle);
  }

  /// Closes a watch stream through the Rust bridge.
  OperitByteBuffer CloseWatchStream(BridgeHandle handle, const std::string& subscription_id) {
    return close_watch_stream_(handle, subscription_id.c_str());
  }

  /// Starts the Rust Web Access server.
  std::string StartWebAccessServer(BridgeHandle handle,
                                   const std::string& bind_host,
                                   const std::string& requested_port,
                                   const std::string& auth_token,
                                   const std::string& server_name,
                                   const std::string& advertise,
                                   const std::string& pairing_code_path) {
    return TakeString(start_web_access_server_(handle,
                                               bind_host.c_str(),
                                               requested_port.c_str(),
                                               auth_token.c_str(),
                                               server_name.c_str(),
                                               advertise.c_str(),
                                               pairing_code_path.c_str()));
  }

  /// Discovers nearby Operit devices through the Rust bridge.
  std::string DiscoverDevices(BridgeHandle handle, const std::string& timeout_ms) {
    return TakeString(discover_devices_(handle, timeout_ms.c_str()));
  }

  /// Stops the Rust Web Access server.
  std::string StopWebAccessServer(BridgeHandle handle) {
    return TakeString(stop_web_access_server_(handle));
  }

  /// Starts one remote pairing request through the Rust bridge.
  std::string RemotePairStart(BridgeHandle handle,
                              const std::string& base_url,
                              const std::string& token_hash,
                              const std::string& client_device_info) {
    return TakeString(
        remote_pair_start_(handle, base_url.c_str(), token_hash.c_str(), client_device_info.c_str()));
  }

  /// Finishes one remote pairing request through the Rust bridge.
  std::string RemotePairFinish(BridgeHandle handle,
                               const std::string& pairing_id,
                               const std::string& pairing_code) {
    return TakeString(remote_pair_finish_(handle, pairing_id.c_str(), pairing_code.c_str()));
  }

  /// Delivers one normalized OpenHarmony event through the Rust bridge.
  std::string EmitRuntimeEvent(BridgeHandle handle, const std::string& event_json) {
    return TakeString(emit_runtime_event_(handle, event_json.c_str()));
  }

  /// Frees an owned Rust byte buffer.
  void FreeBytes(OperitByteBuffer buffer) { free_bytes_(buffer); }

 private:
  /// Resolves one bridge symbol from the loaded library.
  template <typename T>
  T Load(const char* name) {
    return reinterpret_cast<T>(dlsym(library_, name));
  }

  /// Assigns a C++ error string.
  static void AssignError(std::string* error, const char* message) {
    if (error != nullptr) {
      *error = message == nullptr ? "" : message;
    }
  }

  /// Copies and frees one Rust-owned C string.
  std::string TakeString(char* raw) {
    if (raw == nullptr) {
      return std::string();
    }
    std::string value(raw);
    free_string_(raw);
    return value;
  }

  std::mutex mutex_;
  void* library_ = nullptr;
  BridgeCreateWithStorageRootsAndSystemLanguage create_with_storage_roots_ = nullptr;
  BridgeCreateError create_error_ = nullptr;
  BridgeDestroy destroy_ = nullptr;
  BridgeNativeCall native_call_ = nullptr;
  BridgeCall push_open_ = nullptr;
  BridgeCall push_item_ = nullptr;
  BridgePushClose push_close_ = nullptr;
  BridgeCall watch_snapshot_ = nullptr;
  BridgeCall watch_stream_ = nullptr;
  BridgeNextWatchChannelEvent next_watch_channel_event_ = nullptr;
  BridgeCloseWatchStream close_watch_stream_ = nullptr;
  BridgeStartWebAccessServer start_web_access_server_ = nullptr;
  BridgeDiscoverDevices discover_devices_ = nullptr;
  BridgeStopWebAccessServer stop_web_access_server_ = nullptr;
  BridgeRemotePairStart remote_pair_start_ = nullptr;
  BridgeRemotePairFinish remote_pair_finish_ = nullptr;
  BridgeEmitRuntimeEvent emit_runtime_event_ = nullptr;
  BridgeFreeBytes free_bytes_ = nullptr;
  BridgeFreeString free_string_ = nullptr;
};

OperitBridgeLibrary g_bridge_library;

/// Throws one JavaScript error.
napi_value ThrowError(napi_env env, const std::string& message) {
  napi_throw_error(env, "OPERIT_RUNTIME_NATIVE", message.c_str());
  return nullptr;
}

/// Ensures the Rust bridge library is loaded for one native call.
bool EnsureBridgeReady(napi_env env) {
  std::string error;
  if (g_bridge_library.EnsureReady(&error)) {
    return true;
  }
  ThrowError(env, error);
  return false;
}

/// Reads N-API callback arguments.
std::vector<napi_value> CallbackArgs(napi_env env, napi_callback_info info, size_t count) {
  std::vector<napi_value> args(count);
  size_t argc = count;
  napi_get_cb_info(env, info, &argc, args.data(), nullptr, nullptr);
  if (argc != count) {
    napi_throw_error(env, "OPERIT_RUNTIME_NATIVE", "invalid native argument count");
    return {};
  }
  return args;
}

/// Reads one UTF-8 JavaScript string.
std::string ReadString(napi_env env, napi_value value) {
  size_t length = 0;
  napi_get_value_string_utf8(env, value, nullptr, 0, &length);
  std::vector<char> buffer(length + 1);
  napi_get_value_string_utf8(env, value, buffer.data(), buffer.size(), &length);
  return std::string(buffer.data(), length);
}

/// Reads one Rust bridge handle from a JavaScript BigInt.
BridgeHandle ReadHandle(napi_env env, napi_value value) {
  uint64_t raw = 0;
  bool lossless = false;
  napi_get_value_bigint_uint64(env, value, &raw, &lossless);
  if (!lossless || raw == 0) {
    napi_throw_error(env, "OPERIT_RUNTIME_NATIVE", "runtime handle is invalid");
    return nullptr;
  }
  return reinterpret_cast<BridgeHandle>(raw);
}

/// Reads one JavaScript ArrayBuffer into a native byte view.
bool ReadArrayBuffer(napi_env env, napi_value value, unsigned char** data, size_t* len) {
  void* raw = nullptr;
  napi_status status = napi_get_arraybuffer_info(env, value, &raw, len);
  if (status != napi_ok || raw == nullptr) {
    napi_throw_error(env, "OPERIT_RUNTIME_NATIVE", "expected ArrayBuffer bytes");
    return false;
  }
  *data = static_cast<unsigned char*>(raw);
  return true;
}

/// Frees one Rust byte buffer after the JavaScript ArrayBuffer is collected.
void FinalizeRustBytes(napi_env, void*, void* hint) {
  auto* buffer = static_cast<OperitByteBuffer*>(hint);
  g_bridge_library.FreeBytes(*buffer);
  delete buffer;
}

/// Converts an owned Rust byte buffer into a JavaScript ArrayBuffer.
napi_value OwnedBytes(napi_env env, OperitByteBuffer value) {
  napi_value result = nullptr;
  if (value.ptr == nullptr || value.len == 0) {
    void* data = nullptr;
    napi_create_arraybuffer(env, 0, &data, &result);
    return result;
  }
  auto* buffer = new OperitByteBuffer{value.ptr, value.len};
  napi_create_external_arraybuffer(env, value.ptr, value.len, FinalizeRustBytes, buffer, &result);
  return result;
}

/// Converts a C++ string into a JavaScript string.
napi_value StringValue(napi_env env, const std::string& value) {
  napi_value result = nullptr;
  napi_create_string_utf8(env, value.c_str(), value.size(), &result);
  return result;
}

/// Creates one Rust runtime bridge handle.
napi_value Create(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 3);
  if (args.size() != 3) {
    return ThrowError(env, "create expects runtime root, workspace root, and system language code");
  }
  auto runtime_root = ReadString(env, args[0]);
  auto workspace_root = ReadString(env, args[1]);
  auto system_language_code = ReadString(env, args[2]);
  BridgeHandle handle =
      g_bridge_library.Create(runtime_root, workspace_root, system_language_code);
  if (handle == nullptr) {
    return ThrowError(env, g_bridge_library.CreateError());
  }
  napi_value result = nullptr;
  napi_create_bigint_uint64(env, reinterpret_cast<uint64_t>(handle), &result);
  return result;
}

/// Destroys one Rust runtime bridge handle.
napi_value Destroy(napi_env env, napi_callback_info info) {
  auto args = CallbackArgs(env, info, 1);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle != nullptr) {
    g_bridge_library.Destroy(handle);
  }
  napi_value result = nullptr;
  napi_get_undefined(env, &result);
  return result;
}

/// Calls one Rust Core Link byte endpoint.
napi_value CallBytes(napi_env env,
                     napi_callback_info info,
                     OperitByteBuffer (OperitBridgeLibrary::*method)(BridgeHandle,
                                                                     const unsigned char*,
                                                                     size_t)) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 2);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  unsigned char* data = nullptr;
  size_t len = 0;
  if (handle == nullptr || !ReadArrayBuffer(env, args[1], &data, &len)) {
    return nullptr;
  }
  return OwnedBytes(env, (g_bridge_library.*method)(handle, data, len));
}

/// Calls the Rust call endpoint.
napi_value Call(napi_env env, napi_callback_info info) {
  return CallBytes(env, info, &OperitBridgeLibrary::Call);
}

/// Calls the Rust push-open endpoint.
napi_value PushOpen(napi_env env, napi_callback_info info) {
  return CallBytes(env, info, &OperitBridgeLibrary::PushOpen);
}

/// Calls the Rust push-item endpoint.
napi_value PushItem(napi_env env, napi_callback_info info) {
  return CallBytes(env, info, &OperitBridgeLibrary::PushItem);
}

/// Calls the Rust watch-snapshot endpoint.
napi_value WatchSnapshot(napi_env env, napi_callback_info info) {
  return CallBytes(env, info, &OperitBridgeLibrary::WatchSnapshot);
}

/// Calls the Rust watch-stream endpoint.
napi_value WatchStream(napi_env env, napi_callback_info info) {
  return CallBytes(env, info, &OperitBridgeLibrary::WatchStream);
}

/// Calls the Rust push-close endpoint.
napi_value PushClose(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 2);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return OwnedBytes(env, g_bridge_library.PushClose(handle, ReadString(env, args[1])));
}

/// Reads one Rust watch-channel event.
napi_value NextWatchChannelEvent(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 1);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return OwnedBytes(env, g_bridge_library.NextWatchChannelEvent(handle));
}

/// Calls the Rust close-watch-stream endpoint.
napi_value CloseWatchStream(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 2);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return OwnedBytes(env, g_bridge_library.CloseWatchStream(handle, ReadString(env, args[1])));
}

/// Starts the Rust Web Access server.
napi_value StartWebAccessServer(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 7);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return StringValue(env, g_bridge_library.StartWebAccessServer(handle,
                                                                ReadString(env, args[1]),
                                                                ReadString(env, args[2]),
                                                                ReadString(env, args[3]),
                                                                ReadString(env, args[4]),
                                                                ReadString(env, args[5]),
                                                                ReadString(env, args[6])));
}

/// Discovers nearby Operit devices through Rust.
napi_value DiscoverDevices(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 2);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return StringValue(env, g_bridge_library.DiscoverDevices(handle, ReadString(env, args[1])));
}

/// Stops the Rust Web Access server.
napi_value StopWebAccessServer(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 1);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return StringValue(env, g_bridge_library.StopWebAccessServer(handle));
}

/// Starts one remote pairing request through Rust.
napi_value RemotePairStart(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 4);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return StringValue(env, g_bridge_library.RemotePairStart(handle,
                                                           ReadString(env, args[1]),
                                                           ReadString(env, args[2]),
                                                           ReadString(env, args[3])));
}

/// Finishes one remote pairing request through Rust.
napi_value RemotePairFinish(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 3);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return StringValue(env, g_bridge_library.RemotePairFinish(handle,
                                                            ReadString(env, args[1]),
                                                            ReadString(env, args[2])));
}

/// Delivers one normalized OpenHarmony event through Rust.
napi_value EmitRuntimeEvent(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 2);
  if (args.empty()) {
    return nullptr;
  }
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) {
    return nullptr;
  }
  return StringValue(env, g_bridge_library.EmitRuntimeEvent(handle, ReadString(env, args[1])));
}

/// Registers one named N-API function.
void DefineFunction(napi_env env,
                    napi_value exports,
                    const char* name,
                    napi_callback callback,
                    napi_property_descriptor* descriptor) {
  *descriptor = {name, nullptr, callback, nullptr, nullptr, nullptr, napi_default, nullptr};
}

/// Initializes the OpenHarmony native runtime module.
napi_value Init(napi_env env, napi_value exports) {
  napi_property_descriptor descriptors[16];
  DefineFunction(env, exports, "create", Create, &descriptors[0]);
  DefineFunction(env, exports, "destroy", Destroy, &descriptors[1]);
  DefineFunction(env, exports, "call", Call, &descriptors[2]);
  DefineFunction(env, exports, "pushOpen", PushOpen, &descriptors[3]);
  DefineFunction(env, exports, "pushItem", PushItem, &descriptors[4]);
  DefineFunction(env, exports, "pushClose", PushClose, &descriptors[5]);
  DefineFunction(env, exports, "watchSnapshot", WatchSnapshot, &descriptors[6]);
  DefineFunction(env, exports, "watchStream", WatchStream, &descriptors[7]);
  DefineFunction(env, exports, "nextWatchChannelEvent", NextWatchChannelEvent, &descriptors[8]);
  DefineFunction(env, exports, "closeWatchStream", CloseWatchStream, &descriptors[9]);
  DefineFunction(env, exports, "startWebAccessServer", StartWebAccessServer, &descriptors[10]);
  DefineFunction(env, exports, "discoverDevices", DiscoverDevices, &descriptors[11]);
  DefineFunction(env, exports, "stopWebAccessServer", StopWebAccessServer, &descriptors[12]);
  DefineFunction(env, exports, "remotePairStart", RemotePairStart, &descriptors[13]);
  DefineFunction(env, exports, "remotePairFinish", RemotePairFinish, &descriptors[14]);
  DefineFunction(env, exports, "emitRuntimeEvent", EmitRuntimeEvent, &descriptors[15]);
  napi_define_properties(env, exports, 16, descriptors);
  return exports;
}

}  // namespace

NAPI_MODULE(operit_runtime_ohos, Init)
