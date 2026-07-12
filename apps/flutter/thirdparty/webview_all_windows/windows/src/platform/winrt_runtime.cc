#include "platform/winrt_runtime.h"

#include <cstdint>
#include <strsafe.h>

namespace webview_all_windows {

WinrtRuntime::WinrtRuntime(RO_INIT_TYPE init_type) {
  combase_ = LoadLibraryW(L"combase.dll");
  core_messaging_ = LoadLibraryW(L"coremessaging.dll");
  if (combase_ == nullptr || core_messaging_ == nullptr) {
    return;
  }

  if (!LoadFunction(combase_, "WindowsCreateStringReference",
                    create_string_reference_) ||
      !LoadFunction(combase_, "RoGetActivationFactory",
                    get_activation_factory_) ||
      !LoadFunction(combase_, "RoInitialize", ro_initialize_) ||
      !LoadFunction(combase_, "RoUninitialize", ro_uninitialize_) ||
      !LoadFunction(core_messaging_, "CreateDispatcherQueueController",
                    create_dispatcher_queue_controller_)) {
    return;
  }

  const HRESULT result = ro_initialize_(init_type);
  initialized_ = SUCCEEDED(result) || result == S_FALSE;
  available_ = initialized_ || result == RPC_E_CHANGED_MODE;
}

WinrtRuntime::~WinrtRuntime() {
  if (initialized_ && ro_uninitialize_ != nullptr) {
    ro_uninitialize_();
  }
  if (core_messaging_ != nullptr) {
    FreeLibrary(core_messaging_);
  }
  if (combase_ != nullptr) {
    FreeLibrary(combase_);
  }
}

HRESULT WinrtRuntime::CreateStringReference(PCWSTR value, HSTRING *string,
                                            HSTRING_HEADER *header) const {
  if (!available_ || create_string_reference_ == nullptr || value == nullptr) {
    return E_FAIL;
  }

  size_t length = 0;
  HRESULT result = StringCchLengthW(value, STRSAFE_MAX_CCH, &length);
  if (FAILED(result) || length > UINT32_MAX) {
    return FAILED(result) ? result
                          : HRESULT_FROM_WIN32(ERROR_ARITHMETIC_OVERFLOW);
  }

  return create_string_reference_(value, static_cast<UINT32>(length), header,
                                  string);
}

HRESULT WinrtRuntime::GetActivationFactory(HSTRING runtime_class,
                                           REFIID interface_id,
                                           void **factory) const {
  if (!available_ || get_activation_factory_ == nullptr) {
    return E_FAIL;
  }
  return get_activation_factory_(runtime_class, interface_id, factory);
}

HRESULT WinrtRuntime::CreateDispatcherQueueController(
    DispatcherQueueOptions options,
    ABI::Windows::System::IDispatcherQueueController **controller) const {
  if (!available_ || create_dispatcher_queue_controller_ == nullptr) {
    return E_FAIL;
  }
  return create_dispatcher_queue_controller_(options, controller);
}

} // namespace webview_all_windows
