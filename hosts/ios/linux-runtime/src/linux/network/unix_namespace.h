#pragma once
#include "fs/filesystem.h"
#include <map>

namespace operit::arm64 {
struct SocketState;
class UnixNamespace {
    std::string temporary_;
    std::map<std::string, std::string> aliases_, abstract_;
    void prepare();

  public:
    ~UnixNamespace();
    std::string pathname(const Path &path);
    std::string abstract(const std::string &name);
};
struct NetworkNamespace {
    UnixNamespace unix;
    std::map<std::string, std::vector<uint8_t>> address_names;
    std::vector<std::weak_ptr<SocketState>> sockets;
    ~NetworkNamespace();
};
} // namespace operit::arm64
