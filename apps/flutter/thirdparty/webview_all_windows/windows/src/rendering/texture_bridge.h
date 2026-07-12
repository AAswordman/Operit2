#pragma once

#include <windows.graphics.capture.h>
#include <wrl.h>

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <deque>
#include <functional>
#include <mutex>
#include <optional>
#include <thread>
#include <utility>
#include <vector>

#include "rendering/graphics_context.h"

namespace webview_all_windows {

struct Size {
  size_t width;
  size_t height;
};

class TextureBridge {
public:
  typedef std::function<void()> FrameAvailableCallback;
  typedef std::function<void(Size size)> SurfaceSizeChangedCallback;
  typedef std::function<void(bool success, const std::vector<uint8_t> &data,
                             Size size)>
      FrameCopyCallback;
  typedef std::chrono::duration<double, std::milli> FrameDuration;

  TextureBridge(GraphicsContext *graphics_context,
                ABI::Windows::UI::Composition::IVisual *visual);
  virtual ~TextureBridge();

  bool Start();
  void Stop();

  /// Changes the callback invoked after a compositor frame arrives.
  void SetOnFrameAvailable(FrameAvailableCallback callback);

  /// Changes the callback invoked after the captured surface changes size.
  void SetOnSurfaceSizeChanged(SurfaceSizeChangedCallback callback);

  void NotifySurfaceSizeChanged();
  void SetFpsLimit(std::optional<int> max_fps);

  /// Returns whether the compositor has produced a frame for readback.
  bool HasLatestFrame();

  /// Copies the latest compositor frame on the readback worker.
  void CopyLatestFrameAsync(FrameCopyCallback callback);

  /// Stops the readback worker before bridge callbacks are destroyed.
  void StopFrameReadback();

protected:
  bool is_running_ = false;

  const GraphicsContext *graphics_context_;
  std::mutex mutex_;
  std::optional<FrameDuration> frame_duration_ = std::nullopt;

  FrameAvailableCallback frame_available_;
  SurfaceSizeChangedCallback surface_size_changed_;
  std::atomic<bool> needs_update_ = false;
  winrt::com_ptr<ID3D11Texture2D> last_frame_;
  std::optional<std::chrono::high_resolution_clock::time_point>
      last_frame_timestamp_;

  struct FrameReadbackRequest {
    winrt::com_ptr<ID3D11Texture2D> texture;
    FrameCopyCallback callback;
  };
  std::mutex frame_readback_mutex_;
  std::condition_variable frame_readback_condition_;
  std::deque<FrameReadbackRequest> frame_readback_requests_;
  std::thread frame_readback_thread_;
  bool stop_frame_readback_ = false;

  winrt::com_ptr<ABI::Windows::Graphics::Capture::IGraphicsCaptureItem>
      capture_item_;
  winrt::com_ptr<ABI::Windows::Graphics::Capture::IDirect3D11CaptureFramePool>
      frame_pool_;
  winrt::com_ptr<ABI::Windows::Graphics::Capture::IGraphicsCaptureSession>
      capture_session_;

  EventRegistrationToken on_closed_token_ = {};
  EventRegistrationToken on_frame_arrived_token_ = {};

  virtual void StopInternal();
  void OnFrameArrived();
  bool ShouldDropFrame();

  /// Runs the blocking D3D staging readback queue on its worker thread.
  void RunFrameReadbackWorker();

  /// Copies one captured D3D texture into tightly packed BGRA bytes.
  bool CopyFrame(ID3D11Texture2D *texture, std::vector<uint8_t> *bytes,
                 Size *size);

  // corresponds to DXGI_FORMAT_B8G8R8A8_UNORM
  static constexpr auto kPixelFormat = ABI::Windows::Graphics::DirectX::
      DirectXPixelFormat::DirectXPixelFormat_B8G8R8A8UIntNormalized;
};

} // namespace webview_all_windows
