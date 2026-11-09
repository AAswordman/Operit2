#!/usr/bin/env python3
"""Compact sequence-AOT profiles without retaining diagnostic addresses or traces."""
import argparse
import collections
import hashlib
import json
from pathlib import Path

from generate_guest_aot import sequences


def compact(paths):
    training = collections.Counter()
    sources = []
    for path in paths:
        data = path.read_bytes()
        profile = json.loads(data)
        sources.append({'sha256': hashlib.sha256(data).hexdigest(), 'schema': profile.get('schema')})
        for samples, words in sequences(profile):
            training[tuple(words)] += samples
    return {
        'schema': 'operit.guest-aot-fixture.v1',
        'purpose': 'sequence-aot-only; not a control-flow trace profile',
        'sources': sources,
        'blocks': [{'samples': samples, 'words': [hex(word) for word in words]}
                   for words, samples in training.items()],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', action='append', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.resolve() in {path.resolve() for path in args.profile}:
        parser.error('output must not overwrite an input profile')
    result = compact(args.profile)
    args.output.write_text(json.dumps(result, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    main()
