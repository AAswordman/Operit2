#!/usr/bin/env python3
"""Emit a bounded static handler set. This tool runs on the build host only.

Each variant is an ordinary assembly label. Guest input produces operation
records referencing this immutable set; it never produces executable bytes.
"""
import argparse
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--definitions',type=Path,required=True);p.add_argument('--output',type=Path,required=True);args=p.parse_args();args.output.mkdir(parents=True,exist_ok=True)
handlers=[];tables=[];cpp=['#include "cpu/aarch64/block_graph.h"','#include "cpu/aarch64/execution_diagnostics.h"','namespace operit::arm64 {','extern "C" {'];cases=[]
conds=['eq','ne','hs','lo','mi','pl','vs','vc','hi','ls','ge','lt','gt','le','al','al']
def emit(*lines):handlers.extend(lines)
# Profiles count selected register operand slots, not estimated guest work.
# Mutating a profile during emission avoids duplicating the role rules in C++.
active_profile=None;pending_profiles=[];definition_names=[]
def binding(group,role):
    if active_profile is None:return
    limit=8 if group.startswith('vector') else 6
    if role>limit:return  # GP immediate/zero/discard roles have no register slot.
    slot={'gp_read':0,'gp_write':2,'vector_read':4,'vector_write':6}[group]
    active_profile[slot+int(role==limit)]+=1
def begin_handler(label,flags=0):
    global active_profile
    active_profile=[0]*8+[flags];pending_profiles.append(active_profile)
    emit(label+':',f'    DIAG_HANDLER @PROFILE{len(pending_profiles)-1}@')
def gp(role,field,dest,bits=64):
    binding('gp_read',role)
    r='x' if bits==64 else 'w'
    if role<6:emit(f'    mov {r}{dest}, {r}{19+role}')
    elif role==6:emit(f'    ldrh w12, [x28, #{field}]',f'    ldr {r}{dest}, [x27, x12]')
    elif field=='OP_B':emit(f'    ldr {r}{dest}, [x28, #OP_IMMEDIATE]')
    else:emit(f'    mov {r}{dest}, {r}zr')
def input_gp(role,field,scratch,bits=64):
    r='x' if bits==64 else 'w'
    if role<6:
        binding('gp_read',role)
        return f'{r}{19+role}'
    gp(role,field,scratch,bits)
    return f'{r}{scratch}'
def output_gp(role,scratch=11,bits=64):
    if role<6:binding('gp_write',role)
    return ('x' if bits==64 else 'w')+str(19+role if role<6 else scratch)
def complete_gp(role,field,src=11):
    if role>=6:store(role,field,src)
def input_fp(role,field,scratch):
    if role<8:
        binding('vector_read',role)
        return f'd{8+role}'
    vector(role,field,scratch)
    return f'd{scratch}'
def store(role,field,src=11,bits=64):
    # A W destination always zero-extends the architectural X register.
    binding('gp_write',role)
    if role<6:emit(f'    mov x{19+role}, x{src}')
    elif role==6:emit(f'    ldrh w12, [x28, #{field}]',f'    str x{src}, [x27, x12]')
def vector(role,field,dest):
    binding('vector_read',role)
    if role<8:emit(f'    fmov d{dest}, d{8+role}')
    else:emit(f'    ldrh w12, [x28, #{field}]',f'    ldr d{dest}, [x27, x12]')
def vstore(role,src=0,scalar=True):
    binding('vector_write',role)
    if role<8:emit(f'    mov v{role+8}.16b, v{src}.16b')
    else:emit('    ldrh w12, [x28, #OP_D]',f'    str q{src}, [x27, x12]')
def finish():emit('    ADVANCE')
def taken(local=False):
    emit('    DIAG_COUNT '+('ED_EVENT_LOCAL_EDGE' if local else 'ED_EVENT_CROSS_EDGE_ATTEMPT'))
    emit('    ldrh w12, [x28, #OP_EDGE_PREFIX]','    add x26, x6, x12','    ldr x16, [x28, #OP_TARGET]')
    if not local:emit('    cbz x16, Lslow')
    emit('    mov x28, x16','    DISPATCH')
def memory_check(size,writing,offset=True):
    emit('    DIAG_COUNT '+('ED_EVENT_MEMORY_WRITE_ATTEMPT' if writing else 'ED_EVENT_MEMORY_READ_ATTEMPT'))
    if offset:emit('    ldr x12, [x28, #OP_IMMEDIATE]','    add x9, x9, x12')
    emit(f'    bl Lmemoryguard_{int(writing)}_{size}')
