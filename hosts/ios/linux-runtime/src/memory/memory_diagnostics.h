#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
#include <cstdlib>
#include <unistd.h>
#endif

namespace operit::arm64::memory_detail {
// Counters are local to the executing host thread. No atomics, clocks or I/O
// occur at an instrumentation site. A disabled build erases every site,
// including its arguments, and does not allocate diagnostic TLS. *_bytes
// counts lengths supplied to equality checks, not libc's early-exit read
// traffic. TLB counters cover C++ paths, not native ASM hits.
#define OPERIT_MEMORY_DIAGNOSTIC_COUNTERS(X) \
    X(execution_scopes) X(serial_execution_scopes) X(nested_scopes) \
    X(validate_lines_calls) X(validate_lines_hits) X(validate_lines_partial_hits) \
    X(validate_lines_ineligible) X(validate_lines_failures) X(validate_lines_epoch_failures) \
    X(validate_lines_requested) X(validate_lines_reused) X(validate_lines_compared) \
    X(validate_lines_inconclusive) X(validate_lines_repeated_mismatches) \
    X(code_lines_recompared) X(code_lines_recompared_after_mismatch) \
    X(compare_64_calls) X(compare_128_calls) X(compare_256_calls) X(compare_larger_calls) \
    X(validate_snapshot_calls) X(validate_snapshot_hits) X(validate_snapshot_partial_hits) \
    X(validate_snapshot_slow) X(validate_snapshot_cache_resets) \
    X(validate_snapshot_failures) X(validate_snapshot_epoch_failures) \
    X(snapshot_calls) X(snapshot_hits) X(snapshot_unchanged) X(snapshot_copies) \
    X(code_compare_calls) X(code_compare_bytes) X(code_compare_equal) X(code_compare_unequal) \
    X(line_compare_calls) X(line_compare_bytes) \
    X(exact_compare_calls) X(exact_compare_bytes) \
    X(snapshot_compare_calls) X(snapshot_compare_bytes) \
    X(file_metadata_calls) X(file_metadata_hits) X(file_metadata_misses) \
    X(file_metadata_unscoped) X(file_metadata_collisions) \
    X(file_metadata_first_fstats) X(file_metadata_repeat_fstats) X(file_metadata_untracked_fstats) \
    X(fstat_calls) X(fstat_failures) X(file_page_checks) X(file_page_failures) \
    X(validate_file_calls) X(validate_file_hits) \
    X(tlb_lookups) X(tlb_first_hits) X(tlb_second_hits) X(tlb_misses) \
    X(tlb_parallel_lookups) X(tlb_fallbacks) X(tlb_refills) \
    X(tlb_resident_attempts) X(tlb_resident_hits) X(tlb_resident_declines) \
    X(page_access_calls) X(page_access_locks)

class MemoryDiagnostics {
  public:
    enum class Counter : size_t {
#define OPERIT_MEMORY_DECLARE_COUNTER(name) name,
        OPERIT_MEMORY_DIAGNOSTIC_COUNTERS(OPERIT_MEMORY_DECLARE_COUNTER)
#undef OPERIT_MEMORY_DECLARE_COUNTER
        count
    };
    using Counts = std::array<uint64_t, size_t(Counter::count)>;
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
    static constexpr bool enabled = true;
  private:
    // Trivial TLS keeps destructor-registration guards off the counting path.
    inline static thread_local Counts counts_{};
    struct DumpOnExit {
        ~DumpOnExit() {
            const auto *value = std::getenv("OPERIT_MEMORY_DIAGNOSTICS");
            if (value && *value && *value != '0') MemoryDiagnostics::dump();
        }
    };
  public:
    static void arm() noexcept {
        static thread_local DumpOnExit dump_on_exit;
        (void)dump_on_exit;
    }
    static void add(Counter counter, uint64_t amount = 1) noexcept {
        counts_[size_t(counter)] += amount;
    }
    static Counts snapshot() noexcept { return counts_; }
    static void reset() noexcept { counts_.fill(0); }
    // Explicit dumps are unconditional; only automatic exit dumps consult the
    // environment. Repeated dumps are cumulative until reset() is called.
    static void dump(FILE *output = stderr) noexcept {
        static constexpr const char *names[] = {
#define OPERIT_MEMORY_COUNTER_NAME(name) #name,
            OPERIT_MEMORY_DIAGNOSTIC_COUNTERS(OPERIT_MEMORY_COUNTER_NAME)
#undef OPERIT_MEMORY_COUNTER_NAME
        };
        if (!output) return;
        ::flockfile(output);
        std::fprintf(output, "operit-memory pid=%ld thread=%p", long(::getpid()),
                     static_cast<void *>(&counts_));
        for (size_t index = 0; index < counts_.size(); ++index)
            std::fprintf(output, " %s=%llu", names[index],
                         static_cast<unsigned long long>(counts_[index]));
        std::fputc('\n', output);
        ::funlockfile(output);
    }
#else
    static constexpr bool enabled = false;
    static void arm() noexcept {}
    static void add(Counter, uint64_t = 1) noexcept {}
    static Counts snapshot() noexcept { return {}; }
    static void reset() noexcept {}
    static void dump(FILE * = stderr) noexcept {}
#endif
};
#undef OPERIT_MEMORY_DIAGNOSTIC_COUNTERS
} // namespace operit::arm64::memory_detail

#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
#define OPERIT_MEMORY_DIAGNOSTIC(name, amount) \
    ::operit::arm64::memory_detail::MemoryDiagnostics::add( \
        ::operit::arm64::memory_detail::MemoryDiagnostics::Counter::name, (amount))
#else
#define OPERIT_MEMORY_DIAGNOSTIC(name, amount) ((void)0)
#endif

namespace operit::arm64::memory_detail {
enum class CodeComparison { Line, Exact, Snapshot };
[[gnu::always_inline]] inline bool code_bytes_equal(const void *actual, const void *expected, size_t bytes,
                             [[maybe_unused]] CodeComparison kind) noexcept {
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
    OPERIT_MEMORY_DIAGNOSTIC(code_compare_calls, 1);
    OPERIT_MEMORY_DIAGNOSTIC(code_compare_bytes, bytes);
    if (kind == CodeComparison::Line) {
        OPERIT_MEMORY_DIAGNOSTIC(line_compare_calls, 1);
        OPERIT_MEMORY_DIAGNOSTIC(line_compare_bytes, bytes);
    } else if (kind == CodeComparison::Exact) {
        OPERIT_MEMORY_DIAGNOSTIC(exact_compare_calls, 1);
        OPERIT_MEMORY_DIAGNOSTIC(exact_compare_bytes, bytes);
    } else {
        OPERIT_MEMORY_DIAGNOSTIC(snapshot_compare_calls, 1);
        OPERIT_MEMORY_DIAGNOSTIC(snapshot_compare_bytes, bytes);
    }
    if (bytes <= 64) OPERIT_MEMORY_DIAGNOSTIC(compare_64_calls, 1);
    else if (bytes <= 128) OPERIT_MEMORY_DIAGNOSTIC(compare_128_calls, 1);
    else if (bytes <= 256) OPERIT_MEMORY_DIAGNOSTIC(compare_256_calls, 1);
    else OPERIT_MEMORY_DIAGNOSTIC(compare_larger_calls, 1);
#endif
    const bool equal = std::memcmp(actual, expected, bytes) == 0;
    if (equal) OPERIT_MEMORY_DIAGNOSTIC(code_compare_equal, 1);
    else OPERIT_MEMORY_DIAGNOSTIC(code_compare_unequal, 1);
    return equal;
}
} // namespace operit::arm64::memory_detail
