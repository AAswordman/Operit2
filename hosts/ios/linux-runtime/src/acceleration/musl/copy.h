#pragma once
#include "acceleration/intrinsic_context.h"
#include "acceleration/scalar_ops.h"

namespace operit::arm64::fixed {
inline void copy(IntrinsicContext &c, bool loop_only = false) {
    auto &cpu = c.cpu;
    auto &memory = c.memory;
    auto &x = cpu.x;
    if (loop_only)
        goto L183c0;
    goto L18290;
L18290:
    {
        if (!c.begin<0x18290, 4>())
            return;
        x[4] = x[1] + x[2];
        x[5] = x[0] + x[2];
        scalar::arithmetic<64>(cpu, x[2], 0x80, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 8)) {
            c.finish<4, 0x18390>();
            goto L18390;
        }
        c.finish<4, 0x182a0>();
        goto L182a0;
    }
L182a0:
    {
        if (!c.begin<0x182a0, 2>())
            return;
        scalar::arithmetic<64>(cpu, x[2], 0x20, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 8)) {
            c.finish<2, 0x18320>();
            goto L18320;
        }
        c.finish<2, 0x182a8>();
        goto L182a8;
    }
L182a8:
    {
        if (!c.begin<0x182a8, 2>())
            return;
        scalar::arithmetic<64>(cpu, x[2], 0x10, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 3)) {
            c.finish<2, 0x182c4>();
            goto L182c4;
        }
        c.finish<2, 0x182b0>();
        goto L182b0;
    }
L182b0:
    {
        if (!c.begin<0x182b0, 5>())
            return;
        {
            c.access<0x182b0, 0>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1], values.data());
            x[6] = values[0];
            x[7] = values[1];
        }
        {
            c.access<0x182b4, 1>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x10, values.data());
            x[12] = values[0];
            x[13] = values[1];
        }
        {
            c.access<0x182b8, 2>();
            std::array<uint64_t, 2> values{x[6], x[7]};
            memory.copy_in<sizeof(values)>(x[0], values.data());
            if (!c.stored<0x182bc, 3>())
                return;
        }
        {
            c.access<0x182bc, 3>();
            std::array<uint64_t, 2> values{x[12], x[13]};
            memory.copy_in<sizeof(values)>(x[5] + -0x10, values.data());
            if (!c.stored<0x182c0, 4>())
                return;
        }
        c.returned<5>();
        return;
    }
L182c4:
    {
        if (!c.begin<0x182c4, 1>())
            return;
        if (((uint32_t(x[2]) >> 3) & 1) == 0) {
            c.finish<1, 0x182e0>();
            goto L182e0;
        }
        c.finish<1, 0x182c8>();
        goto L182c8;
    }
L182c8:
    {
        if (!c.begin<0x182c8, 5>())
            return;
        {
            c.access<0x182c8, 0>();
            x[6] = memory.load<8>(x[1]);
        }
        {
            c.access<0x182cc, 1>();
            x[7] = memory.load<8>(x[4] + -0x8);
        }
        {
            c.access<0x182d0, 2>();
            memory.store<8>(x[0], x[6]);
            if (!c.stored<0x182d4, 3>())
                return;
        }
        {
            c.access<0x182d4, 3>();
            memory.store<8>(x[5] + -0x8, x[7]);
            if (!c.stored<0x182d8, 4>())
                return;
        }
        c.returned<5>();
        return;
    }
L182e0:
    {
        if (!c.begin<0x182e0, 1>())
            return;
        if (((uint32_t(x[2]) >> 2) & 1) == 0) {
            c.finish<1, 0x182f8>();
            goto L182f8;
        }
        c.finish<1, 0x182e4>();
        goto L182e4;
    }
L182e4:
    {
        if (!c.begin<0x182e4, 5>())
            return;
        {
            c.access<0x182e4, 0>();
            x[6] = uint32_t(memory.load<4>(x[1]));
        }
        {
            c.access<0x182e8, 1>();
            x[8] = uint32_t(memory.load<4>(x[4] + -0x4));
        }
        {
            c.access<0x182ec, 2>();
            memory.store<4>(x[0], uint32_t(x[6]));
            if (!c.stored<0x182f0, 3>())
                return;
        }
        {
            c.access<0x182f0, 3>();
            memory.store<4>(x[5] + -0x4, uint32_t(x[8]));
            if (!c.stored<0x182f4, 4>())
                return;
        }
        c.returned<5>();
        return;
    }
