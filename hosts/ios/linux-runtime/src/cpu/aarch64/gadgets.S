// Static ARM64 threaded interpreter. No generated or writable host code.
#include "gadget_offsets.h"
#if defined(__aarch64__)
#if defined(__APPLE__)
#define SYM(name) _##name
#else
#define SYM(name) name
#endif
.text
.p2align 2
.globl SYM(operit_arm64_gadgets_run)
SYM(operit_arm64_gadgets_run):
.cfi_startproc
    // Leaf interpreter: x0=CPU, x1=end, x2=current, x3=start, x4=NZCV,
    // x5=TLB, x6=epoch, x7=file generation. x8-x17 are handler scratch.
    // No callee-saved registers, stack frame, host calls or x18 use.
    ldp x5, x6, [x1]
    ldr x7, [x1, #16]
    add x1, x2, x3, lsl #5
    mov x3, x2
    str xzr, [x0, #G_CPU_ZERO]
    ldrb w4, [x0, #G_CPU_NZCV]
    lsl x4, x4, #28
    cmp x2, x1
    b.hs .Lreturn
    ldr x17, [x2, #G_ENTRY]
    cbz x17, .Lreturn
    br x17

// Separate indirect branch sites keep the predecessor visible to the branch
// predictor. The compiler cannot tail-merge these statically compiled tails.
.macro NEXT
    add x2, x2, #G_SIZE
    cmp x2, x1
    b.hs .Lreturn
    ldr x17, [x2, #G_ENTRY]
    cbz x17, .Lreturn
    br x17
.endm
.macro READ field, target
    ldrb w9, [x2, #\field]
    ldr \target, [x0, x9, lsl #3]
.endm
.macro WRITE_REGISTER field, target
    ldrb w9, [x2, #\field]
    cmp w9, #G_ZERO_SLOT
    b.eq Lwrite_done\@
    str \target, [x0, x9, lsl #3]
Lwrite_done\@:
.endm
.macro WRITE target
    WRITE_REGISTER G_RD, \target
    NEXT
.endm
.macro FUNCTION name
    .p2align 2
\name:
.endm
// Decode selects an exact operation; these bodies never decode flags or mode.
.macro BINARY name, operation, lhs, rhs, immediate, sets_flags, invert
    FUNCTION Loperit_gadget_\name
    READ G_RN, x8
.if \immediate
    ldr x10, [x2, #G_IMMEDIATE]
.else
    READ G_RM, x10
.endif
.if \invert
    mvn \rhs, \rhs
.endif
    \operation \lhs, \lhs, \rhs
.if \sets_flags
    mrs x4, nzcv
.endif
    WRITE x8
.endm
.macro CARRY name, operation, lhs, rhs, sets_flags
    FUNCTION Loperit_gadget_\name
    READ G_RN, x8
    READ G_RM, x10
    msr nzcv, x4
    \operation \lhs, \lhs, \rhs
.if \sets_flags
    mrs x4, nzcv
.endif
    WRITE x8
.endm
.macro PRODUCT name, operation, lhs, rhs, accumulator, accumulated
    FUNCTION Loperit_gadget_\name
    READ G_RN, x8
    READ G_RM, x10
.if \accumulated
    READ G_RA, x11
    \operation \lhs, \lhs, \rhs, \accumulator
.else
    \operation \lhs, \lhs, \rhs
.endif
    WRITE x8
.endm
#define GADGET_BASIC(name)
#define GADGET_BINARY(name, operation, lhs, rhs, immediate, sets_flags, invert) BINARY name, operation, lhs, rhs, immediate, sets_flags, invert
#define GADGET_CARRY(name, operation, lhs, rhs, sets_flags) CARRY name, operation, lhs, rhs, sets_flags
#define GADGET_PRODUCT(name, operation, lhs, rhs, accumulator, accumulated) PRODUCT name, operation, lhs, rhs, accumulator, accumulated
#define GADGET_MEMORY(name, size, writing, pair)
#include "gadget_entries.inc"
#undef GADGET_MEMORY
#undef GADGET_BASIC
#undef GADGET_BINARY
#undef GADGET_CARRY
#undef GADGET_PRODUCT
FUNCTION Loperit_gadget_noop
    NEXT
FUNCTION Loperit_gadget_move32
    ldr w8, [x2, #G_IMMEDIATE]
    WRITE x8
FUNCTION Loperit_gadget_keep32
    READ G_RD, x8
    ldr x10, [x2, #G_AUXILIARY]
    ldr x11, [x2, #G_IMMEDIATE]
    and w8, w8, w10
    orr w8, w8, w11
    WRITE x8
FUNCTION Loperit_gadget_select32
    ldrb w10, [x2, #G_SHIFT]
    lsr x9, x4, #28
    add x10, x10, x9, lsl #4
    adr x14, .Lconditions
    ldrb w10, [x14, x10]
    cbz w10, 1f
    READ G_RN, x8
    mov w8, w8
    WRITE x8
1:
    READ G_RM, x8
    ldrb w15, [x2, #G_FLAGS]
    tbz w15, #0, 2f
    mvn w8, w8
2:
    tbz w15, #1, 3f
    add w8, w8, #1
3:
    mov w8, w8
    WRITE x8
FUNCTION Loperit_gadget_move64
    ldr x8, [x2, #G_IMMEDIATE]
    WRITE x8
FUNCTION Loperit_gadget_keep64
    READ G_RD, x8
    ldr x10, [x2, #G_AUXILIARY]
    ldr x11, [x2, #G_IMMEDIATE]
    and x8, x8, x10
    orr x8, x8, x11
    WRITE x8
FUNCTION Loperit_gadget_select64
    ldrb w10, [x2, #G_SHIFT]
    lsr x9, x4, #28
    add x10, x10, x9, lsl #4
    adr x14, .Lconditions
    ldrb w10, [x14, x10]
    cbz w10, 1f
    READ G_RN, x8
    WRITE x8
1:
    READ G_RM, x8
    ldrb w15, [x2, #G_FLAGS]
    tbz w15, #0, 2f
    mvn x8, x8
2:
    tbz w15, #1, 3f
    add x8, x8, #1
3:
    WRITE x8
FUNCTION Loperit_gadget_copy32
    READ G_RM, x8
    mov w8, w8
    WRITE x8
FUNCTION Loperit_gadget_copy64
    READ G_RM, x8
    WRITE x8
FUNCTION Loperit_gadget_conditional
    ldrb w10, [x2, #G_FLAGS]
    lsr x9, x4, #28
    add x10, x10, x9, lsl #4
    adr x14, .Lconditions
    ldrb w10, [x14, x10]
    cbnz w10, .Lreturn
    NEXT
FUNCTION Loperit_gadget_compare_branch32
    READ G_RD, x8
    cmp w8, #0
    b Lcompare_branch
FUNCTION Loperit_gadget_compare_branch64
    READ G_RD, x8
    cmp x8, #0
Lcompare_branch:
    cset w8, ne
    ldrb w9, [x2, #G_FLAGS]
    cmp w8, w9
    b.eq .Lreturn
    NEXT
FUNCTION Loperit_gadget_test_branch
    READ G_RD, x8
    ldrb w9, [x2, #G_SHIFT]
    lsr x8, x8, x9
    and w8, w8, #1
    ldrb w9, [x2, #G_FLAGS]
    cmp w8, w9
    b.eq .Lreturn
    NEXT
// x10: guest address; x11: deferred writeback; w15: flags; w16: mode.
.macro ADDRESS
    ldrb w15, [x2, #G_FLAGS]
    READ G_RN, x10
    and w16, w15, #3
    cmp w16, #1
    b.eq 1f
    ldr x14, [x2, #G_IMMEDIATE]
    add x11, x10, x14
    cmp w16, #2
    csel x10, x10, x11, eq
    b 5f
1:
    READ G_RM, x14
    tbz w15, #4, 3f
    tbnz w15, #5, 2f
    mov w14, w14
    b 3f
2:
    sxtw x14, w14
3:
    tbz w15, #6, 4f
    ldrb w9, [x2, #G_SHIFT]
    lsl x14, x14, x9
4:
    add x10, x10, x14
5:
.endm
// Only resident, serial, private pages are safe for direct access. Every miss
// returns before touching guest bytes or applying register writeback.
.macro TRANSLATE size, writing
    lsr x14, x10, #48
    cbnz x14, .Lreturn
    and x14, x10, #4095
    cmp x14, #(4096 - \size)
    b.hi .Lreturn
    lsr x9, x10, #12
    eor x9, x9, x9, lsr #10
    and x9, x9, #1023
    add x12, x5, x9, lsl #6
    ldr x9, [x12, #G_TLB_TAG]
    and x17, x10, #0xfffffffffffff000
    cmp x9, x17
    b.ne .Lreturn
    ldr x9, [x12, #G_TLB_EPOCH]
    cmp x9, x6
    b.ne .Lreturn
    ldr w9, [x12, #G_TLB_PERMISSIONS]
.if \writing
    tbz w9, #1, .Lreturn
    tbnz w9, #2, .Lreturn
.else
    tbz w9, #0, .Lreturn
.endif
    ldrh w9, [x12, #G_TLB_SHARED]
    cbnz w9, .Lreturn
    ldr x9, [x12, #G_TLB_FILE]
    cbz x9, Lfile_checked\@
    cbz x7, .Lreturn
    ldr x9, [x12, #G_TLB_FILE_GENERATION]
    cmp x9, x7
    b.ne .Lreturn
Lfile_checked\@:
    ldr x13, [x12, #G_TLB_DATA]
    add x13, x13, x14
.endm
// Offset mode is resolved by the decoder. No runtime mode flags, index
// extension, post-index selection or deferred base writeback are needed.
.macro OFFSET_MEMORY name, size, writing, pair
    FUNCTION Loperit_gadget_\name
    READ G_RN, x10
    ldr x14, [x2, #G_IMMEDIATE]
    add x10, x10, x14
    TRANSLATE \size, \writing
.if \writing
    READ G_RD, x8
.if \pair
    READ G_RA, x10
.endif
    ldr x9, [x12, #G_TLB_RESERVATION]
    str xzr, [x9, #G_RESERVATION_TOKEN]
.if \pair
    stp x8, x10, [x13]
.else
    str x8, [x13]
.endif
.else
.if \pair
    ldp x8, x10, [x13]
    WRITE_REGISTER G_RD, x8
    WRITE_REGISTER G_RA, x10
    NEXT
.else
    ldr x8, [x13]
    WRITE x8
.endif
.endif
.if \writing
    NEXT
.endif
.endm
#define GADGET_BASIC(name)
#define GADGET_BINARY(name, operation, lhs, rhs, immediate, sets_flags, invert)
#define GADGET_CARRY(name, operation, lhs, rhs, sets_flags)
#define GADGET_PRODUCT(name, operation, lhs, rhs, accumulator, accumulated)
#define GADGET_MEMORY(name, size, writing, pair) OFFSET_MEMORY name, size, writing, pair
#include "gadget_entries.inc"
#undef GADGET_BASIC
#undef GADGET_BINARY
#undef GADGET_CARRY
#undef GADGET_PRODUCT
#undef GADGET_MEMORY
.macro WRITEBACK
    cmp w16, #2
    b.lo 1f
    ldrb w9, [x2, #G_RN]
    str x11, [x0, x9, lsl #3]
1:
    NEXT
.endm
FUNCTION Loperit_gadget_load1
    ADDRESS
    TRANSLATE 1, 0
    ldrb w8, [x13]
    tbz w15, #2, 1f
    sxtb x8, w8
    tbz w15, #3, 1f
    mov w8, w8
1:
    ldrb w9, [x2, #G_RD]
    cmp w9, #G_ZERO_SLOT
    b.eq 2f
    str x8, [x0, x9, lsl #3]
2:
    WRITEBACK
FUNCTION Loperit_gadget_store1
    ADDRESS
    TRANSLATE 1, 1
    READ G_RD, x8
    ldr x9, [x12, #G_TLB_RESERVATION]
    str xzr, [x9, #G_RESERVATION_TOKEN]
    strb w8, [x13]
    WRITEBACK
FUNCTION Loperit_gadget_load2
    ADDRESS
    TRANSLATE 2, 0
    ldrh w8, [x13]
    tbz w15, #2, 1f
    sxth x8, w8
    tbz w15, #3, 1f
    mov w8, w8
1:
    ldrb w9, [x2, #G_RD]
    cmp w9, #G_ZERO_SLOT
    b.eq 2f
    str x8, [x0, x9, lsl #3]
2:
    WRITEBACK
FUNCTION Loperit_gadget_store2
    ADDRESS
    TRANSLATE 2, 1
    READ G_RD, x8
    ldr x9, [x12, #G_TLB_RESERVATION]
    str xzr, [x9, #G_RESERVATION_TOKEN]
    strh w8, [x13]
    WRITEBACK
FUNCTION Loperit_gadget_load4
    ADDRESS
    TRANSLATE 4, 0
    ldr w8, [x13]
    tbz w15, #2, 1f
    sxtw x8, w8
    tbz w15, #3, 1f
    mov w8, w8
1:
    ldrb w9, [x2, #G_RD]
    cmp w9, #G_ZERO_SLOT
    b.eq 2f
    str x8, [x0, x9, lsl #3]
2:
    WRITEBACK
FUNCTION Loperit_gadget_store4
    ADDRESS
    TRANSLATE 4, 1
    READ G_RD, x8
    ldr x9, [x12, #G_TLB_RESERVATION]
    str xzr, [x9, #G_RESERVATION_TOKEN]
    str w8, [x13]
    WRITEBACK
FUNCTION Loperit_gadget_load8
    ADDRESS
    TRANSLATE 8, 0
    ldr x8, [x13]
    ldrb w9, [x2, #G_RD]
    cmp w9, #G_ZERO_SLOT
    b.eq 2f
    str x8, [x0, x9, lsl #3]
2:
    WRITEBACK
FUNCTION Loperit_gadget_store8
    ADDRESS
    TRANSLATE 8, 1
    READ G_RD, x8
    ldr x9, [x12, #G_TLB_RESERVATION]
    str xzr, [x9, #G_RESERVATION_TOKEN]
    str x8, [x13]
    WRITEBACK
FUNCTION Loperit_gadget_pair_load4
    ADDRESS
    TRANSLATE 8, 0
    ldp w8, w10, [x13]
    tbz w15, #2, 1f
    sxtw x8, w8
    sxtw x10, w10
1:
    ldrb w9, [x2, #G_RD]
    cmp w9, #G_ZERO_SLOT
    b.eq 2f
    str x8, [x0, x9, lsl #3]
2:
    ldrb w9, [x2, #G_RA]
    cmp w9, #G_ZERO_SLOT
    b.eq 3f
    str x10, [x0, x9, lsl #3]
3:
    WRITEBACK
FUNCTION Loperit_gadget_pair_store4
    ADDRESS
    TRANSLATE 8, 1
    READ G_RD, x8
    READ G_RA, x10
    ldr x9, [x12, #G_TLB_RESERVATION]
    str xzr, [x9, #G_RESERVATION_TOKEN]
    stp w8, w10, [x13]
    WRITEBACK
FUNCTION Loperit_gadget_pair_load8
    ADDRESS
    TRANSLATE 16, 0
    ldp x8, x10, [x13]
    ldrb w9, [x2, #G_RD]
    cmp w9, #G_ZERO_SLOT
    b.eq 2f
    str x8, [x0, x9, lsl #3]
2:
    ldrb w9, [x2, #G_RA]
    cmp w9, #G_ZERO_SLOT
    b.eq 3f
    str x10, [x0, x9, lsl #3]
3:
    WRITEBACK
FUNCTION Loperit_gadget_pair_store8
    ADDRESS
    TRANSLATE 16, 1
    READ G_RD, x8
    READ G_RA, x10
    ldr x9, [x12, #G_TLB_RESERVATION]
    str xzr, [x9, #G_RESERVATION_TOKEN]
    stp x8, x10, [x13]
    WRITEBACK
.Lreturn:
    lsr x9, x4, #28
    strb w9, [x0, #G_CPU_NZCV]
    sub x0, x2, x3
    lsr x0, x0, #5
    ret
.cfi_endproc
.p2align 4
.Lconditions:
    .byte 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 1
    .byte 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1
    .byte 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 1, 1
    .byte 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1
    .byte 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1
    .byte 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1
    .byte 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1
    .byte 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1
    .byte 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1
    .byte 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1
    .byte 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 1
    .byte 0, 1, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1
    .byte 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1
    .byte 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1
    .byte 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1
    .byte 1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1
#if defined(__APPLE__)
.section __DATA_CONST,__const
#else
.section .data.rel.ro,"aw"
#endif
.p2align 3
.globl SYM(operit_arm64_gadget_entries)
SYM(operit_arm64_gadget_entries):
#define GADGET_BASIC(name) .quad Loperit_gadget_##name
#define GADGET_BINARY(name, operation, lhs, rhs, immediate, sets_flags, invert) GADGET_BASIC(name)
#define GADGET_CARRY(name, operation, lhs, rhs, sets_flags) GADGET_BASIC(name)
#define GADGET_PRODUCT(name, operation, lhs, rhs, accumulator, accumulated) GADGET_BASIC(name)
#define GADGET_MEMORY(name, size, writing, pair) GADGET_BASIC(name)
#include "gadget_entries.inc"
#undef GADGET_MEMORY
#undef GADGET_BASIC
#undef GADGET_BINARY
#undef GADGET_CARRY
#undef GADGET_PRODUCT
#endif
