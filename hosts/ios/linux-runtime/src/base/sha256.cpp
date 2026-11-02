#include "base/sha256.h"
#include "base/support.h"
#ifdef __APPLE__
#include <CommonCrypto/CommonDigest.h>
#else
#include <openssl/sha.h>
#endif
namespace operit::arm64 {
std::array<uint8_t, 32> sha256(const void *data, size_t length) {
    std::array<uint8_t, 32> result{};
#ifdef __APPLE__
    if (length > UINT32_MAX)
        throw Failure("SHA-256 input exceeds component limit");
    CC_SHA256(data, static_cast<CC_LONG>(length), result.data());
#else
    SHA256(static_cast<const unsigned char *>(data), length, result.data());
#endif
    return result;
}
} // namespace operit::arm64
