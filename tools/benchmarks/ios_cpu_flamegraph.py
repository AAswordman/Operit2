#!/usr/bin/env python3
"""Combine simulator Time Profiler XML, Dart CPU samples and process CPU counters.

Uses real running-thread samples only; it never treats blocked stacks as CPU work.
The source recording must match the wall-clock boundaries in the stage report.
"""
import argparse
import collections
import datetime
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET


def clean(name):
    name = re.sub(r'::h[0-9a-f]{16}$', '', name)
    for token, value in {'$LT$': '<', '$GT$': '>', '$C$': ',', '$RF$': '&', '$LP$': '(', '$RP$': ')'}.items():
        name = name.replace(token, value)
    return re.sub(r'\$u([0-9a-f]+)\$', lambda m: chr(int(m[1], 16)), name).replace('..', '::')


def native_samples(path, toc):
    root = ET.parse(path).getroot()
    ids = {n.get('id'): n for n in root.iter() if n.get('id')}
    def resolve(n):
        return ids[n.get('ref')] if n.get('ref') else n
    def number(n):
        return int(resolve(n).text or 0)
    columns = [n.text for n in root.findall('.//schema/col/mnemonic')]
    origin = datetime.datetime.fromisoformat(ET.parse(toc).findtext('.//start-date')).timestamp() * 1e6
    rows = []
    for row in root.iter('row'):
        values = dict(zip(columns, row))
        state = resolve(values['thread-state'])
        if (state.get('fmt') or state.text) != 'Running':
            continue
        thread = resolve(values['thread']).get('fmt', '').split(' (0x')[0]
        frames = []
        for frame in resolve(values['stack']):
            if frame.tag != 'frame':
                continue
            frame = resolve(frame)
            frames.append(clean(frame.get('name') or frame.get('addr', '?')))
        if frames:
            rows.append((origin + number(values['time']) / 1000, number(values['weight']) / 1e6, [thread] + list(reversed(frames))))
    return rows


def dart_samples(path):
    profile = json.loads(path.read_text())
    def describe(index):
        entry = profile['functions'][index]
        fn = entry['function']
        name = fn.get('name', '?')
        owner = fn.get('owner', {})
        if owner.get('type') == '@Class':
            name = owner['name'] + '.' + name
        location = fn.get('location', {})
        uri = location.get('script', {}).get('uri') or entry.get('resolvedUrl', '')
        if '/apps/flutter/app/' in uri:
            uri = uri.split('/apps/flutter/app/', 1)[1]
        elif '/packages/flutter/' in uri:
            uri = 'flutter/' + uri.split('/packages/flutter/', 1)[1]
        return clean(name), uri, location.get('line')
    frames = [describe(i) for i in range(len(profile['functions']))]
    return [(sample['timestamp'], profile['samplePeriod'] / 1000,
             [f"Dart thread {sample['tid']}"] + [frames[i][0] for i in reversed(sample['stack'])]) for sample in profile['samples']], frames, profile


def tree(rows):
    root = {'name': 'All CPU samples', 'value': 0, 'children': {}}
    for _, weight, stack in rows:
        node = root
        node['value'] += weight
        for frame in stack:
            node = node['children'].setdefault(frame, {'name': frame, 'value': 0, 'children': {}})
            node['value'] += weight
    def compact(node):
        node['children'] = [compact(c) for c in sorted(node['children'].values(), key=lambda c: -c['value'])]
        return node
    return compact(root)


