#!/usr/bin/env python3
"""Measure an existing CLI; never builds fixtures or changes Worker acceptance."""
import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path
import platform
import selectors
import shutil
import subprocess
import tempfile
import time

from probe_node import FIXTURES, validated
from probe_support import execute


def timed_phases(command, timeout):
    """Timestamp diagnostic stderr at the host, independent of guest clocks."""
    import os
    started = time.monotonic()
    chunks = {'stdout': bytearray(), 'stderr': bytearray()}
    pending = bytearray()
    phases = []
    timed_out = False
    with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
        with selectors.DefaultSelector() as poll:
            for name in chunks:
                poll.register(getattr(process, name), selectors.EVENT_READ, name)
            while poll.get_map():
                remaining = timeout - (time.monotonic() - started)
                if remaining <= 0:
                    timed_out = True
                    process.kill()
                    break
                for key, _ in poll.select(min(remaining, 0.1)):
                    data = os.read(key.fileobj.fileno(), 65536)
                    if not data:
                        poll.unregister(key.fileobj)
                        continue
                    chunks[key.data].extend(data)
                    if key.data != 'stderr':
                        continue
                    pending.extend(data)
                    while b'\n' in pending:
                        line, _, tail = pending.partition(b'\n')
                        pending[:] = tail
                        try:
                            event = json.loads(line)
                        except (ValueError, UnicodeDecodeError):
                            continue
                        if event.get('diagnostic') == 'worker-phase':
                            event['host_seconds_since_launch'] = time.monotonic() - started
                            phases.append(event)
            process.wait()
    return {'command': command, 'returncode': process.returncode, 'timed_out': timed_out,
            'wall_seconds': time.monotonic() - started, 'phases': phases,
            **{name: value.decode(errors='replace') for name, value in chunks.items()}}


def instrument(source):
    # Instrument a temporary COPY; the original acceptance input stays byte-identical.
    changes = {
        '  const shared =': "  console.error(JSON.stringify({diagnostic: 'worker-phase', phase: 'main-entry'}));\n  const shared =",
        "    console.error('worker probe timed out');": "    console.error(JSON.stringify({diagnostic: 'worker-phase', phase: 'deadline'}));\n    console.error('worker probe timed out');",
        '      if (message.ready) {': "      if (message.ready) {\n        console.error(JSON.stringify({diagnostic: 'worker-phase', phase: 'ready', index}));",
        '          Atomics.store(state, 0, workers + 1);': "          console.error(JSON.stringify({diagnostic: 'worker-phase', phase: 'all-ready'}));\n          Atomics.store(state, 0, workers + 1);",
        '        result = message;': "        console.error(JSON.stringify({diagnostic: 'worker-phase', phase: 'result', index}));\n        result = message;",
        "    worker.on('exit', code => {": "    worker.on('exit', code => {\n      console.error(JSON.stringify({diagnostic: 'worker-phase', phase: 'exit', index, code}));",
    }
    for old, new in changes.items():
        if source.count(old) != 1:
            raise ValueError('Worker fixture changed; review diagnostic instrumentation: ' + old)
        source = source.replace(old, new)
    return source


