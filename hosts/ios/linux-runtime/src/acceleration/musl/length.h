#pragma once
#include "acceleration/fixed_execution.h"
#include "acceleration/conditions.h"

namespace operit::arm64::fixed {
inline void length(FixedExecution &c) {
    auto &cpu = c.cpu;
    auto &memory = c.memory;
    auto &x = cpu.x;
    goto L5ec28;
L5ec28:
    {
        if (!c.begin<0x5ec28, 2>())
            return;
        x[1] = x[0];
        c.finish<2, 0x5ec34>();
        goto L5ec34;
    }
L5ec30:
    {
        if (!c.begin<0x5ec30, 1>())
            return;
        x[1] = x[1] + 0x1;
        c.finish<1, 0x5ec34>();
        goto L5ec34;
    }
L5ec34:
    {
        if (!c.begin<0x5ec34, 2>())
            return;
        cpu.logical_flags(x[1] & 0x7, 64);
        if (scalar::condition(cpu, 0)) {
            c.finish<2, 0x5ec48>();
            goto L5ec48;
        }
        c.finish<2, 0x5ec3c>();
        goto L5ec3c;
    }
L5ec3c:
    {
        if (!c.begin<0x5ec3c, 2>())
            return;
        {
            c.access<0x5ec3c, 0>();
            x[2] = uint32_t(memory.load<1>(x[1]));
        }
        if (uint32_t(x[2]) != 0) {
            c.finish<2, 0x5ec30>();
            goto L5ec30;
        }
        c.finish<2, 0x5ec44>();
        goto L5ec44;
    }
L5ec44:
    {
        if (!c.begin<0x5ec44, 1>())
            return;
        c.finish<1, 0x5ec7c>();
        goto L5ec7c;
    }
L5ec48:
    {
        if (!c.begin<0x5ec48, 3>())
            return;
        x[4] = -0x101010101010102ull;
        x[4] = (x[4] & ~uint64_t{0xffff}) | 0xfeff;
        c.finish<3, 0x5ec58>();
        goto L5ec58;
    }
L5ec54:
    {
        if (!c.begin<0x5ec54, 1>())
            return;
        x[1] = x[1] + 0x8;
        c.finish<1, 0x5ec58>();
        goto L5ec58;
    }
L5ec58:
    {
        if (!c.begin<0x5ec58, 5>())
            return;
        {
            c.access<0x5ec58, 0>();
            x[3] = memory.load<8>(x[1]);
        }
        x[2] = x[3] + x[4];
        x[2] = x[2] & ~x[3];
        cpu.logical_flags(x[2] & 0x8080808080808080ull, 64);
        if (scalar::condition(cpu, 0)) {
            c.finish<5, 0x5ec54>();
            goto L5ec54;
        }
        c.finish<5, 0x5ec6c>();
        goto L5ec6c;
    }
L5ec6c:
    {
        if (!c.begin<0x5ec6c, 1>())
            return;
        c.finish<1, 0x5ec74>();
        goto L5ec74;
    }
L5ec70:
    {
        if (!c.begin<0x5ec70, 1>())
            return;
        x[1] = x[1] + 0x1;
        c.finish<1, 0x5ec74>();
        goto L5ec74;
    }
L5ec74:
    {
        if (!c.begin<0x5ec74, 2>())
            return;
        {
            c.access<0x5ec74, 0>();
            x[2] = uint32_t(memory.load<1>(x[1]));
        }
        if (uint32_t(x[2]) != 0) {
            c.finish<2, 0x5ec70>();
            goto L5ec70;
        }
        c.finish<2, 0x5ec7c>();
        goto L5ec7c;
    }
L5ec7c:
    {
        if (!c.begin<0x5ec7c, 2>())
            return;
        x[0] = x[1] - x[0];
        c.returned<2>();
        return;
    }
}
} // namespace operit::arm64::fixed
