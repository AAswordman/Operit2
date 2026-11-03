#pragma once
#include <atomic>
#include <cassert>
#include <cstdint>
#include <thread>

namespace operit::arm64::memory_detail {
// Page guards cover a scalar access, a short copy or an atomic instruction.
// libc++ shared_mutex takes a host mutex even for concurrent code-page reads;
// keep that read/read traffic out of the host kernel. Mapping changes and raw
// spans still use AccessSynchronization's separate writer gate.
class PageMutex {
    static constexpr uint32_t writer = uint32_t{1} << 31;
    std::atomic<uint32_t> state_{0}, waiting_writers_{0};

    static void wait(unsigned &attempts) noexcept {
        if (++attempts == 64) {
            attempts = 0;
            std::this_thread::yield();
        }
    }

  public:
    PageMutex() = default;
    PageMutex(const PageMutex &) = delete;
    PageMutex &operator=(const PageMutex &) = delete;

    void lock_shared() noexcept {
        unsigned attempts = 0;
        uint32_t observed = state_.load(std::memory_order_relaxed);
        for (;;) {
            // Stop admitting new readers while a writer drains existing ones.
            if (!waiting_writers_.load(std::memory_order_relaxed) && observed < writer - 1 &&
                state_.compare_exchange_weak(observed, observed + 1,
                                             std::memory_order_acquire, std::memory_order_relaxed))
                return;
            wait(attempts);
            observed = state_.load(std::memory_order_relaxed);
        }
    }
    void unlock_shared() noexcept {
        [[maybe_unused]] const auto previous = state_.fetch_sub(1, std::memory_order_release);
        assert(previous && !(previous & writer));
    }
    void lock() noexcept {
        waiting_writers_.fetch_add(1, std::memory_order_relaxed);
        unsigned attempts = 0;
        for (;;) {
            uint32_t expected = 0;
            if (state_.compare_exchange_weak(expected, writer,
                                             std::memory_order_acquire, std::memory_order_relaxed)) {
                waiting_writers_.fetch_sub(1, std::memory_order_relaxed);
                return;
            }
            wait(attempts);
        }
    }
    void unlock() noexcept {
        assert(state_.load(std::memory_order_relaxed) == writer);
        state_.store(0, std::memory_order_release);
    }
};
} // namespace operit::arm64::memory_detail
