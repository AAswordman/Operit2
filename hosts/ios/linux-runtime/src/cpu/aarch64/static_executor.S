#include "executor_abi.h"
#ifdef __APPLE__
#define SYM(x) _##x
#else
#define SYM(x) x
#endif
.macro STATIC_ADDRESS reg, label
#ifdef __APPLE__
    adrp \reg, \label@PAGE
    add \reg, \reg, \label@PAGEOFF
#else
    adrp \reg, \label
    add \reg, \reg, :lo12:\label
#endif
.endm
.macro DIAG_COUNT event, base=x16, value=x17, amount=1
#if OPERIT_EXECUTION_DIAGNOSTICS
    add \base, x27, #EF_DIAGNOSTICS
    ldr \value, [\base, #(ED_EVENTS + 8 * \event)]
    add \value, \value, #\amount
    str \value, [\base, #(ED_EVENTS + 8 * \event)]
#endif
.endm
// Only dead handler-entry temporaries are clobbered: x0/x1/x2/x16/x17/x30.
// The shared leaf preserves NZCV, x3-x15, all resident GP/vector state, and SP.
.macro DIAG_HANDLER profile=0
#if OPERIT_EXECUTION_DIAGNOSTICS
    mov w0, #\profile
    bl Ldiagnostic_handler
#endif
.endm
.macro DIAG_BRANCH index, taken=0, fused=0
#if OPERIT_EXECUTION_DIAGNOSTICS
    add x16, x27, #EF_DIAGNOSTICS
    ldr x17, [x16, #(ED_BRANCH_CONDITIONS + \index * 8 + \taken * 256 + \fused * 512)]
    add x17, x17, #1
    str x17, [x16, #(ED_BRANCH_CONDITIONS + \index * 8 + \taken * 256 + \fused * 512)]
#endif
.endm
// Integer and bit-copy handlers retain the host FP environment. Only the
// first native FP handler after a C++ boundary switches to guest controls.
.macro FP_ENTER
    tbnz x8, #EXEC_FP_ACTIVE_BIT, 1f
    bl Lfp_enter
1:
.endm
.macro DISPATCH
    ldr x16, [x28]
    br x16
.endm
.macro ADVANCE
    ldr x16, [x28, #OP_SIZE]
    add x28, x28, #OP_SIZE
    br x16
.endm
// Conditional blocks always end with an explicit Next record. Consume that
// edge here so the untaken path does not dispatch through a second handler.
.macro FALLTHROUGH local=0
    DIAG_COUNT ED_EVENT_NEXT_ELIDED
    .if \local
    DIAG_COUNT ED_EVENT_LOCAL_EDGE
    DIAG_COUNT ED_EVENT_FALLTHROUGH_LOCAL
    ldrh w12, [x28, #OP_EDGE_PREFIX]
    add x26, x6, x12
    ldr x28, [x28, #OP_SIZE+OP_TARGET]
#if OPERIT_ARM64_DIRECT_CHAINS
    b LBlockBody
#else
    DISPATCH
#endif
    .else
    DIAG_COUNT ED_EVENT_CROSS_EDGE_ATTEMPT
    DIAG_COUNT ED_EVENT_FALLTHROUGH_CROSS
    ldrh w12, [x28, #OP_EDGE_PREFIX]
    add x26, x6, x12
    ldr x16, [x28, #OP_SIZE+OP_TARGET]
    add x28, x28, #OP_SIZE
    cbnz x16, 1f
    b Lslow
1:
    mov x28, x16
    DISPATCH
    .endif
.endm
.text
.p2align 4
.globl SYM(operit_static_execute)
SYM(operit_static_execute):
.cfi_startproc
    stp x29, x30, [sp, #-224]!
.cfi_def_cfa_offset 224
.cfi_offset x29, -224
.cfi_offset x30, -216
    mov x29, sp
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    stp x23, x24, [sp, #48]
    stp x25, x26, [sp, #64]
    stp x27, x28, [sp, #80]
.cfi_offset x19, -208
.cfi_offset x20, -200
.cfi_offset x21, -192
.cfi_offset x22, -184
.cfi_offset x23, -176
.cfi_offset x24, -168
.cfi_offset x25, -160
.cfi_offset x26, -152
.cfi_offset x27, -144
.cfi_offset x28, -136
    stp q8, q9, [sp, #96]
    stp q10, q11, [sp, #128]
    stp q12, q13, [sp, #160]
    stp q14, q15, [sp, #192]
.cfi_offset 72, -128
.cfi_offset 73, -112
.cfi_offset 74, -96
.cfi_offset 75, -80
.cfi_offset 76, -64
.cfi_offset 77, -48
.cfi_offset 78, -32
.cfi_offset 79, -16
    mov x27, x0
    mov x28, x1
    mrs x9, fpcr
    mrs x10, fpsr
    stp x9, x10, [x27, #EF_HOST_FPCR]
Lreload:
    // X3 carries guest X30 and uses its shadow home at C++ boundaries.
    ldr x3, [x27, #EF_RESIDENT_X30]
    ldr x4, [x27, #EF_CODE_EPOCH]
    ldr x5, [x27, #EF_BUDGET]
    ldr x7, [x27, #EF_PAGES]
    ldr x8, [x27, #EF_BLOCK_HITS]
    ldr x6, [x27, #EF_BLOCK_BASE]
    ldp x19, x20, [x27, #0]
    ldp x21, x22, [x27, #16]
    ldp x23, x24, [x27, #32]
    ldp q8, q9, [x27, #48]
    ldp q10, q11, [x27, #80]
    ldp q12, q13, [x27, #112]
    ldp q14, q15, [x27, #144]
    ldp x25, x26, [x27, #EF_FLAGS]
    // Published block counts are clean, so loading x8 also clears FP-active.
    // Keep the host environment until a handler executes FP_ENTER.
Ldispatch:
    ldr x16, [x28]
    br x16
LBlock:
    DIAG_HANDLER
    ldr x10, [x27, #EF_EPOCH]
    ldar x9, [x4]
    cmp x9, x10
    b.ne Lslow
    sub x9, x5, x26
    ldr x10, [x28, #OP_IMMEDIATE]
    cmp x9, x10
    b.lo Lslow
    add x28, x28, #OP_SIZE
LBlockBody:
    mov x6, x26
    DIAG_HANDLER
    // The serial native reader pins code; parallel execution still observes
    // concurrent code writes at each guest block boundary.
    cbnz x7, Lblock_admitted
    ldr x10, [x27, #EF_EPOCH]
    ldar x9, [x4]
    cmp x9, x10
    b.ne Lslow
Lblock_admitted:
    DIAG_COUNT ED_EVENT_BLOCK_ADMITTED
    add x8, x8, #1
    ADVANCE
LBridge:
    DIAG_HANDLER
    DIAG_COUNT ED_EVENT_BRIDGE_ENTRY
    ldr x9, [x28, #OP_IMMEDIATE]
    ldp x10, x11, [x9, #48]
    ldr x10, [x10]
    cmp x10, x11
    b.ne Lslow
    ldr x10, [x9, #64]
    ldr x10, [x10]
    ldr x11, [x27, #EF_EPOCH]
    cmp x10, x11
    b.ne Lslow
    ldr x10, [x27, #EF_PROOF_GENERATION]
    cbz x10, Lslow
    ldr x11, [x9, #72]
    ldr x11, [x11]
    cmn x11, #1
    b.eq Lbridge_valid
    cmp x10, x11
    b.eq Lbridge_valid
    ldp x12, x13, [x9, #96]
    cbz x13, Lslow
    ldr x14, [x27, #EF_EPOCH]
Lbridge_proof:
    ldp x15, x16, [x12], #16
    ldp x0, x1, [x15, #8]
    cmp x0, x14
    b.ne Lslow
    cmn x1, #1
    b.eq Lbridge_proof_lines
    cmp x1, x10
    b.ne Lbridge_proof_missing
Lbridge_proof_lines:
    ldr x0, [x15, #24]
    and x0, x0, x16
    cmp x0, x16
    b.eq Lbridge_proof_next
Lbridge_proof_missing:
    DIAG_COUNT ED_EVENT_NATIVE_PROOF_ATTEMPT, x0, x1
    bl Lnative_code_proof
    cbz x11, Lslow
    DIAG_COUNT ED_EVENT_NATIVE_PROOF_SUCCESS, x0, x1
Lbridge_proof_next:
    subs x13, x13, #1
    b.ne Lbridge_proof
    // Every required line is proven against the target's owned snapshots.
    // Only serial execution exposes a nonzero observation scope here.
    ldr x11, [x9, #72]
    str x10, [x11]
Lbridge_valid:
    DIAG_COUNT ED_EVENT_BRIDGE_TRANSFERRED
    ldr x9, [x9, #80]
    mov x10, #1
    str x10, [x9]
    str x9, [x27, #EF_GRAPH]
    // Every bridge target is the Block header returned by select().
    ldr x28, [x28, #OP_TARGET]
#if OPERIT_ARM64_DIRECT_CHAINS
    b LBlock
#else
    DISPATCH
#endif
#if OPERIT_ARM64_ADAPTIVE_CALLS
// Temporary bridge for a cold direct call. Only this bridge counts; once the
// threshold is reached the slow path rebuilds the caller or falls back.
LHotBridge:
    ldr w10, [x28, #OP_SIZE+OM_PAYLOAD]
    subs w10, w10, #1
    str w10, [x28, #OP_SIZE+OM_PAYLOAD]
    b.eq Lslow
    b LBridge
#endif
// x15 proof, x16 required lines, x10 scope, x14 code epoch. Native execution
// retains the mapping reader; only x0-x2/x11 and v0-v4 are scratch here.
// An absent observation or a contended page returns to the precise C++ path.
Lnative_code_proof:
    ldp x0, x1, [x15, #CP_NATIVE_EPOCH]
    cmp x0, x14
    b.ne Lnative_code_proof_fail
    ldr x0, [x27, #EF_MAPPING_EPOCH]
    ldr x0, [x0]
    cmp x1, x0
    b.ne Lnative_code_proof_fail
    ldr x0, [x15, #CP_NATIVE_FILE]
    cbz x0, Lnative_code_proof_fail
    eor x1, x0, x0, lsr #8
    ubfx x1, x1, #4, #6
    add x1, x1, x1, lsl #1
    ldr x2, [x27, #EF_FILE_OBSERVATIONS]
    add x2, x2, x1, lsl #3
    ldp x1, x11, [x2]
    cmp x1, x0
    b.ne Lnative_code_proof_fail
    cmp x11, x10
    b.ne Lnative_code_proof_fail
    ldr x0, [x15, #CP_NATIVE_FILE_OFFSET]
    ldr x11, [x2, #16]
    add x11, x11, #4095
    lsr x11, x11, #12
    cmp x11, x0, lsr #12
    b.ls Lnative_code_proof_fail
    ldr x1, [x15, #CP_NATIVE_MUTEX]
    cbz x1, Lnative_code_proof_locked
    ldr w0, [x1, #4]
    cbnz w0, Lnative_code_proof_fail
    ldaxr w0, [x1]
    adds w0, w0, #1
    b.mi Lnative_code_proof_clear
    stxr w2, w0, [x1]
    cbnz w2, Lnative_code_proof_fail
Lnative_code_proof_locked:
    ldp x0, x1, [x15, #8]
    cmp x0, x14
    b.ne Lnative_code_proof_all
    cmp x1, x10
    b.ne Lnative_code_proof_all
    ldr x0, [x15, #24]
    bic x2, x16, x0
    cbnz x2, Lnative_code_proof_loop
    b Lnative_code_proof_publish
Lnative_code_proof_all:
    mov x2, x16
Lnative_code_proof_loop:
    rbit x11, x2
    clz x11, x11
    lsl x11, x11, #6
    ldp x0, x1, [x15, #CP_NATIVE_DATA]
    add x0, x0, x11
    add x1, x1, x11
    ldp q0, q1, [x0]
    ldp q2, q3, [x1]
    eor v0.16b, v0.16b, v2.16b
    eor v1.16b, v1.16b, v3.16b
    orr v0.16b, v0.16b, v1.16b
    ldp q1, q2, [x0, #32]
    ldp q3, q4, [x1, #32]
    eor v1.16b, v1.16b, v3.16b
    eor v2.16b, v2.16b, v4.16b
    orr v1.16b, v1.16b, v2.16b
    orr v0.16b, v0.16b, v1.16b
    umaxv b0, v0.16b
    umov w11, v0.b[0]
    cbnz w11, Lnative_code_proof_mismatch
    sub x11, x2, #1
    ands x2, x2, x11
    b.ne Lnative_code_proof_loop
Lnative_code_proof_publish:
    ldp x0, x11, [x15, #8]
    cmp x0, x14
    ccmp x11, x10, #0, eq
    ldr x0, [x15, #24]
    csel x0, x0, xzr, eq
    orr x0, x0, x16
    stp x14, x10, [x15, #8]
    str x0, [x15, #24]
    mov x11, #1
    b Lnative_code_proof_unlock
Lnative_code_proof_mismatch:
    mov x11, #0
Lnative_code_proof_unlock:
    ldr x1, [x15, #CP_NATIVE_MUTEX]
    cbz x1, Lnative_code_proof_return
Lnative_code_proof_release:
    ldaxr w0, [x1]
    sub w0, w0, #1
    stlxr w2, w0, [x1]
    cbnz w2, Lnative_code_proof_release
Lnative_code_proof_return:
    ret
Lnative_code_proof_clear:
    clrex
Lnative_code_proof_fail:
    mov x11, #0
    ret
LIndirectDispatch:
    DIAG_HANDLER
    DIAG_COUNT ED_EVENT_INDIRECT_CACHE_ENTRY
    ldr x9, [x28, #OP_IMMEDIATE]
    ldp x10, x11, [x9, #48]
    ldr x10, [x10]
    cmp x10, x11
    b.ne Lslow
    ldr x10, [x27, #EF_PENDING_PC]
    lsr x11, x10, #2
    eor x11, x11, x10, lsr #9
    and x11, x11, #31
    add x9, x9, #72
    add x9, x9, x11, lsl #6
    ldp x11, x16, [x9]
    cmp x10, x11
    b.eq Lindirect_hit
    ldp x11, x16, [x9, #16]
    cmp x10, x11
    b.eq Lindirect_hit
    ldp x11, x16, [x9, #32]
    cmp x10, x11
    b.eq Lindirect_hit
    ldp x11, x16, [x9, #48]
    cmp x10, x11
    b.ne Lslow
Lindirect_hit:
    cbz x16, Lslow
    DIAG_COUNT ED_EVENT_INDIRECT_CACHE_HIT, x9, x10
    mov x28, x16
    DISPATCH
LHelper:
    DIAG_HANDLER
    b Lslow
LNop:
    DIAG_HANDLER
    ADVANCE
LJump:
LNext:
    DIAG_HANDLER
    DIAG_COUNT ED_EVENT_CROSS_EDGE_ATTEMPT
    ldrh w12, [x28, #OP_EDGE_PREFIX]
    add x26, x6, x12
    ldr x16, [x28, #OP_TARGET]
    cbz x16, Lslow
    mov x28, x16
    DISPATCH
LLocalNext:
    DIAG_HANDLER
    DIAG_COUNT ED_EVENT_LOCAL_EDGE
    ldrh w12, [x28, #OP_EDGE_PREFIX]
    add x26, x6, x12
    ldr x28, [x28, #OP_TARGET]
    // Shared by Next and Jump: a Jump can target a guarded backedge.
    DISPATCH
// Only selected for Next records, whose in-graph successor is always forward.
LLocalForward:
    DIAG_HANDLER
    DIAG_COUNT ED_EVENT_LOCAL_EDGE
    ldrh w12, [x28, #OP_EDGE_PREFIX]
    add x26, x6, x12
    ldr x28, [x28, #OP_TARGET]
#if OPERIT_ARM64_DIRECT_CHAINS
    b LBlockBody
#else
    DISPATCH
#endif
// No operands or host NZCV are clobbered before a handler starts its work.
// The outer function already owns LR; this internal leaf needs no stack frame.
Lfp_enter:
    DIAG_COUNT ED_EVENT_LAZY_FP_ENTER
    ldr x9, [x27, #EF_GUEST_FPCR]
    msr fpcr, x9
    msr fpsr, xzr
    orr x8, x8, #EXEC_FP_ACTIVE_MASK
    ret
Lslow:
    DIAG_COUNT ED_EVENT_SLOW_ENTRY
    str x3, [x27, #EF_RESIDENT_X30]
    and x9, x8, #EXEC_BLOCK_HITS_MASK
    str x9, [x27, #EF_BLOCK_HITS]
    str x6, [x27, #EF_BLOCK_BASE]
    stp x19, x20, [x27, #0]
    stp x21, x22, [x27, #16]
    stp x23, x24, [x27, #32]
    stp q8, q9, [x27, #48]
    stp q10, q11, [x27, #80]
    stp q12, q13, [x27, #112]
    stp q14, q15, [x27, #144]
    stp x25, x26, [x27, #EF_FLAGS]
    // An inactive native interval has no guest FPSR to merge. Its host
    // environment must remain untouched, including on faults and returns.
    tbz x8, #EXEC_FP_ACTIVE_BIT, Lslow_host
    DIAG_COUNT ED_EVENT_ACTIVE_FP_HOST_SWITCH
    mrs x9, fpsr
    mov x11, #0x9f
    and x9, x9, x11
    ldr x10, [x27, #EF_SHADOW_FPSR]
    orr x9, x9, x10
    str x9, [x27, #EF_SHADOW_FPSR]
    ldp x9, x10, [x27, #EF_HOST_FPCR]
    msr fpcr, x9
    msr fpsr, x10
Lslow_host:
    mov x0, x27
    mov x1, x28
    bl SYM(operit_static_slow)
    cbz x0, Lreturn
    mov x28, x0
    b Lreload
Lreturn:
.cfi_remember_state
    // FPSR is caller-saved: even an integer C++ boundary may change host
    // sticky flags. Restore the invocation's original environment once here.
    ldp x9, x10, [x27, #EF_HOST_FPCR]
    msr fpcr, x9
    msr fpsr, x10
    ldp q8, q9, [sp, #96]
    ldp q10, q11, [sp, #128]
    ldp q12, q13, [sp, #160]
    ldp q14, q15, [sp, #192]
    ldp x19, x20, [sp, #16]
    ldp x21, x22, [sp, #32]
    ldp x23, x24, [sp, #48]
    ldp x25, x26, [sp, #64]
    ldp x27, x28, [sp, #80]
    ldp x29, x30, [sp], #224
.cfi_def_cfa_offset 0
    ret
.cfi_restore_state
#if OPERIT_EXECUTION_DIAGNOSTICS
.p2align 2
Ldiagnostic_handler:
    // Diagnostics alone resolve cold metadata. Keep only scalar snapshot values
    // on this leaf's temporary stack; all native resident state stays untouched.
    sub sp, sp, #32
.cfi_adjust_cfa_offset 32
    // Carry the entry NZCV in the high half while keeping the profile ID low.
    mrs x2, nzcv
    orr x0, x0, x2, lsl #32
    ldrh w1, [x28, #OP_INDEX]
    mov w2, #OP_DETACHED_INDEX
    cmp w1, w2
    b.eq Ldiagnostic_detached_metadata
    sub x1, x28, x1, lsl #5
    ldr x1, [x1, #OP_TARGET]
    ldrh w2, [x28, #OP_INDEX]
    add x1, x1, x2, lsl #4
    b Ldiagnostic_metadata
Ldiagnostic_detached_metadata:
    // This inline record belongs to the source, never the guarded target.
    add x1, x28, #OP_SIZE
Ldiagnostic_metadata:
    ldrb w2, [x1, #OM_KIND]
    ldr w17, [x1, #OM_PAYLOAD]
    ldrb w16, [x1, #OM_MODE]
    orr x17, x17, x16, lsl #32
    orr x17, x17, x2, lsl #56
    str x17, [sp, #8]
    ldr x17, [x1, #OM_ORIGIN]
    cmp w2, #OP_KIND_BRIDGE
    b.eq Ldiagnostic_pc_ready
    cmp w2, #OP_KIND_INDIRECT_DISPATCH
    b.eq Ldiagnostic_pc_ready
    ldr x17, [x17]
    cmp w2, #OP_KIND_NEXT
    b.ne Ldiagnostic_pc_ready
    add x17, x17, #4
Ldiagnostic_pc_ready:
    str x17, [sp, #0]
    ldr x17, [x28, #OP_A]
    and x17, x17, #0x0000ffffffffffff
    ldrh w2, [x1, #OM_PREFIX]
    orr x17, x17, x2, lsl #48
    str x17, [sp, #16]
    add x16, x27, #EF_DIAGNOSTICS
    ldr x1, [sp, #8]
    lsr x1, x1, #56
    ldr x2, [x16, x1, lsl #3]
    add x2, x2, #1
    str x2, [x16, x1, lsl #3]
    add x1, x16, #ED_PROFILES
    and x2, x0, #0xff
    ldr x17, [x1, x2, lsl #3]
    add x17, x17, #1
    str x17, [x1, x2, lsl #3]
    ldr x1, [x16, #ED_SAMPLE_REMAINING]
    sub x1, x1, #1
    str x1, [x16, #ED_SAMPLE_REMAINING]
    cbnz x1, Ldiagnostic_return
    ldr x1, [x16, #ED_SAMPLES]
    add x1, x1, #1
    str x1, [x16, #ED_SAMPLES]
    // A random interval breaks alignment with a periodic guest instruction
    // stream. Update the PRNG only on a sample, outside ordinary count work.
    ldr x1, [x16, #ED_RANDOM_STATE]
    eor x1, x1, x1, lsl #13
    eor x1, x1, x1, lsr #7
    eor x1, x1, x1, lsl #17
    str x1, [x16, #ED_RANDOM_STATE]
    ldr x2, [x16, #ED_SAMPLE_INTERVAL_MASK]
    and x1, x1, x2
    add x1, x1, #1
    str x1, [x16, #ED_SAMPLE_REMAINING]
    ldr x2, [x16, #ED_SITES]
    cbz x2, Ldiagnostic_drop
    ldr x1, [sp, #0]
    eor x1, x1, x28, lsr #4
    eor x1, x1, x1, lsr #17
    ldr x17, [x16, #ED_SITE_SET_MASK]
    and x1, x1, x17
    add x1, x2, x1, lsl #8
Ldiagnostic_way0:
    ldr x2, [x1]
    cbz x2, Ldiagnostic_new
    cmp x2, x28
    b.ne Ldiagnostic_collision0
    ldr x2, [x1, #8]
    ldr x17, [sp, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision0
    ldr x2, [x1, #16]
    ldr x17, [x28, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision0
    ldr x2, [x1, #32]
    ldr x17, [x28, #OP_IMMEDIATE]
    cmp x2, x17
    b.ne Ldiagnostic_collision0
    ldr x2, [x1, #40]
    ldr x17, [sp, #8]
    cmp x2, x17
    b.ne Ldiagnostic_collision0
    ldr x2, [x1, #48]
    ldr x17, [sp, #16]
    cmp x2, x17
    b.ne Ldiagnostic_collision0
    b Ldiagnostic_hit
Ldiagnostic_collision0:
    ldr x2, [x16, #ED_COLLISIONS]
    add x2, x2, #1
    str x2, [x16, #ED_COLLISIONS]
    add x1, x1, #ED_SITE_SIZE
Ldiagnostic_way1:
    ldr x2, [x1]
    cbz x2, Ldiagnostic_new
    cmp x2, x28
    b.ne Ldiagnostic_collision1
    ldr x2, [x1, #8]
    ldr x17, [sp, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision1
    ldr x2, [x1, #16]
    ldr x17, [x28, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision1
    ldr x2, [x1, #32]
    ldr x17, [x28, #OP_IMMEDIATE]
    cmp x2, x17
    b.ne Ldiagnostic_collision1
    ldr x2, [x1, #40]
    ldr x17, [sp, #8]
    cmp x2, x17
    b.ne Ldiagnostic_collision1
    ldr x2, [x1, #48]
    ldr x17, [sp, #16]
    cmp x2, x17
    b.ne Ldiagnostic_collision1
    b Ldiagnostic_hit
Ldiagnostic_collision1:
    ldr x2, [x16, #ED_COLLISIONS]
    add x2, x2, #1
    str x2, [x16, #ED_COLLISIONS]
    add x1, x1, #ED_SITE_SIZE
Ldiagnostic_way2:
    ldr x2, [x1]
    cbz x2, Ldiagnostic_new
    cmp x2, x28
    b.ne Ldiagnostic_collision2
    ldr x2, [x1, #8]
    ldr x17, [sp, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision2
    ldr x2, [x1, #16]
    ldr x17, [x28, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision2
    ldr x2, [x1, #32]
    ldr x17, [x28, #OP_IMMEDIATE]
    cmp x2, x17
    b.ne Ldiagnostic_collision2
    ldr x2, [x1, #40]
    ldr x17, [sp, #8]
    cmp x2, x17
    b.ne Ldiagnostic_collision2
    ldr x2, [x1, #48]
    ldr x17, [sp, #16]
    cmp x2, x17
    b.ne Ldiagnostic_collision2
    b Ldiagnostic_hit
Ldiagnostic_collision2:
    ldr x2, [x16, #ED_COLLISIONS]
    add x2, x2, #1
    str x2, [x16, #ED_COLLISIONS]
    add x1, x1, #ED_SITE_SIZE
Ldiagnostic_way3:
    ldr x2, [x1]
    cbz x2, Ldiagnostic_new
    cmp x2, x28
    b.ne Ldiagnostic_collision3
    ldr x2, [x1, #8]
    ldr x17, [sp, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision3
    ldr x2, [x1, #16]
    ldr x17, [x28, #0]
    cmp x2, x17
    b.ne Ldiagnostic_collision3
    ldr x2, [x1, #32]
    ldr x17, [x28, #OP_IMMEDIATE]
    cmp x2, x17
    b.ne Ldiagnostic_collision3
    ldr x2, [x1, #40]
    ldr x17, [sp, #8]
    cmp x2, x17
    b.ne Ldiagnostic_collision3
    ldr x2, [x1, #48]
    ldr x17, [sp, #16]
    cmp x2, x17
    b.ne Ldiagnostic_collision3
    b Ldiagnostic_hit
Ldiagnostic_collision3:
    ldr x2, [x16, #ED_COLLISIONS]
    add x2, x2, #1
    str x2, [x16, #ED_COLLISIONS]
Ldiagnostic_drop:
    ldr x2, [x16, #ED_DROPPED]
    add x2, x2, #1
    str x2, [x16, #ED_DROPPED]
    b Ldiagnostic_return
Ldiagnostic_new:
    str x28, [x1]
    ldr x2, [sp, #0]
    str x2, [x1, #8]
    ldr x2, [x28]
    str x2, [x1, #16]
    mov x2, #1
    str x2, [x1, #24]
    ldr x2, [x28, #OP_IMMEDIATE]
    str x2, [x1, #32]
    ldp x2, x17, [sp, #8]
    stp x2, x17, [x1, #40]
    and x2, x0, #0xff
    str x2, [x1, #56]
    b Ldiagnostic_return
Ldiagnostic_hit:
    ldr x2, [x1, #24]
    add x2, x2, #1
    str x2, [x1, #24]
Ldiagnostic_return:
    lsr x0, x0, #32
    msr nzcv, x0
    add sp, sp, #32
.cfi_adjust_cfa_offset -32
    ret
#endif
#include "static_handlers.inc"
.cfi_endproc
#if OPERIT_ARM64_ADAPTIVE_CALLS
.globl SYM(operit_static_hot_bridge)
.set SYM(operit_static_hot_bridge), LHotBridge
#endif
#if OPERIT_ARM64_PROFILE_SUPERBLOCKS
#include "profile_superblocks.inc"
#endif
#if OPERIT_ARM64_GUEST_AOT
#include "guest_aot.inc"
#endif
#if OPERIT_ARM64_GUEST_AOT_TRACES
#include "guest_traces.inc"
#endif
.globl SYM(operit_static_kind_FPEnter)
.set SYM(operit_static_kind_FPEnter), Lfp_enter
.globl SYM(operit_static_kind_NativeCodeProof)
.set SYM(operit_static_kind_NativeCodeProof), Lnative_code_proof
#if OPERIT_EXECUTION_DIAGNOSTICS
.globl SYM(operit_static_kind_Diagnostics)
.set SYM(operit_static_kind_Diagnostics), Ldiagnostic_handler
#endif
#ifdef __APPLE__
.section __DATA_CONST,__const
#else
.section .data.rel.ro,"aw"
#endif
#include "static_tables.inc"
#ifndef __APPLE__
.section .note.GNU-stack,"",%progbits
#endif
