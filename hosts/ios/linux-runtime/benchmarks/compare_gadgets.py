#!/usr/bin/env python3
"""Alternate baseline and candidate CLIs on an unchanged guest Python workload."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--repeats', type=int, default=5)
    parser.add_argument('--warmup', type=int, default=1)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.repeats < 2 or args.warmup < 0:
        parser.error('repeats must be >= 2 and warmup must be >= 0')
    binaries = {name: getattr(args, name).resolve() for name in ('baseline', 'candidate')}
    workload = Path(__file__).with_name('python_loop.py').read_text()
    rows = []
    for iteration in range(args.warmup + args.repeats):
        modes = list(binaries)
        if iteration % 2:
            modes.reverse()
        for mode in modes:
            started = time.perf_counter()
            result = subprocess.run(
                [str(binaries[mode]), '--root', str(args.root.resolve()), '--max-steps',
                 '2000000000', '--fixed-acceleration', '--stats', '--', '/usr/bin/python3',
                 '-B', '-c', workload], capture_output=True, text=True, timeout=90)
            wall = time.perf_counter() - started
            match = re.fullmatch(r'loop_seconds=([0-9.]+) checksum=1267865\n', result.stdout)
            if result.returncode or not match:
                raise RuntimeError((mode, result.returncode, result.stdout, result.stderr))
            stats = json.loads(result.stderr.splitlines()[-1])
            if stats['status'] != 0 or stats['unsupported_syscalls']:
                raise RuntimeError(stats)
            row = {'iteration': iteration, 'warmup': iteration < args.warmup, 'mode': mode,
                   'loop_seconds': float(match[1]), 'wall_seconds': wall, 'stats': stats}
            rows.append(row)
            print(json.dumps(row), flush=True)
    medians = {mode: statistics.median(row['loop_seconds'] for row in rows
                                      if row['mode'] == mode and not row['warmup'])
               for mode in binaries}
    report = {'date': time.strftime('%Y-%m-%d'), 'platform': platform.platform(),
              'machine': platform.machine(), 'repeats': args.repeats, 'warmup': args.warmup,
              'scope': 'macOS CLI measurements; no iOS build or device execution',
              'binaries': {name: {'path': str(binary),
                                  'sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
                           for name, binary in binaries.items()},
              'workload_sha256': hashlib.sha256(workload.encode()).hexdigest(),
              'root': str(args.root.resolve()), 'results': rows, 'median_seconds': medians,
              'speedup': medians['baseline'] / medians['candidate']}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'median_seconds': medians, 'speedup': report['speedup']}), flush=True)


if __name__ == '__main__':
    main()