L182f8:
    {
        if (!c.begin<0x182f8, 1>())
            return;
        if (x[2] == 0) {
            c.finish<1, 0x18318>();
            goto L18318;
        }
        c.finish<1, 0x182fc>();
        goto L182fc;
    }
L182fc:
    {
        if (!c.begin<0x182fc, 7>())
            return;
        x[14] = x[2] >> 1;
        {
            c.access<0x18300, 1>();
            x[6] = uint32_t(memory.load<1>(x[1]));
        }
        {
            c.access<0x18304, 2>();
            x[10] = uint32_t(memory.load<1>(x[4] + -0x1));
        }
        {
            c.access<0x18308, 3>();
            x[8] = uint32_t(memory.load<1>(x[1] + x[14]));
        }
        {
            c.access<0x1830c, 4>();
            memory.store<1>(x[0], uint32_t(x[6]));
            if (!c.stored<0x18310, 5>())
                return;
        }
        {
            c.access<0x18310, 5>();
            memory.store<1>(x[0] + x[14], uint32_t(x[8]));
            if (!c.stored<0x18314, 6>())
                return;
        }
        {
            c.access<0x18314, 6>();
            memory.store<1>(x[5] + -0x1, uint32_t(x[10]));
            if (!c.stored<0x18318, 7>())
                return;
        }
        c.finish<7, 0x18318>();
        goto L18318;
    }
L18318:
    {
        if (!c.begin<0x18318, 1>())
            return;
        c.returned<1>();
        return;
    }
L18320:
    {
        if (!c.begin<0x18320, 6>())
            return;
        {
            c.access<0x18320, 0>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1], values.data());
            x[6] = values[0];
            x[7] = values[1];
        }
        {
            c.access<0x18324, 1>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x10, values.data());
            x[8] = values[0];
            x[9] = values[1];
        }
        {
            c.access<0x18328, 2>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x20, values.data());
            x[10] = values[0];
            x[11] = values[1];
        }
        {
            c.access<0x1832c, 3>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x10, values.data());
            x[12] = values[0];
            x[13] = values[1];
        }
        scalar::arithmetic<64>(cpu, x[2], 0x40, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 8)) {
            c.finish<6, 0x18350>();
            goto L18350;
        }
        c.finish<6, 0x18338>();
        goto L18338;
    }
L18338:
    {
        if (!c.begin<0x18338, 5>())
            return;
        {
            c.access<0x18338, 0>();
            std::array<uint64_t, 2> values{x[6], x[7]};
            memory.copy_in<sizeof(values)>(x[0], values.data());
            if (!c.stored<0x1833c, 1>())
                return;
        }
        {
            c.access<0x1833c, 1>();
            std::array<uint64_t, 2> values{x[8], x[9]};
            memory.copy_in<sizeof(values)>(x[0] + 0x10, values.data());
            if (!c.stored<0x18340, 2>())
                return;
        }
        {
            c.access<0x18340, 2>();
            std::array<uint64_t, 2> values{x[10], x[11]};
            memory.copy_in<sizeof(values)>(x[5] + -0x20, values.data());
            if (!c.stored<0x18344, 3>())
                return;
        }
        {
            c.access<0x18344, 3>();
            std::array<uint64_t, 2> values{x[12], x[13]};
            memory.copy_in<sizeof(values)>(x[5] + -0x10, values.data());
            if (!c.stored<0x18348, 4>())
                return;
        }
        c.returned<5>();
        return;
    }
L18350:
    {
        if (!c.begin<0x18350, 4>())
            return;
        {
            c.access<0x18350, 0>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x20, values.data());
            x[14] = values[0];
            x[15] = values[1];
        }
        {
            c.access<0x18354, 1>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x30, values.data());
            x[16] = values[0];
            x[17] = values[1];
        }
        scalar::arithmetic<64>(cpu, x[2], 0x60, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 9)) {
            c.finish<4, 0x18370>();
            goto L18370;
        }
        c.finish<4, 0x18360>();
        goto L18360;
    }
