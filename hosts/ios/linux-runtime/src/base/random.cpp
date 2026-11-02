#include "base/random.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cstdlib>
#include <unistd.h>
#ifndef __APPLE__
#include <sys/random.h>
#endif
namespace operit::arm64 {
void random_bytes(void *destination, size_t length) {
#ifdef __APPLE__
    ::arc4random_buf(destination, length);
#else
    auto *bytes = static_cast<unsigned char *>(destination);
    for (size_t offset = 0; offset < length; offset += 256)
        check_host(::getentropy(bytes + offset, std::min<size_t>(256, length - offset)));
#endif
}
} // namespace operit::arm64
