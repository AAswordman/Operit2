#pragma once
#include "acceleration/intrinsic_context.h"
#include "acceleration/musl/copy.h"
#include "acceleration/scalar_ops.h"

namespace operit::arm64::fixed {
inline void move(IntrinsicContext &c) {
    auto &cpu = c.cpu;
    auto &memory = c.memory;
    auto &x = cpu.x;
    goto L5e398;
L5e398:
    {
        if (!c.begin<0x5e398, 5>())
            return;
        x[5] = x[0];
        x[4] = x[1];
        x[3] = x[2];
        scalar::arithmetic<64>(cpu, x[0], x[1], decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 0)) {
            c.finish<5, 0x5e444>();
            goto L5e444;
        }
        c.finish<5, 0x5e3ac>();
        goto L5e3ac;
    }
L5e3ac:
    {
        if (!c.begin<0x5e3ac, 5>())
            return;
        x[6] = x[1] - x[2];
        x[7] = uint64_t{0} - (x[2] << 1);
        x[6] = x[6] - x[0];
        scalar::arithmetic<64>(cpu, x[6], x[7], decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 9)) {
            c.finish<5, 0x5e3e0>();
            goto L5e3e0;
        }
        c.finish<5, 0x5e3c0>();
        goto L5e3c0;
    }
L5e3c0:
    {
        if (!c.begin<0x5e3c0, 4>())
            return;
        x[0] = x[1] ^ x[0];
        x[0] = x[0] & 0x7;
        scalar::arithmetic<64>(cpu, x[5], x[1], decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 2)) {
            c.finish<4, 0x5e44c>();
            goto L5e44c;
        }
        c.finish<4, 0x5e3d0>();
        goto L5e3d0;
    }
L5e3d0:
    {
        if (!c.begin<0x5e3d0, 1>())
            return;
        if (x[0] == 0) {
            c.finish<1, 0x5e3e4>();
            goto L5e3e4;
        }
        c.finish<1, 0x5e3d4>();
        goto L5e3d4;
    }
L5e3d4:
    {
        if (!c.begin<0x5e3d4, 1>())
            return;
        x[1] = x[5];
        c.finish<1, 0x5e3d8>();
        goto L5e3d8;
    }
L5e3d8:
    {
        if (!c.begin<0x5e3d8, 2>())
            return;
        x[0] = 0x0;
        c.finish<2, 0x5e43c>();
        goto L5e43c;
    }
L5e3e0:
    {
        if (!c.begin<0x5e3e0, 1>())
            return;
        c.finish<1, 0x18290>();
        copy(c);
        return;
    }
L5e3e4:
    {
        if (!c.begin<0x5e3e4, 1>())
            return;
        x[0] = x[5];
        c.finish<1, 0x5e3e8>();
        goto L5e3e8;
    }
L5e3e8:
    {
        if (!c.begin<0x5e3e8, 2>())
            return;
        x[2] = x[0] & 0x7;
        cpu.logical_flags(x[2], 64);
        if (scalar::condition(cpu, 0)) {
            c.finish<2, 0x5e404>();
            goto L5e404;
        }
        c.finish<2, 0x5e3f0>();
        goto L5e3f0;
    }
L5e3f0:
    {
        if (!c.begin<0x5e3f0, 1>())
            return;
        if (x[3] == 0) {
            c.finish<1, 0x5e444>();
            goto L5e444;
        }
        c.finish<1, 0x5e3f4>();
        goto L5e3f4;
    }
L5e3f4:
    {
        if (!c.begin<0x5e3f4, 4>())
            return;
        {
            c.access<0x5e3f4, 0>();
            x[1] = uint32_t(memory.load<1>(x[4]));
            x[4] = x[4] + 0x1;
        }
        x[3] = x[3] - 0x1;
        {
            c.access<0x5e3fc, 2>();
            memory.store<1>(x[0], uint32_t(x[1]));
            x[0] = x[0] + 0x1;
            if (!c.stored<0x5e400, 3>())
                return;
        }
        c.finish<4, 0x5e3e8>();
        goto L5e3e8;
    }
L5e404:
    {
        if (!c.begin<0x5e404, 2>())
            return;
        x[6] = x[3] & 0xfffffffffffffff8ull;
        c.finish<2, 0x5e418>();
        goto L5e418;
    }
L5e40c:
    {
        if (!c.begin<0x5e40c, 3>())
            return;
        {
            c.access<0x5e40c, 0>();
            x[1] = memory.load<8>(x[4] + x[2]);
        }
        {
            c.access<0x5e410, 1>();
            memory.store<8>(x[0] + x[2], x[1]);
            if (!c.stored<0x5e414, 2>())
                return;
        }
        x[2] = x[2] + 0x8;
        c.finish<3, 0x5e418>();
        goto L5e418;
    }
L5e418:
    {
        if (!c.begin<0x5e418, 2>())
            return;
        scalar::arithmetic<64>(cpu, x[2], x[6], decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 1)) {
            c.finish<2, 0x5e40c>();
            goto L5e40c;
        }
        c.finish<2, 0x5e420>();
        goto L5e420;
    }
