#pragma once
#include "fs/virtual/node.h"

namespace operit::arm64 {
struct ProcSource {
    std::function<std::vector<uint32_t>()> processes;
    std::function<uint32_t()> current_pid;
    std::function<ProcSource(uint32_t)> process;
    std::function<std::vector<uint32_t>()> tasks;
    std::function<uint32_t()> current_tid;
    std::function<std::vector<int>()> descriptors;
    std::function<std::string(uint32_t, const std::string &)> contents;
    std::function<std::string(uint32_t, const std::string &)> link;
    std::function<size_t(uint32_t, const std::string &)> set_name;
    std::function<VirtualNode::Ptr(int)> descriptor_link;
    std::function<std::string(int)> descriptor_info;
    std::function<std::string(uint32_t, const std::string &, uint64_t, size_t)> read_range;
    std::function<std::string(uint32_t tid, bool whole_group)> stat_contents;
};
VirtualNode::Ptr process_filesystem(ProcSource source);
} // namespace operit::arm64
