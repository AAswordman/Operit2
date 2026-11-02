#pragma once
#include "runtime/process/process.h"

namespace operit::arm64 {
void load_process_executable(ProcessImage &image, const std::string &path,
                             const std::vector<std::string> &arguments,
                             const std::vector<std::string> &environment,
                             const std::string &working_directory);
} // namespace operit::arm64
