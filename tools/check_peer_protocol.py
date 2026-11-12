#!/usr/bin/env python3
"""Check that published protocol revisions are immutable and changes bump the version.

Cargo's node-runtime build verifies the annotated route contract SHA-256.
This check compares the revision ledger against the PR base to prevent replacing
an old hash without advancing the protocol version.
"""
import argparse
import json
from pathlib import Path
import subprocess

MANIFEST = "core/peer-protocol.json"
ROOT = Path(__file__).resolve().parents[1]


def check_revisions(current, previous=None):
    revisions = current["revisions"]
    last = 0
    hashes = set()
    for revision in revisions:
        version, fingerprint = revision["version"], revision["hash"]
        if (type(version) is not int or not last < version <= 0xFFFFFFFF
                or len(fingerprint) != 64
                or any(c not in "0123456789abcdef" for c in fingerprint)
                or fingerprint in hashes):
            raise ValueError("Protocol revisions require increasing positive u32 versions and unique lowercase SHA-256 hashes")
        last = version
        hashes.add(fingerprint)
    if not revisions:
        raise ValueError("Protocol revisions cannot be empty")
    if previous is not None:
        old = previous["revisions"]
        if revisions[:len(old)] != old:
            raise ValueError("Published protocol revisions are immutable; append a revision with a higher version")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="Git revision before this change (normally the PR base)")
    args = parser.parse_args()
    current = json.loads((ROOT / MANIFEST).read_text())
    previous = None
    if args.base:
        # A missing manifest is allowed only for its initial introduction. A bad
        # Git revision, unavailable shallow history or a failed Git call is fatal.
        subprocess.run(["git", "rev-parse", "--verify", f"{args.base}^{{commit}}"], cwd=ROOT, check=True, capture_output=True)
        paths = subprocess.check_output(["git", "ls-tree", "--name-only", args.base, "--", MANIFEST], cwd=ROOT, text=True)
        if paths.strip():
            previous = json.loads(subprocess.check_output(["git", "show", f"{args.base}:{MANIFEST}"], cwd=ROOT, text=True))
    check_revisions(current, previous)
    print(f"Peer protocol revision ledger OK: v{current['revisions'][-1]['version']}")


if __name__ == "__main__":
    main()
