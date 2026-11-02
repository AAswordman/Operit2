#include "fs/virtual/node.h"
#include "linux/terminal/manager.h"

namespace operit::arm64 {
VirtualNode::Ptr device_filesystem(const std::shared_ptr<TerminalManager> &terminals) {
    auto root = std::make_shared<VirtualNode>(VirtualNode::Kind::Directory, "/dev", 0555);
    root->names = [] {
        return std::vector<std::string>{"null", "zero",  "full",   "random", "urandom",
                                        "fd",   "stdin", "stdout", "stderr", "ptmx", "pts", "tty"};
    };
    root->lookup = [terminals](const std::string &name) -> VirtualNode::Ptr {
        if (name == "ptmx")
            return terminals->multiplexer_node();
        if (name == "pts")
            return terminals->slave_filesystem();
        if (name == "tty")
            return terminals->controlling_node();
        using Kind = VirtualNode::Kind;
        Kind kind;
        if (name == "null")
            kind = Kind::Null;
        else if (name == "zero")
            kind = Kind::Zero;
        else if (name == "full")
            kind = Kind::Full;
        else if (name == "random" || name == "urandom")
            kind = Kind::Random;
        else if (name == "fd" || name == "stdin" || name == "stdout" || name == "stderr") {
            auto link = std::make_shared<VirtualNode>(Kind::Symlink, "/dev/" + name, 0777);
            link->target = [name] {
                return name == "fd" ? "/proc/self/fd"
                                    : "/proc/self/fd/" + std::string(name == "stdin"    ? "0"
                                                                     : name == "stdout" ? "1"
                                                                                        : "2");
            };
            return link;
        } else
            return {};
        return std::make_shared<VirtualNode>(kind, "/dev/" + name, 0666);
    };
    return root;
}
} // namespace operit::arm64
