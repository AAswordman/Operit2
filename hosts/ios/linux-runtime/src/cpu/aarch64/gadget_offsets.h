#pragma once
// Verified against C++ layouts in gadgets.cpp and Memory::gadget_memory().
#define G_ENTRY 0
#define G_IMMEDIATE 8
#define G_AUXILIARY 16
#define G_RD 24
#define G_RN 25
#define G_RM 26
#define G_RA 27
#define G_SHIFT 28
#define G_FLAGS 29
#define G_SIZE 32
#define G_CPU_SP 760
#define G_CPU_NZCV 800
#define G_CPU_ZERO 832
#define G_ZERO_SLOT (G_CPU_ZERO / 8)
#include "memory/translation_layout.h"