L18360:
    {
        if (!c.begin<0x18360, 4>())
            return;
        {
            c.access<0x18360, 0>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x40, values.data());
            x[2] = values[0];
            x[3] = values[1];
        }
        {
            c.access<0x18364, 1>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x30, values.data());
            x[1] = values[0];
            x[4] = values[1];
        }
        {
            c.access<0x18368, 2>();
            std::array<uint64_t, 2> values{x[2], x[3]};
            memory.copy_in<sizeof(values)>(x[5] + -0x40, values.data());
            if (!c.stored<0x1836c, 3>())
                return;
        }
        {
            c.access<0x1836c, 3>();
            std::array<uint64_t, 2> values{x[1], x[4]};
            memory.copy_in<sizeof(values)>(x[5] + -0x30, values.data());
            if (!c.stored<0x18370, 4>())
                return;
        }
        c.finish<4, 0x18370>();
        goto L18370;
    }
L18370:
    {
        if (!c.begin<0x18370, 7>())
            return;
        {
            c.access<0x18370, 0>();
            std::array<uint64_t, 2> values{x[6], x[7]};
            memory.copy_in<sizeof(values)>(x[0], values.data());
            if (!c.stored<0x18374, 1>())
                return;
        }
        {
            c.access<0x18374, 1>();
            std::array<uint64_t, 2> values{x[8], x[9]};
            memory.copy_in<sizeof(values)>(x[0] + 0x10, values.data());
            if (!c.stored<0x18378, 2>())
                return;
        }
        {
            c.access<0x18378, 2>();
            std::array<uint64_t, 2> values{x[14], x[15]};
            memory.copy_in<sizeof(values)>(x[0] + 0x20, values.data());
            if (!c.stored<0x1837c, 3>())
                return;
        }
        {
            c.access<0x1837c, 3>();
            std::array<uint64_t, 2> values{x[16], x[17]};
            memory.copy_in<sizeof(values)>(x[0] + 0x30, values.data());
            if (!c.stored<0x18380, 4>())
                return;
        }
        {
            c.access<0x18380, 4>();
            std::array<uint64_t, 2> values{x[10], x[11]};
            memory.copy_in<sizeof(values)>(x[5] + -0x20, values.data());
            if (!c.stored<0x18384, 5>())
                return;
        }
        {
            c.access<0x18384, 5>();
            std::array<uint64_t, 2> values{x[12], x[13]};
            memory.copy_in<sizeof(values)>(x[5] + -0x10, values.data());
            if (!c.stored<0x18388, 6>())
                return;
        }
        c.returned<7>();
        return;
    }
L18390:
    {
        if (!c.begin<0x18390, 12>())
            return;
        {
            c.access<0x18390, 0>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1], values.data());
            x[12] = values[0];
            x[13] = values[1];
        }
        x[14] = x[0] & 0xf;
        x[3] = x[0] & 0xfffffffffffffff0ull;
        x[1] = x[1] - x[14];
        x[2] = x[2] + x[14];
        {
            c.access<0x183a4, 5>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x10, values.data());
            x[6] = values[0];
            x[7] = values[1];
        }
        {
            c.access<0x183a8, 6>();
            std::array<uint64_t, 2> values{x[12], x[13]};
            memory.copy_in<sizeof(values)>(x[0], values.data());
            if (!c.stored<0x183ac, 7>())
                return;
        }
        {
            c.access<0x183ac, 7>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x20, values.data());
            x[8] = values[0];
            x[9] = values[1];
        }
        {
            c.access<0x183b0, 8>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x30, values.data());
            x[10] = values[0];
            x[11] = values[1];
        }
        {
            c.access<0x183b4, 9>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x40, values.data());
            x[12] = values[0];
            x[13] = values[1];
            x[1] = x[1] + 0x40;
        }
        x[2] = scalar::arithmetic<64>(cpu, x[2], 0x90, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 9)) {
            c.finish<12, 0x183e8>();
            goto L183e8;
        }
        c.finish<12, 0x183c0>();
        goto L183c0;
    }
