#include "cpu/aarch64/vector_memory.h"
#include <cstring>
namespace operit::arm64 {
namespace {
struct StructureTransfer {
    unsigned registers = 0, structures = 1, size = 0, lane = 0;
    bool single = false, replicate = false;
};
bool decode_structure(uint32_t r, StructureTransfer &transfer) {
    transfer.single = (r & 0xbf000000) == 0x0d000000;
    if (!transfer.single && (r & 0xbf200000) != 0x0c000000)
        return false;
    if (!(r & (1u << 23)) && ((r >> 16) & 31))
        return false;
    const unsigned q = (r >> 30) & 1;
    if (transfer.single) {
        const unsigned opcode = (r >> 13) & 7, s = (r >> 12) & 1, encoded_size = (r >> 10) & 3;
        transfer.registers = (((opcode & 1) << 1) | ((r >> 21) & 1)) + 1;
        transfer.replicate = opcode >= 6;
        if (transfer.replicate) {
            if (!(r & (1u << 22)) || s)
                return false;
            transfer.size = 1u << encoded_size;
        } else if (opcode < 2) {
            transfer.size = 1;
            transfer.lane = q * 8 + s * 4 + encoded_size;
        } else if (opcode < 4) {
            if (encoded_size & 1)
                return false;
            transfer.size = 2;
            transfer.lane = q * 4 + s * 2 + encoded_size / 2;
        } else {
            if (encoded_size > 1 || (encoded_size == 1 && s))
                return false;
            transfer.size = encoded_size ? 8 : 4;
            transfer.lane = encoded_size ? q : q * 2 + s;
        }
    } else {
        switch ((r >> 12) & 15) {
        case 7:
            transfer.registers = 1;
            break;
        case 10:
            transfer.registers = 2;
            break;
        case 6:
            transfer.registers = 3;
            break;
        case 2:
            transfer.registers = 4;
            break;
        case 8:
            transfer.registers = transfer.structures = 2;
            break;
        case 4:
            transfer.registers = transfer.structures = 3;
            break;
        case 0:
            transfer.registers = transfer.structures = 4;
            break;
        default:
            return false;
        }
        transfer.size = 1u << ((r >> 10) & 3);
        if (transfer.structures > 1 && transfer.size == 8 && !q)
            return false;
    }
    return true;
}
} // namespace
bool execute_vector_memory(CpuState &cpu, Memory &memory, uint32_t r) {
    StructureTransfer transfer;
    if (!decode_structure(r, transfer))
        return false;
    const bool post = r & (1u << 23), load = r & (1u << 22);
    const unsigned rn = (r >> 5) & 31, rd = r & 31, rm = (r >> 16) & 31;
    const unsigned count = r & (1u << 30) ? 16 : 8, size = transfer.size;
    const unsigned bytes = transfer.registers * (transfer.single ? size : count);
    const uint64_t address = cpu.get(rn, 64, true);
    std::array<uint8_t, 64> data{};
    std::array<std::array<uint8_t, 16>, 4> values{};
    for (unsigned j = 0; j < transfer.registers; ++j)
        if ((transfer.single && !transfer.replicate) || !load)
            values[j] = cpu.v[(rd + j) % 32];
    if (load)
        memory.copy_out(address, data.data(), bytes);
    unsigned offset = 0;
    for (unsigned lane = 0; lane < (transfer.single ? size : count); lane += size) {
        for (unsigned j = 0; j < transfer.registers; ++j) {
            const unsigned position = transfer.single ? transfer.lane * size : lane;
            const unsigned source = !transfer.single && transfer.structures == 1 ? j * count + lane : offset;
            if (load) {
                if (transfer.replicate)
                    for (unsigned k = 0; k < count; k += size)
                        std::memcpy(values[j].data() + k, data.data() + source, size);
                else
                    std::memcpy(values[j].data() + position, data.data() + source, size);
            } else
                std::memcpy(data.data() + source, values[j].data() + position, size);
            offset += size;
        }
    }
    if (load) {
        for (unsigned j = 0; j < transfer.registers; ++j)
            cpu.v[(rd + j) % 32] = values[j];
    } else
        memory.copy_in(address, data.data(), bytes);
    if (post)
        cpu.set(rn, address + (rm == 31 ? bytes : cpu.get(rm)), 64, true);
    return true;
}
} // namespace operit::arm64
