#include "operit/arm64_runtime.h"
#include <cstdio>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <string>

namespace {
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
struct Output {
    std::string text;
    bool bad_count = false;
};
int64_t capture(void *context, int fd, void *bytes, size_t size) {
    auto &output = *static_cast<Output *>(context);
    if (output.bad_count)
        return size + 1;
    if (fd == 1)
        output.text.append(static_cast<const char *>(bytes), size);
    return size;
}
using VM = std::unique_ptr<ort_arm64_vm, decltype(&ort_arm64_destroy)>;
VM create(const char *root, Output &output) {
    ort_arm64_config config{};
    config.root_directory = root;
    config.write = capture;
    config.io_context = &output;
    VM vm(ort_arm64_create(&config), &ort_arm64_destroy);
    require(bool(vm), ort_arm64_last_error(nullptr));
    const char *argv[] = {"/hello", "from-api", nullptr};
    const char *env[] = {"OPERIT=isolated", nullptr};
    require(ort_arm64_load(vm.get(), "/hello", argv, env) == 0, ort_arm64_last_error(vm.get()));
    return vm;
}
} // namespace
int main(int argc, char **argv) {
    try {
        require(argc == 2, "root argument required");
        Output a, b;
        auto first = create(argv[1], a), second = create(argv[1], b);
        ort_arm64_result x{}, y{};
        do {
            x = ort_arm64_run(first.get(), 3);
            y = ort_arm64_run(second.get(), 7);
            require(x.status <= ORT_ARM64_BUDGET && y.status <= ORT_ARM64_BUDGET, "interleaved run failed");
        } while (x.status != ORT_ARM64_EXITED || y.status != ORT_ARM64_EXITED);
        require(a.text == "hello-arm64\n" && b.text == a.text, "VM output leaked across instances");
        require(x.instructions == y.instructions && x.exit_code == 0, "budget changed execution");
        require(ort_arm64_run(first.get(), 0).status == ORT_ARM64_EXITED, "exited state is not stable");
        const char *args[] = {"/hello", nullptr};
        require(ort_arm64_load(first.get(), "/hello", args, nullptr) == -1,
                "second load unexpectedly succeeded");
        Output threaded;
        ort_arm64_config config{};
        config.root_directory = argv[1];
        config.write = capture;
        config.io_context = &threaded;
        VM threads(ort_arm64_create(&config), &ort_arm64_destroy);
        const char *thread_args[] = {"/threads", nullptr};
        require(ort_arm64_load(threads.get(), "/threads", thread_args, nullptr) == 0, "thread fixture load");
        ort_arm64_result task_result{};
        unsigned slices = 0;
        do {
            task_result = ort_arm64_run(threads.get(), 1);
            require(task_result.status <= ORT_ARM64_BUDGET || task_result.status == ORT_ARM64_WAITING,
                    "single-step task resume fault");
            require(++slices < 1000000, "single-step task scheduling stalled");
        } while (task_result.status != ORT_ARM64_EXITED);
        require(task_result.exit_code == 0 && threaded.text == "threads-tls-futex-join-ok\n",
                "budget boundary lost task wait/wake state");
        Output mounted;
        config.io_context = &mounted;
        VM bound(ort_arm64_create(&config), &ort_arm64_destroy);
        require(bool(bound), "mount VM creation");
        require(ort_arm64_bind(bound.get(), argv[1], "/external", 0x8000) == -1,
                "invalid bind flags accepted");
        require(ort_arm64_bind(bound.get(), argv[1], "/external", ORT_ARM64_BIND_READ_ONLY) == 0,
                ort_arm64_last_error(bound.get()));
        const char *bound_args[] = {"/external/hello", nullptr};
        require(ort_arm64_load(bound.get(), "/external/hello", bound_args, nullptr) == 0,
                ort_arm64_last_error(bound.get()));
        require(ort_arm64_bind(bound.get(), argv[1], "/late", 0) == -1, "mount namespace changed after load");
        auto bound_result = ort_arm64_run(bound.get(), 100000);
        require(bound_result.status == ORT_ARM64_EXITED && bound_result.exit_code == 0 &&
                    mounted.text == "hello-arm64\n",
                "ELF execution through external readonly mount");
        Output bad;
        bad.bad_count = true;
        auto invalid = create(argv[1], bad);
        auto failure = ort_arm64_run(invalid.get(), 1000);
        require(failure.status == ORT_ARM64_FAULT, "invalid callback count was accepted");
        require(ort_arm64_run(invalid.get(), 1000).status == ORT_ARM64_FAULT, "fault state is not stable");
        require(ort_arm64_create(nullptr) == nullptr, "invalid config was accepted");
        require(std::strlen(ort_arm64_last_error(nullptr)) > 0, "missing create error");
        std::puts("C API isolation, budget resume and callback checks passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    }
}
