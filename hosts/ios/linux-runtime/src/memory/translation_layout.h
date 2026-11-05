#pragma once
// The resident translation ABI shared with statically compiled CPU handlers.
#define G_TLB_TAG 0
#define G_TLB_EPOCH 8
#define G_TLB_DATA 16
#define G_TLB_RESERVATION 24
#define G_TLB_FILE 32
#define G_TLB_FILE_GENERATION 48
#define G_TLB_PERMISSIONS 56
#define G_TLB_SHARED 60
#define G_TLB_FILE_STORAGE 61
#define G_TLB_SIZE 64
#define G_RESERVATION_TOKEN 8

#ifndef __ASSEMBLER__
#include <cstdint>
namespace operit::arm64 {
struct GadgetMemory {
    const void *translations = nullptr;
    uint64_t epoch = 0, file_generation = 0;
};
}
#endif