L5e420:
    {
        if (!c.begin<0x5e420, 4>())
            return;
        x[4] = x[4] + x[2];
        x[1] = x[0] + x[2];
        x[3] = x[3] - x[2];
        c.finish<4, 0x5e3d8>();
        goto L5e3d8;
    }
L5e430:
    {
        if (!c.begin<0x5e430, 3>())
            return;
        {
            c.access<0x5e430, 0>();
            x[2] = uint32_t(memory.load<1>(x[4] + x[0]));
        }
        {
            c.access<0x5e434, 1>();
            memory.store<1>(x[1] + x[0], uint32_t(x[2]));
            if (!c.stored<0x5e438, 2>())
                return;
        }
        x[0] = x[0] + 0x1;
        c.finish<3, 0x5e43c>();
        goto L5e43c;
    }
L5e43c:
    {
        if (!c.begin<0x5e43c, 2>())
            return;
        scalar::arithmetic<64>(cpu, x[3], x[0], decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 1)) {
            c.finish<2, 0x5e430>();
            goto L5e430;
        }
        c.finish<2, 0x5e444>();
        goto L5e444;
    }
L5e444:
    {
        if (!c.begin<0x5e444, 2>())
            return;
        x[0] = x[5];
        c.returned<2>();
        return;
    }
L5e44c:
    {
        if (!c.begin<0x5e44c, 1>())
            return;
        if (x[0] != 0) {
            c.finish<1, 0x5e4a4>();
            goto L5e4a4;
        }
        c.finish<1, 0x5e450>();
        goto L5e450;
    }
L5e450:
    {
        if (!c.begin<0x5e450, 3>())
            return;
        x[0] = x[5] + x[3];
        cpu.logical_flags(x[0] & 0x7, 64);
        if (scalar::condition(cpu, 0)) {
            c.finish<3, 0x5e470>();
            goto L5e470;
        }
        c.finish<3, 0x5e45c>();
        goto L5e45c;
    }
L5e45c:
    {
        if (!c.begin<0x5e45c, 1>())
            return;
        if (x[3] == 0) {
            c.finish<1, 0x5e444>();
            goto L5e444;
        }
        c.finish<1, 0x5e460>();
        goto L5e460;
    }
L5e460:
    {
        if (!c.begin<0x5e460, 4>())
            return;
        x[3] = x[3] - 0x1;
        {
            c.access<0x5e464, 1>();
            x[0] = uint32_t(memory.load<1>(x[4] + x[3]));
        }
        {
            c.access<0x5e468, 2>();
            memory.store<1>(x[5] + x[3], uint32_t(x[0]));
            if (!c.stored<0x5e46c, 3>())
                return;
        }
        c.finish<4, 0x5e450>();
        goto L5e450;
    }
L5e470:
    {
        if (!c.begin<0x5e470, 3>())
            return;
        x[2] = x[3] & 0x7;
        x[0] = x[3];
        c.finish<3, 0x5e488>();
        goto L5e488;
    }
L5e47c:
    {
        if (!c.begin<0x5e47c, 3>())
            return;
        x[0] = x[0] - 0x8;
        {
            c.access<0x5e480, 1>();
            x[1] = memory.load<8>(x[4] + x[0]);
        }
        {
            c.access<0x5e484, 2>();
            memory.store<8>(x[5] + x[0], x[1]);
            if (!c.stored<0x5e488, 3>())
                return;
        }
        c.finish<3, 0x5e488>();
        goto L5e488;
    }
L5e488:
    {
        if (!c.begin<0x5e488, 2>())
            return;
        scalar::arithmetic<64>(cpu, x[0], x[2], decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 1)) {
            c.finish<2, 0x5e47c>();
            goto L5e47c;
        }
        c.finish<2, 0x5e490>();
        goto L5e490;
    }
L5e490:
    {
        if (!c.begin<0x5e490, 2>())
            return;
        x[3] = x[3] & 0x7;
        c.finish<2, 0x5e4a4>();
        goto L5e4a4;
    }
L5e498:
    {
        if (!c.begin<0x5e498, 3>())
            return;
        x[3] = x[3] - 0x1;
        {
            c.access<0x5e49c, 1>();
            x[0] = uint32_t(memory.load<1>(x[4] + x[3]));
        }
        {
            c.access<0x5e4a0, 2>();
            memory.store<1>(x[5] + x[3], uint32_t(x[0]));
            if (!c.stored<0x5e4a4, 3>())
                return;
        }
        c.finish<3, 0x5e4a4>();
        goto L5e4a4;
    }
L5e4a4:
    {
        if (!c.begin<0x5e4a4, 1>())
            return;
        if (x[3] != 0) {
            c.finish<1, 0x5e498>();
            goto L5e498;
        }
        c.finish<1, 0x5e4a8>();
        goto L5e4a8;
    }
L5e4a8:
    {
        if (!c.begin<0x5e4a8, 1>())
            return;
        c.finish<1, 0x5e444>();
        goto L5e444;
    }
}
} // namespace operit::arm64::fixed
