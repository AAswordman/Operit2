#ifdef __APPLE__
#define EXERCISE _exercise
#else
#define EXERCISE exercise
#endif
.text
.global EXERCISE
EXERCISE:
    hint #25
    hint #29
    hint #27
    hint #31
    adds x3, x0, x1
    adc x4, x0, x1
    stp x3, x4, [x2], #16
    subs x3, x0, x1
    sbc x4, x0, x1
    stp x3, x4, [x2], #16
    adds w3, w0, w1
    adc w4, w0, w1
    stp x3, x4, [x2], #16
    subs w3, w0, w1
    sbc w4, w0, w1
    stp x3, x4, [x2], #16
    and x3, x0, x1, ror #17
    orn x4, x0, x1, asr #23
    stp x3, x4, [x2], #16
    eon w3, w0, w1, lsl #5
    and x4, x0, #0xff00ff00ff00ff00
    stp x3, x4, [x2], #16
    ubfx x3, x0, #17, #31
    sbfx x4, x1, #3, #33
    stp x3, x4, [x2], #16
    mov x3, x0
    bfi x3, x1, #47, #11
    extr x4, x0, x1, #37
    stp x3, x4, [x2], #16
    clz x3, x0
    cls x4, x1
    stp x3, x4, [x2], #16
    rbit x3, x0
    rev x4, x1
    stp x3, x4, [x2], #16
    rev16 x3, x0
    rev32 x4, x1
    stp x3, x4, [x2], #16
    mul x3, x0, x1
    umulh x4, x0, x1
    stp x3, x4, [x2], #16
    smulh x3, x0, x1
    smull x4, w0, w1
    stp x3, x4, [x2], #16
    udiv x3, x0, x1
    sdiv x4, x0, x1
    stp x3, x4, [x2], #16
    lsl x3, x0, x1
    asr x4, x0, x1
    stp x3, x4, [x2], #16
    ror w3, w0, w1
    lsr x4, x0, x1
    stp x3, x4, [x2], #16
    cmp x0, x1
    csel x3, x0, x1, ge
    csneg x4, x0, x1, lo
    stp x3, x4, [x2], #16
    ccmp x0, x1, #5, ne
    mrs x3, nzcv
    cinc x4, x0, eq
    stp x3, x4, [x2], #16
    dup v0.16b, w0
    mov v1.d[0], x1
    mov v1.d[1], x0
    eor v2.16b, v0.16b, v1.16b
    stp q0, q2, [x2], #32
    umov x3, v1.d[1]
    smov x4, v1.s[1]
    stp x3, x4, [x2], #16
    movi v3.4s, #0x12, lsl #8
    mvni v4.8h, #0x34
    stp q3, q4, [x2], #32
    add v5.2d, v1.2d, v2.2d
    sub v6.8h, v1.8h, v0.8h
    stp q5, q6, [x2], #32
    zip1 v5.16b, v1.16b, v2.16b
    zip2 v6.8h, v1.8h, v2.8h
    stp q5, q6, [x2], #32
    cmeq v5.4s, v1.4s, v2.4s
    umaxp v6.4s, v1.4s, v2.4s
    stp q5, q6, [x2], #32
    tbl v5.16b, {v0.16b, v1.16b}, v2.16b
    tbx v6.16b, {v0.16b}, v1.16b
    stp q5, q6, [x2], #32
    fmov x3, d1
    fmov w4, s2
    stp x3, x4, [x2], #16
    fmov d5, x0
    fmov s6, w1
    stp q5, q6, [x2], #32
    movi v5.16b, #0xaa
    tbl v5.8b, {v0.16b}, v1.8b
    movi v6.16b, #0xbb
    tbx v6.8b, {v0.16b, v1.16b}, v2.8b
    stp q5, q6, [x2], #32
    zip1 v5.8b, v1.8b, v2.8b
    add v6.4h, v1.4h, v2.4h
    stp q5, q6, [x2], #32
    uzp1 v5.4s, v1.4s, v2.4s
    uzp2 v6.8h, v1.8h, v2.8h
    stp q5, q6, [x2], #32
    trn1 v5.16b, v1.16b, v2.16b
    trn2 v6.2d, v1.2d, v2.2d
    stp q5, q6, [x2], #32
    uzp1 v5.8b, v1.8b, v2.8b
    uzp2 v6.2s, v1.2s, v2.2s
    stp q5, q6, [x2], #32
    trn1 v5.4h, v1.4h, v2.4h
    trn2 v6.8b, v1.8b, v2.8b
    stp q5, q6, [x2], #32
    fmov d5, #2.0
    fmov s6, #-1.0
    stp q5, q6, [x2], #32
    fmov s5, #0.125
    fmov d6, #-31.0
    stp q5, q6, [x2], #32
    mov x5, sp
    sub sp, sp, #16
    str x0, [sp]
    ldaxr x3, [sp]
    add x3, x3, #1
    stlxr w4, x3, [sp]
    ldar x6, [sp]
    stp x4, x6, [x2], #16
    ldxr w3, [sp]
    clrex
    stxr w4, w1, [sp]
    ldr x6, [sp]
    stp x4, x6, [x2], #16
    mov sp, x5
    ret
