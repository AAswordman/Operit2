#include "fs/virtual/procfs.h"
#include "base/support.h"
#include <algorithm>
#include <climits>

namespace operit::arm64 {
namespace {
using Node = VirtualNode;
int number(const std::string &name) {
    if (name.empty() || (name.size() > 1 && name[0] == '0'))
        return -1;
    uint64_t value = 0;
    for (char c : name) {
        if (c < '0' || c > '9')
            return -1;
        value = value * 10 + unsigned(c - '0');
        if (value > INT_MAX)
            return -1;
    }
    return int(value);
}
Node::Ptr file(const std::string &path, std::function<std::string()> contents, mode_t permissions = 0444) {
    auto node = std::make_shared<Node>(Node::Kind::File, path, permissions);
    node->contents = std::move(contents);
    node->sequence = true;
    return node;
}
Node::Ptr link(const std::string &path, std::function<std::string()> target) {
    auto node = std::make_shared<Node>(Node::Kind::Symlink, path, 0777);
    node->target = std::move(target);
    return node;
}
void task_exists(const ProcSource &source, uint32_t tid) {
    auto tasks = source.tasks();
    if (std::find(tasks.begin(), tasks.end(), tid) == tasks.end())
        guest_error(2);
}
Node::Ptr process_directory(std::shared_ptr<ProcSource> source, uint32_t tid, const std::string &path,
                  bool whole_group = true) {
    task_exists(*source, tid);
    auto root = std::make_shared<Node>(Node::Kind::Directory, path, 0555);
    root->names = [source, tid] {
        task_exists(*source, tid);
        return std::vector<std::string>{"cmdline", "environ", "auxv", "status", "stat", "maps", "statm",
                                        "comm", "task", "fd", "fdinfo", "cwd", "exe", "root", "mounts"};
    };
    root->lookup = [source, tid, path, whole_group](const std::string &name) -> Node::Ptr {
        task_exists(*source, tid);
        std::string child = path + "/" + name;
        if (name == "task") {
            auto node = std::make_shared<Node>(Node::Kind::Directory, child, 0555);
            node->names = [source, tid] {
                task_exists(*source, tid);
                std::vector<std::string> result;
                for (auto id : source->tasks())
                    result.push_back(std::to_string(id));
                return result;
            };
            node->lookup = [source, tid, child](const std::string &id) -> Node::Ptr {
                task_exists(*source, tid);
                int parsed = number(id);
                auto tasks = source->tasks();
                if (parsed < 1 || std::find(tasks.begin(), tasks.end(), uint32_t(parsed)) == tasks.end())
                    return {};
                return process_directory(source, parsed, child + "/" + id, false);
            };
            return node;
        }
        if (name == "fd" || name == "fdinfo") {
            auto node = std::make_shared<Node>(Node::Kind::Directory, child, 0500);
            node->names = [source, tid] {
                task_exists(*source, tid);
                std::vector<std::string> result;
                for (int id : source->descriptors())
                    result.push_back(std::to_string(id));
                return result;
            };
            node->lookup = [source, tid, child, name](const std::string &id) -> Node::Ptr {
                task_exists(*source, tid);
                int fd = number(id);
                auto fds = source->descriptors();
                if (fd < 0 || std::find(fds.begin(), fds.end(), fd) == fds.end())
                    return {};
                if (name == "fd")
                    return source->descriptor_link(fd);
                return file(
                    child + "/" + id,
                    [source, tid, fd] {
                        task_exists(*source, tid);
                        return source->descriptor_info(fd);
                    },
                    0400);
            };
            return node;
        }
        if (name == "cwd" || name == "exe" || name == "root")
            return link(child, [source, tid, name] {
                task_exists(*source, tid);
                return source->link(tid, name);
            });
        const std::vector<std::string> files{"cmdline", "environ", "auxv", "status", "stat",
                                             "maps", "statm", "comm", "mounts"};
        if (std::find(files.begin(), files.end(), name) == files.end())
            return {};
        auto result = file(
            child,
            [source, tid, name, whole_group] {
                task_exists(*source, tid);
                if (name == "stat" && source->stat_contents)
                    return source->stat_contents(tid, whole_group);
                return source->contents(tid, name);
            },
            name == "comm"                        ? 0644
            : name == "environ" || name == "auxv" ? 0400
                                                  : 0444);
        if (name == "cmdline" || name == "environ" || name == "auxv")
            result->read_range = [source, tid, name](uint64_t offset, size_t count) {
                task_exists(*source, tid);
                if (source->read_range)
                    return source->read_range(tid, name, offset, count);
                // Older embedders still fetch fresh contents for every read.
                auto contents = source->contents(tid, name);
                return offset >= contents.size() ? std::string{} : contents.substr(size_t(offset), count);
            };
        if (name == "comm")
            result->write_contents = [source, tid](const std::string &value) {
                task_exists(*source, tid);
                return source->set_name(tid, value);
            };
        return result;
    };
    return root;
}
} // namespace
VirtualNode::Ptr process_filesystem(ProcSource value) {
    auto source = std::make_shared<ProcSource>(std::move(value));
    auto root = std::make_shared<Node>(Node::Kind::Directory, "/proc", 0555);
    root->names = [source] {
        std::vector<std::string> names{"self", "thread-self", "mounts", "meminfo"};
        for (auto pid : source->processes())
            names.push_back(std::to_string(pid));
        return names;
    };
    root->lookup = [source](const std::string &name) -> Node::Ptr {
        if (name == "self")
            return link("/proc/self", [source] { return std::to_string(source->current_pid()); });
        if (name == "thread-self")
            return link("/proc/thread-self", [source] {
                return std::to_string(source->current_pid()) + "/task/" +
                       std::to_string(source->current_tid());
            });
        if (name == "mounts")
            return link("/proc/mounts", [] { return "self/mounts"; });
        if (name == "meminfo")
            return file("/proc/meminfo", [source] { return source->contents(0, "meminfo"); });
        int pid = number(name);
        auto pids = source->processes();
        if (pid > 0 && std::find(pids.begin(), pids.end(), uint32_t(pid)) != pids.end())
            return process_directory(std::make_shared<ProcSource>(source->process(uint32_t(pid))),
                                     uint32_t(pid), "/proc/" + name);
        return {};
    };
    return root;
}
} // namespace operit::arm64
