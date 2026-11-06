#ifndef OPERIT_ARM64_RUNTIME_H
#define OPERIT_ARM64_RUNTIME_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Each instance owns its guest tasks, address space, descriptors and decoded blocks.
 * Serialize calls to one instance. Guest code is never mapped host-executable.
 * Callbacks run synchronously on the thread calling ort_arm64_run. Return a
 * byte count or a negative Linux errno (not Darwin errno). */
typedef struct ort_arm64_vm ort_arm64_vm;
typedef int64_t (*ort_arm64_io)(void *context, int fd, void *bytes, size_t length);

typedef struct {
    const char *root_directory;
    uint64_t resident_memory_limit;
    ort_arm64_io read;
    ort_arm64_io write;
    void *io_context;
    /* Opt-in, digest-pinned native helpers for selected libc functions. */
    int enable_fixed_acceleration;
    /* Development diagnostics: syscall numbers, arguments and Linux return values on host stderr. */
    int trace_syscalls;
    /* Guest execution CPUs: zero selects up to four available host threads. */
    uint32_t execution_threads;
} ort_arm64_config;

typedef enum {
    ORT_ARM64_EXITED = 0,
    ORT_ARM64_BUDGET = 1,
    ORT_ARM64_FAULT = 2,
    ORT_ARM64_UNSUPPORTED = 3,
    ORT_ARM64_WAITING = 4
} ort_arm64_status;

typedef struct {
    ort_arm64_status status;
    int exit_code;
    uint64_t pc;
    /* Retired guest instructions plus one step per fixed helper invocation. */
    uint64_t instructions;
    uint64_t decoded_blocks;
    uint64_t block_cache_hits;
    uint64_t resident_bytes;
    uint64_t syscalls;
    uint64_t unsupported_syscalls;
    uint64_t last_unsupported_syscall;
    uint64_t accelerated_calls;
    uint64_t accelerated_bytes;
    uint32_t execution_threads;
    uint32_t maximum_parallel_tasks;
} ort_arm64_result;

ort_arm64_vm *ort_arm64_create(const ort_arm64_config *config);
/* Register an fd-anchored host directory at an absolute guest path before load.
 * No host mount/chroot privileges are needed. The source must already be
 * accessible to the embedding app (including its platform sandbox grants).
 * Parent directories must exist; the final mountpoint may be absent.
 * flags: ORT_ARM64_BIND_READ_ONLY or 0. Returns 0 or -1 with last_error. */
#define ORT_ARM64_BIND_READ_ONLY 1u
int ort_arm64_bind(ort_arm64_vm *vm, const char *host_directory, const char *guest_path, uint32_t flags);
/* Load once, from a guest absolute path inside root_directory. argv must have
 * argv[0]. Both vectors are NULL terminated. Returns 0, or -1 with last_error. */
int ort_arm64_load(ort_arm64_vm *vm, const char *executable, const char *const *argv,
                   const char *const *envp);
/* The budget is per call; task state and the scheduling quantum are retained
 * when a budget is exhausted.
 * This bounds execution steps, not wall time: synchronous I/O can block.
 * Each fixed helper invocation consumes one step when acceleration is enabled. */
ort_arm64_result ort_arm64_run(ort_arm64_vm *vm, uint64_t instruction_budget);
/* Attach a real guest console after load and before the first run. Control
 * operations return 0 or -1 with last_error; byte I/O returns a count or a
 * negative Linux errno. All calls use the VM's owning serial thread. */
int ort_arm64_terminal_open(ort_arm64_vm *vm, uint32_t cols, uint32_t rows);
int64_t ort_arm64_terminal_read(ort_arm64_vm *vm, void *bytes, size_t length);
int64_t ort_arm64_terminal_write(ort_arm64_vm *vm, const void *bytes, size_t length);
int ort_arm64_terminal_resize(ort_arm64_vm *vm, uint32_t cols, uint32_t rows);
int ort_arm64_terminal_signal(ort_arm64_vm *vm, int signo);
int ort_arm64_terminal_close(ort_arm64_vm *vm);
/* Passing NULL reports the last create failure on this host thread. */
const char *ort_arm64_last_error(const ort_arm64_vm *vm);
void ort_arm64_destroy(ort_arm64_vm *vm);

#ifdef __cplusplus
}
#endif
#endif
