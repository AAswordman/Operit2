#ifdef __APPLE__
#define EXERCISE _exercise_fp
#else
#define EXERCISE exercise_fp
#endif
.text
.global EXERCISE
EXERCISE:
    mrs x6, fpcr
    mrs x7, fpsr
    msr fpcr, x3
    msr fpsr, xzr
    fmov d0, x0
    mov v0.d[1], x1
    fmov d1, x1
    mov v1.d[1], x0
    scvtf d2, x0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    scvtf d2, w0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    scvtf s2, x0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    scvtf s2, w0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    ucvtf d2, x0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    ucvtf d2, w0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    ucvtf s2, x0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    ucvtf s2, w0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fadd d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fadd s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fsub d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fsub s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmul d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmul s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fdiv d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fdiv s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmax d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmax s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmin d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmin s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmaxnm d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmaxnm s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fminnm d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fminnm s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fnmul d2, d0, d1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fnmul s2, s0, s1
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fabs d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fabs s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fneg d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fneg s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fsqrt d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fsqrt s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintn d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintn s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintp d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintp s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintm d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintm s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintz d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintz s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frinta d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frinta s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintx d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frintx s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frinti d2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    frinti s2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtns x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtns w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtnu x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtnu w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtas x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtas w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtau x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtau w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtps x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtps w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtpu x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtpu w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtms x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtms w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtmu x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtmu w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzs x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzs w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzu x4, d0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzu w4, s0
    fmov d2, x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    scvtf v2.2d, v0.2d
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    scvtf v2.4s, v0.4s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    scvtf v2.2s, v0.2s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    ucvtf v2.2d, v0.2d
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    ucvtf v2.4s, v0.4s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    ucvtf v2.2s, v0.2s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzs v2.2d, v0.2d
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzs v2.4s, v0.4s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzs v2.2s, v0.2s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzu v2.2d, v0.2d
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzu v2.4s, v0.4s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvtzu v2.2s, v0.2s
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmadd s2, s0, s1, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmadd d2, d0, d1, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmsub s2, s0, s1, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fmsub d2, d0, d1, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fnmadd s2, s0, s1, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fnmadd d2, d0, d1, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fnmsub s2, s0, s1, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fnmsub d2, d0, d1, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvt s2, d0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcvt d2, s0
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcmp d0,d1
    mrs x4,nzcv
    fmov d2,x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcmpe s0,s1
    mrs x4,nzcv
    fmov d2,x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcmp d0,#0.0
    mrs x4,nzcv
    fmov d2,x4
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    fcsel d2,d0,d1,eq
    str q2,[x2],#16
    mrs x4,fpsr
    str x4,[x2],#8
    msr fpcr, x6
    msr fpsr, x7
    ret
