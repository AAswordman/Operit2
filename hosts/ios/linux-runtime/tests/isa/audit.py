#!/usr/bin/env python3
"""Compile independent instruction forms and compare guest state against ARM64 hardware.

Every form runs separately so one unsupported instruction cannot hide other gaps.
Full audits produce a report even when forms fail. --require-passing turns gaps
into a failing exit code; --family selects a bounded CI regression suite.
"""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import platform
import re
import shutil
import struct
import subprocess
import sys
import tempfile
from catalog import cases

HERE = Path(__file__).resolve().parent

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--family', action='append')
    parser.add_argument('--require-passing', action='store_true')
    parser.add_argument('--register-aliases', action='store_true', help='also test destination/source and source/source overlap')
    args=parser.parse_args()
    if platform.machine().lower() not in ('arm64','aarch64'):
        parser.error('hardware differential audit requires an ARM64 host')
    corpus=[c for c in cases() if not args.family or c['family'] in args.family]
    if args.register_aliases:
        expanded=[]
        for case in corpus:
            expanded.append(case)
            if case['family'].startswith('vector-') and case['asm'].count(',')==2 and 'v2.' in case['asm']:
                for alias in ('destination-n','destination-m','sources'):
                    asm=case['asm']
                    if alias=='destination-n': asm=asm.replace('v1.','v0.')
                    elif alias=='destination-m': asm=asm.replace('v2.','v0.')
                    else: asm=asm.replace('v2.','v1.')
                    expanded.append(dict(case,id=case['id']+'-'+alias,asm=asm))
        corpus=expanded
    if not corpus: parser.error('empty corpus')
    runtime=str(args.runtime.resolve())
    clang=os.environ.get('OPERIT_AARCH64_CLANG') or (str(Path('/opt/homebrew/opt/llvm/bin/clang')) if Path('/opt/homebrew/opt/llvm/bin/clang').is_file() else shutil.which('clang'))
    linker=os.environ.get('OPERIT_AARCH64_LLD') or ('/opt/homebrew/opt/lld/bin/ld.lld' if Path('/opt/homebrew/opt/lld/bin/ld.lld').is_file() else shutil.which('ld.lld'))
    with tempfile.TemporaryDirectory(prefix='operit-isa-audit-') as temp:
        root=Path(temp)
        assembly=['#ifdef __APPLE__','#define NAME(x) _##x','#else','#define NAME(x) x','#endif','.text']
        header=['typedef void (*Sample)(const unsigned long *,unsigned long *);']
        for index,case in enumerate(corpus):
            name='sample_'+str(index)
            header.append(f'extern void {name}(const unsigned long *,unsigned long *);')
            assembly.extend([f'.global NAME({name})',f'NAME({name}):',
                'mov x9,x0','mov x10,x1','ldp x3,x4,[x9]','ldp x5,x6,[x9,#16]',
                'ldp q0,q1,[x9,#32]','ldp q2,q3,[x9,#64]',
                'ldr x7,[x9,#96]','msr nzcv,x7',
                'mrs x11,fpcr','mrs x12,fpsr','ldr x7,[x9,#104]','msr fpcr,x7','msr fpsr,xzr','add x14,x9,#128',
                case['asm'].replace('; ', '\n'),
                'stp x3,x4,[x10]','stp x5,x6,[x10,#16]',
                'stp q0,q1,[x10,#32]','stp q2,q3,[x10,#64]',
                'mrs x7,nzcv','mrs x8,fpsr','stp x7,x8,[x10,#96]',
                *[instruction for offset in range(0,256,16) for instruction in (f'ldr q4,[x9,#{128+offset}]',f'str q4,[x10,#{112+offset}]')],
                'msr fpcr,x11','msr fpsr,x12','ret'])
        header.append('static const unsigned records_per_case[]={'+','.join(str(c['records']) for c in corpus)+'};')
        header.append('static Sample samples[]={'+','.join('sample_'+str(i) for i in range(len(corpus)))+'};')
        (root/'cases.h').write_text('\n'.join(header))
        (root/'cases.S').write_text('\n'.join(assembly)+'\n')
        includes=['-I'+str(root),'-I'+str(HERE.parent/'guest')]
        native=[shutil.which('cc'),'-O2','-DOPERIT_NATIVE',*includes,str(HERE/'harness.c'),str(root/'cases.S'),'-o',str(root/'native')]
        guest=[clang,'--target=aarch64-linux-gnu','-O2','-ffreestanding','-nostdlib','-fno-builtin','-fno-stack-protector','-fno-vectorize','-fno-slp-vectorize','--ld-path='+linker,
               '-static','-no-pie','-Wl,--build-id=none','-Wl,-z,max-page-size=4096',*includes,str(HERE.parent/'guest/start.S'),str(HERE/'harness.c'),str(root/'cases.S'),'-o',str(root/'guest')]
        for command in (native,guest):
            result=subprocess.run(command,capture_output=True,timeout=60)
            if result.returncode:
                print(result.stderr.decode(),file=sys.stderr)
                return 2
        def run(item):
            index,case=item
            record=dict(case)
            try:
                expected=subprocess.run([str(root/'native'),str(index)],capture_output=True,timeout=10)
            except subprocess.TimeoutExpired:
                record.update(status='host-timeout')
                return record
            try:
                result=subprocess.run([runtime,'--root',str(root),'--max-steps','1000000','--','/guest',str(index)],capture_output=True,timeout=10)
            except subprocess.TimeoutExpired as failure:
                record.update(status='guest-timeout',detail=(failure.stderr or b'').decode(errors='replace')[:600])
                return record
            if expected.returncode:
                record.update(status='host-failure',detail=str(expected.returncode))
            elif result.returncode:
                record.update(status='unsupported' if result.returncode==125 else 'guest-failure',detail=result.stderr.decode()[:600],exit_code=result.returncode)
            elif result.stdout!=expected.stdout:
                record.update(status='mismatch',actual_bytes=len(result.stdout),expected_bytes=len(expected.stdout))
                for offset in range(0,min(len(result.stdout),len(expected.stdout)),8):
                    actual=struct.unpack_from('<Q',result.stdout,offset)[0]
                    wanted=struct.unpack_from('<Q',expected.stdout,offset)[0]
                    if actual!=wanted:
                        record.update(first_record=offset//368,first_word=(offset%368)//8,actual=hex(actual),expected=hex(wanted)); break
            else:
                record.update(status='passed',records=case['records'])
            return record
        with ThreadPoolExecutor(max_workers=4) as pool:
            records=list(pool.map(run,enumerate(corpus)))
    summary=dict(Counter(r['status'] for r in records))
    families={}
    for r in records: families.setdefault(r['family'],Counter())[r['status']]+=1
    report=dict(schema=1,profile_complete=False,scope='Executable sampled AArch64 EL0 forms; not exhaustive ISA/encoding coverage',runtime=runtime,host=platform.platform(),summary=summary,families=families,cases=records)
    args.report.parent.mkdir(parents=True,exist_ok=True)
    args.report.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps(dict(summary=summary,families=families),indent=2))
    return int(args.require_passing and any(r['status']!='passed' for r in records))

if __name__=='__main__': sys.exit(main())
