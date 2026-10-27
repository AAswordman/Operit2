#!/usr/bin/env python3
"""Summarize an xctrace thread-state XML export for one attached process.

Export with:
  xcrun xctrace export --input recording.trace \
    --xpath '/trace-toc/run[@number="1"]/data/table[@schema="thread-state"]' \
    --output thread-state.xml

Scheduling transitions are NOT proc_pid_rusage interrupt-wakeup counters.
The "made runnable by" thread is a scheduling clue, not a complete causal trace.
"""
import argparse
import collections
import json
from pathlib import Path
import xml.etree.ElementTree as ET


def summarize(path):
    root = ET.parse(path).getroot()
    ids = {node.get('id'): node for node in root.iter() if node.get('id')}

    def resolve(node):
        return ids[node.get('ref')] if node.get('ref') else node

    def label(node):
        node = resolve(node)
        return node.get('fmt') or node.text or ''

    def number(node):
        return int(resolve(node).text or 0)

    def thread_name(node):
        return label(node).split(' (0x', 1)[0]

    columns = [node.text for node in root.findall('.//schema/col/mnemonic')]
    required = {'start', 'duration', 'thread', 'state', 'made-runnable-by-thread'}
    if not required.issubset(columns):
        raise ValueError('Expected an xctrace thread-state table')
    by_thread = collections.defaultdict(collections.Counter)
    states = collections.Counter()
    first = None
    last = 0
    for row in root.iter('row'):
        values = dict(zip(columns, row))
        start = number(values['start'])
        duration = number(values['duration'])
        first = start if first is None else min(first, start)
        last = max(last, start + duration)
        state = label(values['state'])
        stats = by_thread[thread_name(values['thread'])]
        states[state] += 1
        if state == 'Running':
            stats['runningNs'] += duration
        if state != 'Runnable':
            continue
        stats['runnableTransitions'] += 1
        source = label(values['made-runnable-by-thread'])
        if '(kernel, pid: 0)' in source:
            stats['kernelCausedRunnableTransitions'] += 1
        if '(securityd, pid:' in source:
            stats['securitydCausedRunnableTransitions'] += 1
    seconds = (last - first) / 1e9 if first is not None else 0
    threads = []
    for name, stats in by_thread.items():
        threads.append({
            'name': name,
            'runningSeconds': stats['runningNs'] / 1e9,
            **{key: stats[key] for key in (
                'runnableTransitions', 'kernelCausedRunnableTransitions',
                'securitydCausedRunnableTransitions')},
        })
    threads.sort(key=lambda stats: stats['runnableTransitions'], reverse=True)
    return {
        'durationSeconds': seconds,
        'notes': [
            'Scheduling transitions differ from proc_pid_rusage interrupt wakeups.',
            'Kernel-caused transitions use the exported made-runnable-by thread.',
            'Attached tracing changes execution; measure idle counters separately.',
        ],
        'states': dict(states),
        'threads': threads,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = summarize(args.input)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    for thread in result['threads'][:8]:
        print(json.dumps(thread))


if __name__ == '__main__':
    main()
