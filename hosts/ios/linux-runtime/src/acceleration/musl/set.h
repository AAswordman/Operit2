#pragma once
#include "acceleration/fixed_execution.h"
#include "acceleration/scalar_ops.h"
#include "cpu/aarch64/cache.h"

namespace operit::arm64::fixed {
inline void set(FixedExecution &c) {
    auto &cpu = c.cpu;
    auto &memory = c.memory;
    auto &x = cpu.x;
    goto L18420;
L18420:
    {
        if (!c.begin<0x18420, 4>())
            return;
        cpu.v[0].fill(uint8_t(x[1]));
        x[4] = x[0] + x[2];
        scalar::arithmetic<64>(cpu, x[2], 0x60, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 8)) {
            c.finish<4, 0x184a0>();
            goto L184a0;
        }
        c.finish<4, 0x18430>();
        goto L18430;
    }
L18430:
    {
        if (!c.begin<0x18430, 2>())
            return;
        scalar::arithmetic<64>(cpu, x[2], 0x10, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 2)) {
            c.finish<2, 0x18474>();
            goto L18474;
        }
        c.finish<2, 0x18438>();
        goto L18438;
    }
L18438:
    {
        if (!c.begin<0x18438, 2>())
            return;
        std::memcpy(&x[1], cpu.v[0].data(), sizeof(x[1]));
        if (((uint32_t(x[2]) >> 3) & 1) == 0) {
            c.finish<2, 0x18450>();
            goto L18450;
        }
        c.finish<2, 0x18440>();
        goto L18440;
    }
L18440:
    {
        if (!c.begin<0x18440, 3>())
            return;
        {
            c.access<0x18440, 0>();
            memory.store<8>(x[0], x[1]);
            if (!c.stored<0x18444, 1>())
                return;
        }
        {
            c.access<0x18444, 1>();
            memory.store<8>(x[4] + -0x8, x[1]);
            if (!c.stored<0x18448, 2>())
                return;
        }
        c.returned<3>();
        return;
    }
L18450:
    {
        if (!c.begin<0x18450, 1>())
            return;
        if (((uint32_t(x[2]) >> 2) & 1) == 0) {
            c.finish<1, 0x18460>();
            goto L18460;
        }
        c.finish<1, 0x18454>();
        goto L18454;
    }
L18454:
    {
        if (!c.begin<0x18454, 3>())
            return;
        {
            c.access<0x18454, 0>();
            memory.store<4>(x[0], uint32_t(x[1]));
            if (!c.stored<0x18458, 1>())
                return;
        }
        {
            c.access<0x18458, 1>();
            memory.store<4>(x[4] + -0x4, uint32_t(x[1]));
            if (!c.stored<0x1845c, 2>())
                return;
        }
        c.returned<3>();
        return;
    }
L18460:
    {
        if (!c.begin<0x18460, 1>())
            return;
        if (x[2] == 0) {
            c.finish<1, 0x18470>();
            goto L18470;
        }
        c.finish<1, 0x18464>();
        goto L18464;
    }
L18464:
    {
        if (!c.begin<0x18464, 2>())
            return;
        {
            c.access<0x18464, 0>();
            memory.store<1>(x[0], uint32_t(x[1]));
            if (!c.stored<0x18468, 1>())
                return;
        }
        if (((uint32_t(x[2]) >> 1) & 1) == 0) {
            c.finish<2, 0x18470>();
            goto L18470;
        }
        c.finish<2, 0x1846c>();
        goto L1846c;
    }
L1846c:
    {
        if (!c.begin<0x1846c, 1>())
            return;
        {
            c.access<0x1846c, 0>();
            memory.store<2>(x[4] + -0x2, uint32_t(x[1]));
            if (!c.stored<0x18470, 1>())
                return;
        }
        c.finish<1, 0x18470>();
        goto L18470;
    }
L18470:
    {
        if (!c.begin<0x18470, 1>())
            return;
        c.returned<1>();
        return;
    }
L18474:
    {
        if (!c.begin<0x18474, 2>())
            return;
        {
            c.access<0x18474, 0>();
            memory.copy_in<16>(x[0], cpu.v[0].data());
            if (!c.stored<0x18478, 1>())
                return;
        }
        if (((uint32_t(x[2]) >> 6) & 1) != 0) {
            c.finish<2, 0x18490>();
            goto L18490;
        }
        c.finish<2, 0x1847c>();
        goto L1847c;
    }
L1847c:
    {
        if (!c.begin<0x1847c, 2>())
            return;
        {
            c.access<0x1847c, 0>();
            memory.copy_in<16>(x[4] + -0x10, cpu.v[0].data());
            if (!c.stored<0x18480, 1>())
                return;
        }
        if (((uint32_t(x[2]) >> 5) & 1) == 0) {
            c.finish<2, 0x1848c>();
            goto L1848c;
        }
        c.finish<2, 0x18484>();
        goto L18484;
    }
L18484:
    {
        if (!c.begin<0x18484, 2>())
            return;
        {
            c.access<0x18484, 0>();
            memory.copy_in<16>(x[0] + 0x10, cpu.v[0].data());
            if (!c.stored<0x18488, 1>())
                return;
        }
        {
            c.access<0x18488, 1>();
            memory.copy_in<16>(x[4] + -0x20, cpu.v[0].data());
            if (!c.stored<0x1848c, 2>())
                return;
        }
        c.finish<2, 0x1848c>();
        goto L1848c;
    }
L1848c:
    {
        if (!c.begin<0x1848c, 1>())
            return;
        c.returned<1>();
        return;
    }
