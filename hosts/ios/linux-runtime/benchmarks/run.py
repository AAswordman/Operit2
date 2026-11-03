#!/usr/bin/env python3
"""Measure the same real Alpine workload with fixed helpers disabled/enabled."""
import argparse
import json
from pathlib import Path
import platform
import statistics
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--repeats', type=int, default=7)
    parser.add_argument('--warmup', type=int, default=2)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.repeats < 2 or args.warmup < 0:
        parser.error('repeats must be >= 2 and warmup >= 0')
    runtime, root = str(args.runtime.resolve()), str(args.root.resolve())
    workloads = {
        'dynamic-startup': (['printf', 'arm64-benchmark\\n'], b'arm64-benchmark\n'),
        'shell-loop': (['sh', '-c', 'i=0; while [ "$i" -lt 1000 ]; do i=$((i+1)); done; echo "$i"'], b'1000\n'),
    }
    report = {'host': platform.platform(), 'machine': platform.machine(), 'repeats': args.repeats,
              'warmup': args.warmup, 'clock': 'CLI steady_clock guest execution; wall includes load and startup',
              'warning': 'Host measurements only. Fixed helpers apply only to authenticated ELF images.',
              'workloads': {}}
    for name, (guest_args, expected) in workloads.items():
        samples = {mode: [] for mode in ('interpreter', 'fixed-helpers')}
        for iteration in range(args.warmup + args.repeats):
            modes = list(samples)
            if iteration % 2:
                modes.reverse()
            for mode in modes:
                command = [runtime, '--root', root, '--stats']
                if mode == 'fixed-helpers':
                    command.append('--fixed-acceleration')
                start = time.perf_counter()
                result = subprocess.run([*command, '--', '/bin/busybox', *guest_args], capture_output=True, timeout=30)
                wall = time.perf_counter() - start
                if result.returncode or result.stdout != expected:
                    raise RuntimeError(f'{name}/{mode}: invalid result: {result.stderr.decode()}')
                stats = json.loads(result.stderr.decode().splitlines()[-1])
                if stats['unsupported_syscalls']:
                    raise RuntimeError(f'{name}/{mode}: unsupported syscall in benchmark')
                if iteration >= args.warmup:
                    samples[mode].append({**stats, 'wall_seconds': wall})
        summary = {}
        for mode, rows in samples.items():
            times = sorted(row['seconds'] for row in rows)
            summary[mode] = {
                'median_guest_seconds': statistics.median(times),
                'p95_guest_seconds': times[min(len(times) - 1, int(0.95 * len(times)))],
                'median_wall_seconds': statistics.median(row['wall_seconds'] for row in rows),
                'samples': rows,
            }
        summary['median_guest_speed_ratio'] = summary['interpreter']['median_guest_seconds'] / summary['fixed-helpers']['median_guest_seconds']
        report['workloads'][name] = summary
    encoded = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.write_text(encoded)
    else:
        print(encoded, end='')


if __name__ == '__main__':
    main()
