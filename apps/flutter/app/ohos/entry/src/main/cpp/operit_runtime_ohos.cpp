#include <dlfcn.h>
#include <node_api.h>

#include <cstdint>
#include <cstring>
#include <exception>
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

using BridgeFreeString = void (*)(char*);
using BridgeEmitRuntimeEvent = char* (*)(BridgeHandle, const char*);
using BridgeRuntimeBootstrapRead = char* (*)(const char*);
using BridgeRuntimeBootstrapWrite = char* (*)(const char*, const char*);
using BridgeRetain = BridgeHandle (*)(BridgeHandle);
using BridgeRenderPdf = char* (*)(BridgeHandle, const char*);
using BridgeDataRescue = char* (*)(const char*, const char*);

class OperitBridgeLibrary {
 public:
  /// Keeps the bridge module mapped for retained FFI connections and VM finalizers.
  ~OperitBridgeLibrary() {
    // The module stays mapped for Dart FFI callbacks and native finalizers.
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
    if (Load<char* (*)(BridgeHandle)>("operit_flutter_bridge_ffi_connect") == nullptr) {
      AssignError(error, "operit flutter FFI connect export is missing");
      return false;
    }
    create_error_ = Load<BridgeCreateError>("operit_flutter_bridge_create_error");
    destroy_ = Load<BridgeDestroy>("operit_flutter_bridge_destroy");
    emit_runtime_event_ = Load<BridgeEmitRuntimeEvent>("operit_flutter_bridge_emit_runtime_event");
    runtime_bootstrap_read_ =
        Load<BridgeRuntimeBootstrapRead>("operit_flutter_bridge_runtime_bootstrap_read");
    runtime_bootstrap_write_ =
        Load<BridgeRuntimeBootstrapWrite>("operit_flutter_bridge_runtime_bootstrap_write");

    retain_ = Load<BridgeRetain>("operit_flutter_bridge_ohos_retain");
    render_pdf_ = Load<BridgeRenderPdf>("operit_flutter_bridge_ohos_render_pdf");
    free_string_ = Load<BridgeFreeString>("operit_flutter_bridge_free_string");
    if (create_with_storage_roots_ == nullptr || create_error_ == nullptr ||
        destroy_ == nullptr || free_string_ == nullptr ||
        emit_runtime_event_ == nullptr || runtime_bootstrap_read_ == nullptr ||
        runtime_bootstrap_write_ == nullptr || retain_ == nullptr || render_pdf_ == nullptr) {
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

  /// Creates one retained FFI connection using the host-loaded Rust library.
  std::string ConnectCoreFfi(BridgeHandle handle) {
    const auto connect = Load<char* (*)(BridgeHandle)>("operit_flutter_bridge_ffi_connect");
    return TakeString(connect(handle));
  }



  /// Delivers one normalized OpenHarmony event through the Rust bridge.
  std::string EmitRuntimeEvent(BridgeHandle handle, const std::string& event_json) {
    return TakeString(emit_runtime_event_(handle, event_json.c_str()));
  }

  /// Calls maintenance without creating a Core handle.
  std::string DataRescue(const std::string& root, const std::string& request) {
    const auto call = Load<BridgeDataRescue>("operit_flutter_bridge_data_rescue");
    if (call == nullptr) return "{\"ok\":false,\"error\":\"Data rescue ABI is unavailable\"}";
    return TakeString(call(root.c_str(), request.c_str()));
  }

  /// Retains one bridge reference before dispatching an asynchronous owner operation.
  BridgeHandle Retain(BridgeHandle handle) { return retain_(handle); }

  /// Renders a PDF through the OHOS runtime terminal host.
  std::string RenderPdf(BridgeHandle handle, const std::string& request) {
    return TakeString(render_pdf_(handle, request.c_str()));
  }

  /// Reads the client bootstrap record without creating a Core handle.
  std::string RuntimeBootstrapRead(const std::string& default_runtime_root) {
    return TakeString(runtime_bootstrap_read_(default_runtime_root.c_str()));
  }

  /// Writes the client bootstrap record without creating a Core handle.
  std::string RuntimeBootstrapWrite(const std::string& default_runtime_root,
                                    const std::string& content) {
    return TakeString(
        runtime_bootstrap_write_(default_runtime_root.c_str(), content.c_str()));
  }


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
  BridgeEmitRuntimeEvent emit_runtime_event_ = nullptr;
  BridgeRuntimeBootstrapRead runtime_bootstrap_read_ = nullptr;
  BridgeRuntimeBootstrapWrite runtime_bootstrap_write_ = nullptr;
  BridgeRetain retain_ = nullptr;
  BridgeRenderPdf render_pdf_ = nullptr;
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

/// Starts the Rust Web Access server.

/// Stops the Rust Web Access server.

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

/// Reads the client bootstrap record through the Rust startup Host.
napi_value RuntimeBootstrapRead(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 1);
  if (args.empty()) {
    return nullptr;
  }
  return StringValue(env, g_bridge_library.RuntimeBootstrapRead(ReadString(env, args[0])));
}

/// Writes the client bootstrap record through the Rust startup Host.
napi_value RuntimeBootstrapWrite(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) {
    return nullptr;
  }
  auto args = CallbackArgs(env, info, 2);
  if (args.empty()) {
    return nullptr;
  }
  return StringValue(
      env,
      g_bridge_library.RuntimeBootstrapWrite(
          ReadString(env, args[0]), ReadString(env, args[1])));
}

/// Returns the versioned Rust FFI function table for this host runtime.
napi_value ConnectCoreFfi(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) return nullptr;
  auto args = CallbackArgs(env, info, 1);
  if (args.empty()) return nullptr;
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) return nullptr;
  return StringValue(env, g_bridge_library.ConnectCoreFfi(handle));
}