def top(rows, count=20):
    inclusive, exclusive, total = collections.Counter(), collections.Counter(), 0
    for _, weight, stack in rows:
        total += weight
        for frame in set(stack):
            inclusive[frame] += weight
        if stack:
            exclusive[stack[-1]] += weight
    def entries(counter):
        return [{'function': f, 'percent': round(v / total * 100, 3)} for f, v in counter.most_common(count)] if total else []
    return {'samples': len(rows), 'inclusive': entries(inclusive), 'self': entries(exclusive)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root, out = args.root, args.output
    out.mkdir(parents=True, exist_ok=True)
    report = json.loads((root / 'current-collector.json').read_text())
    native = native_samples(root / 'stacks-native.xml', root / 'stacks-toc.xml')
    cpu = [json.loads(line) for line in (root / 'current-cpu-samples.jsonl').read_text().splitlines()]
    graph = {'stages': [], 'cpu': [], 'notes': [
        'Simulator, Flutter debug, Apple M5 host. These are not physical iPhone power or temperature measurements.',
        'CPU percent: all application threads; one fully occupied core equals 100%. Peak windows are about 100 ms.',
        'Dart and native percentages have different sample denominators; inclusive frames overlap and must not be added.',
        'Single tool calls are too short for reliable 100 ms CPU or stack percentages. Use the explicitly labeled batch200 stages.',
    ]}
    speedscope = {'$schema': 'https://www.speedscope.app/file-format-schema.json', 'shared': {'frames': []}, 'profiles': [], 'activeProfileIndex': 0}
    frame_ids = {}
    def frame_id(name):
        if name not in frame_ids:
            frame_ids[name] = len(speedscope['shared']['frames'])
            speedscope['shared']['frames'].append({'name': name})
        return frame_ids[name]
    for stage in report['stages']:
        start, end = stage['startWallMicros'], stage['endWallMicros']
        rows = [s for s in native if start <= s[0] <= end]
        overlaps = [(s, max(0, min(s['wallMicros'], end) - max(s['intervalStartMicros'], start))) for s in cpu]
        overlaps = [(s, w) for s, w in overlaps if w]
        # Peaks include only complete sampling windows within the stage.
        complete = [s for s in cpu if start <= s['intervalStartMicros'] < s['wallMicros'] <= end]
        stage['cpu'] = {
            'meanPercent': sum(s['cpuPercent'] * w for s, w in overlaps) / sum(w for _, w in overlaps) if overlaps and end - start >= 100000 else None,
            'peak100msPercent': max((s['cpuPercent'] for s in complete), default=None),
            'nativeSampleCount': len(rows),
        }
        if '-single' in stage['name']:
            stage['cpu']['meanPercent'] = None
        native_top = top(rows)
        thread_weights = collections.Counter()
        for _, weight, stack in rows:
            thread_weights[stack[0]] += weight
        total = sum(thread_weights.values())
        stage['nativeThreads'] = [{'thread': t, 'percent': w / total * 100} for t, w in thread_weights.most_common()] if total else []
        dart, frames, raw = dart_samples(root / f"current-dart-{stage['name'].replace(':', '-')}.json")
        stage['nativeTop'] = native_top
        stage['dartTop'] = top(dart)
        inclusive = collections.Counter()
        for sample in raw['samples']:
            inclusive.update(set(sample['stack']))
        stage['dartSourceHotspots'] = [
            {'function': frames[i][0], 'source': frames[i][1], 'line': frames[i][2], 'percent': n / len(dart) * 100}
            for i, n in inclusive.most_common()
            if frames[i][1].startswith(('package:operit2/', 'lib/'))
        ] if dart else []
        graph['stages'].append({'name': stage['name'], 'cpu': stage['cpu'], 'native': tree(rows), 'dart': tree(dart), 'dartSampleCount': len(dart)})
        for kind, data in [('native', rows), ('dart', dart)]:
            speedscope['profiles'].append({'type': 'sampled', 'name': stage['name'] + ' / ' + kind,
                'unit': 'milliseconds', 'startValue': 0, 'endValue': sum(s[1] for s in data),
                'samples': [[frame_id(f) for f in s[2]] for s in data], 'weights': [s[1] for s in data]})
    first, last = report['stages'][0]['startWallMicros'], report['stages'][-1]['endWallMicros']
    graph['cpu'] = [[(s['wallMicros'] - first) / 1e6, s['cpuPercent']] for s in cpu if first <= s['wallMicros'] <= last]
    graph['bounds'] = [[(s['startWallMicros'] - first) / 1e6, (s['endWallMicros'] - first) / 1e6, s['name']] for s in report['stages']]
    graph['reply'] = report.get('reply')
    (out / 'iphone-cpu-stacks.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    (out / 'iphone-profiles.speedscope.json').write_text(json.dumps(speedscope, ensure_ascii=False, separators=(',', ':')))
    template = Path(__file__).with_suffix('.html').read_text()
    (out / 'iphone-flamegraphs.html').write_text(template.replace('__PROFILE_DATA__', json.dumps(graph, ensure_ascii=False).replace('<', '\\u003c')))
    for stage in report['stages']:
        print(stage['name'], json.dumps(stage['cpu']))


if __name__ == '__main__':
    main()
