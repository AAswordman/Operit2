#include "linux/network/resolver.h"
#include <arpa/inet.h>
#include <resolv.h>
#include <sstream>

namespace operit::arm64 {
std::string host_resolver_configuration() {
    struct __res_state state{};
    if (::res_ninit(&state) != 0)
        return {};
    std::ostringstream text;
#ifdef __APPLE__
    union res_sockaddr_union servers[MAXNS]{};
    int count = ::res_getservers(&state, servers, MAXNS);
    for (int i = 0; i < count && i < MAXNS; ++i) {
        char address[INET6_ADDRSTRLEN];
        const void *source =
            servers[i].sin.sin_family == AF_INET      ? static_cast<const void *>(&servers[i].sin.sin_addr)
            : servers[i].sin6.sin6_family == AF_INET6 ? static_cast<const void *>(&servers[i].sin6.sin6_addr)
                                                      : nullptr;
        if (source && ::inet_ntop(servers[i].sin.sin_family, source, address, sizeof(address)))
            text << "nameserver " << address << '\n';
    }
#else
    for (int i = 0; i < state.nscount && i < MAXNS; ++i) {
        char address[INET_ADDRSTRLEN];
        if (::inet_ntop(AF_INET, &state.nsaddr_list[i].sin_addr, address, sizeof(address)))
            text << "nameserver " << address << '\n';
    }
#endif
    if (state.defdname[0])
        text << "search " << state.defdname << '\n';
#ifdef __APPLE__
    ::res_ndestroy(&state);
#else
    ::res_nclose(&state);
#endif
    return text.str();
}
} // namespace operit::arm64
