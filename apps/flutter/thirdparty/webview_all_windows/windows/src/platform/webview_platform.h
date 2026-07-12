#pragma once

#include <winrt/base.h>

#include <memory>
#include <optional>
#include <string>

#include "platform/winrt_runtime.h"
#include "rendering/graphics_context.h"

namespace webview_all_windows {

class WebviewPlatform {
public:
  WebviewPlatform();
  bool IsSupported() { return valid_; }
  std::optional<std::wstring> GetDefaultDataDirectory();
  bool IsGraphicsCaptureSessionSupported();
  GraphicsContext *graphics_context() const { return graphics_context_.get(); };

  WinrtRuntime *runtime() const { return runtime_.get(); }

private:
  std::unique_ptr<WinrtRuntime> runtime_;
  winrt::com_ptr<ABI::Windows::System::IDispatcherQueueController>
      dispatcher_queue_controller_;
  std::unique_ptr<GraphicsContext> graphics_context_;
  bool valid_ = false;
};

} // namespace webview_all_windows
