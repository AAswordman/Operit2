#include "runtime/process/image_loader.h"
#include "elf/loader.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace operit::arm64 {
void load_process_executable(ProcessImage &image, const std::string &path,
                             const std::vector<std::string> &arguments,
                             const std::vector<std::string> &environment,
                             const std::string &working_directory) {
    std::string executable = path;
    std::vector<std::string> args = arguments;
    if (args.empty())
        args.emplace_back("");
    for (unsigned recursion = 0;; ++recursion) {
        const auto source = image.filesystem->resolve(executable);
        const auto info = source.stat();
        if (!S_ISREG(info.st_mode) || !(info.st_mode & 0111))
            guest_error(13);
        executable = source.guest;
        auto descriptor = image.filesystem->open(executable, "/", O_RDONLY, 0);
        std::array<uint8_t, 256> header{};
        const auto length = ::pread(descriptor.get(), header.data(), header.size(), 0);
        check_host(length);
        std::vector<uint8_t> bytes(header.begin(), header.begin() + length);
        if (bytes.size() >= 2 && bytes[0] == '#' && bytes[1] == '!') {
            if (recursion == 5)
                guest_error(40);
            const auto end = std::find(bytes.begin(), bytes.end(), uint8_t('\n'));
            if (end == bytes.end() && bytes.size() >= 256)
                guest_error(8);
            std::string line(bytes.begin() + 2, end);
            const auto begin = line.find_first_not_of(" \t");
            if (begin == std::string::npos)
                guest_error(8);
            const auto split = line.find_first_of(" \t", begin);
            std::string interpreter = line.substr(begin, split - begin);
            std::vector<std::string> replacement{interpreter};
            if (split != std::string::npos) {
                auto optional = line.find_first_not_of(" \t", split);
                auto last = line.find_last_not_of(" \t");
                if (optional != std::string::npos)
                    replacement.push_back(line.substr(optional, last - optional + 1));
            }
            replacement.push_back(executable);
            replacement.insert(replacement.end(), args.begin() + 1, args.end());
            args = std::move(replacement);
            executable = image.filesystem->resolve(interpreter, working_directory).guest;
            continue;
        }
        break;
    }
    std::vector<const char *> argv, envp;
    for (const auto &argument : args)
        argv.push_back(argument.c_str());
    for (const auto &entry : environment)
        envp.push_back(entry.c_str());
    argv.push_back(nullptr);
    envp.push_back(nullptr);
    try {
        ElfLoader loader(*image.memory, *image.filesystem, image.memory->limit(), *image.acceleration);
        const auto main = loader.load(executable, 0x40000000);
        Image interpreter{};
        if (!main.interpreter.empty())
            interpreter = loader.load(main.interpreter, 0x6000000000, true);
        image.entry = {};
        image.entry.sp = build_initial_stack(*image.memory, argv.data(), envp.data(), main, interpreter.base);
        image.entry.pc = main.interpreter.empty() ? main.entry : interpreter.entry;
        image.heap_end = main.end;
        image.executable = executable;
        image.signal_trampoline = signals::install_trampoline(*image.memory);
    } catch (const std::bad_alloc &) {
        guest_error(12);
    } catch (const Failure &) {
        guest_error(8);
    }
}
} // namespace operit::arm64
