#!/usr/bin/env python3
"""Ensures an Alpine runtime root contains the Node.js 24 package."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


def installed_node_version(database: str) -> str | None:
    for record in re.split(r"\n\s*\n", database.strip()):
        fields = {}
        for line in record.splitlines():
            key, separator, value = line.partition(":")
            if separator and key in {"P", "V"}:
                fields.setdefault(key, value)
        if fields.get("P") == "nodejs":
            return fields.get("V")
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("apk_database", type=Path)
    args = parser.parse_args()

    version = installed_node_version(args.apk_database.read_text(encoding="utf-8"))
    if version is None or re.match(r"^24(?:\.|$)", version) is None:
        detected = version or "not installed"
        parser.error(f"Alpine Node.js 24 is required; found {detected}")
    print(f"Verified Alpine Node.js {version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
