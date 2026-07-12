#pragma once

#include <dispatcherqueue.h>
#include <roapi.h>
#include <windows.h>

#include <windows.ui.composition.interop.h>

namespace webview_all_windows {

class WinrtRuntime {
public:
  explicit WinrtRuntime(RO_INIT_TYPE init_type);
  ~WinrtRuntime();

  WinrtRuntime(const WinrtRuntime &) = delete;
  WinrtRuntime &operator=(const WinrtRuntime &) = delete;

  bool available() const { return available_; }

  HRESULT CreateStringReference(PCWSTR value, HSTRING *string,
                                HSTRING_HEADER *header) const;
  HRESULT GetActivationFactory(HSTRING runtime_class, REFIID interface_id,
                               void **factory) const;
  HRESULT CreateDispatcherQueueController(
      DispatcherQueueOptions options,
      ABI::Windows::System::IDispatcherQueueController **controller) const;

private:
  template <typename T>
  bool LoadFunction(HMODULE module, const char *name, T *&target) {
    target = reinterpret_cast<T *>(GetProcAddress(module, name));
    return target != nullptr;
  }

  using WindowsCreateStringReferenceFn = HRESULT
      WINAPI(PCWSTR, UINT32, HSTRING_HEADER *, HSTRING *);
  using RoGetActivationFactoryFn = HRESULT WINAPI(HSTRING, REFIID, void **);
  using RoInitializeFn = HRESULT WINAPI(RO_INIT_TYPE);
  using RoUninitializeFn = void WINAPI();
  using CreateDispatcherQueueControllerFn = HRESULT
      WINAPI(DispatcherQueueOptions,
             ABI::Windows::System::IDispatcherQueueController **);

  HMODULE combase_ = nullptr;
  HMODULE core_messaging_ = nullptr;
  WindowsCreateStringReferenceFn *create_string_reference_ = nullptr;
  RoGetActivationFactoryFn *get_activation_factory_ = nullptr;
  RoInitializeFn *ro_initialize_ = nullptr;
  RoUninitializeFn *ro_uninitialize_ = nullptr;
  CreateDispatcherQueueControllerFn *create_dispatcher_queue_controller_ =
      nullptr;
  bool initialized_ = false;
  bool available_ = false;
};

} // namespace webview_all_windows