def performance_summary(results, cpus, repeats, target_seconds):
    """Require every fresh-process run; never accept only the fastest sample."""
    selected = [result for result in results if result['cpus'] == cpus]
    walls = [result['wall_seconds'] for result in selected]
    complete = len(selected) == repeats
    passed = complete and all(result['passed'] and math.isfinite(result['wall_seconds']) and
                              0 <= result['wall_seconds'] <= target_seconds for result in selected)
    return {'cpus': cpus, 'target_wall_seconds': target_seconds, 'expected_runs': repeats,
            'completed_runs': len(selected), 'complete': complete, 'passed': passed,
            'measurement': 'host process launch through exit; fresh CLI process per run',
            'wall_seconds': walls, 'median_wall_seconds': statistics.median(walls) if walls else None,
            'maximum_wall_seconds': max(walls) if walls else None}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--cpus', type=int, nargs='+', default=[1, 4])
    parser.add_argument('--timeout', type=float, default=60)
    parser.add_argument('--repeats', type=int, default=3, help='fresh CLI launches per CPU count')
    parser.add_argument('--target-seconds', type=float, default=5, help='end-to-end host wall target')
    parser.add_argument('--target-cpus', type=int, help='CPU count for the target; defaults to max --cpus')
    parser.add_argument('--host-node', type=Path)
    parser.add_argument('--phases', action='store_true', help='also run an instrumented copy; diagnostic only')
    parser.add_argument('--sample-seconds', type=int, default=0, help='separate macOS sample run; diagnostic only')
    args = parser.parse_args()
    if any(cpu < 1 or cpu > 64 for cpu in args.cpus) or not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error('CPUs must be 1..64 and timeout must be finite and positive')
    target_cpus = args.target_cpus if args.target_cpus is not None else max(args.cpus)
    if target_cpus not in args.cpus:
        parser.error('--target-cpus must be included in --cpus')
    if args.repeats < 1 or not math.isfinite(args.target_seconds) or args.target_seconds <= 0:
        parser.error('repeats and finite target-seconds must be positive')
    if args.sample_seconds < 0 or args.sample_seconds > 30:
        parser.error('--sample-seconds must be 0..30')
    if args.sample_seconds and (platform.system() != 'Darwin' or not shutil.which('sample')):
        parser.error('sampling requires macOS sample')
    binary, root, output = args.runtime.resolve(), args.root.resolve(), args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    fixture = FIXTURES / 'workers.cjs'
    report = {'binary': str(binary), 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'binary_mtime_ns': binary.stat().st_mtime_ns, 'host': platform.platform(),
              'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(),
              'guest_deadline_ms': 15000, 'guest_node_mode': '--jitless',
              'acceptance': [], 'diagnostics': [], 'complete': False, 'functional_passed': False, 'passed': False,
              'performance': performance_summary([], target_cpus, args.repeats, args.target_seconds)}

    def save():
        temporary = output.with_suffix(output.suffix + '.tmp')
        temporary.write_text(json.dumps(report, indent=2) + '\n')
        temporary.replace(output)

    def command(cpus, fixtures):
        return [str(binary), '--root', str(root), '--fixed-acceleration', '--max-steps', '10000000000',
                '--stats', '--cpus', str(cpus), '--bind-ro', str(fixtures) + ':/operit-probes',
                '--', '/usr/bin/node', '--jitless', '/operit-probes/workers.cjs']

    save()
    if args.host_node:
        host_node = str(args.host_node.expanduser().resolve())
        report['host_node_version'] = execute([host_node, '--version'], args.timeout)
        result = execute([host_node, '--jitless', str(fixture)], args.timeout)
        result['passed'] = validated(result, 'workers')
        report['host_control'] = result
        save()
    for repetition in range(1, args.repeats + 1):
        # Alternate order to reduce a consistent thermal/background-load bias.
        cpu_order = list(dict.fromkeys(args.cpus))
        if repetition % 2 == 0:
            cpu_order.reverse()
        for cpus in cpu_order:
            result = execute(command(cpus, FIXTURES), args.timeout)
            result.update(cpus=cpus, repetition=repetition, passed=validated(result, 'workers'))
            report['acceptance'].append(result)
            report['performance'] = performance_summary(report['acceptance'], target_cpus,
                                                         args.repeats, args.target_seconds)
            save()
            print(json.dumps({'scope': 'acceptance', **{key: result[key] for key in
                  ('cpus', 'repetition', 'passed', 'wall_seconds', 'host_cpu_seconds', 'average_host_cores', 'stats')}}), flush=True)
    if args.phases:
        with tempfile.TemporaryDirectory(prefix='operit-worker-phases-') as directory:
            fixtures = Path(directory)
            (fixtures / 'workers.cjs').write_text(instrument(fixture.read_text()))
            for cpus in dict.fromkeys(args.cpus):
                result = timed_phases(command(cpus, fixtures), args.timeout)
                result.update(cpus=cpus, diagnostic_only=True)
                report['diagnostics'].append(result)
                save()
                print(json.dumps({'scope': 'phases', 'cpus': cpus, 'phases': result['phases']}), flush=True)
    if args.sample_seconds:
        sample_path = output.with_suffix('.sample.txt')
        with output.with_suffix('.sample.stdout').open('w') as stdout, output.with_suffix('.sample.stderr').open('w') as stderr:
            with subprocess.Popen(command(max(args.cpus), FIXTURES), stdout=stdout, stderr=stderr) as process:
                try:
                    sample = subprocess.run(['sample', str(process.pid), str(args.sample_seconds), '1', '-file', str(sample_path)],
                                            capture_output=True, text=True, timeout=args.sample_seconds + 15)
                    report['sample'] = {'path': str(sample_path), 'cpus': max(args.cpus), 'returncode': sample.returncode,
                                        'stdout': sample.stdout, 'stderr': sample.stderr, 'diagnostic_only': True}
                    process.wait(timeout=args.timeout)
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait()
    report['complete'] = True
    report['functional_passed'] = all(item['passed'] for item in report['acceptance']) and report.get('host_control', {}).get('passed', True)
    report['passed'] = report['functional_passed'] and report['performance']['passed']
    save()
    print(json.dumps({'scope': 'performance', **report['performance']}), flush=True)
    print('report:', output)
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