L183c0:
    {
        if (!c.begin<0x183c0, 10>())
            return;
        {
            c.access<0x183c0, 0>();
            std::array<uint64_t, 2> values{x[6], x[7]};
            memory.copy_in<sizeof(values)>(x[3] + 0x10, values.data());
            if (!c.stored<0x183c4, 1>())
                return;
        }
        {
            c.access<0x183c4, 1>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x10, values.data());
            x[6] = values[0];
            x[7] = values[1];
        }
        {
            c.access<0x183c8, 2>();
            std::array<uint64_t, 2> values{x[8], x[9]};
            memory.copy_in<sizeof(values)>(x[3] + 0x20, values.data());
            if (!c.stored<0x183cc, 3>())
                return;
        }
        {
            c.access<0x183cc, 3>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x20, values.data());
            x[8] = values[0];
            x[9] = values[1];
        }
        {
            c.access<0x183d0, 4>();
            std::array<uint64_t, 2> values{x[10], x[11]};
            memory.copy_in<sizeof(values)>(x[3] + 0x30, values.data());
            if (!c.stored<0x183d4, 5>())
                return;
        }
        {
            c.access<0x183d4, 5>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x30, values.data());
            x[10] = values[0];
            x[11] = values[1];
        }
        {
            c.access<0x183d8, 6>();
            std::array<uint64_t, 2> values{x[12], x[13]};
            memory.copy_in<sizeof(values)>(x[3] + 0x40, values.data());
            x[3] = x[3] + 0x40;
            if (!c.stored<0x183dc, 7>())
                return;
        }
        {
            c.access<0x183dc, 7>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[1] + 0x40, values.data());
            x[12] = values[0];
            x[13] = values[1];
            x[1] = x[1] + 0x40;
        }
        x[2] = scalar::arithmetic<64>(cpu, x[2], 0x40, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 8)) {
            c.finish<10, 0x183c0>();
            goto L183c0;
        }
        c.finish<10, 0x183e8>();
        goto L183e8;
    }
L183e8:
    {
        if (loop_only)
            return;
        if (!c.begin<0x183e8, 13>())
            return;
        {
            c.access<0x183e8, 0>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x40, values.data());
            x[14] = values[0];
            x[15] = values[1];
        }
        {
            c.access<0x183ec, 1>();
            std::array<uint64_t, 2> values{x[6], x[7]};
            memory.copy_in<sizeof(values)>(x[3] + 0x10, values.data());
            if (!c.stored<0x183f0, 2>())
                return;
        }
        {
            c.access<0x183f0, 2>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x30, values.data());
            x[6] = values[0];
            x[7] = values[1];
        }
        {
            c.access<0x183f4, 3>();
            std::array<uint64_t, 2> values{x[8], x[9]};
            memory.copy_in<sizeof(values)>(x[3] + 0x20, values.data());
            if (!c.stored<0x183f8, 4>())
                return;
        }
        {
            c.access<0x183f8, 4>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x20, values.data());
            x[8] = values[0];
            x[9] = values[1];
        }
        {
            c.access<0x183fc, 5>();
            std::array<uint64_t, 2> values{x[10], x[11]};
            memory.copy_in<sizeof(values)>(x[3] + 0x30, values.data());
            if (!c.stored<0x18400, 6>())
                return;
        }
        {
            c.access<0x18400, 6>();
            std::array<uint64_t, 2> values;
            memory.copy_out<sizeof(values)>(x[4] + -0x10, values.data());
            x[10] = values[0];
            x[11] = values[1];
        }
        {
            c.access<0x18404, 7>();
            std::array<uint64_t, 2> values{x[12], x[13]};
            memory.copy_in<sizeof(values)>(x[3] + 0x40, values.data());
            if (!c.stored<0x18408, 8>())
                return;
        }
        {
            c.access<0x18408, 8>();
            std::array<uint64_t, 2> values{x[14], x[15]};
            memory.copy_in<sizeof(values)>(x[5] + -0x40, values.data());
            if (!c.stored<0x1840c, 9>())
                return;
        }
        {
            c.access<0x1840c, 9>();
            std::array<uint64_t, 2> values{x[6], x[7]};
            memory.copy_in<sizeof(values)>(x[5] + -0x30, values.data());
            if (!c.stored<0x18410, 10>())
                return;
        }
        {
            c.access<0x18410, 10>();
            std::array<uint64_t, 2> values{x[8], x[9]};
            memory.copy_in<sizeof(values)>(x[5] + -0x20, values.data());
            if (!c.stored<0x18414, 11>())
                return;
        }
        {
            c.access<0x18414, 11>();
            std::array<uint64_t, 2> values{x[10], x[11]};
            memory.copy_in<sizeof(values)>(x[5] + -0x10, values.data());
            if (!c.stored<0x18418, 12>())
                return;
        }
        c.returned<13>();
        return;
    }
}
} // namespace operit::arm64::fixed
