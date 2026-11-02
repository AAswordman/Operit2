#pragma once
#include "base/fd.h"
#include "fs/virtual/node.h"
#include <cstdint>
#include <map>
#include <memory>
#include <string>
#include <vector>

namespace operit::arm64::mounts {
constexpr uint64_t read_only = 1, no_suid = 2, no_devices = 4, no_execute = 8;
constexpr uint64_t remount = 32, bind = 4096, recursive = 16384, silent = 32768;
constexpr uint32_t detach = 2, no_follow = 8;
class TemporaryTree;
// A native directory, an inode pinned by a private hard link, or a contextual
// virtual subtree. Every native open uses openat, never a duplicated file OFD.
struct Source {
    FD root;
    std::string name = ".";
    bool directory = true;
    bool locked_read_only = false;
    std::shared_ptr<TemporaryTree> storage;
    std::string context;
    std::vector<std::string> components;
    VirtualNode::Ptr node;
};
struct Point {
    uint64_t id = 0, parent = 0, flags = 0;
    std::string target, type, label;
    std::shared_ptr<Source> source;
    bool read_only() const {
        return (flags & mounts::read_only) || source->locked_read_only;
    }
};
struct Namespace {
    uint64_t next_id = 1;
    std::shared_ptr<Point> root;
    std::vector<std::shared_ptr<Point>> points;
    // Register built-in mounts once; a fork only rebinds its local context.
    std::map<std::string, std::string> builtins;
};
} // namespace operit::arm64::mounts
