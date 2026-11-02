#include "operit/arm64_runtime.h"
#include <cerrno>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace {
uint64_t number(const char *text) {
    if (!text[0] || text[0] == '-')
        throw std::runtime_error("invalid numeric option");
    char *end = nullptr;
    errno = 0;
    auto value = std::strtoull(text, &end, 10);
    if (errno || *end)
        throw std::runtime_error("invalid numeric option");
    return value;
}
void usage() {
    std::fprintf(stderr, "usage: operit-arm64 --root DIR [--max-steps N] [--memory-mib N] [--cpus N] [--stats] "
                         "[--fixed-acceleration] [--trace-syscalls] [--bind HOST:GUEST] [--bind-ro "
                         "HOST:GUEST] -- /guest/program [args...]\n");
}
} // namespace
int main(int argc, char **argv) {
    try {
        const char *root = nullptr;
        uint64_t budget = 100000000, memory = uint64_t{256} << 20;
        bool stats = false, fixed_acceleration = false, trace_syscalls = false;
        struct Bind {
            std::string source, target;
            uint32_t flags;
        };
        std::vector<Bind> mounts;
        int executable = -1;
        uint32_t execution_threads = 0;
        for (int j = 1; j < argc; ++j) {
            if (!std::strcmp(argv[j], "--")) {
                executable = j + 1;
                break;
            }
            if (!std::strcmp(argv[j], "--fixed-acceleration"))
                fixed_acceleration = true;
            else if (!std::strcmp(argv[j], "--trace-syscalls"))
                trace_syscalls = true;
            else if (!std::strcmp(argv[j], "--stats"))
                stats = true;
            else if (!std::strcmp(argv[j], "--help")) {
                usage();
                return 0;
            } else if (j + 1 < argc &&
                       (!std::strcmp(argv[j], "--bind") || !std::strcmp(argv[j], "--bind-ro"))) {
                bool read_only = !std::strcmp(argv[j], "--bind-ro");
                std::string value = argv[++j];
                auto separator = value.rfind(':');
                if (separator == std::string::npos || !separator || separator + 1 == value.size())
                    throw std::runtime_error("bind syntax is HOST_DIRECTORY:/guest/path");
                mounts.push_back({value.substr(0, separator), value.substr(separator + 1),
                                  read_only ? ORT_ARM64_BIND_READ_ONLY : 0u});
            } else if (j + 1 < argc && !std::strcmp(argv[j], "--root"))
                root = argv[++j];
            else if (j + 1 < argc && !std::strcmp(argv[j], "--cpus")) {
                const auto cpus = number(argv[++j]);
                if (!cpus || cpus > 64)
                    throw std::runtime_error("execution CPUs must be in 1..64");
                execution_threads = uint32_t(cpus);
            } else if (j + 1 < argc && !std::strcmp(argv[j], "--max-steps"))
                budget = number(argv[++j]);
            else if (j + 1 < argc && !std::strcmp(argv[j], "--memory-mib")) {
                uint64_t mib = number(argv[++j]);
                if (!mib || mib > 16384)
                    throw std::runtime_error("memory limit must be 1..16384 MiB");
                memory = mib << 20;
            } else
                throw std::runtime_error(std::string("unknown or incomplete option: ") + argv[j]);
        }
        if (!root || executable < 0 || executable >= argc) {
            usage();
            return 2;
        }
        ort_arm64_config config{};
        config.root_directory = root;
        config.resident_memory_limit = memory;
        config.enable_fixed_acceleration = fixed_acceleration;
        config.trace_syscalls = trace_syscalls;
        config.execution_threads = execution_threads;
        std::unique_ptr<ort_arm64_vm, decltype(&ort_arm64_destroy)> vm(ort_arm64_create(&config),
                                                                       &ort_arm64_destroy);
        if (!vm)
            throw std::runtime_error(ort_arm64_last_error(nullptr));
        for (const auto &mount : mounts)
            if (ort_arm64_bind(vm.get(), mount.source.c_str(), mount.target.c_str(), mount.flags))
                throw std::runtime_error(ort_arm64_last_error(vm.get()));
        const char *environment[] = {"HOME=/root", "PATH=/usr/local/bin:/usr/bin:/bin", "TERM=xterm-256color",
                                     nullptr};
        if (ort_arm64_load(vm.get(), argv[executable], const_cast<const char *const *>(argv + executable),
                           environment))
            throw std::runtime_error(ort_arm64_last_error(vm.get()));
        auto start = std::chrono::steady_clock::now();
        auto result = ort_arm64_run(vm.get(), budget);
        while (result.status == ORT_ARM64_WAITING && result.instructions < budget) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
            result = ort_arm64_run(vm.get(), budget - result.instructions);
        }
        double seconds = std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
        if (result.status == ORT_ARM64_FAULT || result.status == ORT_ARM64_UNSUPPORTED)
            std::fprintf(stderr, "%s\n", ort_arm64_last_error(vm.get()));
        if (stats)
            std::fprintf(
                stderr,
                "{\"status\":%d,\"pc\":%llu,\"instructions\":%llu,\"decoded_blocks\":%llu,\"block_cache_"
                "hits\":%llu,"
                "\"resident_bytes\":%llu,\"syscalls\":%llu,\"unsupported_syscalls\":%llu,\"last_unsupported_"
                "syscall\":%llu,\"accelerated_calls\":%llu,\"accelerated_bytes\":%llu,\"execution_threads\":%u,\"maximum_parallel_tasks\":%u,\"seconds\":%.9f}\n",
                int(result.status), static_cast<unsigned long long>(result.pc),
                static_cast<unsigned long long>(result.instructions),
                static_cast<unsigned long long>(result.decoded_blocks),
                static_cast<unsigned long long>(result.block_cache_hits),
                static_cast<unsigned long long>(result.resident_bytes),
                static_cast<unsigned long long>(result.syscalls),
                static_cast<unsigned long long>(result.unsupported_syscalls),
                static_cast<unsigned long long>(result.last_unsupported_syscall),
                static_cast<unsigned long long>(result.accelerated_calls),
                static_cast<unsigned long long>(result.accelerated_bytes),
                result.execution_threads, result.maximum_parallel_tasks, seconds);
        if (result.status == ORT_ARM64_EXITED)
            return result.exit_code;
        if (result.status == ORT_ARM64_BUDGET)
            return 124;
        return result.status == ORT_ARM64_UNSUPPORTED ? 125 : 126;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 126;
    }
}