L18490:
    {
        if (!c.begin<0x18490, 4>())
            return;
        {
            c.access<0x18490, 0>();
            memory.copy_in<16>(x[0] + 0x10, cpu.v[0].data());
            if (!c.stored<0x18494, 1>())
                return;
        }
        {
            c.access<0x18494, 1>();
            std::array<uint8_t, 32> values;
            std::memcpy(values.data() + 0, cpu.v[0].data(), 16);
            std::memcpy(values.data() + 16, cpu.v[0].data(), 16);
            memory.copy_in<sizeof(values)>(x[0] + 0x20, values.data());
            if (!c.stored<0x18498, 2>())
                return;
        }
        {
            c.access<0x18498, 2>();
            std::array<uint8_t, 32> values;
            std::memcpy(values.data() + 0, cpu.v[0].data(), 16);
            std::memcpy(values.data() + 16, cpu.v[0].data(), 16);
            memory.copy_in<sizeof(values)>(x[4] + -0x20, values.data());
            if (!c.stored<0x1849c, 3>())
                return;
        }
        c.returned<4>();
        return;
    }
L184a0:
    {
        if (!c.begin<0x184a0, 6>())
            return;
        x[1] = uint32_t(uint32_t(x[1]) & 0xff);
        x[3] = x[0] & 0xfffffffffffffff0ull;
        {
            c.access<0x184a8, 2>();
            memory.copy_in<16>(x[0], cpu.v[0].data());
            if (!c.stored<0x184ac, 3>())
                return;
        }
        scalar::arithmetic<64>(cpu, x[2], 0xa0, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 2))
            scalar::arithmetic<32>(cpu, x[1], 0, decoded::set_flags | decoded::subtract);
        else
            cpu.nzcv = 0;
        if (scalar::condition(cpu, 1)) {
            c.finish<6, 0x184fc>();
            goto L184fc;
        }
        c.finish<6, 0x184b8>();
        goto L184b8;
    }
L184b8:
    {
        if (!c.begin<0x184b8, 4>())
            return;
        x[5] = cache_profile::dczid_el0;
        x[5] = x[5] & 0x1f;
        scalar::arithmetic<64>(cpu, x[5], 0x4, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 1)) {
            c.finish<4, 0x184fc>();
            goto L184fc;
        }
        c.finish<4, 0x184c8>();
        goto L184c8;
    }
L184c8:
    {
        if (!c.begin<0x184c8, 6>())
            return;
        {
            c.access<0x184c8, 0>();
            memory.copy_in<16>(x[3] + 0x10, cpu.v[0].data());
            if (!c.stored<0x184cc, 1>())
                return;
        }
        {
            c.access<0x184cc, 1>();
            std::array<uint8_t, 32> values;
            std::memcpy(values.data() + 0, cpu.v[0].data(), 16);
            std::memcpy(values.data() + 16, cpu.v[0].data(), 16);
            memory.copy_in<sizeof(values)>(x[3] + 0x20, values.data());
            if (!c.stored<0x184d0, 2>())
                return;
        }
        x[3] = x[3] & 0xffffffffffffffc0ull;
        x[2] = x[4] - x[3];
        x[2] = x[2] - 0x80;
        c.finish<6, 0x184e0>();
        goto L184e0;
    }
L184e0:
    {
        if (!c.begin<0x184e0, 4>())
            return;
        x[3] = x[3] + 0x40;
        c.partial<0x184e4, 1>();
        return;
    }
L184fc:
    {
        if (!c.begin<0x184fc, 3>())
            return;
        x[2] = x[4] - x[3];
        x[3] = x[3] - 0x10;
        x[2] = x[2] - 0x50;
        c.finish<3, 0x18508>();
        goto L18508;
    }
L18508:
    {
        if (!c.begin<0x18508, 4>())
            return;
        {
            c.access<0x18508, 0>();
            std::array<uint8_t, 32> values;
            std::memcpy(values.data() + 0, cpu.v[0].data(), 16);
            std::memcpy(values.data() + 16, cpu.v[0].data(), 16);
            memory.copy_in<sizeof(values)>(x[3] + 0x20, values.data());
            if (!c.stored<0x1850c, 1>())
                return;
        }
        {
            c.access<0x1850c, 1>();
            std::array<uint8_t, 32> values;
            std::memcpy(values.data() + 0, cpu.v[0].data(), 16);
            std::memcpy(values.data() + 16, cpu.v[0].data(), 16);
            memory.copy_in<sizeof(values)>(x[3] + 0x40, values.data());
            x[3] = x[3] + 0x40;
            if (!c.stored<0x18510, 2>())
                return;
        }
        x[2] = scalar::arithmetic<64>(cpu, x[2], 0x40, decoded::set_flags | decoded::subtract);
        if (scalar::condition(cpu, 8)) {
            c.finish<4, 0x18508>();
            goto L18508;
        }
        c.finish<4, 0x18518>();
        goto L18518;
    }
L18518:
    {
        if (!c.begin<0x18518, 3>())
            return;
        {
            c.access<0x18518, 0>();
            std::array<uint8_t, 32> values;
            std::memcpy(values.data() + 0, cpu.v[0].data(), 16);
            std::memcpy(values.data() + 16, cpu.v[0].data(), 16);
            memory.copy_in<sizeof(values)>(x[4] + -0x40, values.data());
            if (!c.stored<0x1851c, 1>())
                return;
        }
        {
            c.access<0x1851c, 1>();
            std::array<uint8_t, 32> values;
            std::memcpy(values.data() + 0, cpu.v[0].data(), 16);
            std::memcpy(values.data() + 16, cpu.v[0].data(), 16);
            memory.copy_in<sizeof(values)>(x[4] + -0x20, values.data());
            if (!c.stored<0x18520, 2>())
                return;
        }
        c.returned<3>();
        return;
    }
}
} // namespace operit::arm64::fixed