def memory_guard(size,writing):
    # x9 guest address -> x11 host pointer, x13 held read lock or zero.
    # Full tags prove permissions; mapping mutations revoke the entire index.
    alias='operit_static_kind_MemoryGuard'+('Write' if writing else 'Read')+str(size)
    tables.extend(['.globl SYM('+alias+')','.set SYM('+alias+'), Lmemoryguard_'+str(int(writing))+'_'+str(size)])
    emit(f'Lmemoryguard_{int(writing)}_{size}:')
    if not writing:emit('    mov x13, #0')
    unique=len(handlers);fallback=f'Lpagefallback{unique}';done=f'Lpagedone{unique}'
    # A valid tag is a page base below address_limit. Its unsigned difference
    # proves the full tag and access range; bit 0 rejects the UINT64_MAX sentinel.
    emit(f'    cbz x7, {fallback}','    eor x12, x9, x9, lsr #13','    ubfx x12, x12, #12, #13',
         '    add x11, x7, x12, lsl #5',f'    ldp x12, x16, [x11, #{16 if writing else 0}]',
         f'    tbnz x12, #0, {fallback}','    sub x12, x9, x12',f'    cmp x12, #{4096-size}',f'    b.hi {fallback}')
    if writing:emit('    str xzr, [x16, #8]','    ldr x16, [x11, #8]')
    emit('    add x11, x16, x9',
         '    DIAG_COUNT '+('ED_EVENT_PAGE_CACHE_WRITE_HIT' if writing else 'ED_EVENT_PAGE_CACHE_READ_HIT'),
         '    ret x30',fallback+':','    and x10, x9, #4095')
    if size>1:
        within=f'Lmemwithin{unique}'
        emit(f'    cmp x10, #{4096-size}',f'    b.ls {within}',
             '    DIAG_COUNT '+('ED_EVENT_MEMORY_WRITE_CROSS_PAGE' if writing else 'ED_EVENT_MEMORY_READ_CROSS_PAGE'),
             '    b Lslow',within+':')
    emit('    DIAG_COUNT '+('ED_EVENT_TRANSLATION_WRITE_FALLBACK' if writing else 'ED_EVENT_TRANSLATION_READ_FALLBACK'),
         '    adr x2, '+done,'    b Lmemoryfallback_'+str(int(writing)),done+':','    ret x30')
def memory_fallback(writing):
    emit('Lmemoryfallback_'+str(int(writing))+':')
    alias='operit_static_kind_MemoryFallback'+('Write' if writing else 'Read')
    tables.extend(['.globl SYM('+alias+')','.set SYM('+alias+'), Lmemoryfallback_'+str(int(writing))])
    emit('    ldr x11, [x27, #EF_FIRST]','    cbz x11, Lslow','    lsr x12, x9, #12','    eor x12, x12, x12, lsr #10','    and x12, x12, #1023','    add x11, x11, x12, lsl #6','    and x14, x9, #0xfffffffffffff000',
         '    ldr x15, [x27, #EF_MAPPING_EPOCH]','    ldr x15, [x15]')
    # Two-way probing uses a private continuation label per variant.
    unique=len(handlers);hit=f'Lmemhit{unique}';second=f'Lmemsecond{unique}'
    for way in range(2):
        miss=second if way==0 else 'Lslow'
        emit('    ldp x16, x17, [x11]','    cmp x16, x14',f'    b.ne {miss}','    cmp x17, x15',f'    b.ne {miss}',
             '    ldrb w16, [x11, #62]',f'    tbz w16, #{1 if writing else 0}, {miss}',
             '    ldr x16, [x11, #32]')
        # Private file copies still depend on file-size metadata checks.
        valid=f'Lmemvalid{unique}_{way}'
        observed=f'Lfileobserved{unique}_{way}'
        emit(f'    cbz x16, {valid}','    DIAG_COUNT ED_EVENT_FILE_BACKED_PROBE, x1, x17','    ldr x17, [x27, #EF_FILE_GENERATION]','    cbz x17, Lslow',
             '    ldr x14, [x11, #48]','    cmp x14, x17',f'    b.eq {observed}',
             '    ldr x13, [x27, #EF_FILE_OBSERVATIONS]',
             '    lsr x14, x16, #4','    eor x14, x14, x16, lsr #12','    and x14, x14, #63',
             '    add x14, x14, x14, lsl #1','    add x13, x13, x14, lsl #3',
             '    ldr x14, [x13]','    cmp x14, x16','    b.ne Lslow',
             '    ldr x14, [x13, #8]','    cmp x14, x17','    b.ne Lslow',
             '    ldr x14, [x13, #16]','    add x14, x14, #4095','    and x14, x14, #0xfffffffffffff000',
             '    ldr x16, [x11, #40]','    cmp x16, x14','    b.hs Lslow',
             '    str x17, [x11, #48]','    DIAG_COUNT ED_EVENT_FILE_OBSERVATION_HIT, x1, x14',
             observed+':','    DIAG_COUNT ED_EVENT_FILE_VALIDATED_HIT, x1, x14','    mov x13, #0',
             valid+':',f'    DIAG_COUNT ED_EVENT_FALLBACK_WAY{way}_HIT, x1, x17',f'    b {hit}')
        if way==0:emit(second+':','    ldr x11, [x27, #EF_SECOND]','    add x11, x11, x12, lsl #6')
    emit(hit+':')
    if writing:emit('    ldr x16, [x11, #24]','    str xzr, [x16, #8]')
    if not writing:
        unlocked=f'Lfileunlocked{unique}';retry=f'Lfilelock{unique}';blocked=f'Lfileblocked{unique}'
        emit('    ldrb w16, [x11, #61]',f'    cbz w16, {unlocked}',
             '    DIAG_COUNT ED_EVENT_FILE_LOCK_ATTEMPT, x1, x17',
             '    ldr x13, [x11, #24]',retry+':','    ldr w16, [x13, #4]',f'    cbnz w16, {blocked}',
             '    ldaxr w16, [x13]','    add w17, w16, #2',f'    tbnz w17, #31, {blocked}',
             '    add w16, w16, #1','    stxr w17, w16, [x13]',f'    cbnz w17, {retry}','    DIAG_COUNT ED_EVENT_FILE_LOCK_ACQUIRED, x1, x17',f'    b {unlocked}',
             blocked+':','    clrex','    DIAG_COUNT ED_EVENT_FILE_LOCK_BLOCKED, x1, x17','    b Lslow',unlocked+':')
    # Refill collisions and private file copies after their scope size proof.
    # Physical file storage still acquires its lock on every access.
    uncached=f'Lmemuncached{unique}'
    if not writing:emit(f'    cbnz x13, {uncached}')
    emit('    eor x12, x9, x9, lsr #13','    ubfx x12, x12, #12, #13',
         '    add x12, x7, x12, lsl #5','    and x14, x9, #0xfffffffffffff000',
         '    ldp x16, x17, [x11, #16]','    sub x16, x16, x14','    ldrb w15, [x11, #62]',
         '    tst w15, #1','    csinv x1, x14, xzr, ne','    tst w15, #2',
         '    csinv x15, x14, xzr, ne','    stp x1, x16, [x12]','    stp x15, x17, [x12, #16]',uncached+':',
         '    ldr x11, [x11, #16]','    add x11, x11, x10','    br x2')
