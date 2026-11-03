"""Independent AArch64 instruction-form corpus; no emulator implementation imports.

Scope: EL0 ARMv8-A integer, FP and Advanced SIMD forms. Optional extensions are
separate. This is a growing executable corpus, not a claim of exhaustive encoding
coverage. Operand edge cases are supplied by harness.c and compared with hardware.
"""

def cases():
    output = []
    def add(family, asm, feature='armv8-a'):
        output.append(dict(id=f'{len(output):04d}', family=family, asm=asm, feature=feature, records=16 if family=='floating-immediate' else 480 if 'floating' in family else 80))
    for sign in (1,-1):
        for exponent in range(-3,5):
            for fraction in range(16):
                value=sign*(1+fraction/16)*(2.0**exponent)
                for destination in ('s0','d0','v0.2s','v0.4s','v0.2d'):
                    add('floating-immediate',f'fmov {destination}, #{value}')
    for width in ('w', 'x'):
        a,b,c,d=(width+str(i) for i in (3,4,5,6))
        for op in ('add','adds','sub','subs','and','ands','orr','eor','bic','bics','orn','eon'):
            for shift in ('', ', lsl #3', ', lsr #1', ', asr #7'):
                add('integer-arithmetic',f'{op} {a}, {b}, {c}{shift}')
        add('integer-arithmetic',f'cmp {a}, {b}')
        for op in ('adc','adcs','sbc','sbcs','udiv','sdiv','lsl','lsr','asr','ror'):
            add('integer-arithmetic',f'{op} {a}, {b}, {c}')
        for op in ('rbit','rev16','rev','clz','cls'):
            add('integer-bit',f'{op} {a}, {b}')
        for op in ('madd','msub'):
            add('integer-multiply',f'{op} {a}, {b}, {c}, {d}')
        for op in ('csel','csinc','csinv','csneg'):
            for cond in ('eq','lt','hi','al','nv'):
                add('integer-conditional',f'{op} {a}, {b}, {c}, {cond}')
        for op in ('ccmp','ccmn'):
            for rhs in (c,'#15'):
                add('integer-conditional',f'{op} {b}, {rhs}, #9, gt')
        for op in ('ubfm','sbfm','bfm'):
            for r,s in ((0,7),(7,3),(4,15),(31,31)):
                add('integer-bit',f'{op} {a}, {b}, #{r}, #{s}')
        add('integer-bit',f'extr {a}, {b}, {c}, #7')
    for op in ('smulh','umulh'):
        add('integer-multiply',f'{op} x3, x4, x5')
    for op in ('smaddl','smsubl','umaddl','umsubl'):
        add('integer-multiply',f'{op} x3, w4, w5, x6')
    for width in ('s','d'):
        for op in ('fadd','fsub','fmul','fdiv','fmax','fmin','fmaxnm','fminnm','fnmul'):
            add('floating',f'{op} {width}0, {width}1, {width}2')
        for op in ('fabs','fneg','fsqrt','frintn','frintp','frintm','frintz','frinta','frintx','frinti'):
            add('floating',f'{op} {width}0, {width}1')
        for op in ('fmadd','fmsub','fnmadd','fnmsub'):
            add('floating',f'{op} {width}0, {width}1, {width}2, {width}3')
        for op in ('fcmp','fcmpe'):
            add('floating',f'{op} {width}1, {width}2')
        for op in ('fcvtzs','fcvtzu','fcvtns','fcvtnu','fcvtps','fcvtpu','fcvtms','fcvtmu','fcvtas','fcvtau'):
            for gpr in ('w','x'):
                add('floating-conversion',f'{op} {gpr}3, {width}1')
        for op in ('scvtf','ucvtf'):
            for gpr in ('w','x'):
                add('floating-conversion',f'{op} {width}0, {gpr}4')
    for gpr,bits in (('w',32),('x',64)):
        for real in ('s','d'):
            for fraction in range(1,bits+1):
                for op in ('scvtf','ucvtf'):
                    add('floating-fixed-conversion',f'{op} {real}0, {gpr}4, #{fraction}')
                for op in ('fcvtzs','fcvtzu'):
                    add('floating-fixed-conversion',f'{op} {gpr}3, {real}1, #{fraction}')
    for shape in ('8b','16b','4h','8h','2s','4s','2d'):
        for op in ('add','sub','cmeq','cmgt','cmge','cmhi','cmhs','cmtst','sshl','ushl','srshl','urshl'):
            add('vector-arithmetic',f'{op} v0.{shape}, v1.{shape}, v2.{shape}')
        for op in ('abs','neg','sqabs','sqneg','cmgt','cmge','cmeq','cmle','cmlt'):
            zero=', #0' if op.startswith('cm') else ''
            add('vector-unary',f'{op} v0.{shape}, v1.{shape}{zero}')
        for op in ('uzp1','uzp2','trn1','trn2','zip1','zip2'):
            add('vector-permute',f'{op} v0.{shape}, v1.{shape}, v2.{shape}')
    for shape in ('8b','16b','4h','8h','2s','4s'):
        for op in ('mul','mla','mls','smax','smin','umax','umin','sabd','uabd','saba','uaba','shadd','uhadd','srhadd','urhadd','shsub','uhsub','sqadd','uqadd','sqsub','uqsub','addp','smaxp','sminp','umaxp','uminp'):
            add('vector-arithmetic',f'{op} v0.{shape}, v1.{shape}, v2.{shape}')
        for op in ('addv','smaxv','sminv','umaxv','uminv','saddlv','uaddlv'):
            scalar='b' if 'b' in shape else 'h' if 'h' in shape else 's'
            if shape=='2s': continue
            if op.endswith('lv'): scalar={'b':'h','h':'s','s':'d'}[scalar]
            add('vector-reduction',f'{op} {scalar}0, v1.{shape}')
    for shape in ('8b','16b'):
        for op in ('cnt','rbit','rev16','rev32','rev64','not'):
            add('vector-unary',f'{op} v0.{shape}, v1.{shape}')
        for op in ('and','orr','eor','bic','orn','bsl','bit','bif'):
            add('vector-logical',f'{op} v0.{shape}, v1.{shape}, v2.{shape}')
        for offset in (0,1,7):
            add('vector-permute',f'ext v0.{shape}, v1.{shape}, v2.{shape}, #{offset}')
        for op in ('tbl','tbx'):
            add('vector-permute',f'{op} v0.{shape}, {{v1.16b,v2.16b}}, v3.{shape}')
    for shape,bits in (('8b',8),('16b',8),('4h',16),('8h',16),('2s',32),('4s',32),('2d',64)):
        for op in ('sshr','ushr','ssra','usra','srshr','urshr','srsra','ursra','sri'):
            for shift in (1,bits):
                add('vector-shift',f'{op} v0.{shape}, v1.{shape}, #{shift}')
        for op in ('shl','sli'):
            for shift in (0,bits-1):
                add('vector-shift',f'{op} v0.{shape}, v1.{shape}, #{shift}')
    for small,large in (('8b','8h'),('4h','4s'),('2s','2d')):
        bits={'8b':8,'4h':16,'2s':32}[small]
        upper={'8b':'16b','4h':'8h','2s':'4s'}[small]
        for op in ('shrn','rshrn','sqshrn','uqshrn','sqshrun','sqrshrn','uqrshrn','sqrshrun'):
            for shift in (1,bits):
                add('vector-narrow-shift',f'{op} v0.{small},v1.{large},#{shift}')
                add('vector-narrow-shift',f'{op}2 v0.{upper},v1.{large},#{shift}')
                if op not in ('shrn','rshrn'):
                    add('scalar-simd',f'{op} {small[-1]}0,{large[-1]}1,#{shift}')
    for shape,bits in (('8b',8),('16b',8),('4h',16),('8h',16),('2s',32),('4s',32),('2d',64)):
        for op in ('sqshl','uqshl','sqshlu'):
            for shift in (0,bits-1):
                add('vector-shift',f'{op} v0.{shape},v1.{shape},#{shift}')
        for op in ('sqshl','uqshl','sqrshl','uqrshl'):
            add('vector-arithmetic',f'{op} v0.{shape},v1.{shape},v2.{shape}')
    for lane,bits in (('b',8),('h',16),('s',32),('d',64)):
        for op in ('sqshl','uqshl','sqshlu'):
            for shift in (0,bits-1):
                add('scalar-simd',f'{op} {lane}0,{lane}1,#{shift}')
        for op in ('sqshl','uqshl','sqrshl','uqrshl'):
            add('scalar-simd',f'{op} {lane}0,{lane}1,{lane}2')
    for small,large in (('8b','8h'),('4h','4s'),('2s','2d')):
        upper={'8b':'16b','4h':'8h','2s':'4s'}[small]
        for op in ('saddl','uaddl','ssubl','usubl','smull','umull','smlal','umlal','smlsl','umlsl','sabdl','uabdl','sabal','uabal'):
            add('vector-widen',f'{op} v0.{large}, v1.{small}, v2.{small}')
            add('vector-widen',f'{op}2 v0.{large}, v1.{upper}, v2.{upper}')
        for op in ('saddw','uaddw','ssubw','usubw'):
            add('vector-widen',f'{op} v0.{large}, v1.{large}, v2.{small}')
            add('vector-widen',f'{op}2 v0.{large}, v1.{large}, v2.{upper}')
        for op in ('sshll','ushll'):
            add('vector-widen',f'{op} v0.{large}, v1.{small}, #1')
            add('vector-widen',f'{op}2 v0.{large}, v1.{upper}, #1')
        for op in ('xtn','sqxtn','uqxtn','sqxtun'):
            add('vector-narrow',f'{op} v0.{small}, v1.{large}')
            add('vector-narrow',f'{op}2 v0.{upper}, v1.{large}')
    for lane,bits in (('b',8),('h',16),('s',32),('d',64)):
        for index in (0,128//bits-1):
            add('vector-copy',f'mov v0.{lane}[{index}], v1.{lane}[0]')
            scalar=lane+'0'
            add('vector-copy',f'mov {scalar}, v1.{lane}[{index}]')
    for shape in ('2s','4s','2d'):
        for op in ('fadd','fsub','fmul','fdiv','fmax','fmin','fmaxnm','fminnm','fmla','fmls','fcmge','fcmgt','fcmeq','facge','facgt','fabd','frecps','frsqrts'):
            add('vector-floating',f'{op} v0.{shape}, v1.{shape}, v2.{shape}')
        for op in ('fabs','fneg','fsqrt','frintn','frintp','frintm','frintz','frinta','frintx','frinti','scvtf','ucvtf','fcvtzs','fcvtzu','frecpe','frsqrte'):
            add('vector-floating',f'{op} v0.{shape}, v1.{shape}')
    for op in ('add','sub','cmeq','cmgt','cmge','cmhi','cmhs','cmtst','sshl','ushl','srshl','urshl'):
        add('scalar-simd',f'{op} d0,d1,d2')
    for lane in ('b','h','s','d'):
        for op in ('sqadd','uqadd','sqsub','uqsub'):
            add('scalar-simd',f'{op} {lane}0,{lane}1,{lane}2')
        for op in ('sqabs','sqneg'):
            add('scalar-simd',f'{op} {lane}0,{lane}1')
    for op in ('abs','neg','cmgt','cmge','cmeq','cmle','cmlt'):
        add('scalar-simd',f'{op} d0,d1'+(',#0' if op.startswith('cm') else ''))
    add('scalar-simd','addp d0,v1.2d')
    for reg,op in (('w3','ldr'),('x3','ldr'),('x3','ldrsw'),('s0','ldr'),('d0','ldr'),('q0','ldr')):
        add('memory-literal',f'{op} {reg},1f; b 2f; .balign 16; 1: .quad 0x80000000abcdef01,0xff00ff00ff00ff00; 2:')
    for condition in ('eq','ne','cs','cc','mi','pl','vs','vc','hi','ls','ge','lt','gt','le','al','nv'):
        add('branch',f'b.{condition} 1f; mov x3,#7; b 2f; 1: mov x3,#9; 2:')
    for width in ('w','x'):
        for op in ('cbz','cbnz'):
            add('branch',f'{op} {width}4,1f; mov x3,#7; b 2f; 1: mov x3,#9; 2:')
        for op in ('tbz','tbnz'):
            for bit in (0,31) if width=='w' else (0,31,63):
                add('branch',f'{op} {width}4,#{bit},1f; mov x3,#7; b 2f; 1: mov x3,#9; 2:')
    # Memory forms use an initialized 256-byte scratch region at x14. Record
    # the relative writeback address rather than native/guest absolute pointers.
    for reg,op in (('w3','ldrb'),('w3','ldrsb'),('x3','ldrsb'),('w3','ldrh'),('w3','ldrsh'),('x3','ldrsh'),('w3','ldr'),('x3','ldr'),('x3','ldrsw'),('b0','ldr'),('h0','ldr'),('s0','ldr'),('d0','ldr'),('q0','ldr')):
        for address in ('[x14, #16]','[x14], #16','[x14, #16]!'):
            add('memory-load', f'{op} {reg}, {address}; sub x6,x14,x9')
    for reg,op in (('w3','strb'),('w3','strh'),('w3','str'),('x3','str'),('b0','str'),('h0','str'),('s0','str'),('d0','str'),('q0','str')):
        for address in ('[x14, #16]','[x14], #16','[x14, #16]!'):
            add('memory-store', f'{op} {reg}, {address}; sub x6,x14,x9')
    for op in ('ldp','stp','ldnp','stnp'):
        for regs in ('w3,w4','x3,x4','s0,s1','d0,d1','q0,q1'):
            add('memory-pair',f'{op} {regs},[x14,#16]; sub x6,x14,x9')
    for op in ('ldp','stp'):
        for regs in ('x3,x4','q0,q1'):
            for address in ('[x14],#32','[x14,#32]!'):
                add('memory-pair',f'{op} {regs},{address}; sub x6,x14,x9')
    for shape in ('8b','16b','4h','8h','2s','4s','2d'):
        for count in (1,2,3,4):
            regs=','.join(f'v{j}.{shape}' for j in range(count))
            for op in ('ld1','st1'):
                add('memory-structure',f'{op} {{{regs}}},[x14]; sub x6,x14,x9')
        for count in (2,3,4):
            regs=','.join(f'v{j}.{shape}' for j in range(count))
            for op in (f'ld{count}',f'st{count}'):
                add('memory-structure',f'{op} {{{regs}}},[x14]; sub x6,x14,x9')
    for lane,lanes in (('b',16),('h',8),('s',4),('d',2)):
        for count in (1,2,3,4):
            for index in (0,lanes-1):
                regs=','.join(f'v{j}.{lane}' for j in range(count))
                for op in (f'ld{count}',f'st{count}'):
                    add('memory-lane',f'{op} {{{regs}}}[{index}],[x14]; sub x6,x14,x9')
            shape=str(lanes)+lane
            regs=','.join(f'v{j}.{shape}' for j in range(count))
            add('memory-replicate',f'ld{count}r {{{regs}}},[x14]; sub x6,x14,x9')
    return output
