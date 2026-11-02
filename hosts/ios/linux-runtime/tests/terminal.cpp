#include "operit/arm64_runtime.h"
#include <chrono>
#include <cstdio>
#include <memory>
#include <stdexcept>
#include <string>
#include <thread>

namespace {
using Clock = std::chrono::steady_clock;
void require(bool condition, const std::string &message) {
    if (!condition)
        throw std::runtime_error(message);
}
class Terminal {
  public:
    explicit Terminal(const char *root) : vm_(nullptr, &ort_arm64_destroy) {
        ort_arm64_config config{};
        config.root_directory = root;
        config.resident_memory_limit = uint64_t{256} << 20;
        config.enable_fixed_acceleration = 1;
        vm_.reset(ort_arm64_create(&config));
        require(bool(vm_), ort_arm64_last_error(nullptr));
        const char *argv[] = {"/bin/sh", "-i", nullptr};
        const char *env[] = {"HOME=/root", "PATH=/usr/bin:/bin:/usr/sbin:/sbin", "TERM=xterm-256color",
                             "PS1=ORT> ", nullptr};
        require(ort_arm64_load(vm_.get(), argv[0], argv, env) == 0, error());
        require(ort_arm64_terminal_open(vm_.get(), 80, 24) == 0, error());
    }
    void write(const std::string &text) {
        size_t offset = 0;
        const auto deadline = Clock::now() + std::chrono::seconds(5);
        while (offset < text.size()) {
            const auto written =
                ort_arm64_terminal_write(vm_.get(), text.data() + offset, text.size() - offset);
            require(written >= 0 || written == -11, error());
            if (written > 0)
                offset += size_t(written);
            else
                step();
            require(Clock::now() < deadline, "terminal input stalled");
        }
    }
    void expect(const std::string &text, size_t from = 0) {
        const auto deadline = Clock::now() + std::chrono::seconds(10);
        while (output_.find(text, from) == std::string::npos) {
            step();
            require(result_.status != ORT_ARM64_EXITED, "shell exited before expected output: " + output_);
            require(Clock::now() < deadline, "missing terminal output: " + text + "\nreceived: " + output_);
        }
    }
    void wait() {
        const auto deadline = Clock::now() + std::chrono::seconds(5);
        do {
            step();
            require(result_.status != ORT_ARM64_EXITED, "shell exited while waiting for input");
            require(Clock::now() < deadline, "terminal never yielded for input");
        } while (result_.status != ORT_ARM64_WAITING);
    }
    void resize() {
        require(ort_arm64_terminal_resize(vm_.get(), 101, 37) == 0, error());
    }
    void interrupt() {
        require(ort_arm64_terminal_signal(vm_.get(), 2) == 0, error());
    }
    void exit() {
        write("exit 7\n");
        const auto deadline = Clock::now() + std::chrono::seconds(5);
        do {
            step();
            require(Clock::now() < deadline, "shell exit stalled");
        } while (result_.status != ORT_ARM64_EXITED);
        require(result_.exit_code == 7, "shell exit status was lost");
        require(result_.unsupported_syscalls == 0, "terminal used unsupported syscall");
        require(ort_arm64_terminal_close(vm_.get()) == 0, error());
    }
    size_t size() const {
        return output_.size();
    }

  private:
    std::string error() const {
        return ort_arm64_last_error(vm_.get());
    }
    void step() {
        result_ = ort_arm64_run(vm_.get(), 50000);
        require(result_.status != ORT_ARM64_FAULT && result_.status != ORT_ARM64_UNSUPPORTED, error());
        char bytes[8192];
        for (;;) {
            const auto count = ort_arm64_terminal_read(vm_.get(), bytes, sizeof(bytes));
            if (count == 0 || count == -11 || (count == -5 && result_.status == ORT_ARM64_EXITED))
                break;
            require(count > 0, error());
            output_.append(bytes, size_t(count));
        }
        if (result_.status == ORT_ARM64_WAITING)
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    std::unique_ptr<ort_arm64_vm, decltype(&ort_arm64_destroy)> vm_;
    ort_arm64_result result_{};
    std::string output_;
};
} // namespace
int main(int argc, char **argv) {
    try {
        require(argc == 2, "Alpine root argument required");
        Terminal terminal(argv[1]);
        terminal.expect("ORT> ");
        terminal.write("stty -echo\n");
        terminal.wait();
        terminal.resize();
        auto start = terminal.size();
        terminal.write("stty size; printf '__TERMINAL_OK__\\n'\n");
        terminal.expect("37 101\r\n", start);
        terminal.expect("__TERMINAL_OK__\r\n", start);
        terminal.wait();
        terminal.write("sleep 30\n");
        terminal.wait();
        start = terminal.size();
        terminal.write(std::string(1, '\x03'));
        terminal.expect("ORT> ", start);
        terminal.write("sleep 30\n");
        terminal.wait();
        start = terminal.size();
        terminal.interrupt();
        terminal.expect("ORT> ", start);
        start = terminal.size();
        terminal.write("printf '__AFTER_INTERRUPT__\\n'\n");
        terminal.expect("__AFTER_INTERRUPT__\r\n", start);
        terminal.exit();
        std::puts("public terminal API: shell, input/output, resize, Ctrl-C, foreground signal, exit passed");
        return 0;
    } catch (const std::exception &failure) {
        std::fprintf(stderr, "%s\n", failure.what());
        return 1;
    }
}
