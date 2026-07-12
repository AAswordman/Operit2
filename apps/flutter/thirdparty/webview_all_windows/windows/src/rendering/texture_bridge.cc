#include "rendering/texture_bridge.h"

#include <windows.foundation.h>

#include <algorithm>
#include <atomic>
#include <cassert>
#include <cstring>

#include "util/direct3d11.interop.h"
#include "util/logging.h"

namespace webview_all_windows {
namespace {
const int kNumBuffers = 1;
} // namespace

TextureBridge::TextureBridge(GraphicsContext *graphics_context,
                             ABI::Windows::UI::Composition::IVisual *visual)
    : graphics_context_(graphics_context) {
  capture_item_ =
      graphics_context_->CreateGraphicsCaptureItemFromVisual(visual);
  assert(capture_item_);

  capture_item_->add_Closed(
      Microsoft::WRL::Callback<ABI::Windows::Foundation::ITypedEventHandler<
          ABI::Windows::Graphics::Capture::GraphicsCaptureItem *,
          IInspectable *>>(
          [](ABI::Windows::Graphics::Capture::IGraphicsCaptureItem *item,
             IInspectable *args) -> HRESULT {
            util::LogWarning("Capture item was closed.");
            return S_OK;
          })
          .Get(),
      &on_closed_token_);
}

TextureBridge::~TextureBridge() {
  StopFrameReadback();
  const std::lock_guard<std::mutex> lock(mutex_);
  StopInternal();
  if (capture_item_) {
    capture_item_->remove_Closed(on_closed_token_);
  }
}

bool TextureBridge::Start() {
  const std::lock_guard<std::mutex> lock(mutex_);
  if (is_running_ || !capture_item_) {
    return false;
  }

  ABI::Windows::Graphics::SizeInt32 size;
  capture_item_->get_Size(&size);

  frame_pool_ = graphics_context_->CreateCaptureFramePool(
      graphics_context_->device(),
      static_cast<ABI::Windows::Graphics::DirectX::DirectXPixelFormat>(
          kPixelFormat),
      kNumBuffers, size);
  assert(frame_pool_);

  frame_pool_->add_FrameArrived(
      Microsoft::WRL::Callback<ABI::Windows::Foundation::ITypedEventHandler<
          ABI::Windows::Graphics::Capture::Direct3D11CaptureFramePool *,
          IInspectable *>>(
          [this](ABI::Windows::Graphics::Capture::IDirect3D11CaptureFramePool
                     *pool,
                 IInspectable *args) -> HRESULT {
            OnFrameArrived();
            return S_OK;
          })
          .Get(),
      &on_frame_arrived_token_);

  if (FAILED(frame_pool_->CreateCaptureSession(capture_item_.get(),
                                               capture_session_.put()))) {
    util::LogWarning("Creating capture session failed.");
    return false;
  }

  if (SUCCEEDED(capture_session_->StartCapture())) {
    is_running_ = true;
    return true;
  }

  return false;
}

void TextureBridge::Stop() {
  const std::lock_guard<std::mutex> lock(mutex_);
  StopInternal();
}

// Changes the frame callback while synchronized with capture delivery.
void TextureBridge::SetOnFrameAvailable(FrameAvailableCallback callback) {
  const std::lock_guard<std::mutex> lock(mutex_);
  frame_available_ = std::move(callback);
}

// Changes the size callback while synchronized with capture delivery.
void TextureBridge::SetOnSurfaceSizeChanged(
    SurfaceSizeChangedCallback callback) {
  const std::lock_guard<std::mutex> lock(mutex_);
  surface_size_changed_ = std::move(callback);
}

void TextureBridge::StopInternal() {
  if (is_running_) {
    is_running_ = false;
    frame_pool_->remove_FrameArrived(on_frame_arrived_token_);
    auto closable =
        capture_session_.try_as<ABI::Windows::Foundation::IClosable>();
    assert(closable);
    closable->Close();
    capture_session_ = nullptr;
  }
}

void TextureBridge::OnFrameArrived() {
  bool has_frame = false;
  FrameAvailableCallback frame_available;
  {
    const std::lock_guard<std::mutex> lock(mutex_);
    if (!is_running_) {
      return;
    }

    winrt::com_ptr<ABI::Windows::Graphics::Capture::IDirect3D11CaptureFrame>
        frame;
    auto hr = frame_pool_->TryGetNextFrame(frame.put());
    if (SUCCEEDED(hr) && frame) {
      winrt::com_ptr<
          ABI::Windows::Graphics::DirectX::Direct3D11::IDirect3DSurface>
          frame_surface;

      if (SUCCEEDED(frame->get_Surface(frame_surface.put()))) {
        last_frame_ =
            util::TryGetDXGIInterfaceFromObject<ID3D11Texture2D>(frame_surface);
        has_frame = !ShouldDropFrame();
      }
    }

    if (needs_update_) {
      ABI::Windows::Graphics::SizeInt32 size;
      capture_item_->get_Size(&size);
      frame_pool_->Recreate(
          graphics_context_->device(),
          static_cast<ABI::Windows::Graphics::DirectX::DirectXPixelFormat>(
              kPixelFormat),
          kNumBuffers, size);
      needs_update_ = false;
    }

    if (has_frame) {
      frame_available = frame_available_;
    }
  }

  if (frame_available) {
    frame_available();
  }
}

bool TextureBridge::ShouldDropFrame() {
  if (!frame_duration_.has_value()) {
    return false;
  }
  auto now = std::chrono::high_resolution_clock::now();

  bool should_drop_frame = false;
  if (last_frame_timestamp_.has_value()) {
    auto diff = std::chrono::duration_cast<std::chrono::milliseconds>(
        now - last_frame_timestamp_.value());
    should_drop_frame = diff < frame_duration_.value();
  }

  if (!should_drop_frame) {
    last_frame_timestamp_ = now;
  }
  return should_drop_frame;
}

void TextureBridge::NotifySurfaceSizeChanged() {
  const std::lock_guard<std::mutex> lock(mutex_);
  needs_update_ = true;
}

void TextureBridge::SetFpsLimit(std::optional<int> max_fps) {
  const std::lock_guard<std::mutex> lock(mutex_);
  auto value = max_fps.value_or(0);
  if (value != 0) {
    frame_duration_ = FrameDuration(1000.0 / value);
  } else {
    frame_duration_.reset();
    last_frame_timestamp_.reset();
  }
}

// Returns whether a captured compositor texture is available for readback.
bool TextureBridge::HasLatestFrame() {
  const std::lock_guard<std::mutex> lock(mutex_);
  return last_frame_.get() != nullptr;
}

// Queues one captured texture for CPU readback on the worker thread.
void TextureBridge::CopyLatestFrameAsync(FrameCopyCallback callback) {
  winrt::com_ptr<ID3D11Texture2D> texture;
  {
    const std::lock_guard<std::mutex> lock(mutex_);
    texture = last_frame_;
  }
  if (!texture) {
    callback(false, {}, {0, 0});
    return;
  }

  {
    const std::lock_guard<std::mutex> lock(frame_readback_mutex_);
    if (!frame_readback_thread_.joinable()) {
      frame_readback_thread_ =
          std::thread([this]() { RunFrameReadbackWorker(); });
    }
    frame_readback_requests_.push_back(
        FrameReadbackRequest{std::move(texture), std::move(callback)});
  }
  frame_readback_condition_.notify_one();
}

// Stops the CPU readback worker and discards work owned by a closing bridge.
void TextureBridge::StopFrameReadback() {
  {
    const std::lock_guard<std::mutex> lock(frame_readback_mutex_);
    if (stop_frame_readback_) {
      return;
    }
    stop_frame_readback_ = true;
    frame_readback_requests_.clear();
  }
  frame_readback_condition_.notify_one();
  if (frame_readback_thread_.joinable()) {
    frame_readback_thread_.join();
  }
}

// Processes queued compositor texture readbacks away from the platform thread.
void TextureBridge::RunFrameReadbackWorker() {
  for (;;) {
    FrameReadbackRequest request;
    {
      std::unique_lock<std::mutex> lock(frame_readback_mutex_);
      frame_readback_condition_.wait(lock, [this]() {
        return stop_frame_readback_ || !frame_readback_requests_.empty();
      });
      if (stop_frame_readback_) {
        return;
      }
      request = std::move(frame_readback_requests_.front());
      frame_readback_requests_.pop_front();
    }

    std::vector<uint8_t> data;
    Size size = {0, 0};
    const bool success = CopyFrame(request.texture.get(), &data, &size);
    request.callback(success, data, size);
  }
}

// Copies one GPU texture into tightly packed BGRA bytes.
bool TextureBridge::CopyFrame(ID3D11Texture2D *texture,
                              std::vector<uint8_t> *bytes, Size *size) {
  if (!texture || !bytes || !size) {
    return false;
  }

  D3D11_TEXTURE2D_DESC src_desc = {};
  texture->GetDesc(&src_desc);
  if (src_desc.Width == 0 || src_desc.Height == 0) {
    return false;
  }

  D3D11_TEXTURE2D_DESC staging_desc = src_desc;
  staging_desc.BindFlags = 0;
  staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
  staging_desc.MiscFlags = 0;
  staging_desc.Usage = D3D11_USAGE_STAGING;

  winrt::com_ptr<ID3D11Texture2D> staging_texture;
  if (FAILED(graphics_context_->d3d_device()->CreateTexture2D(
          &staging_desc, nullptr, staging_texture.put()))) {
    util::LogWarning("Creating staging browser surface texture failed.");
    return false;
  }

  auto device_context = graphics_context_->d3d_device_context();
  device_context->CopyResource(staging_texture.get(), texture);

  D3D11_MAPPED_SUBRESOURCE mapped = {};
  if (FAILED(device_context->Map(staging_texture.get(), 0, D3D11_MAP_READ, 0,
                                 &mapped))) {
    util::LogWarning("Mapping browser surface staging texture failed.");
    return false;
  }

  const size_t width = src_desc.Width;
  const size_t height = src_desc.Height;
  const size_t row_bytes = width * 4;
  bytes->resize(row_bytes * height);
  for (size_t row = 0; row < height; row++) {
    std::memcpy(bytes->data() + row * row_bytes,
                static_cast<const uint8_t *>(mapped.pData) +
                    row * mapped.RowPitch,
                row_bytes);
  }
  device_context->Unmap(staging_texture.get(), 0);

  *size = Size{width, height};
  return true;
}

} // namespace webview_all_windows
