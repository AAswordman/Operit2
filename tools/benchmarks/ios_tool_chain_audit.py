#!/usr/bin/env python3
"""Partition the existing iOS native samples by tool-call ownership.

Consumes the phase-filtered Speedscope export and its matching CPU report.
Every sample enters exactly one bucket; inclusive probes are kept separately.
This analyzes an existing recording and never starts or configures an app.
"""
import argparse
import collections
import json
from pathlib import Path


def owns_engine_lookup(stack):
    return any('PackageManagerJsRuntime' in frame and 'toolpkg_execution_engine' in frame
               for frame in stack)


def owns_logger(stack):
    return any('operit_util::AppLogger' in frame for frame in stack)


def owns_notification(stack):
    return any('notify_waiters' in frame for frame in stack)


def category(stack):
    if owns_engine_lookup(stack):
        return 'package_engine_lookup'
    if owns_logger(stack):
        return 'synchronous_logging'
    if owns_notification(stack):
        return 'watch_notification'
    if stack[-1] == 'kevent':
        return 'other_reactor_kevent'
    return 'remaining_execution'


def audit(profiles, report):
    frames = [frame['name'] for frame in profiles['shared']['frames']]
    phases = {phase['name']: phase for phase in report['stages']}
    result = {
        'source': 'iphone-profiles.speedscope.json',
        'cpuSource': 'iphone-cpu-stacks.json',
        'scope': 'Existing iOS simulator / Flutter debug recording, 200 sequential calls per tool',
        'method': 'First matching bucket: engine lookup, logger, notification, other kevent, remaining.',
        'limitations': [
            'Bucket percentages describe native sample weight, not measured energy or guaranteed savings.',
            'Engine lookup includes manager cloning, engine cache access and destruction of the cloned manager.',
            'Other reactor kevent remains unattributed; do not treat it as busy waiting.',
            'CPU time is an estimate from 100 ms process counters; per-call values are batch amortization only.',
            'CPU time includes every application thread, profiling and other work in the same interval.',
        ],
        'stages': [],
    }
    for profile in profiles['profiles']:
        if not profile['name'].endswith('-batch200 / native'):
            continue
        name = profile['name'].removesuffix(' / native')
        phase = phases[name]
        samples = profile['samples']
        weights = profile['weights']
        if len(samples) != len(weights) or len(samples) != phase['cpu']['nativeSampleCount']:
            raise ValueError(f'Mismatched sample counts for {name}')
        rows = [(weight, [frames[index] for index in stack])
                for stack, weight in zip(samples, weights)]
        total = sum(weights)
        if not total or any(not stack for _, stack in rows):
            raise ValueError(f'Empty native sample set or stack for {name}')
        counts, totals, examples = collections.Counter(), collections.Counter(), {}
        for weight, stack in rows:
            bucket = category(stack)
            counts[bucket] += 1
            totals[bucket] += weight
            examples.setdefault(bucket, stack)
        if sum(counts.values()) != len(samples) or abs(sum(totals.values()) - total) > 1e-8:
            raise ValueError(f'Incomplete partition for {name}')
        duration = (phase['endWallMicros'] - phase['startWallMicros']) / 1e6
        cpu_seconds = phase['cpu']['meanPercent'] / 100 * duration
        probes = {}
        for label, predicate in {
            'clone_inclusive': lambda stack: any('clone' in frame for frame in stack),
            'memmove_self': lambda stack: stack[-1] == '_platform_memmove',
            'filesystem_leaf_under_logger': lambda stack: owns_logger(stack) and stack[-1] in
                {'stat', '__open', 'mkdir', 'write', 'close'},
            'tool_selection_inclusive': lambda stack: any('PackageManagerJsRuntime' in frame and
                'select_tool' in frame for frame in stack),
        }.items():
            matches = [(weight, stack) for weight, stack in rows if predicate(stack)]
            probes[label] = {'samples': len(matches),
                             'percent': round(sum(weight for weight, _ in matches) / total * 100, 3)}
        result['stages'].append({
            'name': name,
            'calls': 200,
            'nativeSamples': len(samples),
            'durationSeconds': duration,
            'meanCpuPercent': phase['cpu']['meanPercent'],
            'approximateProcessCpuSeconds': cpu_seconds,
            'batchAmortizedCpuMillisPerCall': cpu_seconds * 1000 / 200,
            'exclusiveBuckets': [
                {'category': bucket, 'samples': counts[bucket],
                 'percent': round(totals[bucket] / total * 100, 3),
                 'representativeStack': examples[bucket]}
                for bucket in ('package_engine_lookup', 'synchronous_logging', 'watch_notification',
                               'other_reactor_kevent', 'remaining_execution')
            ],
            'overlappingProbes': probes,
        })
    if len(result['stages']) != 2:
        raise ValueError('Expected exactly two tool pressure-test phases')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path('docs/assets/performance-2026-10-27'))
    args = parser.parse_args()
    result = audit(json.loads((args.assets / 'iphone-profiles.speedscope.json').read_text()),
                   json.loads((args.assets / 'iphone-cpu-stacks.json').read_text()))
    output = args.assets / 'iphone-tool-chain-audit.json'
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    for phase in result['stages']:
        print(phase['name'], f"{phase['nativeSamples']} samples")
        for bucket in phase['exclusiveBuckets']:
            print(f"  {bucket['category']}: {bucket['samples']} ({bucket['percent']:.3f}%)")
    print(output)


if __name__ == '__main__':
    main()
