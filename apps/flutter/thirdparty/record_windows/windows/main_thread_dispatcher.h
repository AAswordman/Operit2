#pragma once

#include <windows.h>

#include <deque>
#include <functional>
#include <mutex>
#include <utility>

namespace record_windows {

constexpr UINT WM_RUN_DELEGATE = WM_USER + 101;

/// Owns callbacks for one engine and cancels them before that engine is released.
class MainThreadDispatcher {
 public:
  /// Captures the native view handle without retaining a registrar object.
  explicit MainThreadDispatcher(HWND window) : window_(window) {}

  /// Queues work only while the owning window accepts dispatch messages.
  bool Post(std::function<void()> callback) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (closed_) return false;
    callbacks_.push_back(std::move(callback));
    if (!PostMessage(window(), WM_RUN_DELEGATE, 0, 0)) {
      callbacks_.pop_back();
      return false;
    }
    return true;
  }

  /// Runs one callback on the owning window's platform thread.
  void RunNext() {
    std::function<void()> callback;
    {
      std::lock_guard<std::mutex> lock(mutex_);
      if (callbacks_.empty()) return;
      callback = std::move(callbacks_.front());
      callbacks_.pop_front();
    }
    callback();
  }

  /// Invalidates queued and subsequent work before the engine is destroyed.
  void Close() {
    std::lock_guard<std::mutex> lock(mutex_);
    closed_ = true;
    callbacks_.clear();
  }

  /// Resolves the owner after Flutter reparents its view during window startup.
  HWND window() const { return GetAncestor(window_, GA_ROOT); }

 private:
  const HWND window_;
  std::mutex mutex_;
  std::deque<std::function<void()>> callbacks_;
  bool closed_ = false;
};

}  // namespace record_windows
