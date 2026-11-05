#pragma once
#include <atomic>
#include <cassert>
#include <memory>
#include <mutex>
#include "memory/page_mutex.h"
#include <stdexcept>
#include <thread>
#include <vector>

namespace operit::arm64::memory_detail {
// Readers publish to separate cache lines. Mapping changes and raw spans wait
// for those short accesses without contending on a counter in the hot path.
class AccessSynchronization {
    struct alignas(128) ReaderSlot {
        std::atomic<bool> active{false};
    };
    struct ThreadState {
        std::shared_ptr<ReaderSlot> slot;
        unsigned reads = 0, writes = 0;
    };
    inline static std::recursive_mutex writers_;
    inline static std::mutex registry_mutex_;
    inline static std::vector<std::weak_ptr<ReaderSlot>> readers_;
    inline static std::atomic<bool> pending_{false};
    static ThreadState &thread() {
        static thread_local ThreadState state;
        return state;
    }
    static ReaderSlot &slot(ThreadState &state) {
        if (!state.slot) {
            auto result = std::make_shared<ReaderSlot>();
            std::lock_guard<std::mutex> lock(registry_mutex_);
            readers_.emplace_back(result);
            state.slot = std::move(result);
        }
        return *state.slot;
    }
    static void wait(unsigned &attempts) noexcept {
        if (++attempts == 64) {
            attempts = 0;
            std::this_thread::yield();
        }
    }
    static void close() {
        // New slots cannot publish until registration releases this mutex;
        // existing readers can leave without acquiring it.
        std::lock_guard<std::mutex> registry(registry_mutex_);
        pending_.store(true, std::memory_order_seq_cst);
        for (auto it = readers_.begin(); it != readers_.end();) {
            if (auto reader = it->lock()) {
                unsigned attempts = 0;
                while (reader->active.load(std::memory_order_seq_cst))
                    wait(attempts);
                ++it;
            } else
                it = readers_.erase(it);
        }
    }

  public:
    static bool exclusive() noexcept {
        return thread().writes != 0;
    }
    static unsigned read_depth() noexcept {
        return thread().reads;
    }
    class ReadGuard {
        ThreadState *state_ = nullptr;

      public:
        ReadGuard() = default;
        explicit ReadGuard(bool enabled) {
            lock(enabled);
        }
        void lock(bool enabled) {
            assert(!state_);
            if (!enabled)
                return;
            auto &state = thread();
            if (state.writes)
                return;
            if (!state.reads) {
                auto &reader = slot(state);
                unsigned attempts = 0;
                for (;;) {
                    while (pending_.load(std::memory_order_seq_cst))
                        wait(attempts);
                    reader.active.store(true, std::memory_order_seq_cst);
                    if (!pending_.load(std::memory_order_seq_cst))
                        break;
                    reader.active.store(false, std::memory_order_seq_cst);
                }
            }
            ++state.reads;
            state_ = &state;
        }
        // Retain the same thread state until unlock; no second TLS lookup is needed.
        bool active() const noexcept {
            return state_ != nullptr;
        }
        void unlock() noexcept {
            if (!state_)
                return;
            auto &state = *state_;
            if (!--state.reads)
                state.slot->active.store(false, std::memory_order_seq_cst);
            state_ = nullptr;
        }
        ~ReadGuard() {
            unlock();
        }
        ReadGuard(const ReadGuard &) = delete;
        ReadGuard &operator=(const ReadGuard &) = delete;
    };
    class WriteGuard {
        bool active_ = false;

      public:
        explicit WriteGuard(bool enabled) {
            if (!enabled)
                return;
            auto &state = thread();
            if (!state.writes) {
                assert(!state.reads);
                writers_.lock();
                try {
                    close();
                } catch (...) {
                    writers_.unlock();
                    throw;
                }
            }
            ++state.writes;
            active_ = true;
        }
        ~WriteGuard() {
            if (!active_)
                return;
            if (!--thread().writes) {
                pending_.store(false, std::memory_order_seq_cst);
                writers_.unlock();
            }
        }
        WriteGuard(const WriteGuard &) = delete;
        WriteGuard &operator=(const WriteGuard &) = delete;
    };
    class PageGuard {
        inline static thread_local PageGuard *held_ = nullptr;
        PageGuard *previous_ = nullptr;
        PageMutex *mutex_ = nullptr;
        bool writing_ = false;

      public:
        PageGuard() = default;
        void lock(PageMutex &mutex, bool writing) {
            assert(!mutex_);
            for (auto *guard = held_; guard; guard = guard->previous_) {
                if (guard->mutex_ != &mutex)
                    continue;
                if (writing && !guard->writing_)
                    throw std::logic_error("write access nested under a shared page lock");
                return;
            }
            if (writing)
                mutex.lock();
            else
                mutex.lock_shared();
            mutex_ = &mutex;
            writing_ = writing;
            previous_ = held_;
            held_ = this;
        }
        ~PageGuard() {
            if (!mutex_)
                return;
            assert(held_ == this);
            held_ = previous_;
            if (writing_)
                mutex_->unlock();
            else
                mutex_->unlock_shared();
        }
        PageGuard(const PageGuard &) = delete;
        PageGuard &operator=(const PageGuard &) = delete;
    };
};
} // namespace operit::arm64::memory_detail