def memory_release():
    unique=len(handlers);done=f'Lfileunlockdone{unique}';retry=f'Lfileunlock{unique}'
    emit(f'    cbz x13, {done}',retry+':','    ldxr w14, [x13]','    sub w14, w14, #1',
         '    stlxr w15, w14, [x13]',f'    cbnz w15, {retry}',done+':')
for writing in (False,True):memory_fallback(writing)
for writing in (False,True):
    for size in (1,2,4,8,16,32):memory_guard(size,writing)
for line in args.definitions.read_text().splitlines():
    if not line or line.startswith('#'):continue
    name,kind,mn,fc=line.split();fc=int(fc);labels=[];definition_names.append(name)
    emit('Lstatic_kind_'+name+':')
    tables.extend(['.globl SYM(operit_static_kind_'+name+')', '.set SYM(operit_static_kind_'+name+'), Lstatic_kind_'+name])
    original_kind=kind
    writeback=kind.startswith('wb')
    if writeback:kind=kind[2:]
    indexed=kind in ('pload','pstore','pvload','pvstore','psignedload')
    if indexed:kind=kind[1:]
    if kind=='sysread':configs=[(64,0,d,7,7) for d in range(8)]
    elif kind=='syswrite':configs=[(64,0,7,a,7) for a in range(8)]
    elif kind=='conditionalcmp':configs=[(bits,f,7,a,b) for bits in (32,64) for f in range(fc) for a in range(8) for b in range(8)]
    elif kind=='fpunary':configs=[(bits,f,d,a,7) for bits in (32,64) for f in range(fc) for d in range(9) for a in range(9)]
    elif kind in ('gptov','uitod'):configs=[(bits,f,d,a,7) for bits in ((64,) if kind=='uitod' else (32,64)) for f in range(fc) for d in range(9) for a in range(8)]
    elif kind=='vtogp':configs=[(bits,f,d,a,7) for bits in (32,64) for f in range(fc) for d in range(8) for a in range(9)]
    elif kind=='vimm':configs=[(bits,0,d,7,7) for bits in (32,64) for d in range(9)]
    elif kind in ('pairload','pairstore','vpairload','vpairstore'):
        roles=9 if kind.startswith('v') else 8
        configs=[(bits,f,d,a,b) for bits in ((32,64,128) if kind.startswith('v') else (32,64)) for f in range(fc) for d in range(roles) for a in range(8) for b in range(roles)]
    elif kind=='signedload':configs=[(bits,f,d,a,7) for bits in (8,16,32) for f in range(2) for d in range(8) for a in range(8)]
    elif kind in ('move','keep'):configs=[(bits,0,d,7,7) for bits in (32,64) for d in range(8)]
    elif kind in ('load','vload'):configs=[(bits,0,d,a,7) for bits in ((8,16,32,64) if kind=='load' else (32,64,128)) for d in range(8 if kind=='load' else 9) for a in range(8)]
    elif kind in ('store','vstore'):configs=[(bits,0,7,a,b) for bits in ((8,16,32,64) if kind=='store' else (32,64,128)) for a in range(8) for b in range(8 if kind=='store' else 9)]
    elif kind=='high':configs=[(64,0,d,a,b) for d in range(8) for a in range(8) for b in range(8)]
    elif kind in ('prepare','operand'):configs=[(bits,f,7,a,7) for bits in (32,64) for f in range(fc) for a in range(8)]
    elif kind in ('prepared','unary','bitfield'):configs=[(bits,f,d,a,7) for bits in (32,64) for f in range(fc) for d in range(8) for a in range(8)]
    elif kind=='comparebranch':configs=[(bits,f,7,a,7) for bits in (32,64) for f in range(fc) for a in range(8)]
    elif kind=='fpma':configs=[(64,f,d,a,b) for f in range(fc) for d in range(9) for a in range(9) for b in range(9)]
    elif kind=='fpcmp':configs=[(64,f,7,a,b) for f in range(2) for a in range(9) for b in range(9)]
    elif kind in ('fp','fpsqrt','fpselect'):configs=[(64,0,d,a,b) for d in range(9) for a in range(9) for b in (range(9) if kind in ('fp','fpselect') else [7])]
    else:configs=[(bits,f,d,a,b) for bits in (32,64) for f in range(fc) for d in range(8) for a in range(8) for b in range(8)]
    for bits,f,d,a,b in configs:
        label=f'L{name}_{bits}_{f}_{d}_{a}_{b}';labels.append(label);emit('.p2align 2')
        profile_flags=(1 if kind=='conditionalcmp' else 0)|(2 if kind in ('load','signedload','store','vload','vstore','pairload','pairstore','vpairload','vpairstore') else 0)|(4 if writeback or kind in ('pairload','pairstore','vpairload','vpairstore') else 0)|(8 if kind=='fpma' else 0)
        begin_handler(label,profile_flags);r='x' if bits==64 else 'w'
        if kind in ('fp','fpsqrt','fpcmp','fpma','fpselect','fpunary','uitod'):emit('    FP_ENTER')
        if kind=='sysread':
            dest=output_gp(d)
            if name=='ReadTls':emit(f'    ldr {dest}, [x27, #EF_SHADOW_TLS]')
            else:emit(f'    mov {dest}, x25')
            complete_gp(d,'OP_D');finish()
        elif kind=='syswrite':
            src=input_gp(a,'OP_A',9)
            if name=='WriteTls':emit(f'    str {src}, [x27, #EF_SHADOW_TLS]')
            else:emit(f'    and x25, {src}, #0xf0000000')
            finish()
        elif kind=='conditionalcmp':
            emit('    msr nzcv, x25',f'    b.{conds[f//2]} {label}_compare',
                 '    ldrb w25, [x28, #OP_D]','    lsl x25, x25, #28')
            finish();emit(label+'_compare:')
            left=input_gp(a,'OP_A',9,bits);right=input_gp(b,'OP_B',10,bits)
            emit(f'    {"cmp" if f&1 else "cmn"} {left}, {right}','    mrs x25, nzcv');finish()
        elif kind=='fpunary':
            binding('vector_read',a)
            if d<8:binding('vector_write',d)
            vr='d' if bits==64 else 's'
            if a<8:src=f'{vr}{8+a}'
            else:emit('    ldrh w12, [x28, #OP_A]',f'    ldr {vr}0, [x27, x12]');src=f'{vr}0'
            dest=f'{vr}{8+d if d<8 else 0}'
            emit(f'    {("fmov","fabs","fneg")[f]} {dest}, {src}')
            if d>=8:vstore(d)
            finish()
        elif kind in ('move','keep'):
            if kind=='move':
                emit(f'    ldr {output_gp(d,bits=bits)}, [x28, #OP_IMMEDIATE]')
                complete_gp(d,'OP_D');finish();continue
            emit('    ldr x11, [x28, #OP_IMMEDIATE]')
            if kind=='keep':
                gp(d,'OP_D',9,bits);emit('    ldrb w12, [x28, #OP_B]','    lsl x12, x12, #4','    mov x10, #65535','    lsl x10, x10, x12','    bic x9, x9, x10','    orr x11, x11, x9')
            if bits==32:emit('    mov w11, w11')
            store(d,'OP_D');finish()
        elif name=='Or' and a==7 and b<7:
            # Unprepared ORR with ZR as its left operand is a register copy.
            source=input_gp(b,'OP_B',11,bits);dest=output_gp(d,bits=bits)
            # A resident W self-copy must still clear the architectural high half.
            if source!=dest or (bits==32 and b<6):emit(f'    mov {dest}, {source}')
            complete_gp(d,'OP_D');finish()
        elif kind in ('binary','prepared','multiplyacc','high'):
            left=input_gp(a,'OP_A',9,bits)
            right=f'{r}15' if kind=='prepared' else input_gp(b,'OP_B',10,bits)
            dest=output_gp(d,bits=bits)
            if name in ('Adc','Sbc'):emit('    msr nzcv, x25')
            instruction=mn+('s' if f and name in ('Add','Sub','Adc','Sbc','And','AddPrepared','SubPrepared','AndPrepared') else '')
            emit(f'    {instruction} {dest}, {left}, {right}'+(f', {r}15' if kind=='multiplyacc' else ''))
            if f:emit('    mrs x25, nzcv')
            complete_gp(d,'OP_D');finish()
        elif kind=='operand':
            gp(a,'OP_A',15,bits);finish()
        elif kind=='prepare':
            gp(a,'OP_A',15,bits)
            emit('    ldr x13, [x28, #OP_IMMEDIATE]')
            mode=f%12
            if mode>=4:
                width=(8,16,32,64)[(mode-4)//2]
                if width<64:
                    if mode&1:emit(f'    {"sxtb" if width==8 else "sxth" if width==16 else "sxtw"} x15, w15')
                    else:emit('    mov w15, w15' if width==32 else f'    {"uxtb" if width==8 else "uxth"} w15, w15')
                mnemonic='lsl'
            else:mnemonic=('lsl','lsr','asr','ror')[mode]
            emit(f'    {mnemonic} {r}15, {r}15, {r}13')
            if f>=12:emit(f'    mvn {r}15, {r}15')
            finish()
        elif kind=='unary':
            left=input_gp(a,'OP_A',9,bits);dest=output_gp(d,bits=bits)
            mnemonic=('rbit','rev16','rev32' if bits==64 else 'rev','rev','clz','cls')[f]
            emit(f'    {mnemonic} {dest}, {left}');complete_gp(d,'OP_D');finish()
        elif kind=='bitfield':
            gp(a,'OP_A',9,bits);gp(d,'OP_D',10,bits)
            emit('    ldrh w12, [x28, #OP_B]','    and x13, x12, #63',f'    ror {r}11, {r}9, {r}13','    ldr x13, [x28, #OP_IMMEDIATE]',f'    and {r}11, {r}11, {r}13')
            if f==1:emit(f'    bic {r}14, {r}10, {r}13',f'    orr {r}11, {r}11, {r}14')
            emit('    ldr x13, [x28, #OP_TARGET]',f'    and {r}11, {r}11, {r}13')
            if f==0:
                emit('    ubfx x12, x12, #6, #6',f'    lsr {r}14, {r}9, {r}12',f'    and {r}14, {r}14, #1',f'    neg {r}14, {r}14',f'    bic {r}14, {r}14, {r}13',f'    orr {r}11, {r}11, {r}14')
            elif f==1:emit(f'    bic {r}10, {r}10, {r}13',f'    orr {r}11, {r}11, {r}10')
            store(d,'OP_D');finish()
        elif kind=='extract':
            gp(a,'OP_A',9,bits)
            if b==7:emit(f'    mov {r}10, {r}zr')
            else:gp(b,'OP_B',10,bits)
            emit('    ldr x12, [x28, #OP_IMMEDIATE]',f'    mov {r}11, {r}10',f'    cbz x12, {label}_done',f'    lsr {r}11, {r}10, {r}12',f'    mov x13, #{bits}','    sub x13, x13, x12',f'    lsl {r}9, {r}9, {r}13',f'    orr {r}11, {r}11, {r}9',label+'_done:');store(d,'OP_D');finish()
        elif kind=='select':
            gp(a,'OP_A',9,bits)
            if b==7:emit(f'    mov {r}10, {r}zr')
            else:gp(b,'OP_B',10,bits)
            emit('    ldrb w12, [x28, #OP_EXTRA]',f'    tbz w12, #0, {label}_increment',f'    mvn {r}10, {r}10',label+'_increment:',f'    tbz w12, #1, {label}_select',f'    add {r}10, {r}10, #1',label+'_select:',
                 '    msr nzcv, x25','    ldr x16, [x28, #OP_IMMEDIATE]','    STATIC_ADDRESS x12, Lgp_conditions','    ldr x16, [x12, x16, lsl #3]','    add x16, x12, x16',f'    adr x17, {label}_done','    br x16',label+'_done:')
            # CSEL on X operands is also correct for W sources, except CSINV's
            # upper bits; truncate after selection for every 32-bit destination.
            if bits==32:emit('    mov w11, w11')
            store(d,'OP_D');finish()
        elif kind=='comparebranch':
            emit(f'    DIAG_BRANCH {f}, 0, 1')
            left=input_gp(a,'OP_A',9,bits);emit(f'    ldr {r}10, [x28, #OP_IMMEDIATE]',f'    cmp {left}, {r}10','    mrs x25, nzcv',f'    b.{conds[f&15]} {label}_taken',f'    FALLTHROUGH {int(f>=16)}');emit(label+'_taken:',f'    DIAG_BRANCH {f}, 1, 1');taken(f>=16)
        elif kind in ('fp','fpsqrt','fpma','fpselect','fpcmp'):
            if kind in ('fp','fpsqrt','fpcmp'):
                left=input_fp(a,'OP_A',0)
                right=input_fp(b,'OP_B',1) if kind!='fpsqrt' else None
                if kind=='fpcmp':
                    emit(f'    fcmp {left}, '+('#0.0' if f else right),'    mrs x25, nzcv')
                else:
                    if d<8:binding('vector_write',d)
                    dest=f'd{8+d}' if d<8 else 'd0'
                    emit(f'    {mn} {dest}, {left}'+(f', {right}' if kind=='fp' else ''))
                    if d>=8:vstore(d)
                finish();continue
            vector(a,'OP_A',0)
            if kind!='fpsqrt':vector(b,'OP_B',1)
            if kind=='fpma':
                emit('    ldrb w16, [x28, #OP_EXTRA]','    STATIC_ADDRESS x12, Lfma_sources','    ldr x16, [x12, x16, lsl #3]','    add x16, x12, x16',f'    adr x17, {label}_compute','    br x16',label+'_compute:')
                emit(f'    {("fmadd","fmsub","fnmadd","fnmsub")[f]} d0, d0, d1, d2')
            elif kind=='fpselect':
                emit('    msr nzcv, x25','    ldrb w16, [x28, #OP_IMMEDIATE]','    STATIC_ADDRESS x12, Lfp_conditions','    ldr x16, [x12, x16, lsl #3]','    add x16, x12, x16',f'    adr x17, {label}_compute','    br x16',label+'_compute:')
            elif kind=='fpcmp':
                emit('    fcmp d0, #0.0' if f else '    fcmp d0, d1','    mrs x25, nzcv');finish();continue
            else:emit(f'    {mn} d0, d0'+(', d1' if kind=='fp' else ''))
            vstore(d);finish()
        elif kind in ('gptov','uitod','vtogp','vimm'):
            if kind=='vtogp':
                binding('vector_read',a)
                if a<8:emit(f'    mov v0.16b, v{a+8}.16b')
                else:emit('    ldrh w12, [x28, #OP_A]','    ldr q0, [x27, x12]')
                emit('    umov x11, v0.d[1]' if f else f'    fmov {r}11, {"s" if bits==32 else "d"}0')
                store(d,'OP_D');finish()
            else:
                if kind=='vimm':emit('    ldr x9, [x28, #OP_IMMEDIATE]')
                else:gp(a,'OP_A',9,bits)
                if kind=='uitod':emit('    ucvtf d0, x9')
                elif f:
                    binding('vector_read',d)
                    if d<8:emit(f'    mov v0.16b, v{d+8}.16b')
                    else:emit('    ldrh w12, [x28, #OP_D]','    ldr q0, [x27, x12]')
                    emit('    ins v0.d[1], x9')
                else:emit(f'    fmov {"s" if bits==32 else "d"}0, {r}9')
                vstore(d);finish()
        elif kind in ('pairload','pairstore','vpairload','vpairstore'):
            gp(a,'OP_A',9)
            emit('    ldr x12, [x28, #OP_IMMEDIATE]','    add x0, x9, x12',
                 '    ldrb w12, [x28, #OP_EXTRA]','    cmp w12, #1','    csel x9, x9, x0, eq')
            writing=kind.endswith('store');vecpair=kind.startswith('v')
            memory_check(bits//4,writing,offset=False)
            if vecpair:
                vr='s' if bits==32 else 'd' if bits==64 else 'q'
                if writing:
                    for role,field,dest in [(d,'OP_D',0),(b,'OP_B',1)]:
                        binding('vector_read',role)
                        if role<8:emit(f'    mov v{dest}.16b, v{role+8}.16b')
                        else:emit(f'    ldrh w12, [x28, #{field}]',f'    ldr q{dest}, [x27, x12]')
                    emit(f'    stp {vr}0, {vr}1, [x11]')
                else:
                    emit(f'    ldp {vr}0, {vr}1, [x11]');vstore(d,0)
                    binding('vector_write',b)
                    if b<8:emit(f'    mov v{b+8}.16b, v1.16b')
                    else:emit('    ldrh w12, [x28, #OP_B]','    str q1, [x27, x12]')
            elif writing:
                gp(d,'OP_D',1,bits)
                if b==7:emit(f'    mov {r}2, {r}zr')
                else:gp(b,'OP_B',2,bits)
                emit(f'    stp {r}1, {r}2, [x11]')
            else:
                emit('    ldpsw x1, x2, [x11]' if f and bits==32 else f'    ldp {r}1, {r}2, [x11]')
                store(d,'OP_D',1);store(b,'OP_B',2)
            if not writing:memory_release()
            emit('    ldrb w12, [x28, #OP_EXTRA]',f'    tbz w12, #0, {label}_done')
            store(a,'OP_A',0);emit(label+'_done:');finish()
        elif kind in ('load','signedload','store','vload','vstore'):
            gp(a,'OP_A',9)
            if indexed:emit('    add x9, x9, x15')
            size=bits//8
            if writeback:
                emit('    ldr x12, [x28, #OP_IMMEDIATE]','    add x0, x9, x12',
                     '    ldrb w12, [x28, #OP_EXTRA]','    cmp w12, #2','    csel x9, x9, x0, eq')
            memory_check(size,kind in ('store','vstore'),offset=not writeback)
            if kind=='signedload':
                load=('ldrsb','ldrsh','ldrsw')[0 if bits==8 else 1 if bits==16 else 2]
                result='w' if f and bits<32 else 'x'
                dest=output_gp(d,10,32 if result=='w' else 64)
                emit(f'    {load} {dest}, [x11]');complete_gp(d,'OP_D',10)
            elif kind=='load':
                dest=output_gp(d,10,64 if bits==64 else 32)
                emit(f'    {"ldrb" if bits==8 else "ldrh" if bits==16 else "ldr"} {dest}, [x11]');complete_gp(d,'OP_D',10)
            elif kind=='store':
                # gp source loading may use x12 but leaves the checked pointer x11.
                source=('xzr' if bits==64 else 'wzr') if b==7 else input_gp(b,'OP_B',10,64 if bits==64 else 32)
                emit(f'    {"strb" if bits==8 else "strh" if bits==16 else "str"} {source}, [x11]')
            elif kind=='vload':
                if d<8:binding('vector_write',d)
                dest=(8+d) if d<8 else 0
                emit(f'    ldr {"s" if bits==32 else "d" if bits==64 else "q"}{dest}, [x11]')
                if d>=8:vstore(d)
            else:
                binding('vector_read',b)
                if b<8:emit(f'    str {"s" if bits==32 else "d" if bits==64 else "q"}{8+b}, [x11]')
                else:emit('    ldrh w12, [x28, #OP_B]','    add x12, x27, x12',f'    ldr q0, [x12]',f'    str {"s" if bits==32 else "d" if bits==64 else "q"}0, [x11]')
            if kind in ('load','signedload','vload'):memory_release()
            if writeback:store(a,'OP_A',0)
            finish()
    table='operit_handlers_'+name;cpp.append(f'extern const void *const {table}[];');tables.extend(['.p2align 3',f'SYM({table}):']+['    .quad '+label for label in labels])
    if kind=='sysread':index='d'
    elif kind=='syswrite':index='a'
    elif kind=='conditionalcmp':index=f'(((bits==64)*{fc}+flags)*8+a)*8+b'
    elif kind=='fpunary':index=f'(((bits==64)*{fc}+flags)*9+d)*9+a'
    elif kind=='gptov':index=f'((((bits==64)*{fc}+flags)*9+d)*8+a)'
    elif kind=='uitod':index='(d*8+a)'
    elif kind=='vtogp':index=f'((((bits==64)*{fc}+flags)*8+d)*9+a)'
    elif kind=='vimm':index='((bits==64)*9+d)'
    elif kind in ('pairload','pairstore','vpairload','vpairstore'):
        roles=9 if kind.startswith('v') else 8
        index=f'(((((bits==32?0:bits==64?1:2)*{fc}+flags)*{roles}+d)*8+a)*{roles}+b)'
    elif kind=='signedload':index='((((bits==8?0:bits==16?1:2)*2+flags)*8+d)*8+a)'
    elif kind in ('move','keep'):index='((bits==64)*8+d)'
    elif kind=='load':index='(((bits==8?0:bits==16?1:bits==32?2:3)*8+d)*8+a)'
    elif kind=='vload':index='(((bits==32?0:bits==64?1:2)*9+d)*8+a)'
    elif kind=='store':index='(((bits==8?0:bits==16?1:bits==32?2:3)*8+a)*8+b)'
    elif kind=='vstore':index='(((bits==32?0:bits==64?1:2)*8+a)*9+b)'
    elif kind=='high':index='((d*8+a)*8+b)'
    elif kind in ('prepare','operand'):index=f'(((bits==64)*{fc}+flags)*8+a)'
    elif kind in ('prepared','unary','bitfield'):index=f'((((bits==64)*{fc}+flags)*8+d)*8+a)'
    elif kind=='comparebranch':index=f'(((bits==64)*{fc}+flags)*8+a)'
    elif kind in ('fp','fpselect'):index='((d*9+a)*9+b)'
    elif kind=='fpma':index='(((flags*9+d)*9+a)*9+b)'
    elif kind=='fpcmp':index='((flags*9+a)*9+b)'
    elif kind=='fpsqrt':index='(d*9+a)'
    else:index=f'((((bits==64)*{fc}+flags)*8+d)*8+a)*8+b'
    cases.append(f'case StaticCode::{name}: return {table}[{index}];')
active_profile=None
for condition in range(16):
    emit(f'Lgp_condition{condition}:',f'    csel x11, x9, x10, {conds[condition]}','    br x17')
emit('.p2align 3','Lgp_conditions:')
for condition in range(16):emit(f'    .quad Lgp_condition{condition}-Lgp_conditions')
# Shared leaf source/condition selectors have no ABI transition. x17 is a
# private continuation. Keep their relative pointer tables in the const section.
for source in range(9):
    emit(f'Lfma_source{source}:','    DIAG_COUNT '+('ED_EVENT_VECTOR_SELECTED_RESIDENT_READ' if source<8 else 'ED_EVENT_VECTOR_SELECTED_SPILL_READ')+', x1, x2')
    if source<8:emit(f'    fmov d2, d{8+source}')
    else:emit('    ldr x12, [x28, #OP_IMMEDIATE]','    ldr d2, [x27, x12]')
    emit('    br x17')
for condition in range(16):
    emit(f'Lfp_condition{condition}:',f'    b.{conds[condition]} Lfp_condition{condition}_return','    fmov d0, d1',f'Lfp_condition{condition}_return:','    br x17')
# PC-relative signed offsets avoid absolute addresses embedded in executable text.
emit('.p2align 3','Lfma_sources:')
for source in range(9):emit(f'    .quad Lfma_source{source}-Lfma_sources')
emit('.p2align 3','Lfp_conditions:')
for condition in range(16):emit(f'    .quad Lfp_condition{condition}-Lfp_conditions')
# Basic control handlers with bounded role variations.
basics=['Block','Bridge','IndirectDispatch','Next','Helper','Nop','Jump','BlockBody'];base_labels=['L'+n for n in basics]
# Debug aliases carry no executable bytes and sit outside the executor CFI.
for name in basics+['Slow','Reload','Dispatch','Return','LocalNext']:
    label='L'+(name.lower() if name in ('Slow','Reload','Dispatch','Return') else name)
    tables.extend(['.globl SYM(operit_static_kind_'+name+')','.set SYM(operit_static_kind_'+name+'), '+label])
for name in ('Call','Indirect','IndirectCall','CompareBranch','TestBranch','Branch'):
    emit('Lstatic_kind_'+name+':')
    tables.extend(['.globl SYM(operit_static_kind_'+name+')','.set SYM(operit_static_kind_'+name+'), Lstatic_kind_'+name])
    labels=[]
    configs=range(32) if name=='Branch' else range(16) if name=='Call' else range(8) if name=='Indirect' else range(64)
    for index in configs:
        label=f'L{name}_{index}';labels.append(label);begin_handler(label)
        if name=='Branch':
            emit(f'    DIAG_BRANCH {index}')
            condition=index&15
            # x25 already holds architectural NZCV in bits 31:28. Simple
            # conditions can branch on those bits without writing host NZCV.
            if condition<8:
                bit=(30,29,31,28)[condition//2]
                emit(f'    {"tbz" if condition&1 else "tbnz"} x25, #{bit}, {label}_taken',
                     f'    FALLTHROUGH {int(index>=16)}')
            elif condition<14:
                emit('    msr nzcv, x25',f'    b.{conds[condition]} {label}_taken',
                     f'    FALLTHROUGH {int(index>=16)}')
            # AL and NV both mean always for AArch64 B.cond, matching the
            # existing condition table. Fall directly into the taken edge.
            emit(label+'_taken:',f'    DIAG_BRANCH {index}, 1');taken(index>=16)
        elif name in ('Call','Indirect','IndirectCall'):
            a=index%8;d=index//8 if name=='IndirectCall' else index%8
            if name!='Call':gp(a,'OP_A',9);emit('    str x9, [x27, #EF_PENDING_PC]')
            if name!='Indirect':emit('    ldr x11, [x28, #OP_IMMEDIATE]');store(d,'OP_D')
            if name=='Call':taken(index>=8)
            else:
                # link() binds every installed indirect edge to a PIC record.
                emit('    DIAG_COUNT ED_EVENT_INDIRECT_EDGE','    ldrh w12, [x28, #OP_EDGE_PREFIX]','    add x26, x6, x12','    ldr x16, [x28, #OP_TARGET]','    cbz x16, Lslow','    mov x28, x16','    b LIndirectDispatch')
        else:
            bits=64 if index//32 else 32;f=(index//8)&3;a=index%8;gp(a,'OP_A',9,bits)
            if name=='TestBranch':emit('    ldrh w12, [x28, #OP_IMMEDIATE]','    lsr x9, x9, x12','    and x9, x9, #1')
            emit(f'    {"cbnz" if f&1 else "cbz"} {"x" if bits==64 else "w"}9, {label}_taken',f'    FALLTHROUGH {int(bool(f&2))}');emit(label+'_taken:');taken(bool(f&2))
    table='operit_handlers_'+name;cpp.append(f'extern const void *const {table}[];');tables.extend(['.p2align 3',f'SYM({table}):']+['    .quad '+label for label in labels])
    index='flags' if name=='Branch' else 'flags*8+d' if name=='Call' else 'a' if name=='Indirect' else 'd*8+a' if name=='IndirectCall' else '(bits==64)*32+flags*8+a'
    cases.append(f'case StaticCode::{name}: return {table}[{index}];')
cpp.append('extern const void *const operit_handlers_local[];');tables.extend(['.p2align 3','SYM(operit_handlers_local):','    .quad LLocalNext']);
cpp.append('extern const void *const operit_handlers_basic[];');tables.extend(['.p2align 3','SYM(operit_handlers_basic):']+['    .quad '+label for label in base_labels]);cpp.append('}')
cpp.append('const void *static_entry(StaticCode code,unsigned bits,unsigned d,unsigned a,unsigned b,unsigned flags) { switch(code) {')
cpp.extend(f'case StaticCode::{name}:return '+(f'flags ? operit_handlers_local[0] : ' if name in ('Next','Jump') else '')+f'operit_handlers_basic[{n}];' for n,name in enumerate(basics));cpp+=cases;cpp+=[f'default: return operit_handlers_basic[{basics.index("Helper")}];','}}','}']
active_profile=None
profiles=[(0,)*9];profile_ids={profiles[0]:0}
for number,profile in enumerate(pending_profiles):
    key=tuple(profile)
    if key not in profile_ids:profile_ids[key]=len(profiles);profiles.append(key)
    pending_profiles[number]=profile_ids[key]
assert len(profiles)<=256, 'Increase ED_PROFILE_CAPACITY and the assembly profile mask together'
for number,line in enumerate(handlers):
    if '@PROFILE' in line:
        index=int(line.split('@PROFILE')[1].split('@')[0])
        handlers[number]=f'    DIAG_HANDLER {pending_profiles[index]}'
cpp.extend(['#if OPERIT_EXECUTION_DIAGNOSTICS','namespace operit::arm64 {',
            'const ExecutionDiagnosticProfile *execution_diagnostic_profiles(size_t &count) noexcept {',
            '    static constexpr ExecutionDiagnosticProfile profiles[] = {'])
cpp.extend('        {{'+','.join(map(str,profile[:8]))+'},'+str(profile[8])+'},' for profile in profiles)
cpp.extend(['    };','    count=sizeof(profiles)/sizeof(profiles[0]);return profiles;','}',
            'const char *execution_diagnostic_static_code_name(uint8_t code) noexcept { switch(StaticCode(code)) {'])
cpp.extend(f'case StaticCode::{name}:return "{name}";' for name in basics+definition_names+['Call','Indirect','IndirectCall','CompareBranch','TestBranch','Branch'])
cpp.extend(['default:return "Unknown";','}}','}','#endif'])
# Conditional AArch64 branches have shorter reach than the full handler set.
# Emit a nearby inverse guard and an unconditional slow-edge branch.
import re
expanded=[]
inverse={'eq':'ne','ne':'eq','hi':'ls','lo':'hs','hs':'lo'}
for number,line in enumerate(handlers):
    match=re.fullmatch(r'    (cbz|cbnz|tbz|tbnz|b\.[a-z]+) (.*?)(?:, )?Lslow',line)
    if match:
        instruction,operands=match.groups();label=f'Lslow_skip{number}'
        if instruction.startswith('b.'):
            expanded.extend([f'    b.{inverse[instruction[2:]]} {label}','    b Lslow',label+':'])
        else:
            instruction={'cbz':'cbnz','cbnz':'cbz','tbz':'tbnz','tbnz':'tbz'}[instruction]
            expanded.extend([f'    {instruction} {operands.rstrip(", ")}, {label}','    b Lslow',label+':'])
    else:expanded.append(line)
handlers=expanded
for i in range(len(tables)-1,-1,-1):
    if tables[i].startswith('SYM('):tables.insert(i,'.globl '+tables[i][:-1])
(args.output/'static_handlers.inc').write_text('\n'.join(handlers)+'\n');(args.output/'static_tables.inc').write_text('\n'.join(tables)+'\n');(args.output/'static_entries.cpp').write_text('\n'.join(cpp)+'\n')
