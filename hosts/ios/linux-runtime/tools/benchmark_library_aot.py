#!/usr/bin/env python3
"""Compare offline AOT configurations on Python and jitless Node guest workloads.

Times are fresh-process wall time and guest-reported work time, recorded
separately. This tool never turns on native replacements or host JIT execution.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys
import time

GUEST_ENV = ['HOME=/root', 'PATH=/usr/local/bin:/usr/bin:/bin', 'TERM=xterm-256color',
             'PYTHONMALLOC=malloc', 'PYTHONDONTWRITEBYTECODE=1']
NODE_INTEGER = '''const t=process.hrtime.bigint();let x=0;
for(let i=0;i<100000;i++)x=(x+i)|0;
console.log(JSON.stringify({work_seconds:Number(process.hrtime.bigint()-t)/1e9,checksum:x}));'''
NODE_MATH = '''const sources=[[.28,.30,1],[.74,.68,.85],[.52,.12,.55]];
const k=30*Math.PI,wsum=sources.reduce((s,p)=>s+p[2],0),start=process.hrtime.bigint();
let checksum=0;
for(let y=0;y<100;y++){let v=y/99;for(let x=0;x<100;x++){let u=x/99,amp=0;
for(const [sx,sy,w] of sources){const dx=u-sx,dy=v-sy,d=Math.sqrt(dx*dx+dy*dy);
amp+=w*Math.sin(k*d)/(1+3.2*d);}
const t=Math.max(0,Math.min(1,.5+1.24*amp/wsum)),glow=t*t*(3-2*t);
checksum+=Math.trunc(255*glow);}}
console.log(JSON.stringify({work_seconds:Number(process.hrtime.bigint()-start)/1e9,checksum}));'''
NODE_JSON = '''const crypto=require('node:crypto'),h=crypto.createHash('sha256'),t=process.hrtime.bigint();
for(let i=0;i<1500;i++){const data=JSON.stringify({number:i,items:[i,i*7,String(i)]});
h.update(data.replace(/[0-9]+/g,'#'));}
console.log(JSON.stringify({work_seconds:Number(process.hrtime.bigint()-t)/1e9,checksum:h.digest('hex')}));'''
PYTHON_JSON = '''import hashlib,json,re,time
t=time.perf_counter();h=hashlib.sha256()
for i in range(1500):
 data=json.dumps({'number':i,'items':[i,i*7,str(i)]},sort_keys=True)
 h.update(re.sub(r'[0-9]+','#',data).encode())
print(json.dumps({'work_seconds':time.perf_counter()-t,'checksum':h.hexdigest()}))'''


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('control', 'candidate', 'root', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--repeats', type=int, default=7)
    parser.add_argument('--timeout', type=float, default=90)
    parser.add_argument('--workload', action='append', help='repeat to run only the named workloads')
    parser.add_argument('--host-counters', action='store_true', help='record macOS time -l hardware and CPU counters')
    args = parser.parse_args()
    if args.repeats < 1 or args.timeout <= 0:
        parser.error('repeats and timeout must be positive')
    if args.host_counters and sys.platform != 'darwin':
        parser.error('--host-counters requires macOS time -l')
    args.output.mkdir(parents=True, exist_ok=True)
    raw = args.output / 'measurements.jsonl'
    if raw.exists():
        parser.error('output already contains measurements; select a fresh directory')
    root = args.root.resolve()
    binaries = {name: getattr(args, name).resolve() for name in ('control', 'candidate')}
    workload_path = Path(__file__).resolve().parents[1] / 'benchmarks/python_loop.py'
    workloads = {
        'python-math': ['/usr/bin/python3', '-B', '-c', workload_path.read_text()],
        'python-json': ['/usr/bin/python3', '-B', '-c', PYTHON_JSON],
        'node-integer': ['/usr/bin/node', '--jitless', '-e', NODE_INTEGER],
        'node-math': ['/usr/bin/node', '--jitless', '-e', NODE_MATH],
        'node-json': ['/usr/bin/node', '--jitless', '-e', NODE_JSON],
        'node-startup': ['/usr/bin/node', '--jitless', '-e', 'console.log(JSON.stringify({checksum:1}))'],
    }
    if args.workload:
        unknown = set(args.workload) - workloads.keys()
        if unknown:
            parser.error('unknown workload: ' + ', '.join(sorted(unknown)))
        workloads = {name: workloads[name] for name in dict.fromkeys(args.workload)}
    env = {k: v for k, v in os.environ.items() if not k.startswith(('OPERIT_', 'ISH_'))}
    files = [workload_path, root / 'usr/bin/python3.12', root / 'usr/lib/libpython3.12.so.1.0',
             root / 'usr/bin/node', root / 'lib/ld-musl-aarch64.so.1']
    metadata = {
        'binaries': {name: {'path': str(p), 'sha256': sha(p), 'bytes': p.stat().st_size}
                     for name, p in binaries.items()},
        'guest_files': {str(p): {'sha256': sha(p), 'bytes': p.stat().st_size} for p in files},
        'guest_environment': GUEST_ENV, 'workloads': workloads, 'cpus': 1,
        'benchmark_sha256': sha(__file__), 'host_counters_enabled': args.host_counters,
        'repeats': args.repeats, 'target_seconds': .05,
        'host': subprocess.check_output(['sw_vers'], text=True),
        'cpu': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
        'notes': ['Warm-up does not preserve execution graphs between processes.',
                  'Node uses --jitless. Fixed native replacements and diagnostic instrumentation are disabled.',
                  'Whole-process time includes spawn, guest initialization, work and shutdown.',
                  'Profile collection workloads are not inferred by this benchmark.'],
    }
    (args.output / 'environment.json').write_text(json.dumps(metadata, indent=2) + '\n')
    rows, expected = [], {}
    with raw.open('w') as output:
        for workload, guest in workloads.items():
            for iteration in range(-1, args.repeats):
                order = ('control', 'candidate') if iteration % 2 == 0 else ('candidate', 'control')
                for mode in order:
                    command = [str(binaries[mode]), '--root', str(root), '--cpus', '1', '--stats',
                               '--max-steps', '10000000000', '--', '/usr/bin/env', '-i', *GUEST_ENV, *guest]
                    if args.host_counters:
                        command = ['/usr/bin/time', '-l', *command]
                    start = time.perf_counter()
                    result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=args.timeout)
                    elapsed = time.perf_counter() - start
                    row = {'workload': workload, 'iteration': iteration, 'mode': mode,
                           'process_seconds': elapsed, 'returncode': result.returncode,
                           'stdout': result.stdout, 'stderr': result.stderr, 'command': command}
                    if args.host_counters:
                        counters = {}
                        for key, label in (('instructions_retired', 'instructions retired'),
                                           ('cycles_elapsed', 'cycles elapsed'),
                                           ('maximum_resident_set_bytes', 'maximum resident set size'),
                                           ('page_reclaims', 'page reclaims'), ('page_faults', 'page faults')):
                            found = re.search(r'^\s*(\d+)\s+' + label + r'$', result.stderr, re.MULTILINE)
                            if found:
                                counters[key] = int(found[1])
                        found = re.search(r'([0-9.]+)\s+real\s+([0-9.]+)\s+user\s+([0-9.]+)\s+sys', result.stderr)
                        if found:
                            counters.update(real_seconds=float(found[1]), user_seconds=float(found[2]), system_seconds=float(found[3]))
                        row['whole_process_host_counters'] = counters
                    try:
                        if result.returncode:
                            raise RuntimeError(f'{workload} {mode} failed: {result.stderr}')
                        stats = next((json.loads(line) for line in result.stderr.splitlines()
                                      if line.startswith('{"status"')), None)
                        if not stats or stats['status'] or stats['accelerated_calls'] or stats['unsupported_syscalls']:
                            raise RuntimeError(f'{workload} {mode}: unexpected runtime stats {stats}')
                        if workload == 'python-math':
                            match = re.search(r'loop_seconds=([0-9.]+) checksum=(\d+)', result.stdout)
                            if not match or int(match[2]) != 1267865:
                                raise RuntimeError(f'{workload}: unexpected output {result.stdout}')
                            work, checksum = float(match[1]), int(match[2])
                        else:
                            report = next(json.loads(line) for line in result.stdout.splitlines() if line.startswith('{'))
                            work, checksum = report.get('work_seconds'), report['checksum']
                            if workload == 'node-integer' and checksum != 704982704:
                                raise RuntimeError('Node integer checksum mismatch')
                            if workload == 'node-math' and checksum != 1267865:
                                raise RuntimeError('Node math checksum mismatch')
                        if workload in expected and expected[workload] != checksum:
                            raise RuntimeError(f'{workload}: changed checksum')
                        expected[workload] = checksum
                        row.update(work_seconds=work, checksum=checksum, stats=stats)
                    finally:
                        output.write(json.dumps(row) + '\n');output.flush()
                    if iteration >= 0:
                        rows.append(row)
            print(workload, 'completed', flush=True)
    summary = {}
    for workload in workloads:
        modes = {}
        for mode in binaries:
            selected = [row for row in rows if row['workload'] == workload and row['mode'] == mode]
            process = statistics.median(row['process_seconds'] for row in selected)
            work = [row['work_seconds'] for row in selected if row['work_seconds'] is not None]
            modes[mode] = {'process_seconds': process, 'work_seconds': statistics.median(work) if work else None,
                           'process_target_met': process <= .05,
                           'work_target_met': statistics.median(work) <= .05 if work else None}
        summary[workload] = modes
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary, indent=2))


if __name__ == '__main__':
    main()