enum class OwnerOperation { DataRescue, RenderPdf };

/// Owns blocking platform work and a retained runtime reference until completion.
struct OwnerWork {
  napi_async_work work = nullptr;
  napi_deferred deferred = nullptr;
  OwnerOperation operation;
  BridgeHandle handle = nullptr;
  std::string root;
  std::string request;
  std::string response;
  std::string error;

  /// Releases only the worker's bridge reference after the platform operation ends.
  ~OwnerWork() { if (handle != nullptr) g_bridge_library.Destroy(handle); }
};

/// Executes blocking Rust storage or document work outside the ArkUI event loop.
void ExecuteOwnerWork(napi_env, void* data) {
  auto* work = static_cast<OwnerWork*>(data);
  try {
    switch (work->operation) {
      case OwnerOperation::DataRescue:
        work->response = g_bridge_library.DataRescue(work->root, work->request);
        break;
      case OwnerOperation::RenderPdf:
        work->response = g_bridge_library.RenderPdf(work->handle, work->request);
        break;
    }
  } catch (const std::exception& error) { work->error = error.what(); }
  catch (...) { work->error = "Unexpected native owner operation failure"; }
}

/// Settles the owner promise and releases every worker resource.
void CompleteOwnerWork(napi_env env, napi_status status, void* data) {
  std::unique_ptr<OwnerWork> work(static_cast<OwnerWork*>(data));
  if (status == napi_ok && work->error.empty()) {
    napi_resolve_deferred(env, work->deferred, StringValue(env, work->response));
  } else {
    napi_value error = nullptr;
    napi_create_error(env, nullptr, StringValue(env, status == napi_ok ? work->error : "Native owner work was cancelled"), &error);
    napi_reject_deferred(env, work->deferred, error);
  }
  napi_delete_async_work(env, work->work);
}

/// Queues one owned operation after its inputs and lifetime have been fixed.
napi_value QueueOwnerWork(napi_env env, std::unique_ptr<OwnerWork> work) {
  napi_value promise = nullptr;
  if (napi_create_promise(env, &work->deferred, &promise) != napi_ok) return ThrowError(env, "Cannot create owner promise");
  if (napi_create_async_work(env, nullptr, StringValue(env, "operit-owner-work"), ExecuteOwnerWork, CompleteOwnerWork, work.get(), &work->work) != napi_ok) return ThrowError(env, "Cannot create owner worker");
  if (napi_queue_async_work(env, work->work) != napi_ok) {
    napi_delete_async_work(env, work->work);
    return ThrowError(env, "Cannot queue owner worker");
  }
  work.release();
  return promise;
}

/// Dispatches blocking maintenance without starting Core.
napi_value DataRescue(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) return nullptr;
  auto args = CallbackArgs(env, info, 2);
  if (args.size() != 2) return nullptr;
  auto work = std::make_unique<OwnerWork>();
  work->operation = OwnerOperation::DataRescue;
  work->root = ReadString(env, args[0]);
  work->request = ReadString(env, args[1]);
  return QueueOwnerWork(env, std::move(work));
}

/// Dispatches PDF rasterization while retaining its existing runtime host.
napi_value RenderPdf(napi_env env, napi_callback_info info) {
  if (!EnsureBridgeReady(env)) return nullptr;
  auto args = CallbackArgs(env, info, 2);
  if (args.size() != 2) return nullptr;
  BridgeHandle handle = ReadHandle(env, args[0]);
  if (handle == nullptr) return nullptr;
  auto work = std::make_unique<OwnerWork>();
  work->operation = OwnerOperation::RenderPdf;
  work->handle = g_bridge_library.Retain(handle);
  work->request = ReadString(env, args[1]);
  return QueueOwnerWork(env, std::move(work));
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
  napi_property_descriptor descriptors[8];
  DefineFunction(env, exports, "create", Create, &descriptors[0]);
  DefineFunction(env, exports, "destroy", Destroy, &descriptors[1]);
  DefineFunction(env, exports, "emitRuntimeEvent", EmitRuntimeEvent, &descriptors[2]);
  DefineFunction(env, exports, "runtimeBootstrapRead", RuntimeBootstrapRead, &descriptors[3]);
  DefineFunction(env, exports, "runtimeBootstrapWrite", RuntimeBootstrapWrite, &descriptors[4]);
  DefineFunction(env, exports, "connectCoreFfi", ConnectCoreFfi, &descriptors[5]);
  DefineFunction(env, exports, "dataRescue", DataRescue, &descriptors[6]);
  DefineFunction(env, exports, "renderPdf", RenderPdf, &descriptors[7]);
  napi_define_properties(env, exports, 8, descriptors);
  return exports;
}

}  // namespace

NAPI_MODULE(operit_runtime_ohos, Init)
