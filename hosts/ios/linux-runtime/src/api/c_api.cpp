#include "operit/arm64_runtime.h"
#include "runtime/machine.h"

struct ort_arm64_vm {
    operit::arm64::Machine machine;
    explicit ort_arm64_vm(const ort_arm64_config &c) : machine(c) {}
};
namespace {
thread_local std::string creation_error;
}
extern "C" ort_arm64_vm *ort_arm64_create(const ort_arm64_config *config) {
    try {
        if (!config || !config->root_directory || !config->root_directory[0])
            throw operit::arm64::Failure("root_directory is required");
        auto *vm = new ort_arm64_vm(*config);
        creation_error.clear();
        return vm;
    } catch (const std::exception &failure) {
        creation_error = failure.what();
    } catch (...) {
        creation_error = "runtime creation failed";
    }
    return nullptr;
}
extern "C" int ort_arm64_bind(ort_arm64_vm *vm, const char *source, const char *target, uint32_t flags) {
    if (!vm)
        return -1;
    return vm->machine.bind(source, target, flags);
}
extern "C" int ort_arm64_load(ort_arm64_vm *vm, const char *path, const char *const *argv,
                              const char *const *envp) {
    if (!vm)
        return -1;
    return vm->machine.load(path, argv, envp);
}
extern "C" ort_arm64_result ort_arm64_run(ort_arm64_vm *vm, uint64_t budget) {
    if (!vm) {
        ort_arm64_result result{};
        result.status = ORT_ARM64_FAULT;
        return result;
    }
    return vm->machine.run(budget);
}
extern "C" int ort_arm64_terminal_open(ort_arm64_vm *vm, uint32_t columns, uint32_t rows) {
    if (!vm) {
        creation_error = "terminal operation requires a VM";
        return -1;
    }
    return vm->machine.terminal_open(columns, rows);
}
extern "C" int64_t ort_arm64_terminal_read(ort_arm64_vm *vm, void *bytes, size_t length) {
    if (!vm) {
        creation_error = "terminal operation requires a VM";
        return -22;
    }
    return vm->machine.terminal_read(bytes, length);
}
extern "C" int64_t ort_arm64_terminal_write(ort_arm64_vm *vm, const void *bytes, size_t length) {
    if (!vm) {
        creation_error = "terminal operation requires a VM";
        return -22;
    }
    return vm->machine.terminal_write(bytes, length);
}
extern "C" int ort_arm64_terminal_resize(ort_arm64_vm *vm, uint32_t columns, uint32_t rows) {
    if (!vm) {
        creation_error = "terminal operation requires a VM";
        return -1;
    }
    return vm->machine.terminal_resize(columns, rows);
}
extern "C" int ort_arm64_terminal_signal(ort_arm64_vm *vm, int number) {
    if (!vm) {
        creation_error = "terminal operation requires a VM";
        return -1;
    }
    return vm->machine.terminal_signal(number);
}
extern "C" int ort_arm64_terminal_close(ort_arm64_vm *vm) {
    if (!vm) {
        creation_error = "terminal operation requires a VM";
        return -1;
    }
    return vm->machine.terminal_close();
}
extern "C" const char *ort_arm64_last_error(const ort_arm64_vm *vm) {
    return vm ? vm->machine.error.c_str() : creation_error.c_str();
}
extern "C" void ort_arm64_destroy(ort_arm64_vm *vm) {
    delete vm;
}
