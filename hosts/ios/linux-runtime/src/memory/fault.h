#pragma once
#include "base/support.h"

namespace operit::arm64 {
struct MemoryFault : Failure {
    enum class Kind { Unmapped, Protection, Bus };
    uint64_t address;
    unsigned permissions;
    Kind kind;
    MemoryFault(uint64_t at, unsigned access, Kind reason)
        : Failure(std::string(reason == Kind::Unmapped     ? "unmapped guest address "
                              : reason == Kind::Protection ? "guest memory permission fault at "
                                                           : "guest file mapping bus fault at ") +
                  hex(at)),
          address(at), permissions(access), kind(reason) {}
};
} // namespace operit::arm64
